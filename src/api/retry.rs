//! Retrying transient failures.

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use super::error::{Result, is_retryable_status};
use super::transport::{TmfRequest, TmfResponse, Transport};

/// Waits for a duration.
///
/// Implement this to drive [`RetryTransport`] from a runtime other than
/// `tokio` — `async-std`, `smol`, a wasm timer, or a fake clock in a test.
pub trait Sleeper: Send + Sync + 'static {
    /// Completes after `duration` has elapsed.
    fn sleep(&self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;
}

/// A [`Sleeper`] backed by `tokio::time`.
#[cfg(feature = "transport-reqwest")]
#[cfg_attr(docsrs, doc(cfg(feature = "transport-reqwest")))]
#[derive(Debug, Clone, Copy, Default)]
pub struct TokioSleeper;

#[cfg(feature = "transport-reqwest")]
impl Sleeper for TokioSleeper {
    fn sleep(&self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(tokio::time::sleep(duration))
    }
}

/// How many times, and how patiently, to retry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct RetryPolicy {
    /// Attempts *after* the first. Zero disables retrying.
    pub max_retries: u32,
    /// Delay before the first retry; doubles each attempt.
    pub base_delay: Duration,
    /// Ceiling on the computed delay.
    pub max_delay: Duration,
    /// Whether to obey a `Retry-After` header when the server sends one.
    pub honor_retry_after: bool,
    /// The longest `Retry-After` this client will wait out.
    ///
    /// Separate from [`max_delay`](Self::max_delay), which bounds the
    /// *computed* backoff. Clamping the server's own instruction to that bound
    /// would be the worst of both: a gateway that says "wait 60 seconds" would
    /// be re-asked after ten, still rate-limited, burning the whole retry
    /// budget without ever waiting long enough to succeed. So a `Retry-After`
    /// longer than this ends the retries and returns the response instead —
    /// the caller learns it was rate-limited and for how long, which is
    /// something it can act on, rather than a retry storm it cannot see.
    pub max_retry_after: Duration,
    /// Whether to spread retries randomly over the backoff window.
    ///
    /// On by default. Without it, every client that saw the same outage retries
    /// in the same instant and the recovering server is knocked over again.
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_millis(250),
            max_delay: Duration::from_secs(10),
            honor_retry_after: true,
            max_retry_after: Duration::from_secs(60),
            jitter: true,
        }
    }
}

impl RetryPolicy {
    /// Sets the number of retries after the first attempt.
    #[must_use]
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Sets the delay before the first retry.
    #[must_use]
    pub fn base_delay(mut self, delay: Duration) -> Self {
        self.base_delay = delay;
        self
    }

    /// Sets the ceiling on the computed delay.
    #[must_use]
    pub fn max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    /// Sets whether `Retry-After` overrides the computed backoff.
    #[must_use]
    pub fn honor_retry_after(mut self, honor: bool) -> Self {
        self.honor_retry_after = honor;
        self
    }

    /// Sets the longest `Retry-After` this client will wait out.
    ///
    /// A server asking for longer ends the retries; see
    /// [`max_retry_after`](Self::max_retry_after).
    #[must_use]
    pub fn max_retry_after(mut self, limit: Duration) -> Self {
        self.max_retry_after = limit;
        self
    }

    /// Sets whether to spread retries over the backoff window.
    #[must_use]
    pub fn jitter(mut self, jitter: bool) -> Self {
        self.jitter = jitter;
        self
    }

    /// The backoff ceiling for attempt `attempt` (1-based), capped at
    /// `max_delay`.
    ///
    /// This is the *window*, not the wait: with jitter on, the actual wait is
    /// drawn from `[0, window]`. That is the "full jitter" strategy, which
    /// spreads a thundering herd better than adding a small random offset to a
    /// fixed delay.
    #[must_use]
    pub fn window_for(&self, attempt: u32) -> Duration {
        let factor = 2u32.saturating_pow(attempt.saturating_sub(1));
        self.base_delay.saturating_mul(factor).min(self.max_delay)
    }

