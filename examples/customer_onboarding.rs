//! Onboard a customer across two APIs: create a party in TMF632, then engage
//! it as a customer in TMF629 — the split TM Forum draws between *who someone
//! is* and *what role they play*.
//!
//! Runs against the in-process mock, so no network is needed:
//!
//! ```console
//! cargo run --example customer_onboarding --features api-tmf629,api-tmf632,mock
//! ```

use futures::StreamExt;

use rutmf::api::{HubCreate, HubOps, Query, tmf629::CustomerClient, tmf632::PartyClient};
use rutmf::core::{EventKind, Party, Ref};
use rutmf::customer::{Customer, CustomerCreate, CustomerUpdate};
use rutmf::mock::MockTmfServer;
use rutmf::party::{ContactMedium, IndividualCreate, IndividualState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // One mock server standing in for both APIs; in production these would be
    // two hosts, and only the base URL would differ.
    let server = MockTmfServer::new();
    let parties = PartyClient::new(server.base_url(), server.transport())?;
    let customers = CustomerClient::new(server.base_url(), server.transport())?;

    // 1. The party: who this person is, independent of any commercial role.
    println!("== creating the party (TMF632) ==");

    let individual = parties
        .create_individual(
            &IndividualCreate::builder()
                .given_name("Ada")
                .family_name("Lovelace")
                .status(IndividualState::Validated)
                .contact_medium(vec![
                    ContactMedium::email("ada@example.com"),
                    ContactMedium::phone("+49 30 1234567"),
                ])
                .build(),
        )
        .await?;

    let party_id = individual.id.clone().expect("server assigns an id");
    println!(
        "  individual {party_id}: {} {}",
        individual.given_name.as_deref().unwrap_or("-"),
        individual.family_name.as_deref().unwrap_or("-")
    );
    for medium in individual.contact_medium.iter().flatten() {
        println!(
            "    {:?}: {}",
            medium.kind(),
            medium
                .email_address
                .as_deref()
                .or(medium.phone_number.as_deref())
                .unwrap_or("-")
        );
    }

    // 2. The role: that party, engaged as a customer.
    println!("\n== engaging them as a customer (TMF629) ==");

    let customer = customers
        .create_customer(
            &CustomerCreate::builder()
                .name("Ada Lovelace")
                .engaged_party(Ref::<Party>::new(&party_id).with_name("Ada Lovelace"))
                .status("Active")
                .build(),
        )
        .await?;

    let customer_id = customer.id.clone().expect("server assigns an id");
    println!(
        "  customer {customer_id} -> party {}",
        customer.engaged_party.as_ref().unwrap().id
    );

    // 3. Subscribe to changes. Every client exposes the same hub operations,
    //    and the event class name is derived from the resource type — a
    //    misspelled filter would register happily and then deliver nothing.
    println!("\n== subscribing to events ==");

    let hub = customers
        .register_listener(&HubCreate::for_resource::<Customer>(
            "https://my-service/callbacks/customer",
            EventKind::StateChange,
        ))
        .await?;
    println!(
        "  subscription {} -> {}",
        hub.id.as_deref().unwrap_or("-"),
        hub.callback.as_deref().unwrap_or("-")
    );

    // 4. Suspend the customer. TMF629 v5.0.1 requires a patch to restate `name`
    //    and `engagedParty`, which the type makes non-optional rather than
    //    leaving you to discover via a 400.
    println!("\n== suspending the customer ==");

    let suspended = customers
        .update_customer(
            &customer_id,
            &CustomerUpdate::builder()
                .name("Ada Lovelace")
                .engaged_party(Ref::<Party>::new(&party_id))
                .status("Suspended")
                .status_reason("Payment overdue")
                .build(),
        )
        .await?;

    println!(
        "  status: {:?} ({})",
        suspended.status.as_deref().unwrap_or("-"),
        suspended.status_reason.as_deref().unwrap_or("-")
    );

    // 5. Read the book back.
    println!("\n== all customers ==");
    let mut all = customers.stream_customers(Query::new().limit(50));
    while let Some(customer) = all.next().await {
        let customer = customer?;
        println!(
            "  {:<10} {:<16} {}",
            customer.id.as_deref().unwrap_or("-"),
            customer.name.as_deref().unwrap_or("-"),
            customer.status.as_deref().unwrap_or("-")
        );
    }

    Ok(())
}
