//! Walk the chain from what a customer has, down to what it runs on:
//! TMF637 product → TMF638 service → TMF639 resource — and back up to the
//! TMF634 catalog that says what the resource *is*.
//!
//! This is the other half of the commerce loop. `order_lifecycle` ends when an
//! order completes; this one starts from what that order produced.
//!
//! The catalog step is the seam worth noticing: a resource in the inventory
//! points at a `ResourceSpecification` published by a catalog, and because both
//! are real types here, the reference resolves into something you can read.
//!
//! Runs against the in-process mock, so no network is needed:
//!
//! ```console
//! cargo run --example inventory_chain \
//!   --features api-tmf634,api-tmf637,api-tmf638,api-tmf639,mock
//! ```

use rutmf::api::{
    FilterOp, Query, tmf634::ResourceCatalogClient, tmf637::ProductInventoryClient,
    tmf638::ServiceInventoryClient, tmf639::ResourceInventoryClient,
};
use rutmf::core::{Ref, ServiceSpecification};
use rutmf::mock::MockTmfServer;
use rutmf::product::{ProductCreate, ProductStatus};
use rutmf::resource::{
    Resource, ResourceAdministrativeState, ResourceAlarmStatus, ResourceAvailabilityStatus,
    ResourceCreate, ResourceOperationalState, ResourceSpecificationCreate,
};
use rutmf::service::{ServiceCreate, ServiceOperatingStatus, ServiceState, ServiceUpdate};

#[allow(
    clippy::too_many_lines,
    reason = "a worked example reads better as one narrative"
)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // One mock standing in for three APIs; in production these would be three
    // hosts, and only the base URL would differ.
    let server = MockTmfServer::new();
    let catalog = ResourceCatalogClient::new(server.base_url(), server.transport())?;
    let products = ProductInventoryClient::new(server.base_url(), server.transport())?;
    let services = ServiceInventoryClient::new(server.base_url(), server.transport())?;
    let resources = ResourceInventoryClient::new(server.base_url(), server.transport())?;

    // 0. The catalog: what a port *is*, before any exist. TMF634 publishes the
    //    specification; TMF639 instantiates it.
    println!("== the specification (TMF634) ==");

    let spec = catalog
        .create_resource_specification(
            &ResourceSpecificationCreate::builder()
                .name("SFP+ 10G Optical Port")
                .lifecycle_status("Active")
                .description("A 10-gigabit optical port on an access node")
                .build(),
        )
        .await?;

    let spec_id = spec.id.clone().expect("the server assigns an id");
    println!(
        "  specification {spec_id}: {} ({:?})",
        display(spec.name.as_deref()),
        spec.kind(),
    );

    // 1. The resource: a physical port, instantiating that specification.
    //    Status is not one field but several, and they are independent —
    //    X.731, which TMF639 inherits.
    println!("\n== the resource (TMF639) ==");

    let port = resources
        .create_resource(
            &ResourceCreate::builder()
                .name("Optical port 3/1/2")
                .category("PhysicalPort")
                .resource_specification(Ref::new(&spec_id))
                .operational_state(ResourceOperationalState::Enabled)
                .administrative_state(ResourceAdministrativeState::Unlocked)
                .availability_status(ResourceAvailabilityStatus::Online)
                .build(),
        )
        .await?;

    let port_id = port.id.clone().expect("the server assigns an id");
    println!("  resource {port_id}: {}", display(port.name.as_deref()));
    println!(
        "    operational={:?} administrative={:?} availability={:?}",
        port.operational_state, port.administrative_state, port.availability_status
    );

    // 2. The service: what runs on the port.
    println!("\n== the service (TMF638) ==");

    let access = services
        .create_service(
            &ServiceCreate::builder()
                // TMF638 requires both of these on create.
                .state(ServiceState::Active)
                .service_specification(Ref::<ServiceSpecification>::new("SS-broadband"))
                .name("Broadband access")
                .supporting_resource(vec![Ref::<Resource>::new(&port_id)])
                .build(),
        )
        .await?;

    let service_id = access.id.clone().expect("the server assigns an id");
    println!(
        "  service {service_id}: {}",
        display(access.name.as_deref())
    );
    println!("    lifecycle state: {:?}", access.state);
    // `operatingStatus` is absent from the create schema entirely: the network
    // reports it, the client does not assert it.
    println!("    operating status: {:?}", access.operating_status);

    // 3. The product: what the customer actually bought.
    println!("\n== the product (TMF637) ==");

    let subscription = products
        .create_product(
            &ProductCreate::builder()
                .name("Fibre 500")
                .status(ProductStatus::Active)
                .is_customer_visible(true)
                .realizing_service(vec![Ref::new(&service_id)])
                .build(),
        )
        .await?;

    let product_id = subscription.id.clone().expect("the server assigns an id");
    println!(
        "  product {product_id}: {} ({:?})",
        display(subscription.name.as_deref()),
        subscription.status
    );

    // 4. Walk it back down. Each hop is a typed reference, so nothing here is
    //    a string the compiler cannot check.
    println!("\n== walking the chain ==");

    let product = products.get_product(&product_id, &Query::new()).await?;
    println!("  {}", display(product.name.as_deref()));

    for service_ref in product.realizing_service.iter().flatten() {
        let service = services.get_service(&service_ref.id, &Query::new()).await?;
        println!("    └─ {}", display(service.name.as_deref()));

        for resource_ref in service.supporting_resource.iter().flatten() {
            let resource = resources
                .get_resource(&resource_ref.id, &Query::new())
                .await?;
            println!("        └─ {}", display(resource.name.as_deref()));
        }
    }

    // 5. Something goes wrong in the network. The service degrades; its
    //    *lifecycle* state does not change, because the customer still has it.
    println!("\n== the port takes an alarm ==");

    let faulty = resources
        .update_resource(
            &port_id,
            &Resource::builder()
                .availability_status(ResourceAvailabilityStatus::Degraded)
                .alarm_status(vec![ResourceAlarmStatus::Minor])
                .build(),
        )
        .await?;
    println!(
        "  resource: availability={:?} alarms={:?}",
        faulty.availability_status,
        faulty.alarm_status.as_deref().unwrap_or_default()
    );

    let degraded = services
        .update_service(
            &service_id,
            &ServiceUpdate::builder()
                .operating_status(ServiceOperatingStatus::Degraded)
                .build(),
        )
        .await?;
    println!(
        "  service:  state={:?} operating={:?}",
        degraded.state, degraded.operating_status
    );
    println!("  the customer still has the product; it is just not working well.");

    // 6. Find everything unhealthy, using a TMF630 filter operator rather than
    //    fetching the world and filtering client-side.
    println!("\n== every service that is not simply running ==");

    let unhealthy = services
        .list_services(&Query::new().filter_op("operatingStatus", FilterOp::Ne, "running"))
        .await?;
    for service in &unhealthy.items {
        println!(
            "  {:<10} {:<20} {:?}",
            display(service.id.as_deref()),
            display(service.name.as_deref()),
            service.operating_status
        );
    }

    Ok(())
}

fn display(value: Option<&str>) -> &str {
    value.unwrap_or("-")
}
