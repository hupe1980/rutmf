//! The client error model.

use http::StatusCode;

use crate::core::TmfError;

use super::transport::TransportError;

/// Anything that can go wrong calling a TM Forum API.
///
/// A server that answers with a TMF630 error body surfaces as [`Error::Api`]
/// with the body parsed, so `code` and `reason` are available as data rather
/// than as a formatted string.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The server returned an error status with a well-formed TMF error body.
    #[error("{status}: {error}")]
    Api {
        /// The HTTP status code.
        status: StatusCode,
        /// The parsed TMF630 error body.
        error: Box<TmfError>,
    },

    /// The server returned an error status without a usable TMF error body.
    #[error("HTTP {status}: {body}")]
    Status {
        /// The HTTP status code.
        status: StatusCode,
        /// The raw response body, truncated for display.
        body: String,
    },

    /// `202 Accepted`: the server took the request but has not completed it,
    /// so there is no resource to return yet.
    ///
    /// Every v5 `POST` and `PATCH` declares this alongside its synchronous
    /// answer — a deployment may fulfil a write asynchronously. It is reported
    /// as an error because the call asked for a resource and did not get one,
    /// not because anything went wrong: match on it and poll.
    #[error("{status}: accepted for asynchronous processing{}", monitor.as_deref().map_or(String::new(), |m| format!(", monitor at {m}")))]
    Accepted {
        /// The status the server answered with.
        status: StatusCode,
        /// Where to poll, from `Location` or `Content-Location`, when the
        /// server named somewhere.
        monitor: Option<String>,
    },

    /// The response body could not be decoded into the expected type.
    #[error("failed to decode response body: {0}")]
    Decode(#[source] serde_json::Error),

    /// The request body could not be encoded.
    #[error("failed to encode request body: {0}")]
    Encode(#[source] serde_json::Error),

    /// The underlying HTTP stack failed.
    #[error("transport error: {0}")]
    Transport(#[source] TransportError),

    /// The client was configured with an unusable base URL.
    #[error("invalid base URL: {0}")]
    InvalidBaseUrl(String),

    /// A conditional write was asked for against a resource the server issued
    /// no `ETag` for.
    ///
    /// [`Tagged::update`] and [`Tagged::remove`] make a write conditional on the
    /// tag the read carried. When the server sent none there is nothing to make
    /// it conditional on, and sending the write anyway would be exactly the
    /// silent overwrite the precondition exists to prevent — so it is refused
    /// and said out loud instead.
    ///
    /// Use the client's plain update or delete method to write unconditionally.
    ///
    /// [`Tagged::update`]: super::Tagged::update
    /// [`Tagged::remove`]: super::Tagged::remove
    #[error("the server issued no ETag, so this write cannot be made conditional")]
    NoEntityTag,

    /// A server-supplied URL pointed outside the API's own origin, and was
    /// refused rather than followed.
    ///
    /// Both the `href` of a `…Ref` and a `Link: rel="next"` header are written
    /// by the server, and the transport attaches this client's credentials to
    /// whatever URL it is handed. Following one to another origin would hand a
    /// bearer token to whoever that origin belongs to, so the default is to
    /// refuse.
    ///
    /// Deliberate federation across origins is still available, explicitly:
    /// [`TmfClient::get_cross_origin`] and
    /// [`ResolveRef::resolve_cross_origin`].
    ///
    /// [`TmfClient::get_cross_origin`]: super::TmfClient::get_cross_origin
    /// [`ResolveRef::resolve_cross_origin`]: super::ResolveRef::resolve_cross_origin
    #[error("refusing to follow {url} — it leaves the origin of {base}")]
    CrossOrigin {
        /// The URL that was refused.
        url: String,
        /// The client's own base URL, whose origin it had to match.
        base: String,
    },
}

impl Error {
    /// The HTTP status, when the failure came from a server response.
    #[must_use]
    pub fn status(&self) -> Option<StatusCode> {
        match self {
            Self::Api { status, .. }
            | Self::Status { status, .. }
            | Self::Accepted { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// Whether the server accepted the write but has not completed it.
    ///
    /// ```no_run
    /// # async fn demo(
    /// #     client: rutmf::api::tmf620::ProductCatalogClient,
    /// #     body: rutmf::product::ProductOfferingCreate,
    /// # ) -> rutmf::api::Result<()> {
    /// match client.create_product_offering(&body).await {
    ///     Ok(offering) => println!("created {:?}", offering.id),
    ///     Err(error) if error.is_accepted() => {
    ///         println!("queued; poll {:?}", error.monitor());
    ///     }
    ///     Err(error) => return Err(error),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn is_accepted(&self) -> bool {
        matches!(self, Self::Accepted { .. })
    }

    /// Where to poll for an asynchronous write, when the server named a place.
    #[must_use]
    pub fn monitor(&self) -> Option<&str> {
        match self {
            Self::Accepted { monitor, .. } => monitor.as_deref(),
            _ => None,
        }
    }

    /// The parsed TMF error body, when the server supplied one.
    #[must_use]
    pub fn tmf_error(&self) -> Option<&TmfError> {
        match self {
            Self::Api { error, .. } => Some(error),
            _ => None,
        }
    }

    /// Whether retrying the identical request could plausibly succeed.
    ///
    /// True for 408, 429 and 5xx — except 501 Not Implemented, which is a
    /// statement about the endpoint rather than about this moment — and for
    /// transport failures. Whether the *request* may be re-sent is a separate
    /// question, and one [`RetryTransport`](super::RetryTransport) answers by
    /// method.
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Transport(_) => true,
            Self::Api { status, .. } | Self::Status { status, .. } => is_retryable_status(*status),
            _ => false,
        }
    }

    /// Whether the failure was a 404.
    #[must_use]
    pub fn is_not_found(&self) -> bool {
        self.status() == Some(StatusCode::NOT_FOUND)
    }

    /// Whether a conditional write was refused because the resource had changed.
    ///
    /// The outcome [`Tagged::update`] exists to produce: someone else edited the
    /// resource between the read and the write, so the write was refused rather
    /// than allowed to discard their change. Re-read and decide what to do —
    /// which is a decision only the caller can make, and the reason this is not
    /// retried automatically.
    ///
    /// True for both shapes a `412` can arrive in, so it does not depend on
    /// whether the server sent a TMF630 error body with it.
    ///
    /// [`Tagged::update`]: super::Tagged::update
    #[must_use]
    pub fn is_precondition_failed(&self) -> bool {
        self.status()
            .is_some_and(super::conditional::is_precondition_failed)
    }
}

/// Whether a status says "this might work later".
///
/// Shared with [`RetryTransport`](super::RetryTransport), which sees a status
/// before the client layer has turned it into an [`Error`].
#[must_use]
pub(crate) fn is_retryable_status(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::REQUEST_TIMEOUT | StatusCode::TOO_MANY_REQUESTS
    ) || (status.is_server_error() && status != StatusCode::NOT_IMPLEMENTED)
}

/// Convenience alias for results from this crate's clients.
pub type Result<T> = std::result::Result<T, Error>;
