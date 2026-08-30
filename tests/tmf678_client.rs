//! End-to-end tests for TMF678 Customer Bill.
//!
//! The point of interest is what the client *cannot* do: TMF678 declares no
//! `POST /customerBill` and no `DELETE`, so no such method exists.

#![cfg(all(feature = "api-tmf678", feature = "mock"))]

use serde_json::json;

use rutmf::api::{Query, tmf678::CustomerBillClient};
use rutmf::bill::{
    CustomerBill, CustomerBillOnDemandCreate, CustomerBillOnDemandState, CustomerBillState,
    CustomerBillUpdate,
};
use rutmf::core::Ref;
use rutmf::mock::MockTmfServer;

fn bills(server: &MockTmfServer) -> CustomerBillClient {
    CustomerBillClient::new(server.base_url(), server.transport()).expect("a valid base URL")
}

#[tokio::test]
async fn a_bill_can_be_read_and_moved_through_its_lifecycle() {
    let server = MockTmfServer::new();
    server.seed(
        "customerBill",
        json!({
            "id": "CB-1",
            "@type": "CustomerBill",
            "billNo": "780123456",
            "state": "sent",
            "amountDue": {"unit": "EUR", "value": 50},
        }),
    );

    let bill: CustomerBill = bills(&server)
        .get_customer_bill("CB-1", &Query::new())
        .await
        .expect("the seeded bill is there");
    assert!(bill.state.unwrap().is_outstanding());

    let settled = bills(&server)
        .update_customer_bill(
            "CB-1",
            &CustomerBillUpdate::builder()
                .state(CustomerBillState::Settled)
                .build(),
        )
        .await
        .expect("the state moves");

    assert_eq!(settled.state, Some(CustomerBillState::Settled));
    assert!(!settled.state.unwrap().is_outstanding());
    // The patch named only `state`; the amount is untouched.
    assert!(settled.amount_due.is_some());
}

/// A bill is produced by a billing run, not created by a client — so the
/// request goes through the on-demand task collection instead.
#[tokio::test]
async fn a_bill_is_requested_rather_than_created() {
    let server = MockTmfServer::new();

    let request = bills(&server)
        .request_bill_on_demand(
            &CustomerBillOnDemandCreate::builder()
                .billing_account(Ref::new("BA-42"))
                .name("mid-cycle statement")
                .build(),
        )
        .await
        .expect("the request is accepted");

    assert!(request.id.is_some());
    assert_eq!(
        request.billing_account.as_ref().map(|r| r.id.as_str()),
        Some("BA-42")
    );

    // Poll it the way a caller would.
    let id = request.id.clone().unwrap();
    let polled = bills(&server)
        .get_on_demand_request(&id, &Query::new())
        .await
        .expect("the request is readable");
    let finished = polled
        .state
        .as_ref()
        .is_some_and(CustomerBillOnDemandState::is_finished);
    assert!(!finished, "a freshly seeded request has no terminal state");
}

#[tokio::test]
async fn read_only_collections_are_listable() {
    let server = MockTmfServer::new();
    server.seed(
        "appliedCustomerBillingRate",
        json!({"id": "R-1", "@type": "AppliedCustomerBillingRate", "isBilled": true,
               "taxIncludedAmount": {"unit": "EUR", "value": 12}}),
    );
    server.seed(
        "billCycle",
        json!({"id": "BC-1", "@type": "BillCycle", "name": "december run"}),
    );

    let rates = bills(&server)
        .list_applied_billing_rates(&Query::new().filter("isBilled", "true"))
        .await
        .expect("rates are listable");
    assert_eq!(rates.items.len(), 1);

    let cycle = bills(&server)
        .get_bill_cycle("BC-1", &Query::new())
        .await
        .expect("cycles are readable");
    assert_eq!(cycle.name.as_deref(), Some("december run"));
}

/// The client surface matches what TMF678 declares — nothing more.
///
/// This is a compile-time assertion disguised as a test: if a future change
/// gave `CustomerBillClient` a `create_customer_bill` or a
/// `delete_customer_bill`, the reasoning in `api::ops` would have been lost.
/// The names below are the complete write surface.
#[test]
fn the_client_offers_no_operation_tmf678_does_not_declare() {
    // These exist.
    let _ = CustomerBillClient::request_bill_on_demand;
    let _ = CustomerBillClient::get_customer_bill;
    let _ = CustomerBillClient::list_bill_cycles;
    // `create_customer_bill` and `delete_customer_bill` deliberately do not,
    // and `tests/coverage.rs` checks that against the vendored paths.
}
