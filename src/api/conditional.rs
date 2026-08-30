//! Conditional requests: reading a resource with its entity-tag, and writing it
//! back only while it is still that resource.
//!
//! A TMF `PATCH` is read-modify-write, so two clients editing different members
//! of one resource each discard the other's change — with `200` to both.
//! `If-Match` (RFC 9110 §13.1.1) is the guard, and [`TmfHandler`] is the other
//! end of the same exchange.
//!
//! ```no_run
//! # async fn demo(client: rutmf::api::tmf622::ProductOrderClient) -> rutmf::api::Result<()> {
//! use rutmf::api::{Conditional, Query, Tagged};
//! use rutmf::order::{ProductOrder, ProductOrderUpdate};
//!
//! let held: Tagged<ProductOrder> = client.inner().fetch("42", &Query::new()).await?;
//! println!("{:?}", held.state); // `Tagged<T>` derefs to `T`
//!
//! // A `412` if anyone edited the order in between, rather than an overwrite.
//! let update = ProductOrderUpdate::builder().note(vec![]).build();
//! held.update(client.inner(), &update).await?;
//! # Ok(())
//! # }
//! ```
//!
//! The v5 documents declare no request headers at all, so this is RFC 9110
//! rather than TMF. A deployment that ignores the precondition answers as it
//! would without one, and [`Conditional::fetch`] reports whether a tag was
//! issued, so the two are distinguishable.
//!
//! [`TmfHandler`]: crate::server::TmfHandler

use std::ops::Deref;

use http::{HeaderValue, Method, StatusCode, header};
use serde::de::DeserializeOwned;

use crate::core::{Entity, PatchBody};

use super::client::TmfClient;
use super::error::{Error, Result};
use super::patch::Patch;
use super::query::Query;
use super::resolve::Resolvable;
use super::transport::TmfRequest;

/// An HTTP entity-tag, exactly as a server issued it.
///
/// Opaque by design: RFC 9110 §8.8.3 gives a tag no structure a client may
/// interpret, so the only thing to do with one is send it back. It keeps its
/// quotes and any `W/` prefix, because a tag that has been re-spelled is a tag
/// the server will not recognise.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EntityTag(String);

impl EntityTag {
    /// The tag as the server wrote it, quotes and all.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether this is a *weak* validator (`W/"…"`).
    ///
    /// A weak tag can never satisfy `If-Match`, which requires strong
    /// comparison — so a server issuing one is telling you it does not support
    /// optimistic concurrency on this resource, whatever else it does.
    #[must_use]
    pub fn is_weak(&self) -> bool {
        self.0.starts_with("W/")
    }

    /// Reads the `ETag` of a response, if it carried a usable one.
    fn from_headers(headers: &http::HeaderMap) -> Option<Self> {
        let raw = headers.get(header::ETAG)?.to_str().ok()?.trim();
        (!raw.is_empty()).then(|| Self(raw.to_owned()))
    }
}

impl std::fmt::Display for EntityTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A resource together with the entity-tag the server issued for it.
///
/// Derefs to the resource, so it reads like the resource; the tag is what makes
/// a later write conditional. [`update`](Self::update) and
/// [`remove`](Self::remove) are the point of holding one.
#[derive(Debug, Clone, PartialEq)]
pub struct Tagged<T> {
    resource: T,
    etag: Option<EntityTag>,
}

impl<T> Tagged<T> {
    /// Pairs a resource with a tag, for a transport or cache of your own.
    #[must_use]
    pub fn new(resource: T, etag: Option<EntityTag>) -> Self {
        Self { resource, etag }
    }

    /// The resource.
    #[must_use]
    pub fn resource(&self) -> &T {
        &self.resource
    }

    /// The tag the server issued, if it issued one.
    ///
    /// `None` means the server sent no `ETag`, and therefore that a conditional
    /// write against it is not possible — see [`Error::NoEntityTag`].
    #[must_use]
    pub fn etag(&self) -> Option<&EntityTag> {
        self.etag.as_ref()
    }

    /// Discards the tag, keeping the resource.
    #[must_use]
    pub fn into_inner(self) -> T {
        self.resource
    }

    /// The tag this resource must still carry for a write to be allowed.
    fn precondition(&self) -> Result<&EntityTag> {
        self.etag.as_ref().ok_or(Error::NoEntityTag)
    }
}

impl<T> Deref for Tagged<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.resource
    }
}

impl<T: Entity + Resolvable> Tagged<T> {
    /// The id this resource is addressed by.
    fn address(&self) -> Result<&str> {
        self.resource.id().ok_or_else(|| {
            Error::InvalidBaseUrl(format!(
                "the {} that was read carries no id, so there is nothing to write back to",
                T::TYPE_NAME
            ))
        })
    }

