//! Pagination.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;

use super::error::Result;
use super::query::Query;

/// One page of a collection, with whatever the server said about the rest.
///
/// TM Forum paginates with `offset`/`limit` query parameters. How a server
/// tells you there is *more* is where deployments differ, and this captures
/// three answers:
///
/// - `X-Total-Count` — how many resources match overall. The v5 documents
///   declare this (and `X-Result-Count`) on every collection response; they are
///   the only two response headers the fourteen vendored specifications declare
///   at all.
/// - `206 Partial Content` — TMF630's own signal that the response is a slice
///   of a larger match. A server may omit the counters (computing a total can be
///   expensive) and still answer `206`, which says "there is more" without
///   saying how much.
/// - `Link: <…>; rel="next"` (RFC 8288) — where the next page is. **Not** in
///   the v5 documents: this is an accommodation for real deployments, where API
///   gateways in front of a TMF service commonly add it and page by an opaque
///   cursor rather than an index.
/// - none of them, in which case a short page is the only signal.
///
/// [`Page::has_more`] applies them in that order of reliability, so a server
/// that omits the counters still paginates correctly. [`paginate`] turns
/// repeated calls into a single [`Stream`](futures_core::Stream).
///
/// A `200` is deliberately *not* read as "there is no more": plenty of
/// deployments answer `200` to everything.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Page<T> {
    /// The resources in this page.
    pub items: Vec<T>,
    /// `X-Total-Count`: how many resources match the query overall.
    ///
    /// `None` when the server omits the header, which conformant servers may do
    /// if computing the total is expensive.
    pub total_count: Option<usize>,
    /// `X-Result-Count`: how many resources this page carries.
    pub result_count: Option<usize>,
    /// The URL of the next page, from a `Link: …; rel="next"` header.
    ///
    /// Servers that cannot afford a total count often send this instead.
    pub next_link: Option<String>,
    /// Whether the server answered `206 Partial Content`.
    ///
    /// TMF630 marks a slice of a larger match this way, so `true` says more
    /// resources exist. `false` says nothing: `200` means "complete collection"
    /// to TMF630 and "the default status" to most deployments.
    pub partial: bool,
    /// The offset this page was requested at.
    pub offset: usize,
}

impl<T> Page<T> {
    /// A page of `items` with no pagination signals.
    ///
    /// The clients build pages from a response; this is for driving
    /// [`paginate`] over a collection of your own — a cache, a merged view,
    /// or a hand-written transport in a test.
    #[must_use]
    pub fn new(items: Vec<T>) -> Self {
        Self {
            result_count: Some(items.len()),
            items,
            total_count: None,
            next_link: None,
            partial: false,
            offset: 0,
        }
    }

    /// Marks the page as a `206`: a slice of a larger match.
    #[must_use]
    pub fn with_partial(mut self, partial: bool) -> Self {
        self.partial = partial;
        self
    }

    /// Sets `X-Total-Count`: how many resources match overall.
    #[must_use]
    pub fn with_total_count(mut self, total: usize) -> Self {
        self.total_count = Some(total);
        self
    }

    /// Sets the URL of the next page.
    #[must_use]
    pub fn with_next_link(mut self, link: impl Into<String>) -> Self {
        self.next_link = Some(link.into());
        self
    }

    /// Sets the offset this page was requested at.
    #[must_use]
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    /// Whether the page carries no resources.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Number of resources in this page.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// The offset that would fetch the next page.
    #[must_use]
    pub fn next_offset(&self) -> usize {
        self.offset + self.items.len()
    }

    /// Whether more resources are known to follow.
    ///
    /// Prefers `X-Total-Count`, which answers exactly; then the `206` TMF630
    /// uses to mark a partial collection; then a `rel="next"` link; and falls
    /// back to "the page came back full".
    ///
    /// The `206` is what keeps a server that omits the counters *and* caps the
    /// page size from truncating the stream: its short page would otherwise read
    /// as the end of the collection.
    ///
    /// An empty page always ends the sequence: a server that keeps returning
    /// nothing must not be polled forever.
    #[must_use]
    pub fn has_more(&self, page_size: usize) -> bool {
        if self.items.is_empty() {
            return false;
        }
        if let Some(total) = self.total_count {
            return self.next_offset() < total;
        }
        if self.partial || self.next_link.is_some() {
            return true;
        }
        self.items.len() >= page_size
    }
}

