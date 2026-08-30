//! The assurance loop: something breaks, someone is told, someone fixes it,
//! and the customer is credited — TMF642 alarm → TMF621 ticket → TMF678 bill.
//!
//! The other examples are about selling and delivering. This one is about what
//! happens afterwards, and it shows the two shapes that make the assurance APIs
//! different from the commerce ones:
//!
//! - **Operations are resources.** Acknowledging and clearing an alarm are
//!   `POST`s to their own collections, not `PATCH`es. Each acts on a *pattern*,
//!   so one request covers every matching alarm.
//! - **Some things are read-only on purpose.** An issued bill cannot be created
//!   or deleted through the API, and `CustomerBillUpdate` has no `amountDue` —
//!   an invoice is evidence, not a record a client rewrites.
//!
//! Runs against the in-process mock, so no network is needed:
//!
//! ```console
//! cargo run --example assurance_workflow --features api-tmf621,api-tmf642,api-tmf678,mock
//! ```

use rutmf::alarm::{
    AckAlarmCreate, AlarmCreate, AlarmState, AlarmType, AlarmUpdate, ClearAlarmCreate,
    PerceivedSeverity,
};
use rutmf::api::{
    FilterOp, Query, tmf621::TroubleTicketClient, tmf642::AlarmClient, tmf678::CustomerBillClient,
};
use rutmf::bill::{CustomerBillState, CustomerBillUpdate};
use rutmf::core::{Note, Ref, Timestamp};
use rutmf::mock::MockTmfServer;
use rutmf::ticket::{RelatedEntity, TroubleTicketCreate, TroubleTicketStatus, TroubleTicketUpdate};

