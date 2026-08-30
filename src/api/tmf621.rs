//! **TMF621 Trouble Ticket v5.0.1** — the client.
//!
//! Covers both resources of the v5 API — `troubleTicket` and
//! `troubleTicketSpecification` — and the notification `hub`.
//!
//! This is the assurance side of the crate. Where the catalog and inventory
//! APIs describe what a customer has, a [`TroubleTicket`] describes what is
//! wrong with it.
//!
//! [`TroubleTicket`]: crate::ticket::TroubleTicket

use crate::ticket::{
    TroubleTicket, TroubleTicketCreate, TroubleTicketSpecification,
    TroubleTicketSpecificationCreate, TroubleTicketSpecificationUpdate, TroubleTicketUpdate,
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
pub const API_PATH: &str = "tmf-api/troubleTicket/v5";

/// The specification version this client was modelled from.
///
/// The nine covered APIs are not all on the same patch release, so this is
/// per-API rather than a single crate-wide constant. Asserted against the
/// vendored corpus by `every_client_reports_the_version_it_was_modelled_from`.
pub const SPEC_VERSION: &str = "5.0.1";

const TICKETS: &str = "troubleTicket";
const SPECIFICATIONS: &str = "troubleTicketSpecification";

/// A client for TMF621 Trouble Ticket.
///
/// ```no_run
/// use rutmf::api::{Query, Transport, tmf621::TroubleTicketClient};
///
/// # async fn demo(transport: impl Transport + 'static) -> rutmf::api::Result<()> {
/// let client = TroubleTicketClient::new("https://mycsp.com/tmf-api/troubleTicket/v5", transport)?;
///
/// let page = client
///     .list_trouble_tickets(&Query::new().filter("status", "inProgress").limit(20))
///     .await?;
///
/// for ticket in page {
///     println!("{:?}: {:?}", ticket.id, ticket.status);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct TroubleTicketClient {
    inner: TmfClient,
}

impl TroubleTicketClient {
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
        TICKETS,
        read = TroubleTicket,
        create = TroubleTicketCreate,
        update = TroubleTicketUpdate,
        list = list_trouble_tickets,
        stream = stream_trouble_tickets,
        get = get_trouble_ticket,
        new = create_trouble_ticket,
        patch = update_trouble_ticket,
        delete = delete_trouble_ticket,
        doc = "a trouble ticket"
    );

    resource_ops!(
        SPECIFICATIONS,
        read = TroubleTicketSpecification,
        create = TroubleTicketSpecificationCreate,
        update = TroubleTicketSpecificationUpdate,
        list = list_trouble_ticket_specifications,
        stream = stream_trouble_ticket_specifications,
        get = get_trouble_ticket_specification,
        new = create_trouble_ticket_specification,
        patch = update_trouble_ticket_specification,
        delete = delete_trouble_ticket_specification,
        doc = "a trouble ticket specification"
    );
}

impl HubOps for TroubleTicketClient {
    fn hub_client(&self) -> &TmfClient {
        &self.inner
    }
}
