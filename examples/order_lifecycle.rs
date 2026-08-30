//! Place a product order and follow it to completion — the loop from catalog
//! to fulfilment, and what cancellation actually looks like in TMF622.
//!
//! ```console
//! cargo run --example order_lifecycle --features api-tmf620,api-tmf622,mock
//! ```

use futures::StreamExt;
use serde_json::json;

use rutmf::api::{
    Conditional, Query, Tagged, tmf620::ProductCatalogClient, tmf622::ProductOrderClient,
};
use rutmf::core::{Entity, ItemAction, Party, Ref, RelatedParty};
use rutmf::mock::MockTmfServer;
use rutmf::order::{
    CancelProductOrderCreate, InitialProductOrderState, Note, ProductOrder, ProductOrderCreate,
    ProductOrderItemCreate, ProductOrderState, ProductOrderUpdate,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = seeded_catalog();
    let catalog = ProductCatalogClient::new(server.base_url(), server.transport())?;
    let orders = ProductOrderClient::new(server.base_url(), server.transport())?;

    // 1. Find something sellable.
    println!("== choosing from the catalog (TMF620) ==");
    let page = catalog
        .list_product_offerings(&Query::new().filter("isSellable", "true"))
        .await?;
    for offering in &page.items {
        println!(
            "  {} — {}",
            offering.id.as_deref().unwrap_or("-"),
            offering.name.as_deref().unwrap_or("-")
        );
    }

    let chosen = page.items.first().expect("catalog is seeded");
    let offering_ref = chosen
        .reference()
        .expect("a listed offering always carries the id the server assigned");

    // 2. Place the order. The create body carries no server-owned member, and
    //    the initial state is restricted to what a client may legitimately ask.
    println!("\n== placing the order (TMF622) ==");
    let order = orders
        .create_product_order(
            &ProductOrderCreate::builder()
                .product_order_item(vec![ProductOrderItemCreate::add("1", offering_ref.clone())])
                .description("Firewall for the Berlin office")
                .requested_initial_state(InitialProductOrderState::Acknowledged)
                .related_party(vec![RelatedParty::new(
                    "customer",
                    Ref::<Party>::new("4104").with_name("Ada Lovelace"),
                )])
                .note(vec![Note::new("Install before end of quarter")])
                .build(),
        )
        .await?;

    let order_id = order.id.clone().expect("server assigns an id");
    println!(
        "  order {order_id}: {:?}",
        order.description.as_deref().unwrap_or("-")
    );
    for item in order.product_order_item.iter().flatten() {
        println!(
            "    item {} {:?} -> offering {}",
            item.id.as_deref().unwrap_or("-"),
            item.action.clone().unwrap_or(ItemAction::Other("?".into())),
            item.product_offering
                .as_ref()
                .map_or("-", |r| r.id.as_str())
        );
    }

    // 3. The provider works the order. Each step is a state the client reads,
    //    never one it invents.
    println!("\n== fulfilment ==");
    for state in [ProductOrderState::InProgress, ProductOrderState::Completed] {
        let updated = orders
            .update_product_order(
                &order_id,
                &ProductOrderUpdate::builder().state(state).build(),
            )
            .await?;
        let now = updated.state.as_ref().expect("state was just set");
        println!("  {now:?}  terminal={}", now.is_terminal());
    }

    // 4. Two operators editing one order: what stops the second silently
    //    discarding the first.
    demonstrate_concurrent_edit(&orders, &order_id).await?;

    // 5. Cancelling is a request, not an edit.
    demonstrate_cancellation(&orders, offering_ref).await?;

    // 6. The order book.
    println!("\n== order book ==");
    let mut all = orders.stream_product_orders(Query::new().limit(50));
    while let Some(order) = all.next().await {
        let order = order?;
        println!(
            "  {:<4} {:<32} {:?}",
            order.id.as_deref().unwrap_or("-"),
            order.description.as_deref().unwrap_or("-"),
            order
                .state
                .map_or("(none)".to_owned(), |s| format!("{s:?}"))
        );
    }

    Ok(())
}