#[allow(
    clippy::too_many_lines,
    reason = "a worked example reads better as one narrative"
)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // One mock standing in for three APIs; in production these would be three
    // hosts, and only the base URL would differ.
    let server = MockTmfServer::new();
    let alarms = AlarmClient::new(server.base_url(), server.transport())?;
    let tickets = TroubleTicketClient::new(server.base_url(), server.transport())?;
    let bills = CustomerBillClient::new(server.base_url(), server.transport())?;

    // ----------------------------------------------------------------------
    // 1. The network reports a fault.
    // ----------------------------------------------------------------------
    println!("== the alarm (TMF642) ==");

    let alarm = alarms
        .create_alarm(
            &AlarmCreate::builder()
                // TMF642 requires seven members on create — an alarm nobody can
                // categorise, place in time or attribute to a system is not
                // actionable, and the type will not build without them.
                .alarm_type(AlarmType::Equipment)
                .perceived_severity(PerceivedSeverity::Critical)
                .probable_cause("lossOfSignal")
                .alarmed_object(Ref::new("port-3/1/2"))
                .alarm_raised_time(at("2026-08-27T09:14:00Z"))
                .source_system_id("ems-north")
                .state(AlarmState::Raised)
                .specific_problem("Optical port 3/1/2 lost light")
                .service_affecting(true)
                .build(),
        )
        .await?;

    let alarm_id = alarm.id.clone().expect("the server assigns an id");
    println!(
        "  alarm {alarm_id}: {} ({:?}, service-affecting={})",
        display(alarm.specific_problem.as_deref()),
        alarm.perceived_severity,
        alarm.service_affecting.unwrap_or(false),
    );

    // ----------------------------------------------------------------------
    // 2. The NOC acknowledges it — a POST to its own collection, not a PATCH.
    // ----------------------------------------------------------------------
    println!("\n== acknowledging (TMF642 tasks) ==");

    let receipt = alarms
        .acknowledge_alarms(
            &AckAlarmCreate::builder()
                // The pattern is what makes this bulk: one request covers every
                // matching alarm, rather than a loop over identifiers.
                .alarm_pattern(vec![Ref::new(&alarm_id)])
                .ack_system_id("noc-console")
                .ack_user_id("operator-42")
                .build(),
        )
        .await?;

    println!(
        "  ackAlarm {}: {} acknowledged by {}",
        display(receipt.id.as_deref()),
        receipt.alarm_pattern.as_ref().map_or(0, Vec::len),
        display(receipt.ack_user_id.as_deref()),
    );

    // ----------------------------------------------------------------------
    // 3. A human takes it on: a trouble ticket, pointed at the alarm.
    // ----------------------------------------------------------------------
    println!("\n== the ticket (TMF621) ==");

    let ticket = tickets
        .create_trouble_ticket(
            &TroubleTicketCreate::builder()
                .description("Optical port 3/1/2 lost light — customer reports outage")
                .severity("critical")
                .ticket_type("networkFault")
                .name("Site 42 outage")
                // `relatedEntity` is deliberately untyped in TMF621: a ticket
                // may be raised against an alarm, a service or a product, so
                // the target is an `EntityRef`.
                .related_entity(vec![
                    RelatedEntity::builder()
                        .entity(Ref::new(&alarm_id))
                        .role("causingAlarm")
                        .build(),
                ])
                .status(TroubleTicketStatus::InProgress)
                .build(),
        )
        .await?;

    let ticket_id = ticket.id.clone().expect("the server assigns an id");
    println!(
        "  ticket {ticket_id}: {:?} — {}",
        ticket.status,
        display(ticket.name.as_deref()),
    );

    // ----------------------------------------------------------------------
    // 4. The field engineer fixes it and resolves the ticket.
    // ----------------------------------------------------------------------
    println!("\n== resolution ==");

    let resolved = tickets
        .update_trouble_ticket(
            &ticket_id,
            &TroubleTicketUpdate::builder()
                .status(TroubleTicketStatus::Resolved)
                .status_change_reason("Fibre re-spliced at the cabinet")
                .note(vec![Note::new("Replaced SFP; light levels nominal.")])
                .build(),
        )
        .await?;

    let status = resolved.status.clone().expect("a status");
    println!(
        "  ticket {ticket_id}: {status:?} — {}",
        display(resolved.status_change_reason.as_deref()),
    );
    // `resolved` is not terminal: a resolved ticket can be reopened or
    // disputed, and only `closed` ends it. Conflating the two would stop a
    // poller — or a customer-communications job — too early.
    println!("    terminal? {}", status.is_terminal());

    // ----------------------------------------------------------------------
    // 5. The alarm clears. Again a task, because clearing is an assertion by
    //    an operator that the condition is gone — which the network may
    //    disagree with by raising it again.
    // ----------------------------------------------------------------------
    println!("\n== clearing (TMF642 tasks) ==");

    alarms
        .clear_alarms(
            &ClearAlarmCreate::builder()
                .alarm_pattern(vec![Ref::new(&alarm_id)])
                .alarm_cleared_time(at("2026-08-27T11:02:00Z"))
                .clear_system_id("noc-console")
                .clear_user_id("operator-42")
                .build(),
        )
        .await?;

    // X.733 reports "the condition is gone" through the severity itself, so
    // there is no separate `cleared` flag to keep in step with it.
    let cleared = alarms
        .update_alarm(
            &alarm_id,
            &AlarmUpdate::builder()
                .perceived_severity(PerceivedSeverity::Cleared)
                .alarm_cleared_time(at("2026-08-27T11:02:00Z"))
                .build(),
        )
        .await?;

    let severity = cleared.perceived_severity.clone().expect("a severity");
    println!(
        "  alarm {alarm_id}: {severity:?} (active={})",
        severity.is_active()
    );

    // Only the severity moved — the fault record itself is intact.
    println!(
        "    probable cause still: {}",
        display(cleared.probable_cause.as_deref()),
    );

    // ----------------------------------------------------------------------
    // 6. The customer is credited. Bills are the read-mostly end of the crate.
    // ----------------------------------------------------------------------
    println!("\n== the bill (TMF678) ==");

    // A bill is produced by a billing run, so the mock is seeded with one:
    // there is no `create_customer_bill`, because TMF678 declares no
    // `POST /customerBill`.
    server.seed(
        "customerBill",
        serde_json::json!({
            "id": "CB-778",
            "@type": "CustomerBill",
            "billNo": "780123456",
            "state": "sent",
            "amountDue": {"unit": "EUR", "value": 49.99},
            "billingAccount": {"id": "BA-42", "@type": "BillingAccountRef"},
        }),
    );

    let bill = bills.get_customer_bill("CB-778", &Query::new()).await?;
    let state = bill.state.clone().expect("a state");
    println!(
        "  bill {} ({}): {state:?}, outstanding={}",
        display(bill.bill_no.as_deref()),
        display(bill.id.as_deref()),
        state.is_outstanding(),
    );

    // The goodwill credit is applied by the billing system; what the API lets a
    // client do is move the bill's state. `CustomerBillUpdate` has no
    // `amountDue` field at all — an issued invoice is evidence, and TMF678's
    // `_MVO` declares only `state` and `billCycle`.
    let settled = bills
        .update_customer_bill(
            "CB-778",
            &CustomerBillUpdate::builder()
                .state(CustomerBillState::Settled)
                .build(),
        )
        .await?;

    println!(
        "  bill {}: {:?}, outstanding={}",
        display(settled.id.as_deref()),
        settled.state,
        settled.state.clone().expect("a state").is_outstanding(),
    );
    println!(
        "    amount unchanged: {:?} — the patch could not have touched it",
        settled.amount_due.as_ref().and_then(|m| m.value),
    );

    // ----------------------------------------------------------------------
    // 7. What a dashboard would ask for: the alarms still worth looking at.
    // ----------------------------------------------------------------------
    println!("\n== what is still open ==");

    let open = alarms
        .list_alarms(&Query::new().filter_op("perceivedSeverity", FilterOp::Ne, "cleared"))
        .await?;

    println!("  {} alarm(s) still active", open.items.len());
    for alarm in open {
        println!(
            "    {}: {:?}",
            display(alarm.id.as_deref()),
            alarm.perceived_severity
        );
    }

    Ok(())
}

fn at(value: &str) -> Timestamp {
    value.parse().expect("a valid RFC 3339 timestamp")
}

fn display(value: Option<&str>) -> &str {
    value.unwrap_or("-")
}