    /// `PATCH`es this resource, but only while it is still the one that was
    /// read.
    ///
    /// Sends the tag this value was read with as `If-Match`, so a server that
    /// has since accepted someone else's edit answers `412` — see
    /// [`Error::is_precondition_failed`]. The body is the same [`Patch`] every
    /// other update takes.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NoEntityTag`] when the server issued no `ETag`: there is
    /// nothing to make the write conditional on, and sending it anyway is the
    /// overwrite this exists to prevent. Use the client's plain update method to
    /// write unconditionally.
    pub async fn update<'a, U: PatchBody + 'a>(
        &self,
        client: &TmfClient,
        body: impl Into<Patch<'a, U>>,
    ) -> Result<T::Output> {
        client
            .patch_if_match(T::COLLECTION, self.address()?, body, self.precondition()?)
            .await
    }

    /// `DELETE`s this resource, but only while it is still the one that was
    /// read.
    ///
    /// The delete half of [`update`](Self::update).
    ///
    /// # Errors
    ///
    /// Returns [`Error::NoEntityTag`] when the server issued no `ETag`.
    pub async fn remove(&self, client: &TmfClient) -> Result<()> {
        client
            .delete_if_match(T::COLLECTION, self.address()?, self.precondition()?)
            .await
    }
}

/// Reading a resource with the tag that makes a later write conditional.
///
/// Implemented for [`TmfClient`], which every per-API client exposes through
/// `inner()`. The collection comes from the resource type, so no call site
/// spells a path.
#[allow(async_fn_in_trait, reason = "used directly, not as a trait object")]
pub trait Conditional {
    /// `GET`s one resource together with its entity-tag.
    ///
    /// The typed counterpart of [`TmfClient::get_tagged`].
    ///
    /// ```no_run
    /// # async fn demo(client: &rutmf::api::TmfClient) -> rutmf::api::Result<()> {
    /// use rutmf::api::{Conditional, Query, Tagged};
    /// use rutmf::product::ProductOffering;
    ///
    /// let held: Tagged<ProductOffering> = client.fetch("7655", &Query::new()).await?;
    /// assert!(held.etag().is_some());
    /// # Ok(())
    /// # }
    /// ```
    async fn fetch<T>(&self, id: &str, query: &Query) -> Result<Tagged<T>>
    where
        T: Resolvable<Output = T> + DeserializeOwned;

    /// `GET`s one resource, unless it still carries `held`.
    ///
    /// Returns `Ok(None)` when the server answers `304 Not Modified`: what you
    /// hold is current, and no body was transferred. A server that does not
    /// implement `If-None-Match` answers `200`, so this degrades to
    /// [`fetch`](Self::fetch) rather than breaking.
    ///
    /// ```no_run
    /// # async fn demo(client: &rutmf::api::TmfClient) -> rutmf::api::Result<()> {
    /// use rutmf::api::{Conditional, Query, Tagged};
    /// use rutmf::product::ProductOffering;
    ///
    /// let mut held: Tagged<ProductOffering> = client.fetch("7655", &Query::new()).await?;
    ///
    /// // Later, on the next poll: only pay for the body if it changed.
    /// if let Some(fresh) = client.fetch_if_changed("7655", &Query::new(), held.etag()).await? {
    ///     held = fresh;
    /// }
    /// # let _ = held;
    /// # Ok(())
    /// # }
    /// ```
    async fn fetch_if_changed<T>(
        &self,
        id: &str,
        query: &Query,
        held: Option<&EntityTag>,
    ) -> Result<Option<Tagged<T>>>
    where
        T: Resolvable<Output = T> + DeserializeOwned;
}

impl Conditional for TmfClient {
    async fn fetch<T>(&self, id: &str, query: &Query) -> Result<Tagged<T>>
    where
        T: Resolvable<Output = T> + DeserializeOwned,
    {
        self.get_tagged(T::COLLECTION, id, query).await
    }

    async fn fetch_if_changed<T>(
        &self,
        id: &str,
        query: &Query,
        held: Option<&EntityTag>,
    ) -> Result<Option<Tagged<T>>>
    where
        T: Resolvable<Output = T> + DeserializeOwned,
    {
        // Nothing held is nothing to compare against, so this is a plain read.
        let Some(held) = held else {
            return self.fetch(id, query).await.map(Some);
        };

        let mut request =
            TmfRequest::new(Method::GET, self.url(&format!("{}/{id}", T::COLLECTION)));
        request.query = query.to_params();
        if let Ok(value) = HeaderValue::from_str(held.as_str()) {
            request.headers.insert(header::IF_NONE_MATCH, value);
        }

        let response = self.send_conditional(request).await?;
        if response.status == StatusCode::NOT_MODIFIED {
            return Ok(None);
        }
        Ok(Some(Tagged {
            etag: EntityTag::from_headers(&response.headers),
            resource: super::client::decode(&response)?,
        }))
    }
}