/// Shows a stale write being refused rather than overwriting.
///
/// A TMF `PATCH` is read-modify-write, so two operators editing different
/// members of one order each overwrite the other — and both get `200`, with
/// nothing in either payload to say so. Reading the order with the `ETag` the
/// server issued and writing back through that tag turns the second write into a
/// `412` instead.
async fn demonstrate_concurrent_edit(
    orders: &ProductOrderClient,
    order_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n== two operators, one order ==");

    // Operator A reads it, and holds the tag that came with it.
    let held: Tagged<ProductOrder> = orders.inner().fetch(order_id, &Query::new()).await?;
    println!("  A read the order at {}", held.etag().expect("a tag"));

    // Operator B edits it in the meantime.
    orders
        .update_product_order(
            order_id,
            &ProductOrderUpdate::builder()
                .note(vec![Note::new("B: escalated by the account team")])
                .build(),
        )
        .await?;
    println!("  B added a note");

    // A writes back what A read. Without the precondition this succeeds and B's
    // note is gone.
    let attempt = held
        .update(
            orders.inner(),
            &ProductOrderUpdate::builder()
                .description("A: Firewall for the Berlin office (revised)")
                .build(),
        )
        .await;

    match attempt {
        Err(error) if error.is_precondition_failed() => {
            println!("  A's write was refused: {error}");
        }
        Err(error) => return Err(error.into()),
        Ok(_) => println!("  A's write landed — the server does not enforce If-Match"),
    }

    // Re-read, and the same write goes through.
    let fresh: Tagged<ProductOrder> = orders.inner().fetch(order_id, &Query::new()).await?;
    let merged = fresh
        .update(
            orders.inner(),
            &ProductOrderUpdate::builder()
                .description("A: Firewall for the Berlin office (revised)")
                .build(),
        )
        .await?;
    println!(
        "  after re-reading: {:?}, and B's note survived ({} note(s))",
        merged.description.as_deref().unwrap_or("-"),
        merged.note.iter().flatten().count()
    );

    Ok(())
}

/// Places a second order and asks for it to be cancelled.
///
/// TMF622 models cancellation as its own resource: the request is created, the
/// provider assesses it, and only then does the order's state move.
async fn demonstrate_cancellation(
    orders: &ProductOrderClient,
    offering: Ref<rutmf::product::ProductOffering>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n== requesting cancellation of a second order ==");

    let second = orders
        .create_product_order(
            &ProductOrderCreate::builder()
                .product_order_item(vec![ProductOrderItemCreate::add("1", offering)])
                .description("Ordered by mistake")
                .build(),
        )
        .await?;
    let second_id = second.id.clone().expect("server assigns an id");

    let request = orders
        .request_cancellation(
            &CancelProductOrderCreate::builder()
                .product_order(Ref::<ProductOrder>::new(&second_id))
                .cancellation_reason("Ordered in error")
                .build(),
        )
        .await?;

    println!(
        "  cancellation request {} for order {}",
        request.id.as_deref().unwrap_or("-"),
        request.product_order.as_ref().unwrap().id
    );
    println!("  (the order itself is untouched until the provider assesses it)");
    Ok(())
}

fn seeded_catalog() -> MockTmfServer {
    let server = MockTmfServer::new();
    for (id, name, sellable) in [
        ("7655", "Basic Firewall for Business", true),
        ("7656", "Managed Firewall Premium", true),
        ("7657", "Legacy DSL", false),
    ] {
        server.seed(
            "productOffering",
            json!({
                "id": id,
                "href": format!("/productOffering/{id}"),
                "name": name,
                "isSellable": sellable,
                "lifecycleStatus": "Active",
                "@type": "ProductOffering",
            }),
        );
    }
    server
}
