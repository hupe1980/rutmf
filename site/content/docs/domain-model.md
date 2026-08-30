+++
title = "The domain model"
description = "How rutmf turns TM Forum v5 schemas into Rust types: the read/create/update triple, round-trip fidelity, typed references, decimal money and @type polymorphism."
weight = 20
+++

## Three shapes per resource

TMF v5 defines every resource three times, and the three differ in which members
exist and which are required:

| OAS schema | Rust type | Used for |
|---|---|---|
| `ProductOffering` | `ProductOffering` | `GET` responses — everything optional |
| `ProductOffering_FVO` | `ProductOfferingCreate` | `POST` bodies — required members are non-`Option` |
| `ProductOffering_MVO` | `ProductOfferingUpdate` | `PATCH` bodies — server-owned members absent entirely |

Flattening the three into one all-optional struct leaves every mistake to be
caught by the server at runtime. Keeping them apart moves that failure to compile
time:

```rust
use rutmf::product::ProductOfferingCreate;

// `name`, `lifecycle_status` and `last_update` are required by the `_FVO`
// schema. Leaving one out does not compile.
let body = ProductOfferingCreate::builder()
    .name("Business Internet")
    .lifecycle_status("Active")
    .last_update(chrono::Utc::now())
    .build();
```

`ProductOfferingUpdate` has the mirror-image property: `id`, `href` and
`lastUpdate` are assigned by the server, so they are **not fields on the type**
and cannot be sent by accident.

### Where requiredness binds

The v5 schemas mark members required in three places: on create bodies, on a few
patch bodies, and on nested types such as `ProductOfferingRelationship.id`. This
crate enforces the first two and not the third, under one rule:

> **Requiredness binds where the client authors the payload, and relaxes where
> the client parses one.**

A create body is something you construct, so a missing member should be a
compile error. A nested type inside a `GET` response is something a server hands
you, and refusing to parse an entire catalog because one relationship omitted its
`id` serves nobody. The division is checked against the OpenAPI documents in
both directions, so it cannot drift into carelessness.

## Round-trip fidelity

Real TMF deployments are full of vendor extensions. Anything the crate has no
field for is captured in `extensions`, in document order, and re-emitted:

```rust
let json = r#"{"id":"7655","name":"Basic Firewall","@type":"ProductOffering","x-vendor":{"tier":2}}"#;
let offering: ProductOffering = serde_json::from_str(json)?;

assert_eq!(offering.extensions.get("x-vendor").unwrap()["tier"], 2);
assert_eq!(
    serde_json::to_value(&offering)?,
    serde_json::from_str::<serde_json::Value>(json)?,
);
```

**Decoding then re-encoding is lossless by value.** Precisely:

- every member present in the input **with a value** is present in the output,
  with an equal value — including members the crate has no typed field for;
- members within `extensions` keep their relative order;
- a timestamp keeps the UTC offset it arrived with, so
  `2020-09-23T16:42:23-04:00` does not come back as `20:42:23Z`;
- **nothing is invented.** A payload that omits the spec-mandatory `@type` comes
  back without one, because middleware must not add members to what it relays.

```rust
let offering: ProductOffering = serde_json::from_str(r#"{"id":"7655"}"#)?;

assert_eq!(offering.type_name(), "ProductOffering");               // known from the type
assert_eq!(serde_json::to_string(&offering)?, r#"{"id":"7655"}"#); // unchanged
```

It is **not** byte-for-byte: known members are emitted in declaration order, JSON
number formatting is normalised, and fractional seconds are re-emitted in SI
groups. Compare with `serde_json::Value` equality, not string equality.

This property is enforced across all 591 vendored specification examples plus
`proptest` generators.

### The one exception: an explicit `null`

`{"description": null}` on a member the crate *models* reads as absence and is
not re-emitted: `Option<T>` has two states where this needs three, and a
three-state type on every field would cost every caller for a distinction the v5
schemas make almost nowhere. A `null` on an unmodelled member lands in
`extensions` and does round-trip; `proptest` pins which members the exception
applies to.

Where the distinction is real — RFC 7386 makes `null` how a **merge patch removes
a member**, and setting the field to `None` says only that the patch does not
mention it — the `…Update` types say it:

```rust
use rutmf::product::ProductOfferingUpdate;

let update = ProductOfferingUpdate::builder()
    .name("Business Internet")
    .build()
    .deleting("description");

assert!(update.deletes("description"));
// → {"name": "Business Internet", "@type": "ProductOffering", "description": null}
```

`deleting` takes the **wire** name. Under `Patch::Operations` the same edit is
`JsonPatchOp::remove`, which fails against a member that is not there rather than
silently doing nothing.

## Typed references

TMF payloads are graphs stitched together with `…Ref` objects. Because `Ref<T>`
carries its target in the type system, the compiler tracks what points at what:

```rust
use rutmf::core::Ref;
use rutmf::product::ProductSpecification;

let spec: Ref<ProductSpecification> = Ref::new("9881").with_name("Fibre Access");
// Serialises with "@type": "ProductSpecificationRef"
//                 "@referredType": "ProductSpecification"
```

Building a reference *from* a resource returns an `Option`, because an unsaved
resource has no `id` and is therefore not referenceable — a state worth handling
rather than panicking on:

```rust
use rutmf::core::Entity;

let saved = ProductOffering::builder().id("7655").name("Firewall").build();
assert_eq!(saved.reference().unwrap().id, "7655");

let unsaved = ProductOffering::builder().name("Firewall").build();
assert!(unsaved.reference().is_none());
```

With a client in hand, a reference resolves to the resource it names — no
turbofish, no path string. See
[Calling an API](@/docs/calling-an-api.md#following-a-reference).

### Four schemas that are not references

`QuoteItemRef`, `AgreementItemRef`, `ProductOrderItemRef` and
`ProductOfferingQualificationItemRef` are named `…Ref` but address a line
*within* a parent resource and carry no `id` at all. They are their own structs,
because `Ref<T>` requires an `id` and would fail to parse them outright.

## Money is a decimal

The v5 OAS types `Money.value` as `number/float`. Storing money in binary
floating point is a defect no matter what the schema says, so values parse into
`rust_decimal::Decimal`.

The codec accepts a JSON number *or* a string, and re-emits a JSON number that
keeps integers integral — `50` does not become `50.0`. It is public as
`core::decimal_opt` if you need the same behaviour on your own types.

## `@type` polymorphism

TMF630 v5 leans heavily on `@type` / `@baseType` / `@schemaLocation`. Every
entity carries all three as typed fields, and polymorphic families are modelled
as one struct plus a kind enum rather than as one Rust type per schema.

`CharacteristicValueSpecification` has fourteen v5 subclasses; `Characteristic`
twelve; `ContactMedium` five; `Account`, `PartyRole` and `Resource` four each;
`ResourceSpecification` three. Each family is one struct carrying the
union of the subclasses' members, with a `ValueKind` / `ContactMediumKind` /
`AccountKind` / `PartyRoleKind` / `ResourceKind` /
`ResourceSpecificationKind` enum that has an `Other` arm — so an unrecognised
vendor subclass never fails a parse.

Six enumerations cover those seven families — `ValueKind` serves both
characteristic families, which differ only by suffix — and each offers `all()`,
`from_type_name` and `type_name`. That last one keeps the *write* direction typed
too: creating a supplier does not mean spelling `"Supplier"` at the call site.

`tests/coverage.rs` checks each enumeration against its schema's
`discriminator.mapping` in both directions, because neither failure is visible: a
subclass the crate omits reads back as `Other` and cannot be written, and one it
invents is a `@type` no server has a schema for.

```rust
use rutmf::resource::{Resource, ResourceKind};

let json = r#"{"@type":"SoftwareResource","targetPlatform":"linux/arm64"}"#;
let resource: Resource = serde_json::from_str(json)?;

assert_eq!(resource.kind(), ResourceKind::Software);
// TMF639's hierarchy is two levels deep: a SoftwareResource *is* a
// LogicalResource, so it carries `value` too.
assert!(resource.kind().is_logical());
```

Where a `oneOf` unions structurally identical shapes — `PartyRefOrPartyRoleRef`
is the awkward case — the model reads `@type` and dispatches on it. A serde
`untagged` enum would always select the first arm and silently mislabel half the
payloads. Writing one stays typed for the same reason: which arm a
`RelatedParty` carries follows from whether you hand it a `Ref<Party>` or a
`Ref<PartyRole>`, so it is not a variant to pick.

```rust
use rutmf::core::{Party, PartyRole, Ref, RelatedParty};

let buyer = RelatedParty::new("customer", Ref::<Party>::new("4104"));
let agent = RelatedParty::new("salesAgent", Ref::<PartyRole>::new("77"));
```

### A characteristic's class follows from its value

`Characteristic` is the one family where the subclass is a consequence of the
data: a value of `100` is an `IntegerCharacteristic` and nothing else. The
builder cannot help — it sets `@type` before it has seen the value — so there is
a constructor that derives it.

```rust
use rutmf::core::{Characteristic, ValueKind};

let speed = Characteristic::new("downstreamSpeed", 100);
assert_eq!(speed.type_name(), "IntegerCharacteristic");

// `value_kind()` is what the sender *said*; `ValueKind::of_value` is what the
// value *is*. Kept apart, so a payload where they disagree is visible.
assert_eq!(speed.value_kind(), ValueKind::Integer);
assert_eq!(ValueKind::of_value(speed.value.as_ref().unwrap()), ValueKind::Integer);
```

`valueType` is *not* derived: it looks like a JSON type name and is not one. The
corpus uses it for `Quantity` and `Slice5G JSON descriptor` as readily as for
`string`, so it is a domain label only the caller knows.

### One schema name is not always one type

Two decisions here are worth knowing, because both are the opposite of what
name-based deduplication would do:

- **`Product` is one type.** TMF622 and TMF637 declare the `Product` schema byte
  for byte identically, so what a TMF622 order line acts on *is* a TMF637
  inventory record. A product read out of the inventory can be handed straight
  back to an order line with no conversion.
- **`Feature` is two types.** TMF638 and TMF639 both declare a `Feature`, but a
  service feature is constrained by a `ConstraintRef` and a resource feature by a
  `PolicyRef`, and their `FeatureRelationship`s diverge further still. Merging
  them would let you set a member the server silently drops, so `service::Feature`
  and `resource::Feature` are separate types.

## Shapes beyond create-read-update-delete

Not every TM Forum resource is a CRUD resource, and the types follow the
specification rather than a house style.

**Operations modelled as resources.** Cancelling an order (TMF622),
acknowledging or clearing an alarm (TMF642), requesting a bill outside the cycle
(TMF678) — none of these is a `PATCH`. Each is its own collection you `POST` to
and read back. So those types have a `…Create` and **no** `…Update`, because the
specification declares `POST` and `GET` on them and nothing else:

```rust
use rutmf::alarm::AckAlarmCreate;
use rutmf::core::Ref;

// One request acknowledges every matching alarm — the pattern is the point.
let ack = AckAlarmCreate::builder()
    .alarm_pattern(vec![Ref::new("alarm-1"), Ref::new("alarm-2")])
    .ack_system_id("noc-console")
    .ack_user_id("operator-42")
    .build();
```

**Resources that are read-only on purpose.** TMF678 declares no
`POST /customerBill` and no `DELETE`, because an issued invoice is evidence
rather than a record a client owns. `CustomerBillUpdate` goes further: its
`_MVO` declares only `state` and `billCycle`, so the type has no `amountDue`
field at all. A patch that tried to rewrite what the customer owes does not
compile.

**Enumerations that stay open, and lean the safe way.** Every state enum carries
an `Other(String)` arm so an unrecognised value never fails a parse — and the
predicates on them are deliberately conservative:

```rust
use rutmf::alarm::PerceivedSeverity;
use rutmf::ticket::TroubleTicketStatus;

// An unknown severity counts as active: a dashboard that hides what it does
// not understand hides the fault worth looking at.
assert!(PerceivedSeverity::Other("catastrophic".into()).is_active());

// `resolved` is not terminal — a resolved ticket can be reopened. Only
// `closed` ends it, so a poller does not stop early.
assert!(!TroubleTicketStatus::Resolved.is_terminal());
```

Conservative in that direction, and *not* in the other. TMF622 lists the order
states without saying which are final, and `partial` — some items fulfilled,
others not — could go either way. It counts as terminal: the order will not be
worked again, and leaving it out would make the loop everybody writes never
end.

```rust
use rutmf::order::ProductOrderState;

// `while !order.state.is_terminal() { poll().await }` has to terminate…
assert!(ProductOrderState::Partial.is_terminal());
// …which is a separate question from whether it worked.
assert!(!ProductOrderState::Partial.is_success());
```

Where the vocabulary is a *task* — a cancellation request, a qualification, an
alarm task, an on-demand bill, a catalog import — the pair is spelled
`is_finished` / `is_success`, and every task-shaped enum has both. That includes
`core::TaskState`, the one TMF622 and TMF679 share.

The rule is that **if a specification closes a vocabulary, the model does too**,
even where the specification writes the values inline on a single member rather
than as a named schema. TMF642's `Alarm.state`, TMF622's milestone status and
TMF634's connection association type are all inline enumerations, and all three
are Rust enums here. A `String` in their place compiles for `"pointToPoint"`
where the wire says `"pointtoPoint"` — a request the server rejects, with no
compile error to warn you.

The exception is written down rather than assumed: TMF639's `allocationStatus`
stays a `String`, because the specification names its values in prose and
declares no enumeration. Inventing the vocabulary would be guessing at what a
server accepts.

## Construction and evolution

Every entity is `#[non_exhaustive]`, so a TM Forum minor release adding a member
is not a breaking change for you. The cost is that **struct-update syntax does
not work downstream** — `..Default::default()` will not compile on these types.
The builder is the construction path; `Default` exists for the serde path and for
local mutation:

```rust
let mut patch = ProductOfferingUpdate::default();
patch.name = Some("Renamed".into());
```

## Schemas back out

An opt-in `schemars` feature puts `JsonSchema` on every entity, which closes the
loop if you are generating documentation, validating payloads, or building tool
descriptors for an agent. The hand-coded types (`Ref<T>`, `Extensions`,
`PartyOrPartyRole`) carry hand-written implementations, so the generated schema
matches what the codecs actually produce.