impl TmfClient {
    /// `GET {path}/{id}` returning the resource and its entity-tag.
    ///
    /// The path-taking form of [`Conditional::fetch`], for a collection whose
    /// resource type this crate does not model.
    pub async fn get_tagged<T: DeserializeOwned>(
        &self,
        path: &str,
        id: &str,
        query: &Query,
    ) -> Result<Tagged<T>> {
        let mut request = TmfRequest::new(Method::GET, self.url(&format!("{path}/{id}")));
        request.query = query.to_params();
        let response = self.send(request).await?;
        Ok(Tagged {
            etag: EntityTag::from_headers(&response.headers),
            resource: super::client::decode(&response)?,
        })
    }

    /// `PATCH {path}/{id}`, but only while the resource still carries `etag`.
    ///
    /// The path-taking form of [`Tagged::update`].
    pub async fn patch_if_match<'a, U: PatchBody + 'a, T: DeserializeOwned>(
        &self,
        path: &str,
        id: &str,
        body: impl Into<Patch<'a, U>>,
        etag: &EntityTag,
    ) -> Result<T> {
        let mut request = self.patch_request(path, id, &body.into())?;
        insert_if_match(&mut request, etag);
        super::client::decode(&self.send(request).await?)
    }

    /// `DELETE {path}/{id}`, but only while the resource still carries `etag`.
    ///
    /// The path-taking form of [`Tagged::remove`].
    pub async fn delete_if_match(&self, path: &str, id: &str, etag: &EntityTag) -> Result<()> {
        let mut request = TmfRequest::new(Method::DELETE, self.url(&format!("{path}/{id}")));
        insert_if_match(&mut request, etag);
        self.send(request).await.map(|_| ())
    }
}

/// Attaches the precondition.
///
/// A tag comes off the wire and goes back onto it verbatim, so the only way it
/// can fail to be a header value is if the server sent something that was not
/// one — in which case the precondition is dropped rather than the request. A
/// server that cannot quote its own tags gets an unconditional write, which is
/// what it would have got had it issued no tag at all.
fn insert_if_match(request: &mut TmfRequest, etag: &EntityTag) {
    if let Ok(value) = HeaderValue::from_str(etag.as_str()) {
        request.headers.insert(header::IF_MATCH, value);
    }
}