    /// The delay before attempt `attempt`, with jitter applied if enabled.
    #[must_use]
    pub fn delay_for(&self, attempt: u32) -> Duration {
        let window = self.window_for(attempt);
        if !self.jitter {
            return window;
        }
        // Scaled in `u128` so the arithmetic is exact for any representable
        // window — no float rounding to reason about in a timing path.
        let scaled = window.as_nanos() * u128::from(jitter_permille()) / 1_000;
        Duration::from_nanos(u64::try_from(scaled).unwrap_or(u64::MAX))
    }
}

/// A pseudo-random value in `0..1000`, derived from the clock.
///
/// Retry scheduling wants *spread*, not unpredictability, so a hashed clock
/// reading is ample — and it keeps `rand` out of the dependency graph of a
/// crate whose point is being cheap to depend on.
fn jitter_permille() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    // A multiplicative hash decorrelates the low bits, which on some platforms
    // advance in coarse steps.
    let mixed = u64::from(nanos).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 11;
    u32::try_from(mixed % 1_000).unwrap_or(0)
}

/// A [`Transport`] that retries transient failures.
///
/// API gateways in front of TM Forum deployments rate-limit and occasionally
/// drop connections. `RetryTransport` wraps any [`Transport`] and re-issues a
/// request when the failure is one worth retrying, backing off exponentially
/// with jitter and honouring `Retry-After`.
///
/// ```no_run
/// use std::time::Duration;
/// use rutmf::api::{RetryPolicy, RetryTransport, Transport};
///
/// # fn demo(inner: impl Transport + 'static) {
/// let transport = RetryTransport::new(
///     inner,
///     RetryPolicy::default().max_retries(5).base_delay(Duration::from_millis(200)),
/// );
/// # }
/// ```
///
/// # Which requests are retried
///
/// Only idempotent ones: `GET`, `HEAD`, `PUT` and `DELETE`. `POST` is not
/// idempotent — re-sending one could create a duplicate resource — so it is
/// passed through untouched even when the failure looks transient.
///
/// `PATCH` is not retried either, and that is deliberate rather than an
/// oversight. Three of the four v5 `PATCH` flavours are merges, which happen to
/// be idempotent; the RFC 6902 operation list is not, because `add` on an array
/// *inserts*, so replaying one appends a second copy. The method does not say
/// which flavour it is carrying, so it is treated as the unsafe case.
///
/// # Sleeping
///
/// Backing off requires a timer, and the domain model deliberately has no
/// runtime dependency. So the wait is a seam: [`Sleeper`] is a trait, the
/// `transport-reqwest` feature supplies a `tokio` implementation as the
/// default, and [`RetryTransport::with_sleeper`] takes any other.
///
/// Without a `Sleeper` there is no backoff, only immediate re-sending — which
/// against a rate-limited gateway is worse than not retrying at all. That is
/// why [`RetryTransport::new`] exists only when a default sleeper does.
pub struct RetryTransport<T> {
    inner: T,
    policy: RetryPolicy,
    sleeper: Box<dyn Sleeper>,
}

impl<T: std::fmt::Debug> std::fmt::Debug for RetryTransport<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RetryTransport")
            .field("inner", &self.inner)
            .field("policy", &self.policy)
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "transport-reqwest")]
impl<T> RetryTransport<T> {
    /// Wraps `inner` with `policy`, sleeping on `tokio`.
    #[cfg_attr(docsrs, doc(cfg(feature = "transport-reqwest")))]
    pub fn new(inner: T, policy: RetryPolicy) -> Self {
        Self::with_sleeper(inner, policy, TokioSleeper)
    }

    /// Wraps `inner` with the default policy, sleeping on `tokio`.
    #[cfg_attr(docsrs, doc(cfg(feature = "transport-reqwest")))]
    pub fn with_defaults(inner: T) -> Self {
        Self::new(inner, RetryPolicy::default())
    }
}

