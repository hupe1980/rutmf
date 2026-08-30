//! Sync a product catalog: stream every active offering, resolve its
//! specification reference, and report what changed.
//!
//! Runs against the in-process mock so it works with no network:
//!
//! ```console
//! cargo run --example catalog_sync --features api-tmf620,mock
//! ```

use std::collections::HashMap;

use chrono::{TimeZone, Utc};
use futures::StreamExt;
use rust_decimal::Decimal;
use serde_json::json;
use std::str::FromStr;

use rutmf::api::{Query, ResolveRef, tmf620::ProductCatalogClient};
use rutmf::core::{Money, Ref, TimePeriod};
use rutmf::mock::MockTmfServer;
use rutmf::product::{ProductOffering, ProductOfferingCreate, ProductSpecification};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = seeded_client()?;

    // 1. Stream the whole collection. Paging is handled for us; the server is
    //    asked for 25 at a time regardless of how many offerings exist.
    println!("== active offerings ==");

    let mut offerings =
        client.stream_product_offerings(Query::new().filter("lifecycleStatus", "Active").limit(25));

    let mut by_specification: HashMap<String, Vec<String>> = HashMap::new();

    while let Some(offering) = offerings.next().await {
        let offering: ProductOffering = offering?;
        let name = offering.name.as_deref().unwrap_or("<unnamed>");

        // `product_specification` is a `Ref<ProductSpecification>`, so the
        // compiler knows what this reference points at — `resolve` hands back a
        // `ProductSpecification` with no turbofish and no path string.
        let specification = match &offering.product_specification {
            Some(reference) => {
                let spec: ProductSpecification =
                    reference.resolve(client.inner(), &Query::new()).await?;
                format!(
                    "{} ({})",
                    spec.name.as_deref().unwrap_or("<unnamed>"),
                    reference.id
                )
            }
            None => "<none>".to_owned(),
        };

        println!("  {name:<28} spec={specification}");
        by_specification
            .entry(specification)
            .or_default()
            .push(name.to_owned());
    }

    // 2. Create a new offering. The create body is a distinct type, so the
    //    members TMF620 requires on POST cannot be forgotten.
    println!("\n== creating an offering ==");

    let body = ProductOfferingCreate::builder()
        .name("Business Internet 1G")
        .lifecycle_status("Active")
        .last_update(Utc::now())
        .description("Symmetric gigabit fibre for business premises")
        .is_sellable(true)
        .valid_for(TimePeriod::starting(
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        ))
        .product_specification(Ref::<ProductSpecification>::new("9881").with_name("Fibre Access"))
        .build();

    let created = client.create_product_offering(&body).await?;
    println!(
        "  created id={} name={:?}",
        created.id.unwrap_or_default(),
        created.name
    );

    // 3. Summarise.
    println!("\n== offerings per specification ==");
    let mut specs: Vec<_> = by_specification.iter().collect();
    specs.sort();
    for (specification, names) in specs {
        println!("  {specification}: {}", names.join(", "));
    }

    Ok(())
}

/// Builds a client backed by a mock server holding a small catalog.
fn seeded_client() -> Result<ProductCatalogClient, Box<dyn std::error::Error>> {
    let server = MockTmfServer::new();

    for (id, name, status, spec) in [
        ("7655", "Basic Firewall for Business", "Active", "9881"),
        ("7656", "Managed Firewall Premium", "Active", "9881"),
        ("7657", "Legacy DSL", "Retired", "9882"),
        ("7658", "Business Internet 500M", "Active", "9883"),
    ] {
        server.seed(
            "productOffering",
            json!({
                "id": id,
                "href": format!("{}/productOffering/{id}", server.base_url()),
                "name": name,
                "lifecycleStatus": status,
                "isSellable": status == "Active",
                "productSpecification": {
                    "id": spec,
                    "@type": "ProductSpecificationRef",
                    "@referredType": "ProductSpecification",
                },
                "@type": "ProductOffering",
            }),
        );
    }

    // The specifications the offerings point at, so `resolve` has something to
    // find.
    for (id, name) in [
        ("9881", "Cisco Firepower NGFW"),
        ("9882", "Legacy DSL Access"),
        ("9883", "Fibre Access"),
    ] {
        server.seed(
            "productSpecification",
            json!({
                "id": id,
                "href": format!("{}/productSpecification/{id}", server.base_url()),
                "name": name,
                "lifecycleStatus": "Active",
                "@type": "ProductSpecification",
            }),
        );
    }

    // A price, showing that money is a real decimal rather than an f64.
    let monthly = Money::new("EUR", Decimal::from_str("49.99")?);
    server.seed(
        "productOfferingPrice",
        json!({
            "id": "4501",
            "name": "Monthly recurring",
            "priceType": "recurring",
            "recurringChargePeriodType": "monthly",
            "price": {"unit": monthly.unit, "value": monthly.value},
            "@type": "ProductOfferingPrice",
        }),
    );

    // The mock owns its base URL, so there is no string to keep in step.
    Ok(ProductCatalogClient::new(
        server.base_url(),
        server.transport(),
    )?)
}