/// Whether a status is a failed precondition.
pub(crate) fn is_precondition_failed(status: StatusCode) -> bool {
    status == StatusCode::PRECONDITION_FAILED
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use bytes::Bytes;
    use http::HeaderMap;

    use super::*;
    use crate::api::{TmfResponse, Transport};
    use crate::product::{ProductOffering, ProductOfferingUpdate};

    /// Records the requests it is given and replays a fixed response.
    struct Recording {
        seen: Arc<Mutex<Vec<TmfRequest>>>,
        status: StatusCode,
        etag: Option<&'static str>,
    }

    #[async_trait::async_trait]
    impl Transport for Recording {
        async fn execute(&self, request: TmfRequest) -> Result<TmfResponse> {
            self.seen.lock().expect("uncontended").push(request);
            let mut headers = HeaderMap::new();
            if let Some(etag) = self.etag {
                headers.insert(header::ETAG, HeaderValue::from_static(etag));
            }
            let body = if self.status == StatusCode::NO_CONTENT {
                Bytes::new()
            } else {
                Bytes::from_static(br#"{"id":"7655","name":"Firewall"}"#)
            };
            Ok(TmfResponse::new(self.status, headers, body))
        }
    }

    fn client(
        status: StatusCode,
        etag: Option<&'static str>,
    ) -> (Arc<Mutex<Vec<TmfRequest>>>, TmfClient) {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let transport = Recording {
            seen: seen.clone(),
            status,
            etag,
        };
        (
            seen,
            TmfClient::new(
                "https://host/tmf-api/productCatalogManagement/v5",
                transport,
            )
            .expect("a base URL"),
        )
    }

    #[tokio::test]
    async fn a_read_carries_the_tag_the_server_issued() {
        let (_, client) = client(StatusCode::OK, Some("\"abc123\""));
        let held: Tagged<ProductOffering> = client.fetch("7655", &Query::new()).await.unwrap();

        assert_eq!(held.etag().map(EntityTag::as_str), Some("\"abc123\""));
        // It derefs, so the resource needs no unwrapping to read.
        assert_eq!(held.name.as_deref(), Some("Firewall"));
    }

    #[tokio::test]
    async fn a_conditional_write_sends_the_tag_it_was_read_with() {
        let (seen, client) = client(StatusCode::OK, Some("\"abc123\""));
        let held: Tagged<ProductOffering> = client.fetch("7655", &Query::new()).await.unwrap();

        let update = ProductOfferingUpdate::builder().name("Renamed").build();
        let _: ProductOffering = held.update(&client, &update).await.unwrap();

        let requests = seen.lock().unwrap();
        let patch = requests.last().expect("the PATCH was sent");
        assert_eq!(patch.method, Method::PATCH);
        // The collection came from the type, not from a string at the call site.
        assert!(
            patch.url.ends_with("/productOffering/7655"),
            "{}",
            patch.url
        );
        assert_eq!(
            patch.headers.get(header::IF_MATCH).unwrap(),
            "\"abc123\"",
            "without this the write is the lost update it was meant to prevent"
        );
    }

    #[tokio::test]
    async fn a_delete_is_conditional_too() {
        let (seen, client) = client(StatusCode::NO_CONTENT, Some("\"abc123\""));
        let held = Tagged::new(
            ProductOffering::builder().id("7655").build(),
            Some(EntityTag("\"abc123\"".to_owned())),
        );
        held.remove(&client).await.unwrap();

        let requests = seen.lock().unwrap();
        let delete = requests.last().expect("the DELETE was sent");
        assert_eq!(delete.method, Method::DELETE);
        assert_eq!(delete.headers.get(header::IF_MATCH).unwrap(), "\"abc123\"");
    }

    #[tokio::test]
    async fn a_server_that_issues_no_tag_cannot_be_written_to_conditionally() {
        // Falling back to an unconditional write would be the silent overwrite
        // the whole exchange exists to prevent, so this says so instead.
        let (_, client) = client(StatusCode::OK, None);
        let held: Tagged<ProductOffering> = client.fetch("7655", &Query::new()).await.unwrap();
        assert!(held.etag().is_none());

        let update = ProductOfferingUpdate::builder().name("Renamed").build();
        let error = held
            .update::<ProductOfferingUpdate>(&client, &update)
            .await
            .unwrap_err();
        assert!(matches!(error, Error::NoEntityTag));
    }

    #[tokio::test]
    async fn a_stale_tag_is_a_precondition_failure_rather_than_an_overwrite() {
        let (_, client) = client(StatusCode::PRECONDITION_FAILED, Some("\"abc123\""));
        let held = Tagged::new(
            ProductOffering::builder().id("7655").build(),
            Some(EntityTag("\"stale\"".to_owned())),
        );
        let update = ProductOfferingUpdate::builder().name("Renamed").build();
        let error = held
            .update::<ProductOfferingUpdate>(&client, &update)
            .await
            .unwrap_err();

        assert!(
            error.is_precondition_failed(),
            "a 412 must be recognisable without matching on the status: {error:?}"
        );
    }

    #[tokio::test]
    async fn an_unchanged_resource_costs_no_body() {
        let (seen, client) = client(StatusCode::NOT_MODIFIED, Some("\"abc123\""));
        let held = EntityTag("\"abc123\"".to_owned());

        let fresh: Option<Tagged<ProductOffering>> = client
            .fetch_if_changed("7655", &Query::new(), Some(&held))
            .await
            .unwrap();
        assert!(fresh.is_none(), "304 means what is held is current");

        let requests = seen.lock().unwrap();
        assert_eq!(
            requests[0].headers.get(header::IF_NONE_MATCH).unwrap(),
            "\"abc123\""
        );
    }

    #[tokio::test]
    async fn a_changed_resource_comes_back_with_a_new_tag() {
        let (_, client) = client(StatusCode::OK, Some("\"def456\""));
        let held = EntityTag("\"abc123\"".to_owned());

        let fresh: Tagged<ProductOffering> = client
            .fetch_if_changed("7655", &Query::new(), Some(&held))
            .await
            .unwrap()
            .expect("the server sent a body");
        assert_eq!(fresh.etag().map(EntityTag::as_str), Some("\"def456\""));
        assert_eq!(fresh.name.as_deref(), Some("Firewall"));
    }

    #[tokio::test]
    async fn holding_nothing_is_an_ordinary_read() {
        let (seen, client) = client(StatusCode::OK, Some("\"abc123\""));
        let fresh: Option<Tagged<ProductOffering>> = client
            .fetch_if_changed("7655", &Query::new(), None)
            .await
            .unwrap();
        assert!(fresh.is_some());
        assert!(
            seen.lock().unwrap()[0]
                .headers
                .get(header::IF_NONE_MATCH)
                .is_none(),
            "there was no tag to send"
        );
    }

    #[test]
    fn a_weak_validator_says_it_cannot_guard_a_write() {
        assert!(EntityTag("W/\"abc\"".to_owned()).is_weak());
        assert!(!EntityTag("\"abc\"".to_owned()).is_weak());
    }
}
