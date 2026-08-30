//! **TMF639 Resource Inventory Management v5.0.0** — the client.
//!
//! Covers the single resource of the v5 API, `resource`, plus the notification
//! `hub`.

use crate::resource::{Resource, ResourceCreate};

use super::client::TmfClient;
use super::error::Result;
use super::hub::HubOps;
use super::page::{Page, PageRequest, PageStream, paginate};
use super::patch::Patch;
use super::query::Query;
use super::resource_ops;
use super::transport::Transport;

/// The conventional API root, appended to a host to form the base URL.
///
/// TMF639 declares no usable `servers` block, so the path comes from the wire:
/// seventy `href`s across three specifications — its own, TMF638's and
/// TMF642's — all say `resourceInventoryManagement`, and none says
/// `resourceInventory`.
pub const API_PATH: &str = "tmf-api/resourceInventoryManagement/v5";

/// The specification version this client was modelled from.
///
/// The nine covered APIs are not all on the same patch release, so this is
/// per-API rather than a single crate-wide constant. Asserted against the
/// vendored corpus by `every_client_reports_the_version_it_was_modelled_from`.
pub const SPEC_VERSION: &str = "5.0.0";

const RESOURCES: &str = "resource";

/// A client for TMF639 Resource Inventory Management.
///
/// ```no_run
/// use rutmf::api::{Query, Transport, tmf639::ResourceInventoryClient};
///
/// # async fn demo(transport: impl Transport + 'static) -> rutmf::api::Result<()> {
/// let client = ResourceInventoryClient::from_host("https://mycsp.com", transport)?;
///
/// let page = client
///     .list_resources(&Query::new().filter("administrativeState", "unlocked"))
///     .await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ResourceInventoryClient {
    inner: TmfClient,
}

impl ResourceInventoryClient {
    /// Creates a client for `base_url`, dispatching through `transport`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidBaseUrl`] if the URL is empty.
    ///
    /// [`Error::InvalidBaseUrl`]: super::Error::InvalidBaseUrl
    pub fn new(base_url: impl Into<String>, transport: impl Transport + 'static) -> Result<Self> {
        Ok(Self {
            inner: TmfClient::new(base_url, transport)?,
        })
    }

    /// Creates a client from a host, appending the conventional [`API_PATH`].
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidBaseUrl`] if the host is empty.
    ///
    /// [`Error::InvalidBaseUrl`]: super::Error::InvalidBaseUrl
    pub fn from_host(host: &str, transport: impl Transport + 'static) -> Result<Self> {
        Self::new(
            format!("{}/{API_PATH}", host.trim_end_matches('/')),
            transport,
        )
    }

    /// The underlying generic client, for operations this API does not wrap.
    #[must_use]
    pub fn inner(&self) -> &TmfClient {
        &self.inner
    }

    resource_ops!(
        RESOURCES,
        read = Resource,
        create = ResourceCreate,
        update = Resource,
        list = list_resources,
        stream = stream_resources,
        get = get_resource,
        new = create_resource,
        patch = update_resource,
        delete = delete_resource,
        doc = "a resource"
    );
}

impl HubOps for ResourceInventoryClient {
    fn hub_client(&self) -> &TmfClient {
        &self.inner
    }
}
