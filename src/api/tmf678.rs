//! **TMF678 Customer Bill v5.0.0** — the client.
//!
//! Covers `customerBill`, `customerBillOnDemand`, `appliedCustomerBillingRate`
//! and `billCycle`, plus the notification `hub`.
//!
//! # No resource here has the full CRUD surface
//!
//! This is the API that shows why the operation macros are composable. TMF678
//! declares a different set on every collection, and the client offers exactly
//! that set — so there is no `create_customer_bill` to reach for, because
//! `POST /customerBill` does not exist.
//!
//! | Collection | Operations TMF678 declares |
//! |---|---|
//! | `customerBill` | list, get, patch |
//! | `customerBillOnDemand` | list, get, create |
//! | `appliedCustomerBillingRate` | list, get |
//! | `billCycle` | list, get |

use crate::bill::{
    AppliedCustomerBillingRate, BillCycle, CustomerBill, CustomerBillOnDemand,
    CustomerBillOnDemandCreate, CustomerBillUpdate,
};

use super::client::TmfClient;
use super::error::Result;
use super::hub::HubOps;
use super::page::{Page, PageRequest, PageStream, paginate};
use super::patch::Patch;
use super::query::Query;
use super::transport::Transport;
use super::{op_get, op_list, op_patch, op_stream, readonly_ops, task_ops};

/// The conventional API root, appended to a host to form the base URL.
pub const API_PATH: &str = "tmf-api/customerBillManagement/v5";

/// The specification version this client was modelled from.
///
/// The covered APIs are not all on the same patch release, so this is per-API
/// rather than a single crate-wide constant. Asserted against the vendored
/// corpus by `every_client_reports_the_version_it_was_modelled_from`.
pub const SPEC_VERSION: &str = "5.0.0";

const BILLS: &str = "customerBill";
const ON_DEMAND: &str = "customerBillOnDemand";
const RATES: &str = "appliedCustomerBillingRate";
const CYCLES: &str = "billCycle";

/// A client for TMF678 Customer Bill.
///
/// ```no_run
/// use rutmf::api::{Query, Transport, tmf678::CustomerBillClient};
///
/// # async fn demo(transport: impl Transport + 'static) -> rutmf::api::Result<()> {
/// let client =
///     CustomerBillClient::new("https://mycsp.com/tmf-api/customerBillManagement/v5", transport)?;
///
/// let page = client
///     .list_customer_bills(&Query::new().filter("state", "sent").limit(20))
///     .await?;
///
/// for bill in page {
///     println!("{:?} owes {:?}", bill.bill_no, bill.amount_due);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct CustomerBillClient {
    inner: TmfClient,
}

impl CustomerBillClient {
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

    // A bill can be read and moved through its lifecycle. It cannot be created
    // or deleted: TMF678 declares neither, because an issued invoice is
    // evidence rather than a record a client owns.
    op_list!(BILLS, CustomerBill, list_customer_bills, "a customer bill");
    op_stream!(
        BILLS,
        CustomerBill,
        stream_customer_bills,
        "a customer bill"
    );
    op_get!(BILLS, CustomerBill, get_customer_bill, "a customer bill");
    op_patch!(
        BILLS,
        CustomerBill,
        CustomerBillUpdate,
        update_customer_bill,
        "a customer bill"
    );

    task_ops!(
        ON_DEMAND,
        read = CustomerBillOnDemand,
        create = CustomerBillOnDemandCreate,
        list = list_on_demand_requests,
        stream = stream_on_demand_requests,
        get = get_on_demand_request,
        new = request_bill_on_demand,
        doc = "an on-demand bill request"
    );

    readonly_ops!(
        RATES,
        read = AppliedCustomerBillingRate,
        list = list_applied_billing_rates,
        stream = stream_applied_billing_rates,
        get = get_applied_billing_rate,
        doc = "an applied billing rate"
    );

    readonly_ops!(
        CYCLES,
        read = BillCycle,
        list = list_bill_cycles,
        stream = stream_bill_cycles,
        get = get_bill_cycle,
        doc = "a bill cycle"
    );
}

impl HubOps for CustomerBillClient {
    fn hub_client(&self) -> &TmfClient {
        &self.inner
    }
}
