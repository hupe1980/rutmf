//! **TMF632 Party Management v5.0** — the client.
//!
//! Covers the two resources of the v5 API: `individual` and `organization`,
//! plus the notification `hub`.

use crate::party::{
    Individual, IndividualCreate, IndividualUpdate, Organization, OrganizationCreate,
    OrganizationUpdate,
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
pub const API_PATH: &str = "tmf-api/partyManagement/v5";

/// The specification version this client was modelled from.
///
/// The nine covered APIs are not all on the same patch release, so this is
/// per-API rather than a single crate-wide constant. Asserted against the
/// vendored corpus by `every_client_reports_the_version_it_was_modelled_from`.
pub const SPEC_VERSION: &str = "5.0.0";

const INDIVIDUALS: &str = "individual";
const ORGANIZATIONS: &str = "organization";

/// A client for TMF632 Party Management.
///
/// ```no_run
/// use rutmf::api::{Query, Transport, tmf632::PartyClient};
///
/// # async fn demo(transport: impl Transport + 'static) -> rutmf::api::Result<()> {
/// let client = PartyClient::from_host("https://mycsp.com", transport)?;
///
/// let page = client
///     .list_individuals(&Query::new().filter("familyName", "Lovelace"))
///     .await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct PartyClient {
    inner: TmfClient,
}

impl PartyClient {
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
        INDIVIDUALS,
        read = Individual,
        create = IndividualCreate,
        update = IndividualUpdate,
        list = list_individuals,
        stream = stream_individuals,
        get = get_individual,
        new = create_individual,
        patch = update_individual,
        delete = delete_individual,
        doc = "an individual"
    );

    resource_ops!(
        ORGANIZATIONS,
        read = Organization,
        create = OrganizationCreate,
        update = OrganizationUpdate,
        list = list_organizations,
        stream = stream_organizations,
        get = get_organization,
        new = create_organization,
        patch = update_organization,
        delete = delete_organization,
        doc = "an organization"
    );
}

impl HubOps for PartyClient {
    fn hub_client(&self) -> &TmfClient {
        &self.inner
    }
}
