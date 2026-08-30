//! **TMF620 Product Catalog Management v5.0** — the client.
//!
//! Covers every resource of the v5 API: `productCatalog`, `category`,
//! `productOffering`, `productOfferingPrice` and `productSpecification`, the
//! asynchronous `importJob` / `exportJob` collections, and the notification
//! `hub`.
//!
//! Note the v5 rename: the catalog resource moved from `/catalog` (v4) to
//! `/productCatalog`. This client speaks v5 only.

use crate::product::{
    Category, CategoryCreate, CategoryUpdate, ExportJob, ExportJobCreate, ImportJob,
    ImportJobCreate, ProductCatalog, ProductCatalogCreate, ProductCatalogUpdate, ProductOffering,
    ProductOfferingCreate, ProductOfferingPrice, ProductOfferingPriceCreate,
    ProductOfferingPriceUpdate, ProductOfferingUpdate, ProductSpecification,
    ProductSpecificationCreate, ProductSpecificationUpdate,
};

use super::client::TmfClient;
use super::error::Result;
use super::hub::HubOps;
use super::page::{Page, PageRequest, PageStream, paginate};
use super::patch::Patch;
use super::query::Query;
use super::transport::Transport;
use super::{op_delete, resource_ops, task_ops};

/// The conventional API root, appended to a host to form the base URL.
pub const API_PATH: &str = "tmf-api/productCatalogManagement/v5";

/// The specification version this client was modelled from.
///
/// The nine covered APIs are not all on the same patch release, so this is
/// per-API rather than a single crate-wide constant. Asserted against the
/// vendored corpus by `every_client_reports_the_version_it_was_modelled_from`.
pub const SPEC_VERSION: &str = "5.0.0";

const CATALOGS: &str = "productCatalog";
const CATEGORIES: &str = "category";
const OFFERINGS: &str = "productOffering";
const PRICES: &str = "productOfferingPrice";
const SPECIFICATIONS: &str = "productSpecification";
const IMPORT_JOBS: &str = "importJob";
const EXPORT_JOBS: &str = "exportJob";

/// A client for TMF620 Product Catalog Management.
///
/// ```no_run
/// use rutmf::api::{Query, Transport, tmf620::ProductCatalogClient};
///
/// # async fn demo(transport: impl Transport + 'static) -> rutmf::api::Result<()> {
/// // With the `transport-reqwest` feature, pass `ReqwestTransport::new()?` here.
/// let client = ProductCatalogClient::new(
///     "https://mycsp.com/tmf-api/productCatalogManagement/v5",
///     transport,
/// )?;
///
/// let page = client
///     .list_product_offerings(&Query::new().filter("lifecycleStatus", "Active").limit(20))
///     .await?;
///
/// for offering in page {
///     println!("{:?}", offering.name);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ProductCatalogClient {
    inner: TmfClient,
}

impl ProductCatalogClient {
    /// Creates a client for `base_url`, dispatching through `transport`.
    ///
    /// `base_url` must include the API root, e.g.
    /// `https://mycsp.com/tmf-api/productCatalogManagement/v5`. Use
    /// [`ProductCatalogClient::from_host`] to have it appended for you.
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
        OFFERINGS,
        read = ProductOffering,
        create = ProductOfferingCreate,
        update = ProductOfferingUpdate,
        list = list_product_offerings,
        stream = stream_product_offerings,
        get = get_product_offering,
        new = create_product_offering,
        patch = update_product_offering,
        delete = delete_product_offering,
        doc = "a product offering"
    );

    resource_ops!(
        SPECIFICATIONS,
        read = ProductSpecification,
        create = ProductSpecificationCreate,
        update = ProductSpecificationUpdate,
        list = list_product_specifications,
        stream = stream_product_specifications,
        get = get_product_specification,
        new = create_product_specification,
        patch = update_product_specification,
        delete = delete_product_specification,
        doc = "a product specification"
    );

    resource_ops!(
        PRICES,
        read = ProductOfferingPrice,
        create = ProductOfferingPriceCreate,
        update = ProductOfferingPriceUpdate,
        list = list_product_offering_prices,
        stream = stream_product_offering_prices,
        get = get_product_offering_price,
        new = create_product_offering_price,
        patch = update_product_offering_price,
        delete = delete_product_offering_price,
        doc = "a product offering price"
    );

    resource_ops!(
        CATALOGS,
        read = ProductCatalog,
        create = ProductCatalogCreate,
        update = ProductCatalogUpdate,
        list = list_product_catalogs,
        stream = stream_product_catalogs,
        get = get_product_catalog,
        new = create_product_catalog,
        patch = update_product_catalog,
        delete = delete_product_catalog,
        doc = "a product catalog"
    );

    resource_ops!(
        CATEGORIES,
        read = Category,
        create = CategoryCreate,
        update = CategoryUpdate,
        list = list_categories,
        stream = stream_categories,
        get = get_category,
        new = create_category,
        patch = update_category,
        delete = delete_category,
        doc = "a category"
    );

    task_ops!(
        IMPORT_JOBS,
        read = ImportJob,
        create = ImportJobCreate,
        list = list_import_jobs,
        stream = stream_import_jobs,
        get = get_import_job,
        new = create_import_job,
        doc = "an import job"
    );
    op_delete!(IMPORT_JOBS, delete_import_job, "an import job");

    task_ops!(
        EXPORT_JOBS,
        read = ExportJob,
        create = ExportJobCreate,
        list = list_export_jobs,
        stream = stream_export_jobs,
        get = get_export_job,
        new = create_export_job,
        doc = "an export job"
    );
    op_delete!(EXPORT_JOBS, delete_export_job, "an export job");
}

impl HubOps for ProductCatalogClient {
    fn hub_client(&self) -> &TmfClient {
        &self.inner
    }
}
