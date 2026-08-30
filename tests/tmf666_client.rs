//! End-to-end tests for TMF666 Account Management.
//!
//! The interesting property is that one Rust type serves four collections,
//! because TMF666 models one polymorphic family and then gives each subclass
//! its own path.

#![cfg(all(feature = "api-tmf666", feature = "mock"))]

use serde_json::json;

use rutmf::account::{Account, AccountCreate, AccountKind, AccountUpdate};
use rutmf::api::{Query, tmf666::AccountClient};
use rutmf::core::RelatedParty;
use rutmf::mock::MockTmfServer;

fn accounts(server: &MockTmfServer) -> AccountClient {
    AccountClient::new(server.base_url(), server.transport()).expect("a valid base URL")
}

#[tokio::test]
async fn one_type_serves_four_collections() {
    let server = MockTmfServer::new();
    let client = accounts(&server);

    for (kind, collection) in [
        (AccountKind::Billing, "billingAccount"),
        (AccountKind::Financial, "financialAccount"),
        (AccountKind::Party, "partyAccount"),
        (AccountKind::Settlement, "settlementAccount"),
    ] {
        server.seed(
            collection,
            json!({
                "id": format!("{collection}-1"),
                "@type": kind.type_name(),
                "name": format!("{collection} under test"),
            }),
        );
    }

    let billing: Account = client
        .get_billing_account("billingAccount-1", &Query::new())
        .await
        .expect("the billing account is there");
    assert_eq!(billing.kind(), AccountKind::Billing);

    let financial = client
        .get_financial_account("financialAccount-1", &Query::new())
        .await
        .expect("the financial account is there");
    assert_eq!(financial.kind(), AccountKind::Financial);

    let settlement = client
        .get_settlement_account("settlementAccount-1", &Query::new())
        .await
        .expect("the settlement account is there");
    assert_eq!(settlement.kind(), AccountKind::Settlement);
}

#[tokio::test]
async fn a_billing_account_keeps_its_subclass_members() {
    let server = MockTmfServer::new();
    server.seed(
        "billingAccount",
        json!({
            "id": "BA-42",
            "@type": "BillingAccount",
            "name": "Acme Ltd",
            "ratingType": "postpaid",
            "paymentStatus": "paid",
            "creditLimit": {"unit": "EUR", "value": 5000},
        }),
    );

    let account = accounts(&server)
        .get_billing_account("BA-42", &Query::new())
        .await
        .expect("the seeded account is there");

    // `ratingType` belongs to `BillingAccount` alone; both are typed, so
    // neither lands in `extensions`.
    assert_eq!(account.rating_type.as_deref(), Some("postpaid"));
    assert_eq!(account.payment_status.as_deref(), Some("paid"));
    assert!(account.extensions.is_empty());
}

#[tokio::test]
async fn an_account_round_trips_through_create_and_patch() {
    let server = MockTmfServer::new();
    let client = accounts(&server);

    let created = client
        .create_billing_account(
            &AccountCreate::builder()
                .name("Acme Ltd")
                .related_party(vec![RelatedParty::default()])
                .at_type(AccountKind::Billing.type_name())
                .rating_type("postpaid")
                .build(),
        )
        .await
        .expect("the account is created");

    let id = created.id.clone().expect("the server assigns an id");
    assert_eq!(created.kind(), AccountKind::Billing);

    let updated = client
        .update_billing_account(&id, &AccountUpdate::builder().state("closed").build())
        .await
        .expect("the patch applies");

    assert_eq!(updated.state.as_deref(), Some("closed"));
    assert_eq!(
        updated.rating_type.as_deref(),
        Some("postpaid"),
        "a merge patch must not clear members it does not name"
    );
}

/// The seam TMF666 closes: TMF678 bills name a billing account, and now that
/// account is a real, readable resource rather than a placeholder marker.
#[tokio::test]
async fn a_bill_reference_resolves_to_a_real_account() {
    let server = MockTmfServer::new();
    server.seed(
        "billingAccount",
        json!({"id": "BA-42", "@type": "BillingAccount", "name": "Acme Ltd"}),
    );

    let account = accounts(&server)
        .get_billing_account("BA-42", &Query::new())
        .await
        .expect("the account a bill would name");

    assert_eq!(account.name.as_deref(), Some("Acme Ltd"));
    assert_eq!(account.kind(), AccountKind::Billing);
}
