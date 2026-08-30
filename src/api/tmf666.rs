//! **TMF666 Account Management v5.0.0** — the client.
//!
//! Covers the four account collections — `billingAccount`, `financialAccount`,
//! `partyAccount`, `settlementAccount` — plus `billFormat`,
//! `billPresentationMedia` and `billingCycleSpecification`, and the `hub`.
//!
//! All four account collections serve the same Rust type. TMF666 models
//! `Account` as one family with four `@type`-discriminated subclasses and then
//! gives each its own path, so the methods differ while the shape does not —
//! ask an [`Account`] which subclass it is with
//! [`kind`](crate::account::Account::kind).

use crate::account::{
    Account, AccountCreate, AccountUpdate, BillFormat, BillFormatCreate, BillFormatUpdate,
    BillPresentationMedia, BillPresentationMediaCreate, BillPresentationMediaUpdate,
    BillingCycleSpecification, BillingCycleSpecificationCreate, BillingCycleSpecificationUpdate,
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
pub const API_PATH: &str = "tmf-api/accountManagement/v5";

/// The specification version this client was modelled from.
///
/// The covered APIs are not all on the same patch release, so this is per-API
/// rather than a single crate-wide constant. Asserted against the vendored
/// corpus by `every_client_reports_the_version_it_was_modelled_from`.
pub const SPEC_VERSION: &str = "5.0.0";

const BILLING: &str = "billingAccount";
const FINANCIAL: &str = "financialAccount";
const PARTY: &str = "partyAccount";
const SETTLEMENT: &str = "settlementAccount";
const FORMATS: &str = "billFormat";
const MEDIA: &str = "billPresentationMedia";
const CYCLES: &str = "billingCycleSpecification";

/// A client for TMF666 Account Management.
///
/// ```no_run
/// use rutmf::account::AccountKind;
/// use rutmf::api::{Query, Transport, tmf666::AccountClient};
///
/// # async fn demo(transport: impl Transport + 'static) -> rutmf::api::Result<()> {
/// let client = AccountClient::new("https://mycsp.com/tmf-api/accountManagement/v5", transport)?;
///
/// let page = client.list_billing_accounts(&Query::new().limit(20)).await?;
/// for account in page {
///     assert_eq!(account.kind(), AccountKind::Billing);
///     println!("{:?}", account.name);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct AccountClient {
    inner: TmfClient,
}

impl AccountClient {
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
        BILLING,
        read = Account,
        create = AccountCreate,
        update = AccountUpdate,
        list = list_billing_accounts,
        stream = stream_billing_accounts,
        get = get_billing_account,
        new = create_billing_account,
        patch = update_billing_account,
        delete = delete_billing_account,
        doc = "a billing account"
    );

    resource_ops!(
        FINANCIAL,
        read = Account,
        create = AccountCreate,
        update = AccountUpdate,
        list = list_financial_accounts,
        stream = stream_financial_accounts,
        get = get_financial_account,
        new = create_financial_account,
        patch = update_financial_account,
        delete = delete_financial_account,
        doc = "a financial account"
    );

    resource_ops!(
        PARTY,
        read = Account,
        create = AccountCreate,
        update = AccountUpdate,
        list = list_party_accounts,
        stream = stream_party_accounts,
        get = get_party_account,
        new = create_party_account,
        patch = update_party_account,
        delete = delete_party_account,
        doc = "a party account"
    );

    resource_ops!(
        SETTLEMENT,
        read = Account,
        create = AccountCreate,
        update = AccountUpdate,
        list = list_settlement_accounts,
        stream = stream_settlement_accounts,
        get = get_settlement_account,
        new = create_settlement_account,
        patch = update_settlement_account,
        delete = delete_settlement_account,
        doc = "a settlement account"
    );

    resource_ops!(
        FORMATS,
        read = BillFormat,
        create = BillFormatCreate,
        update = BillFormatUpdate,
        list = list_bill_formats,
        stream = stream_bill_formats,
        get = get_bill_format,
        new = create_bill_format,
        patch = update_bill_format,
        delete = delete_bill_format,
        doc = "a bill format"
    );

    resource_ops!(
        MEDIA,
        read = BillPresentationMedia,
        create = BillPresentationMediaCreate,
        update = BillPresentationMediaUpdate,
        list = list_presentation_media,
        stream = stream_presentation_media,
        get = get_presentation_medium,
        new = create_presentation_medium,
        patch = update_presentation_medium,
        delete = delete_presentation_medium,
        doc = "a bill presentation medium"
    );

    resource_ops!(
        CYCLES,
        read = BillingCycleSpecification,
        create = BillingCycleSpecificationCreate,
        update = BillingCycleSpecificationUpdate,
        list = list_billing_cycle_specifications,
        stream = stream_billing_cycle_specifications,
        get = get_billing_cycle_specification,
        new = create_billing_cycle_specification,
        patch = update_billing_cycle_specification,
        delete = delete_billing_cycle_specification,
        doc = "a billing cycle specification"
    );
}

impl HubOps for AccountClient {
    fn hub_client(&self) -> &TmfClient {
        &self.inner
    }
}