impl<T> RetryTransport<T> {
    /// Wraps `inner` with `policy`, waiting through `sleeper`.
    ///
    /// Use this to drive backoff from a runtime other than `tokio`, or from a
    /// fake clock in a test.
    pub fn with_sleeper(inner: T, policy: RetryPolicy, sleeper: impl Sleeper) -> Self {
        Self {
            inner,
            policy,
            sleeper: Box::new(sleeper),
        }
    }

    /// The wrapped transport.
    pub fn inner(&self) -> &T {
        &self.inner
    }
}

/// Whether re-sending this request could change the outcome on the server.
fn is_idempotent(request: &TmfRequest) -> bool {
    use http::Method;
    matches!(
        request.method,
        Method::GET | Method::HEAD | Method::PUT | Method::DELETE
    )
}

/// The server's requested wait, when it sent one.
///
/// RFC 9110 permits either a delay in seconds or an HTTP date. Both appear in
/// the wild, so both are read; an HTTP date already in the past means "now".
fn retry_after(response: &TmfResponse) -> Option<Duration> {
    let raw = response
        .headers
        .get(http::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim();

    if let Ok(seconds) = raw.parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }

    // An HTTP date: measure from `Date` if the server sent one, so clock skew
    // between client and server does not turn a 5-second wait into an hour.
    let until = httpdate(raw)?;
    let now = response
        .headers
        .get(http::header::DATE)
        .and_then(|v| v.to_str().ok())
        .and_then(httpdate)
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs())
        });
    Some(Duration::from_secs(until.saturating_sub(now)))
}

