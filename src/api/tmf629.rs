//! **TMF629 Customer Management v5.0.1** — the client.
//!
//! Covers the single resource of the v5 API, `customer`, plus the
//! notification `hub`.

use crate::customer::{Customer, CustomerCreate, CustomerUpdate};

use super::client::TmfClient;
use super::error::Result;
use super::hub::HubOps;
use super::page::{Page, PageRequest, PageStream, paginate};
use super::patch::Patch;
use super::query::Query;
use super::resource_ops;
use super::transport::Transport;

/// The conventional API root, appended to a host to form the base URL.
pub const API_PATH: &str = "tmf-api/customerManagement/v5";

/// The specification version this client was modelled from.
///
/// The nine covered APIs are not all on the same patch release, so this is
/// per-API rather than a single crate-wide constant. Asserted against the
/// vendored corpus by `every_client_reports_the_version_it_was_modelled_from`.
pub const SPEC_VERSION: &str = "5.0.1";

const CUSTOMERS: &str = "customer";

/// A client for TMF629 Customer Management.
///
/// ```no_run
/// use rutmf::api::{Query, Transport, tmf629::CustomerClient};
///
/// # async fn demo(transport: impl Transport + 'static) -> rutmf::api::Result<()> {
/// let client = CustomerClient::from_host("https://mycsp.com", transport)?;
///
/// let page = client
///     .list_customers(&Query::new().filter("status", "Active").limit(20))
///     .await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct CustomerClient {
    inner: TmfClient,
}

impl CustomerClient {
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
        CUSTOMERS,
        read = Customer,
        create = CustomerCreate,
        update = CustomerUpdate,
        list = list_customers,
        stream = stream_customers,
        get = get_customer,
        new = create_customer,
        patch = update_customer,
        delete = delete_customer,
        doc = "a customer"
    );
}

impl HubOps for CustomerClient {
    fn hub_client(&self) -> &TmfClient {
        &self.inner
    }
}
