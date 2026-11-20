//! **TMF635 Product Usage Management v5.0.0** — the client.
//!
//! Covers `productUsage` — rated usage records — and the shared `exportJob`
//! collection, plus the notification `hub`.
//!
//! | Collection | Operations TMF635 declares |
//! |---|---|
//! | `productUsage` | list, get, create, patch, delete |
//! | `exportJob` | list, get, create, delete |
//!
//! TMF635 raises no usage events — only `ExportJob` ones — so a subscription is
//! useful for export jobs alone.

use crate::product::ExportJob;
use crate::usage::{
    ProductUsage, ProductUsageCreate, ProductUsageExportJobCreate, ProductUsageUpdate,
};

use super::client::TmfClient;
use super::error::Result;
use super::hub::{HubLookup, HubOps};
use super::page::{Page, PageRequest, PageStream, paginate};
use super::patch::Patch;
use super::query::Query;
use super::transport::Transport;
use super::{Api, Collection, Operation};
use super::{op_delete, resource_ops, task_ops};

/// The conventional API root, appended to a host to form the base URL.
pub const API_PATH: &str = "tmf-api/productUsageManagement/v5";

/// The specification version this client was modelled from.
///
/// The covered APIs are not all on the same patch release, so this is per-API
/// rather than a single crate-wide constant. Asserted against the vendored
/// corpus by `every_client_reports_the_version_it_was_modelled_from`.
pub const SPEC_VERSION: &str = "5.0.0";

/// What TMF635 declares: every collection, including `hub`, with the
/// operations on each.
///
/// [`TmfHandler`](crate::server::TmfHandler) refuses anything else, and
/// `tests/coverage.rs` holds this equal to the specification document.
pub const API: Api = Api::new(
    "TMF635",
    API_PATH,
    SPEC_VERSION,
    &[
        Collection::new(
            "exportJob",
            &[
                Operation::List,
                Operation::Create,
                Operation::Retrieve,
                Operation::Delete,
            ],
        ),
        Collection::crud("productUsage"),
        Collection::new(
            "hub",
            &[Operation::Create, Operation::Retrieve, Operation::Delete],
        ),
    ],
);

const USAGES: &str = "productUsage";
const EXPORT_JOBS: &str = "exportJob";

/// A client for TMF635 Product Usage Management.
///
/// ```no_run
/// use rutmf::api::{Query, Transport, tmf635::ProductUsageClient};
///
/// # async fn demo(transport: impl Transport + 'static) -> rutmf::api::Result<()> {
/// let client = ProductUsageClient::from_host("https://mycsp.com", transport)?;
///
/// let rated = client
///     .list_product_usages(&Query::new().filter("status", "rated").limit(50))
///     .await?;
/// for usage in rated {
///     println!("{:?}: {:?}", usage.id, usage.usage_product_price);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ProductUsageClient {
    inner: TmfClient,
}

impl ProductUsageClient {
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
        Self::new(API.url_on(host), transport)
    }

    /// The underlying generic client, for operations this API does not wrap.
    #[must_use]
    pub fn inner(&self) -> &TmfClient {
        &self.inner
    }

    resource_ops!(
        USAGES,
        read = ProductUsage,
        create = ProductUsageCreate,
        update = ProductUsageUpdate,
        list = list_product_usages,
        stream = stream_product_usages,
        get = get_product_usage,
        new = create_product_usage,
        patch = update_product_usage,
        delete = delete_product_usage,
        doc = "a product usage record"
    );

    task_ops!(
        EXPORT_JOBS,
        read = ExportJob,
        create = ProductUsageExportJobCreate,
        list = list_export_jobs,
        stream = stream_export_jobs,
        get = get_export_job,
        new = create_export_job,
        doc = "an export job"
    );
    op_delete!(EXPORT_JOBS, delete_export_job, "an export job");
}

impl HubOps for ProductUsageClient {
    fn hub_client(&self) -> &TmfClient {
        &self.inner
    }
}

impl HubLookup for ProductUsageClient {}
