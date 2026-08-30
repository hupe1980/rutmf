//! End-to-end tests for the TMF632 (party) and TMF629 (customer) clients,
//! and for the hub/listener subscription pattern they share.

#![cfg(all(feature = "api-tmf629", feature = "api-tmf632", feature = "mock"))]

use futures::StreamExt;
use serde_json::json;

use rutmf::api::{HubCreate, HubOps, Query, tmf629::CustomerClient, tmf632::PartyClient};
use rutmf::core::{EventKind, Party, Ref};
use rutmf::customer::{CustomerCreate, CustomerUpdate};
use rutmf::mock::MockTmfServer;
use rutmf::party::{
    ContactMedium, ContactMediumKind, Individual, IndividualCreate, IndividualState,
};

fn party_client() -> (MockTmfServer, PartyClient) {
    let server = MockTmfServer::new();
    let client = PartyClient::from_host("http://mock.test", server.transport()).unwrap();
    (server, client)
}

#[test]
fn from_host_appends_the_conventional_api_path() {
    let (_server, client) = party_client();
    assert_eq!(
        client.inner().base_url(),
        "http://mock.test/tmf-api/partyManagement/v5"
    );

    let server = MockTmfServer::new();
    let customers = CustomerClient::from_host("http://mock.test/", server.transport()).unwrap();
    // A trailing slash on the host must not produce a doubled separator.
    assert_eq!(
        customers.inner().base_url(),
        "http://mock.test/tmf-api/customerManagement/v5"
    );
}

#[tokio::test]
async fn creates_and_reads_an_individual() {
    let (server, client) = party_client();

    let body = IndividualCreate::builder()
        .given_name("Ada")
        .family_name("Lovelace")
        .status(IndividualState::Validated)
        .contact_medium(vec![ContactMedium::email("ada@example.com")])
        .build();

    let created = client.create_individual(&body).await.unwrap();

    assert_eq!(created.family_name.as_deref(), Some("Lovelace"));
    assert_eq!(created.status, Some(IndividualState::Validated));

    let media = created.contact_medium.as_ref().unwrap();
    assert_eq!(media[0].kind(), ContactMediumKind::Email);
    assert_eq!(media[0].email_address.as_deref(), Some("ada@example.com"));

    assert_eq!(server.collection("individual").len(), 1);
}

#[tokio::test]
async fn filters_and_streams_organizations() {
    let (server, client) = party_client();
    for i in 0..7 {
        server.seed(
            "organization",
            json!({
                "id": i.to_string(),
                "name": format!("Org {i}"),
                "isHeadOffice": i == 0,
                "@type": "Organization",
            }),
        );
    }

    let page = client
        .list_organizations(&Query::new().filter("isHeadOffice", "true"))
        .await
        .unwrap();
    assert_eq!(page.len(), 1);
    assert_eq!(page.items[0].name.as_deref(), Some("Org 0"));

    let all = client
        .stream_organizations(Query::new().limit(2))
        .count()
        .await;
    assert_eq!(all, 7, "stream must cross page boundaries");
}

#[tokio::test]
async fn customer_create_requires_the_engaged_party() {
    let server = MockTmfServer::new();
    let client = CustomerClient::from_host("http://mock.test", server.transport()).unwrap();

    // `name` and `engaged_party` are non-optional: omitting either is a
    // compile error, which is the point of the `_FVO` type.
    let body = CustomerCreate::builder()
        .name("Ada Lovelace")
        .engaged_party(Ref::<Party>::new("4104").with_name("Ada Lovelace"))
        .status("Active")
        .build();

    let created = client.create_customer(&body).await.unwrap();

    assert_eq!(created.name.as_deref(), Some("Ada Lovelace"));
    let engaged = created.engaged_party.as_ref().unwrap();
    assert_eq!(engaged.id, "4104");
    assert_eq!(engaged.at_type, "PartyRef");
    assert_eq!(engaged.referred_type.as_deref(), Some("Party"));
}

#[tokio::test]
async fn customer_patch_restates_the_members_v5_requires() {
    let server = MockTmfServer::new();
    server.seed(
        "customer",
        json!({
            "id": "1",
            "name": "Ada Lovelace",
            "status": "Active",
            "engagedParty": {"id": "4104", "@type": "PartyRef"},
            "@type": "Customer",
        }),
    );
    let client = CustomerClient::from_host("http://mock.test", server.transport()).unwrap();

    // TMF629 v5.0.1 marks name and engagedParty required on the patch schema
    // too, so the type makes restating them mandatory rather than optional.
    let update = CustomerUpdate::builder()
        .name("Ada Lovelace")
        .engaged_party(Ref::<Party>::new("4104"))
        .status("Inactive")
        .build();

    let patched = client.update_customer("1", &update).await.unwrap();
    assert_eq!(patched.status.as_deref(), Some("Inactive"));
    assert_eq!(patched.name.as_deref(), Some("Ada Lovelace"));
}

#[tokio::test]
async fn registers_and_removes_a_listener() {
    let (server, client) = party_client();

    let hub = client
        .register_listener(&HubCreate::for_resource::<Individual>(
            "https://me/callback",
            EventKind::Create,
        ))
        .await
        .unwrap();

    assert_eq!(hub.callback.as_deref(), Some("https://me/callback"));
    assert_eq!(
        hub.query.as_deref(),
        Some("eventType=IndividualCreateEvent"),
        "the event class name is derived from the resource type"
    );
    assert_eq!(
        hub.query.as_deref(),
        Some("eventType=IndividualCreateEvent")
    );
    assert_eq!(server.collection("hub").len(), 1);

    let id = hub.id.clone().expect("server assigns a hub id");
    client.unregister_listener(&id).await.unwrap();
    assert!(server.collection("hub").is_empty());
}

#[tokio::test]
async fn hub_operations_are_available_on_every_client() {
    // The point of `HubOps`: subscription management reads identically
    // regardless of which API you are talking to.
    let server = MockTmfServer::new();
    let customers = CustomerClient::from_host("http://mock.test", server.transport()).unwrap();

    let hub = customers
        .register_listener(&HubCreate::to("https://me/cb"))
        .await
        .unwrap();
    assert_eq!(hub.callback.as_deref(), Some("https://me/cb"));
    assert!(
        hub.query.is_none(),
        "an unfiltered subscription sends no query"
    );
}