/// Parses an IMF-fixdate (`Sun, 06 Nov 1994 08:49:37 GMT`) to a Unix timestamp.
///
/// The one format RFC 9110 requires a server to send. Obsolete formats are not
/// accepted; falling back to the computed backoff is the right answer for them.
///
/// The arithmetic is signed because the input is a header: a server with an
/// unset clock sends a date before 1970, which the epoch conversion goes
/// negative on. In `u64` that underflows — a panic in debug, and in release a
/// wrap to a wait no policy will hold a request open for. The result is clamped
/// at the epoch, so an elapsed wait is no wait.
fn httpdate(raw: &str) -> Option<u64> {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    let raw = raw.strip_suffix(" GMT")?;
    let (_weekday, rest) = raw.split_once(", ")?;
    let mut parts = rest.split(' ');
    let day: i64 = parts.next()?.parse().ok()?;
    let month_name = parts.next()?;
    let month = i64::try_from(MONTHS.iter().position(|m| *m == month_name)?).ok()? + 1;
    let year: i64 = parts.next()?.parse().ok()?;
    let mut clock = parts.next()?.split(':');
    let hour: i64 = clock.next()?.parse().ok()?;
    let minute: i64 = clock.next()?.parse().ok()?;
    let second: i64 = clock.next()?.parse().ok()?;

    // Days from the civil epoch — Howard Hinnant's `days_from_civil`, whose
    // published form is signed for exactly this reason.
    let (y, m) = if month <= 2 {
        (year - 1, month + 9)
    } else {
        (year, month - 3)
    };
    let era = y.div_euclid(400);
    let year_of_era = y - era * 400;
    let day_of_year = (153 * m + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;

    let seconds = days
        .checked_mul(86_400)?
        .checked_add(hour * 3_600 + minute * 60 + second)?;
    // Before the epoch is "already past", which is the same answer a
    // `Retry-After` in the past gets.
    Some(u64::try_from(seconds).unwrap_or(0))
}

#[async_trait::async_trait]
impl<T: Transport> Transport for RetryTransport<T> {
    async fn execute(&self, request: TmfRequest) -> Result<TmfResponse> {
        if self.policy.max_retries == 0 || !is_idempotent(&request) {
            return self.inner.execute(request).await;
        }

        let mut attempt = 0;
        loop {
            let outcome = self.inner.execute(request.clone()).await;
            attempt += 1;

            let wait = match &outcome {
                // The transport reports connection-level failures as errors.
                Err(error) if error.is_retryable() => self.policy.delay_for(attempt),
                // A retryable status arrives as a successful transport call;
                // the client layer is what turns it into an error later.
                Ok(response) if is_retryable_status(response.status) => {
                    let backoff = self.policy.delay_for(attempt);
                    match retry_after(response).filter(|_| self.policy.honor_retry_after) {
                        // The server named a wait longer than this client is
                        // willing to hold a request open for. Retrying sooner
                        // would be ignoring the instruction while pretending to
                        // honour it, so hand the response back instead.
                        Some(wait) if wait > self.policy.max_retry_after => return outcome,
                        // An explicit instruction outranks the computed
                        // backoff, in both directions: a server that says "one
                        // second" knows better than the exponential curve.
                        Some(wait) => wait,
                        None => backoff,
                    }
                }
                _ => return outcome,
            };

            if attempt > self.policy.max_retries {
                return outcome;
            }
            self.sleeper.sleep(wait).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use bytes::Bytes;
    use http::{HeaderMap, HeaderValue, Method, StatusCode};

    use super::*;
    use crate::api::{Error, TransportError};

    /// Records what it was asked to wait for, and returns immediately.
    #[derive(Clone, Default)]
    struct RecordingSleeper(Arc<Mutex<Vec<Duration>>>);

    impl Sleeper for RecordingSleeper {
        fn sleep(&self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
            self.0.lock().expect("sleeper poisoned").push(duration);
            Box::pin(std::future::ready(()))
        }
    }

    /// A transport that fails a fixed number of times, then succeeds.
    struct Flaky {
        calls: Arc<AtomicUsize>,
        fail_times: usize,
        status: StatusCode,
        retry_after: Option<&'static str>,
    }

    #[async_trait::async_trait]
    impl Transport for Flaky {
        async fn execute(&self, _: TmfRequest) -> Result<TmfResponse> {
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            if n < self.fail_times {
                let mut headers = HeaderMap::new();
                if let Some(value) = self.retry_after {
                    headers.insert(http::header::RETRY_AFTER, HeaderValue::from_static(value));
                }
                return Ok(TmfResponse::new(self.status, headers, Bytes::new()));
            }
            Ok(TmfResponse::new(
                StatusCode::OK,
                HeaderMap::new(),
                Bytes::from_static(b"[]"),
            ))
        }
    }

    fn flaky(fail_times: usize, status: StatusCode) -> (Arc<AtomicUsize>, Flaky) {
        let calls = Arc::new(AtomicUsize::new(0));
        (
            calls.clone(),
            Flaky {
                calls,
                fail_times,
                status,
                retry_after: None,
            },
        )
    }

    fn policy() -> RetryPolicy {
        RetryPolicy::default()
            .base_delay(Duration::from_millis(100))
            .jitter(false)
    }

    fn wrap<T: Transport>(inner: T, policy: RetryPolicy) -> (RecordingSleeper, RetryTransport<T>) {
        let sleeper = RecordingSleeper::default();
        (
            sleeper.clone(),
            RetryTransport::with_sleeper(inner, policy, sleeper),
        )
    }

    #[tokio::test]
    async fn retries_until_success() {
        let (calls, inner) = flaky(2, StatusCode::SERVICE_UNAVAILABLE);
        let (sleeper, transport) = wrap(inner, policy());

        let response = transport
            .execute(TmfRequest::new(Method::GET, "http://x/productOffering"))
            .await
            .unwrap();

        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(
            calls.load(Ordering::SeqCst),
            3,
            "two failures then a success"
        );
        assert_eq!(
            *sleeper.0.lock().unwrap(),
            [Duration::from_millis(100), Duration::from_millis(200)],
            "the backoff must actually be waited out, not skipped"
        );
    }

    #[tokio::test]
    async fn gives_up_after_max_retries() {
        let (calls, inner) = flaky(99, StatusCode::BAD_GATEWAY);
        let (_, transport) = wrap(inner, policy().max_retries(2));

        let response = transport
            .execute(TmfRequest::new(Method::GET, "http://x/productOffering"))
            .await
            .unwrap();

        assert_eq!(response.status, StatusCode::BAD_GATEWAY);
        assert_eq!(
            calls.load(Ordering::SeqCst),
            3,
            "one attempt plus two retries"
        );
    }

    #[tokio::test]
    async fn does_not_retry_a_post() {
        let (calls, inner) = flaky(99, StatusCode::SERVICE_UNAVAILABLE);
        let (_, transport) = wrap(inner, policy());

        let _ = transport
            .execute(TmfRequest::new(Method::POST, "http://x/productOffering"))
            .await;

        assert_eq!(calls.load(Ordering::SeqCst), 1, "POST is not idempotent");
    }

    #[tokio::test]
    async fn does_not_retry_a_client_error() {
        let (calls, inner) = flaky(99, StatusCode::NOT_FOUND);
        let (_, transport) = wrap(inner, policy());

        let _ = transport
            .execute(TmfRequest::new(Method::GET, "http://x/a/1"))
            .await;

        assert_eq!(calls.load(Ordering::SeqCst), 1, "404 is permanent");
    }

    #[tokio::test]
    async fn honours_retry_after_in_seconds() {
        let calls = Arc::new(AtomicUsize::new(0));
        let inner = Flaky {
            calls,
            fail_times: 1,
            status: StatusCode::TOO_MANY_REQUESTS,
            retry_after: Some("3"),
        };
        let (sleeper, transport) = wrap(inner, policy());

        transport
            .execute(TmfRequest::new(Method::GET, "http://x/a"))
            .await
            .unwrap();

        assert_eq!(*sleeper.0.lock().unwrap(), [Duration::from_secs(3)]);
    }

    #[tokio::test]
    async fn propagates_a_transport_error_after_retrying() {
        struct AlwaysBroken(Arc<AtomicUsize>);

        #[async_trait::async_trait]
        impl Transport for AlwaysBroken {
            async fn execute(&self, _: TmfRequest) -> Result<TmfResponse> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Err(Error::Transport(TransportError::new("connection reset")))
            }
        }

        let calls = Arc::new(AtomicUsize::new(0));
        let (_, transport) = wrap(AlwaysBroken(calls.clone()), policy().max_retries(2));

        let error = transport
            .execute(TmfRequest::new(Method::GET, "http://x/a/1"))
            .await
            .unwrap_err();

        assert!(matches!(error, Error::Transport(_)));
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn a_long_retry_after_ends_the_retries_rather_than_shortening_the_wait() {
        // Clamping the server's instruction to `max_delay` would re-ask a
        // rate-limited gateway after 10s when it said 300s — spending every
        // retry while the limit is still in force, and calling that "honouring
        // Retry-After".
        let calls = Arc::new(AtomicUsize::new(0));
        let inner = Flaky {
            calls: calls.clone(),
            fail_times: 99,
            status: StatusCode::TOO_MANY_REQUESTS,
            retry_after: Some("300"),
        };
        let (sleeper, transport) = wrap(inner, policy().max_retry_after(Duration::from_secs(60)));

        let response = transport
            .execute(TmfRequest::new(Method::GET, "http://x/a"))
            .await
            .unwrap();

        assert_eq!(response.status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(calls.load(Ordering::SeqCst), 1, "no retry was attempted");
        assert!(
            sleeper.0.lock().unwrap().is_empty(),
            "nothing should have been waited out"
        );
    }

    #[tokio::test]
    async fn a_retry_after_within_the_limit_is_waited_out_in_full() {
        // 30s exceeds the 10s `max_delay` that bounds the *computed* backoff,
        // and must still be honoured: `max_delay` is not a cap on the server.
        let calls = Arc::new(AtomicUsize::new(0));
        let inner = Flaky {
            calls,
            fail_times: 1,
            status: StatusCode::SERVICE_UNAVAILABLE,
            retry_after: Some("30"),
        };
        let (sleeper, transport) = wrap(inner, policy());

        transport
            .execute(TmfRequest::new(Method::GET, "http://x/a"))
            .await
            .unwrap();

        assert_eq!(*sleeper.0.lock().unwrap(), [Duration::from_secs(30)]);
    }

    #[test]
    fn backoff_doubles_and_then_caps() {
        let policy = RetryPolicy::default()
            .base_delay(Duration::from_millis(100))
            .max_delay(Duration::from_millis(350))
            .jitter(false);

        assert_eq!(policy.delay_for(1), Duration::from_millis(100));
        assert_eq!(policy.delay_for(2), Duration::from_millis(200));
        assert_eq!(policy.delay_for(3), Duration::from_millis(350), "capped");
        assert_eq!(
            policy.delay_for(30),
            Duration::from_millis(350),
            "no overflow"
        );
    }

    #[test]
    fn jitter_stays_inside_the_backoff_window() {
        let policy = RetryPolicy::default().base_delay(Duration::from_millis(100));
        for attempt in 1..6 {
            let window = policy.window_for(attempt);
            for _ in 0..50 {
                assert!(policy.delay_for(attempt) <= window);
            }
        }
    }

    #[test]
    fn retry_after_reads_an_http_date_relative_to_the_server_clock() {
        let mut headers = HeaderMap::new();
        headers.insert(
            http::header::RETRY_AFTER,
            HeaderValue::from_static("Sun, 06 Nov 1994 08:49:47 GMT"),
        );
        headers.insert(
            http::header::DATE,
            HeaderValue::from_static("Sun, 06 Nov 1994 08:49:37 GMT"),
        );
        let response = TmfResponse::new(StatusCode::SERVICE_UNAVAILABLE, headers, Bytes::new());
        assert_eq!(retry_after(&response), Some(Duration::from_secs(10)));
    }

    #[test]
    fn a_past_retry_after_date_means_now() {
        let mut headers = HeaderMap::new();
        headers.insert(
            http::header::RETRY_AFTER,
            HeaderValue::from_static("Sun, 06 Nov 1994 08:49:37 GMT"),
        );
        headers.insert(
            http::header::DATE,
            HeaderValue::from_static("Sun, 06 Nov 1994 08:49:47 GMT"),
        );
        let response = TmfResponse::new(StatusCode::SERVICE_UNAVAILABLE, headers, Bytes::new());
        assert_eq!(retry_after(&response), Some(Duration::ZERO));
    }

    #[test]
    fn the_epoch_converts_exactly() {
        assert_eq!(httpdate("Thu, 01 Jan 1970 00:00:00 GMT"), Some(0));
        assert_eq!(httpdate("Sun, 06 Nov 1994 08:49:37 GMT"), Some(784_111_777));
        assert_eq!(httpdate("not a date"), None);
    }

    #[test]
    fn a_date_before_the_epoch_is_the_epoch_rather_than_an_underflow() {
        // A server with an unset clock sends one of these. In `u64` the civil
        // conversion underflows: a panic in a debug build, and in release a wrap
        // to a `Retry-After` of ~5.8e11 years, which no policy waits out — so
        // the retry is abandoned instead of taken.
        for raw in [
            "Mon, 01 Jan 1900 00:00:00 GMT",
            "Wed, 31 Dec 1969 23:59:59 GMT",
            "Fri, 01 Jan 0001 00:00:00 GMT",
        ] {
            assert_eq!(httpdate(raw), Some(0), "{raw}");
        }

        // End to end: such a header must read as "no wait", not as "wait
        // forever", so the request is still retried.
        let mut headers = HeaderMap::new();
        headers.insert(
            http::header::RETRY_AFTER,
            HeaderValue::from_static("Mon, 01 Jan 1900 00:00:00 GMT"),
        );
        let response = TmfResponse::new(StatusCode::SERVICE_UNAVAILABLE, headers, Bytes::new());
        assert_eq!(retry_after(&response), Some(Duration::ZERO));
    }
}
