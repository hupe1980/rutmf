//! End-to-end tests for the TMF669 party-role client.
//!
//! TMF669 is the general case of TMF629: a customer is one party role. These
//! cover the two collections, the four `@type` subclasses that add no members,
//! and the thing adding this API was really for — following the role arm of a
//! `relatedParty` to the resource on the other end.

#![cfg(all(feature = "api-tmf669", feature = "mock"))]

use rutmf::api::{Query, ResolveRef, tmf669::PartyRoleClient};
use rutmf::core::{Entity, Party, PartyOrPartyRole, PartyRole, Ref, RelatedParty};
use rutmf::mock::MockTmfServer;
use rutmf::party::{PartyRoleCreate, PartyRoleKind, PartyRoleSpecificationCreate, PartyRoleUpdate};

fn client() -> (MockTmfServer, PartyRoleClient) {
    let server = MockTmfServer::new();
    let client = PartyRoleClient::from_host("http://mock.test", server.transport()).unwrap();
    (server, client)
}

#[test]
fn from_host_appends_the_conventional_api_path() {
    let (_server, client) = client();
    assert_eq!(
        client.inner().base_url(),
        "http://mock.test/tmf-api/partyRoleManagement/v5"
    );
}

#[tokio::test]
async fn creates_a_role_against_the_party_playing_it() {
    let (_server, client) = client();

    // `engagedParty` and `name` are required by TMF669, and the builder will
    // not produce a body without them.
    let role = client
        .create_party_role(
            &PartyRoleCreate::builder()
                .engaged_party(Ref::<Party>::new("party-7"))
                .name("Northwind as supplier")
                .role("supplier")
                .status("active")
                .build(),
        )
        .await
        .expect("the role is created");

    assert_eq!(role.name.as_deref(), Some("Northwind as supplier"));
    assert_eq!(
        role.engaged_party.as_ref().map(|p| p.id.as_str()),
        Some("party-7")
    );
    // Nothing said otherwise, so this is the base class.
    assert_eq!(role.kind(), PartyRoleKind::PartyRole);
}

#[tokio::test]
async fn a_subclass_is_recovered_from_its_type() {
    let (server, client) = client();

    // The four subclasses add no members at all, so a server sending one is
    // distinguishable only by `@type`.
    for (ty, expected) in [
        ("Supplier", PartyRoleKind::Supplier),
        ("Consumer", PartyRoleKind::Consumer),
        ("Producer", PartyRoleKind::Producer),
        ("BusinessPartner", PartyRoleKind::BusinessPartner),
        ("ReSeller", PartyRoleKind::Other("ReSeller".into())),
    ] {
        server.seed(
            "partyRole",
            serde_json::json!({"id": ty, "@type": ty, "name": ty}),
        );
        let role = client
            .get_party_role(ty, &Query::new())
            .await
            .expect("the role is served");
        assert_eq!(role.kind(), expected, "for @type {ty}");
    }
}

#[tokio::test]
async fn a_related_party_role_reference_resolves_to_the_role() {
    let (_server, client) = client();

    let created = client
        .create_party_role(
            &PartyRoleCreate::builder()
                .engaged_party(Ref::<Party>::new("party-7"))
                .name("Northwind as supplier")
                .build(),
        )
        .await
        .expect("the role is created");
    let id = created.id.clone().expect("the server assigns an id");

    // This is what modelling TMF669 was for. Every resource in the crate
    // carries a `relatedParty`, and each entry may name a party *or a role*.
    // Before TMF669 the role arm pointed at a marker with nothing behind it.
    let related = RelatedParty::new("supplier", Ref::<PartyRole>::new(&id));

    let PartyOrPartyRole::Role(reference) = related
        .party_or_party_role
        .as_ref()
        .expect("the reference is set")
    else {
        panic!("expected the role arm");
    };

    // The reference names `PartyRole`; resolving it hands back the entity.
    let fetched = reference
        .resolve(client.inner(), &Query::new())
        .await
        .expect("the reference resolves");
    assert_eq!(fetched.name.as_deref(), Some("Northwind as supplier"));
}

#[tokio::test]
async fn a_role_reference_carries_the_class_the_specification_names() {
    let (_server, client) = client();

    let role = client
        .create_party_role(
            &PartyRoleCreate::builder()
                .engaged_party(Ref::<Party>::new("party-7"))
                .name("Northwind as supplier")
                .build(),
        )
        .await
        .expect("the role is created");

    let reference = role.reference().expect("an id, so a reference");
    let wire = serde_json::to_value(&reference).unwrap();
    assert_eq!(wire["@type"], "PartyRoleRef");
}

#[tokio::test]
async fn patches_a_role_and_serves_the_specification_collection() {
    let (_server, client) = client();

    let role = client
        .create_party_role(
            &PartyRoleCreate::builder()
                .engaged_party(Ref::<Party>::new("party-7"))
                .name("Northwind as supplier")
                .status("active")
                .build(),
        )
        .await
        .expect("the role is created");
    let id = role.id.clone().expect("an id");

    let updated = client
        .update_party_role(
            &id,
            &PartyRoleUpdate::builder()
                .status("terminated")
                .status_reason("contract ended")
                .build(),
        )
        .await
        .expect("the role is patched");
    assert_eq!(updated.status.as_deref(), Some("terminated"));
    // A merge patch touches only what it names.
    assert_eq!(updated.name.as_deref(), Some("Northwind as supplier"));

    let spec = client
        .create_party_role_specification(
            &PartyRoleSpecificationCreate::builder()
                .name("Supplier")
                .version("1.0")
                .build(),
        )
        .await
        .expect("the specification is created");
    assert_eq!(spec.name.as_deref(), Some("Supplier"));

    let page = client
        .list_party_role_specifications(&Query::new())
        .await
        .expect("the collection is served");
    assert_eq!(page.items.len(), 1);
}
