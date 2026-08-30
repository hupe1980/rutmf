//! **TMF634 Resource Catalog Management v5.0** — the client.
//!
//! Covers every resource of the v5 API: `resourceCatalog`, `resourceCategory`,
//! `resourceCandidate` and `resourceSpecification`, the asynchronous
//! `importJob` / `exportJob` collections, and the notification `hub`.
//!
//! This is the catalog half of the resource domain. What it publishes,
//! [TMF639](super::tmf639) inventories: a
//! [`Resource`](crate::resource::Resource) points back at the
//! [`ResourceSpecification`] a catalog
//! made available.
//!
//! [`ResourceSpecification`]: crate::resource::ResourceSpecification

use crate::product::{ExportJob, ExportJobCreate, ImportJob, ImportJobCreate};
use crate::resource::{
    ResourceCandidate, ResourceCandidateCreate, ResourceCandidateUpdate, ResourceCatalog,
    ResourceCatalogCreate, ResourceCatalogUpdate, ResourceCategory, ResourceCategoryCreate,
    ResourceCategoryUpdate, ResourceSpecification, ResourceSpecificationCreate,
    ResourceSpecificationUpdate,
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
///
/// Note the absence of `Management`, which the sibling catalog API
/// ([TMF620](super::tmf620)) does carry. TMF634's own `servers` block and all
/// 132 `href`s in its examples say `resourceCatalog`; the twenty-nine
/// references to `resourceCatalogManagement` come from *other* specifications
/// pointing inward. The API's own document wins.
pub const API_PATH: &str = "tmf-api/resourceCatalog/v5";

/// The specification version this client was modelled from.
///
/// The nine covered APIs are not all on the same patch release, so this is
/// per-API rather than a single crate-wide constant. Asserted against the
/// vendored corpus by `every_client_reports_the_version_it_was_modelled_from`.
pub const SPEC_VERSION: &str = "5.0.0";

const CATALOGS: &str = "resourceCatalog";
const CATEGORIES: &str = "resourceCategory";
const CANDIDATES: &str = "resourceCandidate";
const SPECIFICATIONS: &str = "resourceSpecification";
const IMPORT_JOBS: &str = "importJob";
const EXPORT_JOBS: &str = "exportJob";

/// A client for TMF634 Resource Catalog Management.
///
/// ```no_run
/// use rutmf::api::{Query, Transport, tmf634::ResourceCatalogClient};
///
/// # async fn demo(transport: impl Transport + 'static) -> rutmf::api::Result<()> {
/// let client = ResourceCatalogClient::new(
///     "https://mycsp.com/tmf-api/resourceCatalog/v5",
///     transport,
/// )?;
///
/// let page = client
///     .list_resource_specifications(&Query::new().filter("lifecycleStatus", "Active").limit(20))
///     .await?;
///
/// for spec in page {
///     println!("{:?} is a {:?}", spec.name, spec.kind());
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ResourceCatalogClient {
    inner: TmfClient,
}

impl ResourceCatalogClient {
    /// Creates a client for `base_url`, dispatching through `transport`.
    ///
    /// `base_url` must include the API root, e.g.
    /// `https://mycsp.com/tmf-api/resourceCatalog/v5`. Use
    /// [`from_host`](Self::from_host) to have it appended for you.
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

    /// Creates a client for `host`, appending the conventional [`API_PATH`].
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
        CATALOGS,
        read = ResourceCatalog,
        create = ResourceCatalogCreate,
        update = ResourceCatalogUpdate,
        list = list_resource_catalogs,
        stream = stream_resource_catalogs,
        get = get_resource_catalog,
        new = create_resource_catalog,
        patch = update_resource_catalog,
        delete = delete_resource_catalog,
        doc = "a resource catalog"
    );

    resource_ops!(
        CATEGORIES,
        read = ResourceCategory,
        create = ResourceCategoryCreate,
        update = ResourceCategoryUpdate,
        list = list_resource_categories,
        stream = stream_resource_categories,
        get = get_resource_category,
        new = create_resource_category,
        patch = update_resource_category,
        delete = delete_resource_category,
        doc = "a resource category"
    );

    resource_ops!(
        CANDIDATES,
        read = ResourceCandidate,
        create = ResourceCandidateCreate,
        update = ResourceCandidateUpdate,
        list = list_resource_candidates,
        stream = stream_resource_candidates,
        get = get_resource_candidate,
        new = create_resource_candidate,
        patch = update_resource_candidate,
        delete = delete_resource_candidate,
        doc = "a resource candidate"
    );

    resource_ops!(
        SPECIFICATIONS,
        read = ResourceSpecification,
        create = ResourceSpecificationCreate,
        update = ResourceSpecificationUpdate,
        list = list_resource_specifications,
        stream = stream_resource_specifications,
        get = get_resource_specification,
        new = create_resource_specification,
        patch = update_resource_specification,
        delete = delete_resource_specification,
        doc = "a resource specification"
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

impl HubOps for ResourceCatalogClient {
    fn hub_client(&self) -> &TmfClient {
        &self.inner
    }
}
