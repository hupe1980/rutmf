//! **TMF622 Product Ordering v5.0.0** — the client.
//!
//! Covers `productOrder`, the `cancelProductOrder` task collection, and the
//! notification `hub`.

use crate::order::{
    CancelProductOrder, CancelProductOrderCreate, ProductOrder, ProductOrderCreate,
    ProductOrderUpdate,
};

use super::client::TmfClient;
use super::error::Result;
use super::hub::HubOps;
use super::page::{Page, PageRequest, PageStream, paginate};
use super::patch::Patch;
use super::query::Query;
use super::transport::Transport;
use super::{resource_ops, task_ops};

/// The conventional API root, appended to a host to form the base URL.
pub const API_PATH: &str = "tmf-api/productOrderingManagement/v5";

/// The specification version this client was modelled from.
///
/// The nine covered APIs are not all on the same patch release, so this is
/// per-API rather than a single crate-wide constant. Asserted against the
/// vendored corpus by `every_client_reports_the_version_it_was_modelled_from`.
pub const SPEC_VERSION: &str = "5.0.0";

const ORDERS: &str = "productOrder";
const CANCELLATIONS: &str = "cancelProductOrder";

/// A client for TMF622 Product Ordering.
///
/// ```no_run
/// use rutmf::api::{Query, Transport, tmf622::ProductOrderClient};
///
/// # async fn demo(transport: impl Transport + 'static) -> rutmf::api::Result<()> {
/// let client = ProductOrderClient::from_host("https://mycsp.com", transport)?;
///
/// let page = client
///     .list_product_orders(&Query::new().filter("state", "inProgress").limit(20))
///     .await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ProductOrderClient {
    inner: TmfClient,
}

impl ProductOrderClient {
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
        ORDERS,
        read = ProductOrder,
        create = ProductOrderCreate,
        update = ProductOrderUpdate,
        list = list_product_orders,
        stream = stream_product_orders,
        get = get_product_order,
        new = create_product_order,
        patch = update_product_order,
        delete = delete_product_order,
        doc = "a product order"
    );

    task_ops!(
        CANCELLATIONS,
        read = CancelProductOrder,
        create = CancelProductOrderCreate,
        list = list_cancellations,
        stream = stream_cancellations,
        get = get_cancellation,
        new = request_cancellation,
        doc = "a cancellation request"
    );
}

impl HubOps for ProductOrderClient {
    fn hub_client(&self) -> &TmfClient {
        &self.inner
    }
}
