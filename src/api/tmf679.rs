//! **TMF679 Product Offering Qualification v5.0.0** — the client.
//!
//! Covers the two resources of the v5 API — `checkProductOfferingQualification`
//! and `queryProductOfferingQualification` — plus the notification `hub`.
//!
//! This is the step between browsing a catalog ([TMF620](super::tmf620)) and
//! placing an order ([TMF622](super::tmf622)): it answers whether a given
//! customer may actually buy a given offering, and what they could have
//! instead if not.

use crate::product::{
    CheckProductOfferingQualification, CheckProductOfferingQualificationCreate,
    CheckProductOfferingQualificationUpdate, QueryProductOfferingQualification,
    QueryProductOfferingQualificationCreate, QueryProductOfferingQualificationUpdate,
};

use super::client::TmfClient;
use super::error::Result;
use super::hub::HubOps;
use super::page::{Page, PageRequest, PageStream, paginate};
use super::patch::Patch;
use super::query::Query;
use super::resource_ops;
use super::transport::Transport;

/// The conventional API root, appended to a host to form the base URL.
pub const API_PATH: &str = "tmf-api/productOfferingQualification/v5";

/// The specification version this client was modelled from.
///
/// The covered APIs are not all on the same patch release, so this is per-API
/// rather than a single crate-wide constant. Asserted against the vendored
/// corpus by `every_client_reports_the_version_it_was_modelled_from`.
pub const SPEC_VERSION: &str = "5.0.0";

const CHECKS: &str = "checkProductOfferingQualification";
const QUERIES: &str = "queryProductOfferingQualification";

/// A client for TMF679 Product Offering Qualification.
///
/// ```no_run
/// use rutmf::api::{Query, Transport, tmf679::ProductOfferingQualificationClient};
///
/// # async fn demo(transport: impl Transport + 'static) -> rutmf::api::Result<()> {
/// let client = ProductOfferingQualificationClient::from_host("https://mycsp.com", transport)?;
///
/// let page = client
///     .list_check_product_offering_qualifications(&Query::new().filter("state", "done"))
///     .await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ProductOfferingQualificationClient {
    inner: TmfClient,
}

impl ProductOfferingQualificationClient {
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
        CHECKS,
        read = CheckProductOfferingQualification,
        create = CheckProductOfferingQualificationCreate,
        update = CheckProductOfferingQualificationUpdate,
        list = list_check_product_offering_qualifications,
        stream = stream_check_product_offering_qualifications,
        get = get_check_product_offering_qualification,
        new = create_check_product_offering_qualification,
        patch = update_check_product_offering_qualification,
        delete = delete_check_product_offering_qualification,
        doc = "a check product offering qualification"
    );

    resource_ops!(
        QUERIES,
        read = QueryProductOfferingQualification,
        create = QueryProductOfferingQualificationCreate,
        update = QueryProductOfferingQualificationUpdate,
        list = list_query_product_offering_qualifications,
        stream = stream_query_product_offering_qualifications,
        get = get_query_product_offering_qualification,
        new = create_query_product_offering_qualification,
        patch = update_query_product_offering_qualification,
        delete = delete_query_product_offering_qualification,
        doc = "a query product offering qualification"
    );
}

impl HubOps for ProductOfferingQualificationClient {
    fn hub_client(&self) -> &TmfClient {
        &self.inner
    }
}
