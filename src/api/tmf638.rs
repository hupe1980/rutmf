//! **TMF638 Service Inventory Management v5.0.0** — the client.
//!
//! Covers the single resource of the v5 API, `service`, plus the notification
//! `hub`.

use crate::service::{Service, ServiceCreate, ServiceUpdate};

use super::client::TmfClient;
use super::error::Result;
use super::hub::HubOps;
use super::page::{Page, PageRequest, PageStream, paginate};
use super::patch::Patch;
use super::query::Query;
use super::resource_ops;
use super::transport::Transport;

/// The conventional API root, appended to a host to form the base URL.
pub const API_PATH: &str = "tmf-api/serviceInventory/v5";

/// The specification version this client was modelled from.
///
/// The nine covered APIs are not all on the same patch release, so this is
/// per-API rather than a single crate-wide constant. Asserted against the
/// vendored corpus by `every_client_reports_the_version_it_was_modelled_from`.
pub const SPEC_VERSION: &str = "5.0.0";

const SERVICES: &str = "service";

/// A client for TMF638 Service Inventory Management.
///
/// ```no_run
/// use rutmf::api::{FilterOp, Query, Transport, tmf638::ServiceInventoryClient};
///
/// # async fn demo(transport: impl Transport + 'static) -> rutmf::api::Result<()> {
/// let client = ServiceInventoryClient::from_host("https://mycsp.com", transport)?;
///
/// // Services whose operation is anything but healthy.
/// let page = client
///     .list_services(&Query::new().filter_op("operatingStatus", FilterOp::Ne, "running"))
///     .await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ServiceInventoryClient {
    inner: TmfClient,
}

impl ServiceInventoryClient {
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
        SERVICES,
        read = Service,
        create = ServiceCreate,
        update = ServiceUpdate,
        list = list_services,
        stream = stream_services,
        get = get_service,
        new = create_service,
        patch = update_service,
        delete = delete_service,
        doc = "a service"
    );
}

impl HubOps for ServiceInventoryClient {
    fn hub_client(&self) -> &TmfClient {
        &self.inner
    }
}