impl<T> IntoIterator for Page<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

/// Extracts the `rel="next"` target from an RFC 8288 `Link` header.
///
/// Returns `None` for a header with no `next` relation, which is what the last
/// page of a collection sends.
///
/// # Why this is a character parser
///
/// The obvious implementation splits the header on `,` and each link on `;`.
/// Both delimiters occur *inside* a link: a URI may contain either, and a
/// parameter value may quote both. TMF630 filters are comma-separated value
/// lists, so a server echoing the client's own filter into the next-page link
/// — `<…?state=held,pending&offset=40>; rel="next"` — produces exactly the
/// header naive splitting mis-reads. The failure is silent: the link is
/// dropped, the stream ends early, and the caller sees a short result rather
/// than an error. So this follows RFC 8288 §B.2: take the URI between angle
/// brackets whole, then read parameters, honouring quoted strings.
#[must_use]
pub fn next_link(header: &str) -> Option<String> {
    let bytes = header.as_bytes();
    let mut at = 0;

    while at < bytes.len() {
        // A link value begins at the next '<'; anything before it is the
        // separator from the previous one.
        let open = at + bytes[at..].iter().position(|b| *b == b'<')?;
        let close = open + 1 + bytes[open + 1..].iter().position(|b| *b == b'>')?;
        let url = &header[open + 1..close];

        // Parameters run to the comma that ends this link value — the one at
        // the top level, not one inside a quoted parameter value.
        let (params, next) = split_link_value(header, close + 1);
        // `<>; rel="next"` is well-formed and names nowhere. Returning it would
        // have the stream fetch the empty URL and fail with a cross-origin
        // error, reporting a refusal where the server simply sent nothing.
        if !url.is_empty() && params_name_the_next_relation(params) {
            return Some(url.to_owned());
        }
        at = next;
    }
    None
}

/// Splits the parameter section of a link value from the rest of the header.
///
/// Returns the parameters and the offset the next link value starts at,
/// treating a comma inside a quoted string as literal.
fn split_link_value(header: &str, from: usize) -> (&str, usize) {
    let bytes = header.as_bytes();
    let mut quoted = false;
    let mut at = from;

    while at < bytes.len() {
        match bytes[at] {
            b'\\' if quoted => at += 1,
            b'"' => quoted = !quoted,
            b',' if !quoted => return (&header[from..at], at + 1),
            _ => {}
        }
        at += 1;
    }
    (&header[from..], bytes.len())
}

/// Whether a link value's parameters declare the `next` relation.
///
/// RFC 8288 §3.3 lets `rel` carry a space-separated list of relation types, so
/// `rel="next last"` names `next` among others.
fn params_name_the_next_relation(params: &str) -> bool {
    split_parameters(params).any(|param| {
        let Some((name, value)) = param.split_once('=') else {
            return false;
        };
        name.trim().eq_ignore_ascii_case("rel")
            && value
                .trim()
                .trim_matches('"')
                .split_whitespace()
                .any(|relation| relation.eq_ignore_ascii_case("next"))
    })
}

/// Splits a link value's parameters on `;`, ignoring separators inside quotes.
fn split_parameters(params: &str) -> impl Iterator<Item = &str> {
    let mut quoted = false;
    let mut start = 0;
    let mut out = Vec::new();
    for (at, byte) in params.bytes().enumerate() {
        match byte {
            b'"' => quoted = !quoted,
            b';' if !quoted => {
                out.push(&params[start..at]);
                start = at + 1;
            }
            _ => {}
        }
    }
    out.push(&params[start..]);
    out.into_iter().map(str::trim).filter(|p| !p.is_empty())
}

/// Where the next page is: an advanced query, or a URL the server handed over.
///
/// A server that pages by cursor puts the continuation in a
/// `Link: …; rel="next"` header, and its cursor is opaque — re-deriving a
/// request from `offset` would fetch the same page forever. So the stream
/// follows the link when there is one, and advances `offset` when there is not.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PageRequest {
    /// Fetch by `offset`/`limit` against the collection.
    Query(Query),
    /// Fetch the absolute URL the previous page's `Link` header named.
    Follow(String),
}

