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

struct Reply(rutmf::api::TmfResponse);

#[async_trait::async_trait]
impl rutmf::api::Transport for Reply {
    async fn execute(
        &self,
        _: rutmf::api::TmfRequest,
    ) -> rutmf::api::Result<rutmf::api::TmfResponse> {
        Ok(self.0.clone())
    }
}

fn partner(
    status: http::StatusCode,
    media: &'static str,
    body: &'static str,
) -> CustomerBillClient {
    let mut headers = http::HeaderMap::new();
    headers.insert(
        http::header::CONTENT_TYPE,
        http::HeaderValue::from_static(media),
    );
    let response = rutmf::api::TmfResponse::new(status, headers, body.into());
    CustomerBillClient::new("https://partner.test", Reply(response)).unwrap()
}

// 004-partner-json/SR-001
#[tokio::test]
async fn partner_credit_retains_the_cent() {
    let client = partner(
        http::StatusCode::OK,
        "application/json",
        r#"{"taxIncludedAmount":{"unit":"EUR","value":-9007199254740992.01}}"#,
    );
    let bill = client
        .get_customer_bill("credit", &Query::new())
        .await
        .unwrap();
    assert_eq!(
        bill.tax_included_amount.unwrap().value,
        Some("-9007199254740992.01".parse().unwrap())
    );
}

// 004-partner-json/SR-002
#[tokio::test]
async fn partner_problem_uses_wire_status_and_preserves_extensions() {
    let client = partner(
        http::StatusCode::NOT_FOUND,
        "Application/Problem+JSON; charset=utf-8",
        r#"{"status":503,"type":"/problems/missing","instance":"/requests/42","title":"Missing bill","detail":"No bill for this account","code":"not-found","pointer":["/id"]}"#,
    );
    let error = client
        .get_customer_bill("missing", &Query::new())
        .await
        .unwrap_err();
    assert!(error.is_not_found());
    assert!(!error.is_retryable());
    let problem = error.problem().unwrap();
    assert_eq!(problem.type_uri, "/problems/missing");
    assert_eq!(problem.instance.as_deref(), Some("/requests/42"));
    assert_eq!(problem.title.as_deref(), Some("Missing bill"));
    assert_eq!(problem.detail.as_deref(), Some("No bill for this account"));
    assert_eq!(problem.status, Some(http::StatusCode::SERVICE_UNAVAILABLE));
    assert_eq!(problem.extensions["code"], "not-found");
    assert_eq!(problem.extensions["pointer"], json!(["/id"]));
}

#[tokio::test]
async fn malformed_problem_members_are_ignored_individually() {
    let client = partner(
        http::StatusCode::BAD_GATEWAY,
        "application/problem+json",
        r#"{"title":5,"detail":"upstream offline","status":"404"}"#,
    );
    let error = client
        .get_customer_bill("42", &Query::new())
        .await
        .unwrap_err();
    assert!(error.is_retryable());
    let problem = error.problem().unwrap();
    assert_eq!(problem.type_uri, "about:blank");
    assert_eq!(problem.title, None);
    assert_eq!(problem.status, None);
    assert_eq!(problem.detail.as_deref(), Some("upstream offline"));
    assert!(problem.extensions.is_empty());
    for body in ["bad gateway", "[]", "null"] {
        let error = partner(
            http::StatusCode::BAD_GATEWAY,
            "application/problem+json",
            body,
        )
        .get_customer_bill("42", &Query::new())
        .await
        .unwrap_err();
        assert!(matches!(error, rutmf::api::Error::Status { body: raw, .. } if raw == body));
    }
}
