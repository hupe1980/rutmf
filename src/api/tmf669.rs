//! **TMF669 Party Role Management v5.0.0** — the client.
//!
//! Covers the two resources of the v5 API — `partyRole` and
//! `partyRoleSpecification` — plus the notification `hub`.
//!
//! TMF669 is the general case of [TMF629](super::tmf629): a customer is one
//! party role, and this API serves all the others. Both collections have the
//! full five-operation surface.

use crate::party::{
    PartyRole, PartyRoleCreate, PartyRoleSpecification, PartyRoleSpecificationCreate,
    PartyRoleSpecificationUpdate, PartyRoleUpdate,
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
pub const API_PATH: &str = "tmf-api/partyRoleManagement/v5";

/// The specification version this client was modelled from.
///
/// The covered APIs are not all on the same patch release, so this is per-API
/// rather than a single crate-wide constant. Asserted against the vendored
/// corpus by `every_client_reports_the_version_it_was_modelled_from`.
pub const SPEC_VERSION: &str = "5.0.0";

const PARTY_ROLES: &str = "partyRole";
const PARTY_ROLE_SPECIFICATIONS: &str = "partyRoleSpecification";

/// A client for TMF669 Party Role Management.
///
/// ```no_run
/// use rutmf::api::{Query, Transport, tmf669::PartyRoleClient};
///
/// # async fn demo(transport: impl Transport + 'static) -> rutmf::api::Result<()> {
/// let client = PartyRoleClient::from_host("https://mycsp.com", transport)?;
///
/// let page = client
///     .list_party_roles(&Query::new().filter("role", "supplier").limit(20))
///     .await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct PartyRoleClient {
    inner: TmfClient,
}

impl PartyRoleClient {
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
        PARTY_ROLES,
        read = PartyRole,
        create = PartyRoleCreate,
        update = PartyRoleUpdate,
        list = list_party_roles,
        stream = stream_party_roles,
        get = get_party_role,
        new = create_party_role,
        patch = update_party_role,
        delete = delete_party_role,
        doc = "a party role"
    );

    resource_ops!(
        PARTY_ROLE_SPECIFICATIONS,
        read = PartyRoleSpecification,
        create = PartyRoleSpecificationCreate,
        update = PartyRoleSpecificationUpdate,
        list = list_party_role_specifications,
        stream = stream_party_role_specifications,
        get = get_party_role_specification,
        new = create_party_role_specification,
        patch = update_party_role_specification,
        delete = delete_party_role_specification,
        doc = "a party role specification"
    );
}

impl HubOps for PartyRoleClient {
    fn hub_client(&self) -> &TmfClient {
        &self.inner
    }
}