/// Streams every resource matching `query`, fetching pages as needed.
///
/// `fetch` is called with a [`PageRequest`]: either a query whose `offset` has
/// been advanced, or the URL the last page's `rel="next"` link pointed at. The
/// stream ends when a page comes back short, the total is reached, or the
/// server stops offering a next link. It terminates on the first error.
///
/// The stream sets an explicit `limit` on every offset request — the one from
/// `query` if you gave one, otherwise [`DEFAULT_PAGE_SIZE`] — because a short
/// page is the fallback end-of-collection signal, and that inference is only
/// sound if the client chose the page size.
///
/// ```no_run
/// # async fn demo(client: rutmf::api::tmf620::ProductCatalogClient) -> rutmf::api::Result<()> {
/// use futures::StreamExt;
/// use rutmf::api::Query;
///
/// let mut offerings = client.stream_product_offerings(Query::new().limit(50));
/// while let Some(offering) = offerings.next().await {
///     println!("{:?}", offering?.name);
/// }
/// # Ok(())
/// # }
/// ```
pub fn paginate<T, F, Fut>(query: Query, fetch: F) -> PageStream<T>
where
    F: Fn(PageRequest) -> Fut + Send + 'static,
    Fut: Future<Output = Result<Page<T>>> + Send + 'static,
    T: Send + 'static,
{
    let page_size = query.limit_value().unwrap_or(DEFAULT_PAGE_SIZE);
    let start = query.offset_value().unwrap_or(0);

    PageStream {
        state: State::Idle(Next::Offset(start)),
        buffer: Vec::new().into_iter(),
        page_size,
        query,
        fetch: Box::new(move |request| Box::pin(fetch(request))),
        visited: std::collections::HashSet::new(),
        following: false,
        done: false,
    }
}

/// The page size [`paginate`] requests when the query names none.
///
/// The value matters only for the short-page heuristic: asking for a specific
/// number is what makes "fewer came back" mean "that was the last page".
pub const DEFAULT_PAGE_SIZE: usize = 20;

type BoxFuture<T> = Pin<Box<dyn Future<Output = Result<Page<T>>> + Send>>;
type FetchFn<T> = Box<dyn Fn(PageRequest) -> BoxFuture<T> + Send>;

/// How the next page will be reached.
enum Next {
    Offset(usize),
    Link(String),
}

enum State<T> {
    Idle(Next),
    Fetching(BoxFuture<T>),
}

/// A [`Stream`] over every resource matching a query. Built by [`paginate`].
///
/// [`Stream`]: futures_core::Stream
pub struct PageStream<T> {
    state: State<T>,
    buffer: std::vec::IntoIter<T>,
    page_size: usize,
    query: Query,
    fetch: FetchFn<T>,
    /// Links already followed. A server that keeps naming a page it has already
    /// served would otherwise stream forever.
    ///
    /// A set rather than a list: this is consulted once per page against every
    /// link seen so far, which over a long collection is the difference between
    /// linear and quadratic work in the number of pages.
    visited: std::collections::HashSet<String>,
    /// Whether the server is leading with `Link` headers. Once it is, the
    /// absence of one ends the collection — falling back to `offset` would be
    /// guessing at a cursor.
    following: bool,
    done: bool,
}

impl<T> std::fmt::Debug for PageStream<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PageStream")
            .field("page_size", &self.page_size)
            .field("done", &self.done)
            .finish_non_exhaustive()
    }
}

impl<T: Unpin> Stream for PageStream<T> {
    type Item = Result<T>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        loop {
            if let Some(item) = this.buffer.next() {
                return Poll::Ready(Some(Ok(item)));
            }
            if this.done {
                return Poll::Ready(None);
            }

            match &mut this.state {
                State::Idle(Next::Offset(offset)) => {
                    let query = this.query.clone().offset(*offset).limit(this.page_size);
                    this.state = State::Fetching((this.fetch)(PageRequest::Query(query)));
                }
                State::Idle(Next::Link(url)) => {
                    let url = url.clone();
                    this.state = State::Fetching((this.fetch)(PageRequest::Follow(url)));
                }
                State::Fetching(future) => match future.as_mut().poll(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(Err(error)) => {
                        this.done = true;
                        return Poll::Ready(Some(Err(error)));
                    }
                    Poll::Ready(Ok(page)) => {
                        this.state = State::Idle(match &page.next_link {
                            // An empty page ends the sequence whatever the
                            // headers say — the rule `Page::has_more` states,
                            // applied on this path too. Without it a server
                            // that keeps naming a *fresh* next page and serving
                            // nothing streams forever, and `visited` grows
                            // without bound while it does.
                            Some(_) if page.items.is_empty() => {
                                this.done = true;
                                Next::Offset(page.next_offset())
                            }
                            // The link is authoritative: a cursor is opaque, so
                            // re-deriving an offset request would loop.
                            Some(url) if !this.visited.contains(url) => {
                                this.visited.insert(url.clone());
                                Next::Link(url.clone())
                            }
                            // A server that keeps naming a page it already
                            // served would otherwise stream forever.
                            Some(_) => {
                                this.done = true;
                                Next::Offset(page.next_offset())
                            }
                            // Once the server is leading, no link means no next
                            // page — its `offset` is not a cursor we may guess.
                            None if this.following => {
                                this.done = true;
                                Next::Offset(page.next_offset())
                            }
                            None => {
                                if !page.has_more(this.page_size) {
                                    this.done = true;
                                }
                                Next::Offset(page.next_offset())
                            }
                        });
                        this.following = matches!(this.state, State::Idle(Next::Link(_)));
                        this.buffer = page.items.into_iter();
                    }
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(items: Vec<u8>, total: Option<usize>, offset: usize) -> Page<u8> {
        let page = Page::new(items).with_offset(offset);
        match total {
            Some(total) => page.with_total_count(total),
            None => page,
        }
    }

    #[test]
    fn has_more_prefers_the_total_count_header() {
        assert!(page(vec![1, 2], Some(5), 0).has_more(2));
        assert!(!page(vec![1, 2], Some(2), 0).has_more(2));
    }

    #[test]
    fn has_more_uses_a_next_link_when_there_is_no_total() {
        // A short page with a `rel="next"` link still has more: some servers
        // page by cursor and cannot afford a count.
        let mut short = page(vec![1], None, 0);
        assert!(!short.has_more(2), "no link, short page: done");
        short.next_link = Some("https://host/productOffering?offset=1".into());
        assert!(short.has_more(2), "the server said there is a next page");
    }

    #[test]
    fn a_206_says_there_is_more_when_no_count_does() {
        // A server may omit the counters — computing a total can be expensive —
        // and cap the page below what was asked for. The result is a *short*
        // page with more to come, which the fallback heuristic reads as the end
        // of the collection. `206` is the signal that tells the two apart, and
        // it is the one TMF630 specifies.
        let capped = page(vec![1, 2], None, 0).with_partial(true);
        assert!(
            capped.has_more(50),
            "a short page marked 206 is not the end of the collection"
        );

        // A `200` carries no information either way: TMF630 reserves it for a
        // complete collection, and most deployments answer it to everything.
        assert!(!page(vec![1, 2], None, 0).has_more(50));

        // An exact count still outranks it — the server knows how many there are.
        let mut counted = page(vec![1, 2], Some(2), 0);
        counted.partial = true;
        assert!(
            !counted.has_more(50),
            "the count is exact; the status is not"
        );
    }

    #[test]
    fn has_more_falls_back_to_a_full_page() {
        assert!(page(vec![1, 2], None, 0).has_more(2));
        assert!(!page(vec![1], None, 0).has_more(2));
        assert!(!page(vec![], None, 0).has_more(2));
    }

    /// A server that keeps offering a *new* next page and serving nothing would
    /// otherwise stream forever, growing `visited` as it went. `Page::has_more`
    /// already says an empty page ends the sequence; the stream has to agree.
    #[tokio::test]
    async fn a_link_after_an_empty_page_does_not_stream_forever() {
        use futures::StreamExt as _;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let calls = Arc::new(AtomicUsize::new(0));
        let seen = calls.clone();
        let mut stream = paginate(Query::new().limit(2), move |_| {
            let calls = seen.clone();
            async move {
                let n = calls.fetch_add(1, Ordering::SeqCst);
                Ok(if n == 0 {
                    Page::new(vec![1u8, 2]).with_next_link("https://host/a?cursor=1")
                } else {
                    // Empty, and always naming somewhere new.
                    Page::new(Vec::new()).with_next_link(format!("https://host/a?cursor={}", n + 1))
                })
            }
        });

        let items: Vec<u8> = (&mut stream)
            .map(|item| item.expect("no error"))
            .collect()
            .await;

        assert_eq!(items, [1, 2]);
        assert_eq!(
            calls.load(Ordering::SeqCst),
            2,
            "the empty page ended it; anything more is an unbounded poll"
        );
    }

    #[test]
    fn an_empty_page_ends_the_sequence_whatever_the_headers_say() {
        // Guards against an infinite stream when a server reports a total it
        // then declines to serve.
        let mut empty = page(vec![], Some(1000), 0);
        empty.next_link = Some("https://host/next".into());
        assert!(!empty.has_more(20));
    }

    #[test]
    fn next_offset_advances_by_page_length() {
        assert_eq!(page(vec![1, 2, 3], None, 10).next_offset(), 13);
    }

    #[test]
    fn parses_the_next_relation_from_a_link_header() {
        let header =
            r#"<https://host/a?offset=0>; rel="prev", <https://host/a?offset=40>; rel="next""#;
        assert_eq!(
            next_link(header).as_deref(),
            Some("https://host/a?offset=40")
        );
        assert_eq!(next_link(r#"<https://host/a>; rel="prev""#), None);
        assert_eq!(next_link("nonsense"), None);
    }

    #[test]
    fn link_relation_matching_is_case_insensitive_and_quote_tolerant() {
        assert!(next_link("<https://host/a>; REL=next").is_some());
    }

    #[test]
    fn a_comma_inside_the_url_does_not_end_the_link() {
        // TMF630 filters are comma-separated value lists, so a server echoing
        // the client's filter into the next-page link produces this. Splitting
        // the header on ',' drops the link and silently truncates the stream.
        let header = r#"<https://host/productOffering?state=held,pending&offset=40>; rel="next""#;
        assert_eq!(
            next_link(header).as_deref(),
            Some("https://host/productOffering?state=held,pending&offset=40")
        );
    }

    #[test]
    fn a_semicolon_inside_the_url_is_not_a_parameter_separator() {
        let header = r#"<https://host/a?cursor=x;y>; rel="next""#;
        assert_eq!(
            next_link(header).as_deref(),
            Some("https://host/a?cursor=x;y")
        );
    }

    #[test]
    fn rel_may_name_several_relation_types() {
        // RFC 8288 §3.3: `rel` is a space-separated list.
        assert_eq!(
            next_link(r#"<https://host/a?offset=40>; rel="next last""#).as_deref(),
            Some("https://host/a?offset=40")
        );
        assert_eq!(
            next_link(r#"<https://host/a>; rel="prev first""#),
            None,
            "a list that does not contain `next` is not a next link"
        );
    }

    #[test]
    fn a_comma_inside_a_quoted_parameter_does_not_end_the_link() {
        let header =
            r#"<https://host/a>; title="one, two"; rel="prev", <https://host/b>; rel="next""#;
        assert_eq!(next_link(header).as_deref(), Some("https://host/b"));
    }

    #[test]
    fn the_next_relation_is_found_among_several_links() {
        let header = concat!(
            r#"<https://host/a?offset=0>; rel="first", "#,
            r#"<https://host/a?offset=20>; rel="prev", "#,
            r#"<https://host/a?offset=60>; rel="next", "#,
            r#"<https://host/a?offset=99>; rel="last""#,
        );
        assert_eq!(
            next_link(header).as_deref(),
            Some("https://host/a?offset=60")
        );
    }

    #[test]
    fn a_malformed_header_yields_no_link_rather_than_panicking() {
        for header in [
            "",
            "nonsense",
            "<unterminated; rel=\"next\"",
            "<>; rel=\"next\"",
            ">;<",
            "<a>; rel=\"next",
        ] {
            let _ = next_link(header);
        }
        assert_eq!(
            next_link("<>; rel=\"next\""),
            None,
            "a link naming nowhere is no link: following it would fail as a \
             cross-origin refusal rather than ending the collection"
        );
    }
}
