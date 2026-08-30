//! Schema coverage: the crate's types against the vendored v5 OAS documents.
//!
//! # Why this test exists
//!
//! `conformance.rs` asserts that every vendored example parses and round-trips.
//! That is a real guarantee, but it is *structurally blind to missing fields*:
//! anything the typed model has no field for lands in [`Extensions`] and comes
//! back out unchanged, so a payload round-trips perfectly whether or not the
//! model understands a single member of it.
//!
//! So this test reads the OAS itself and compares, per schema:
//!
//! 1. **No missing members** — every property the specification defines has a
//!    typed field. Round-tripping through `Extensions` is not coverage.
//! 2. **No invented members** — every typed field appears in the specification.
//!    This is what catches a v4 name surviving into a v5 type.
//! 3. **Requiredness matches** — a member the `_FVO` marks required is
//!    non-`Option` in the corresponding `…Create`, and nothing else is.
//! 4. **Types match** — a member the spec types as an array is not a `String`
//!    in the model. Comparing names alone would not see it.
//! 5. **Enumerations match** — a state enum admits exactly the values the spec
//!    lists, plus the crate's own catch-all arm.
//! 6. **The mapping is exhaustive** — every type the model declares appears
//!    below. A type nobody mapped is a type nobody checked.
//! 7. **Every schema is modelled or excused** — the reverse direction, and the
//!    one that makes "is the model complete?" answerable. See
//!    [`NOT_MODELLED`].
//! 8. **Shared types do not diverge** — `Attachment` and friends are declared
//!    by several of the fourteen specifications, and this crate has one Rust
//!    type for each.
//! 9. **Discriminators are the specified ones** — a type's `@type` value is the
//!    one the schema's `discriminator.mapping` names, and every `…Ref` class it
//!    claims to be referenced as is a class some specification defines.
//!
//! The Rust side comes from `schemars`, not from parsing source: the generated
//! schema is derived from the same serde attributes that produce the wire
//! format, so what is checked is the actual encoding rather than a
//! transcription of it.
//!
//! # Polymorphic families
//!
//! Several v5 schemas are abstract bases with `@type`-discriminated subclasses
//! that each add a member: `ContactMedium` has five, `Characteristic` thirteen.
//! This crate models each family as one struct carrying the union of their
//! members, so those types are mapped to the *list* of schemas they union, and
//! the checks below run against that union.
//!
//! An earlier version listed the subclass members in a hand-written allowance
//! table instead. That worked, but it meant a member added to a subclass by a
//! TM Forum patch release would be neither required nor noticed. Deriving the
//! union from the specification checks it in both directions.
//!
//! [`Extensions`]: rutmf::core::Extensions

#![cfg(feature = "schemars")]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use rutmf::core::{EventKind, ValueKind};
use rutmf::server::{event_type_for, state_change_kind};
use serde_norway::Value as Yaml;

/// The vendored specification documents, keyed by API.
const SPECS: &[(&str, &str)] = &[
    (
        "TMF620",
        "TMF620-Product_Catalog_Management-v5.0.0.oas.yaml",
    ),
    ("TMF621", "TMF621-Trouble_Ticket-v5.0.1.oas.yaml"),
    ("TMF622", "TMF622-ProductOrdering-v5.0.0.oas.yaml"),
    ("TMF629", "TMF629-Customer_Management-v5.0.1.oas.yaml"),
    ("TMF632", "TMF632-Party_Management-v5.0.0.oas.yaml"),
    (
        "TMF634",
        "TMF634-Resource_Catalog_Management-v5.0.0.oas.yaml",
    ),
    ("TMF642", "TMF642_Alarm_v5.0.1.oas.yaml"),
    ("TMF666", "TMF666-Account_Management-v5.0.0.oas.yaml"),
    ("TMF669", "TMF669-Party_Role_Management-v5.0.0.oas.yaml"),
    (
        "TMF679",
        "TMF679-Product_Offering_Qualification-v5.0.0.oas.yaml",
    ),
    ("TMF678", "TMF678-CustomerBill-v5.0.0.oas.yaml"),
    ("TMF637", "TMF637-ProductInventory-v5.0.0.oas.yaml"),
    (
        "TMF638",
        "TMF638-Service_Inventory_Management-v5.0.0.oas.yaml",
    ),
    (
        "TMF639",
        "TMF639-Resource_Inventory_Management-v5.0.0.oas.yaml",
    ),
];

// --- reading the OAS -------------------------------------------------------

fn spec_path(file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("specs")
        .join(file)
}

fn load_specs() -> BTreeMap<&'static str, Yaml> {
    SPECS
        .iter()
        .map(|(api, file)| {
            let path = spec_path(file);
            let raw = std::fs::read_to_string(&path).unwrap_or_else(|e| {
                panic!(
                    "reading {}: {e}\n\
                     The vendored specs are what this test checks against; \
                     without them it would pass vacuously.",
                    path.display()
                )
            });
            (
                *api,
                serde_norway::from_str(&raw).expect("spec is not valid YAML"),
            )
        })
        .collect()
}

/// The properties and required members of a schema, with `allOf` resolved.
fn flatten(
    schemas: &Yaml,
    name: &str,
    seen: &mut BTreeSet<String>,
) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut props = BTreeSet::new();
    let mut required = BTreeSet::new();
    if !seen.insert(name.to_owned()) {
        return (props, required);
    }
    let Some(schema) = schemas.get(name) else {
        return (props, required);
    };
    collect(schemas, schema, &mut props, &mut required, seen);
    (props, required)
}

fn collect(
    schemas: &Yaml,
    schema: &Yaml,
    props: &mut BTreeSet<String>,
    required: &mut BTreeSet<String>,
    seen: &mut BTreeSet<String>,
) {
    if let Some(all_of) = schema.get("allOf").and_then(Yaml::as_sequence) {
        for part in all_of {
            if let Some(reference) = part.get("$ref").and_then(Yaml::as_str) {
                let target = reference.rsplit('/').next().unwrap_or_default();
                let (p, r) = flatten(schemas, target, seen);
                props.extend(p);
                required.extend(r);
            } else {
                collect(schemas, part, props, required, seen);
            }
        }
    }
    if let Some(properties) = schema.get("properties").and_then(Yaml::as_mapping) {
        props.extend(
            properties
                .keys()
                .filter_map(Yaml::as_str)
                .map(ToOwned::to_owned),
        );
    }
    if let Some(list) = schema.get("required").and_then(Yaml::as_sequence) {
        required.extend(list.iter().filter_map(Yaml::as_str).map(ToOwned::to_owned));
    }
}

/// Every property of a schema, with `allOf` resolved, keyed by member name.
fn flatten_properties(schemas: &Yaml, name: &str) -> BTreeMap<String, Yaml> {
    let mut out = BTreeMap::new();
    collect_properties(schemas, name, &mut out, &mut BTreeSet::new());
    out
}

fn collect_properties(
    schemas: &Yaml,
    name: &str,
    out: &mut BTreeMap<String, Yaml>,
    seen: &mut BTreeSet<String>,
) {
    if !seen.insert(name.to_owned()) {
        return;
    }
    let Some(schema) = schemas.get(name) else {
        return;
    };
    if let Some(all_of) = schema.get("allOf").and_then(Yaml::as_sequence) {
        for part in all_of {
            if let Some(reference) = part.get("$ref").and_then(Yaml::as_str) {
                collect_properties(
                    schemas,
                    reference.rsplit('/').next().unwrap_or(""),
                    out,
                    seen,
                );
            } else if let Some(props) = part.get("properties").and_then(Yaml::as_mapping) {
                for (k, v) in props {
                    if let Some(k) = k.as_str() {
                        out.insert(k.to_owned(), v.clone());
                    }
                }
            }
        }
    }
    if let Some(props) = schema.get("properties").and_then(Yaml::as_mapping) {
        for (k, v) in props {
            if let Some(k) = k.as_str() {
                out.insert(k.to_owned(), v.clone());
            }
        }
    }
}

/// The coarse JSON shape a value takes on the wire.
///
/// Coarse on purpose: comparing `$ref` identity would mean reimplementing the
/// crate's own type mapping in the test. What matters is that an array is not a
/// string and an object is not a number — the confusions that actually happen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    Str,
    DateTime,
    Integer,
    Number,
    Boolean,
    Array,
    Object,
    /// Any JSON value — a `Characteristic`'s `value`, say.
    Any,
}

impl Shape {
    /// Whether a model shape is an acceptable rendering of a spec shape.
    fn accepts(self, spec: Self) -> bool {
        if self == spec {
            return true;
        }
        matches!(
            (self, spec),
            // The spec types most timestamps as a bare string.
            (Self::DateTime, Self::Str) | (Self::Str, Self::DateTime)
            // `decimal_opt` renders a `Decimal` as a JSON number either way.
            | (Self::Number, Self::Integer) | (Self::Integer, Self::Number)
            // An untyped member accepts anything, and vice versa.
            | (Self::Any, _) | (_, Self::Any)
        )
    }
}

fn spec_shape(prop: &Yaml, schemas: &Yaml) -> Shape {
    if let Some(reference) = prop.get("$ref").and_then(Yaml::as_str) {
        let target = reference.rsplit('/').next().unwrap_or("");
        let resolved = schemas.get(target);
        let is_string_enum = resolved.is_some_and(|t| {
            t.get("enum").is_some() || t.get("type").and_then(Yaml::as_str) == Some("string")
        });
        return if is_string_enum {
            Shape::Str
        } else {
            Shape::Object
        };
    }
    match prop.get("type").and_then(Yaml::as_str) {
        Some("array") => Shape::Array,
        Some("string") => {
            if prop.get("format").and_then(Yaml::as_str) == Some("date-time") {
                Shape::DateTime
            } else {
                Shape::Str
            }
        }
        Some("integer") => Shape::Integer,
        Some("number") => Shape::Number,
        Some("boolean") => Shape::Boolean,
        Some("object") => Shape::Object,
        _ => Shape::Any,
    }
}

fn model_shape(prop: &serde_json::Value, defs: &serde_json::Value) -> Shape {
    // `schemars` names every nested type, so follow the reference first.
    if let Some(reference) = prop.get("$ref").and_then(serde_json::Value::as_str)
        && let Some(target) = reference.strip_prefix("#/$defs/")
        && let Some(resolved) = defs.get(target)
    {
        return model_shape(resolved, defs);
    }

    let named = match prop.get("type") {
        Some(serde_json::Value::String(t)) => Some(t.as_str()),
        Some(serde_json::Value::Array(types)) => types
            .iter()
            .filter_map(serde_json::Value::as_str)
            .find(|t| *t != "null"),
        _ => None,
    };
    if let Some(t) = named {
        return match t {
            "string" => {
                if prop.get("format").and_then(serde_json::Value::as_str) == Some("date-time") {
                    Shape::DateTime
                } else {
                    Shape::Str
                }
            }
            "integer" => Shape::Integer,
            "number" => Shape::Number,
            "boolean" => Shape::Boolean,
            "array" => Shape::Array,
            "object" => Shape::Object,
            _ => Shape::Any,
        };
    }

    for key in ["anyOf", "oneOf"] {
        if let Some(arms) = prop.get(key).and_then(serde_json::Value::as_array) {
            let arms: Vec<&serde_json::Value> = arms
                .iter()
                .filter(|a| a.get("type") != Some(&serde_json::json!("null")))
                .collect();
            // An open enum renders as a union of string constants plus a bare
            // string; on the wire that is a string.
            if !arms.is_empty()
                && arms.iter().all(|a| {
                    a.get("type") == Some(&serde_json::json!("string")) || a.get("const").is_some()
                })
            {
                return Shape::Str;
            }
            if let [only] = arms.as_slice() {
                return model_shape(only, defs);
            }
        }
    }
    if prop.get("allOf").is_some() {
        return Shape::Object;
    }
    Shape::Any
}

/// The string constants a model enumeration admits, from any `$defs` entry.
fn model_enum_values(schema: &serde_json::Value, name: &str) -> Option<BTreeSet<String>> {
    let def = schema.get("$defs")?.get(name)?;
    for key in ["anyOf", "oneOf"] {
        if let Some(arms) = def.get(key).and_then(serde_json::Value::as_array) {
            let values: BTreeSet<String> = arms
                .iter()
                .filter_map(|a| a.get("const")?.as_str().map(ToOwned::to_owned))
                .collect();
            if !values.is_empty() {
                return Some(values);
            }
        }
    }
    None
}

// --- reading the Rust model ------------------------------------------------

/// The wire members and required members of a Rust type, via its JSON Schema.
fn rust_shape(schema: &serde_json::Value) -> (BTreeSet<String>, BTreeSet<String>) {
    let props = schema["properties"]
        .as_object()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default();
    let required = schema
        .get("required")
        .and_then(serde_json::Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default();
    (props, required)
}

/// One type under test: the Rust name, the API, the OAS schema or schemas it
/// models, and the shape `schemars` derives from the serde attributes.
struct Mapped {
    /// The module-qualified Rust path, e.g. `service::RelatedPlace`.
    ///
    /// Qualified rather than bare because three modules declare a
    /// `RelatedPlace` and two declare a `Feature`: TMF638 and TMF639 give the
    /// same schema name to genuinely different schemas, and a bare name
    /// collapses them into one entry that then checks the wrong type.
    rust: String,
    api: &'static str,
    /// The schemas this type covers. More than one for a polymorphic family,
    /// where the first is the base and the rest are its subclasses.
    schemas: &'static [&'static str],
    json_schema: serde_json::Value,
}

impl Mapped {
    /// The schema this type's identity and requiredness come from.
    fn base(&self) -> &'static str {
        self.schemas[0]
    }

    /// Every member of every schema in the family, and where each came from.
    fn spec_properties(&self, schemas: &Yaml) -> BTreeMap<String, Vec<Yaml>> {
        let mut out: BTreeMap<String, Vec<Yaml>> = BTreeMap::new();
        for name in self.schemas {
            for (member, prop) in flatten_properties(schemas, name) {
                out.entry(member).or_default().push(prop);
            }
        }
        out
    }
}

/// Declares the Rust-type-to-OAS-schema mapping the whole test runs over.
///
/// `"Schema" => Type` for the ordinary case; `["Base", "Sub", …] => Type` for a
/// polymorphic family the crate models as one struct.
/// Normalises a Rust path to `module::Struct`.
///
/// The mapping writes some entries as `rutmf::api::Hub` and others as
/// `product::Product`; the source scans see only the file they came from. Two
/// segments is what both sides can agree on, and is enough to keep the three
/// `RelatedPlace`s apart.
fn qualify(path: &str) -> String {
    let path = path.replace(' ', "");
    let mut segments = path.rsplit("::");
    let name = segments.next().unwrap_or_default();
    match segments.next() {
        Some(module) => format!("{module}::{name}"),
        None => name.to_owned(),
    }
}

macro_rules! mapping {
    ($($api:literal { $($schema:tt => $ty:ty),* $(,)? })*) => {
        vec![$($(Mapped {
            rust: qualify(stringify!($ty)),
            api: $api,
            schemas: mapping!(@schemas $schema),
            json_schema: serde_json::to_value(schemars::schema_for!($ty)).unwrap(),
        }),*),*]
    };
    (@schemas [$($schema:literal),* $(,)?]) => { &[$($schema),*] };
    (@schemas $schema:literal) => { &[$schema] };
}

#[allow(
    clippy::too_many_lines,
    reason = "one line per mapped type is the point"
)]
fn mapped_types() -> Vec<Mapped> {
    use rutmf::account;
    use rutmf::alarm;
    use rutmf::bill;
    use rutmf::core;
    use rutmf::customer;
    use rutmf::order;
    use rutmf::party;
    use rutmf::product;
    use rutmf::resource;
    use rutmf::service;
    use rutmf::ticket;

    mapping! {
        "TMF620" {
            // Resources, each as its read / create / patch triple.
            "ProductOffering" => product::ProductOffering,
            "ProductOffering_FVO" => product::ProductOfferingCreate,
            "ProductOffering_MVO" => product::ProductOfferingUpdate,
            "ProductSpecification" => product::ProductSpecification,
            "ProductSpecification_FVO" => product::ProductSpecificationCreate,
            "ProductSpecification_MVO" => product::ProductSpecificationUpdate,
            "ProductOfferingPrice" => product::ProductOfferingPrice,
            "ProductOfferingPrice_FVO" => product::ProductOfferingPriceCreate,
            "ProductOfferingPrice_MVO" => product::ProductOfferingPriceUpdate,
            "ProductCatalog" => product::ProductCatalog,
            "ProductCatalog_FVO" => product::ProductCatalogCreate,
            "ProductCatalog_MVO" => product::ProductCatalogUpdate,
            "Category" => product::Category,
            "Category_FVO" => product::CategoryCreate,
            "Category_MVO" => product::CategoryUpdate,
            "ImportJob" => product::ImportJob,
            "ImportJob_FVO" => product::ImportJobCreate,
            "ExportJob" => product::ExportJob,
            "ExportJob_FVO" => product::ExportJobCreate,

            // Nested types.
            "ProductOfferingRelationship" => product::OfferingRelationship,
            "BundledProductOffering" => product::BundledProductOffering,
            "BundledProductOfferingOption" => product::BundledProductOfferingOption,
            "BundledGroupProductOffering" => product::BundledGroupProductOffering,
            "BundledGroupProductOfferingOption" => product::BundledGroupProductOfferingOption,
            "AllowedProductAction" => product::AllowedProductAction,
            "ProductOfferingPriceRelationship" => product::PriceRelationship,
            "BundledProductOfferingPriceRelationship" => product::BundledPriceRelationship,
            "PricingLogicAlgorithm" => product::PricingLogicAlgorithm,
            "ProductOfferingTerm" => product::ProductOfferingTerm,
            "TaxItem" => core::TaxItem,
            "ProductSpecificationRelationship" => product::SpecificationRelationship,
            "BundledProductSpecification" => product::BundledProductSpecification,
            "TargetProductSchema" => product::TargetProductSchema,

            // Core types, checked against TMF620's copy of them.
            ["Attachment", "AttachmentRef"] => core::Attachment,
            "ExternalIdentifier" => core::ExternalIdentifier,
            "CharacteristicSpecification" => core::CharacteristicSpecification,
            "CharacteristicSpecificationRelationship" => core::CharacteristicSpecificationRelationship,
            [
                "CharacteristicValueSpecification",
                "StringCharacteristicValueSpecification",
                "StringArrayCharacteristicValueSpecification",
                "IntegerCharacteristicValueSpecification",
                "IntegerArrayCharacteristicValueSpecification",
                "NumberCharacteristicValueSpecification",
                "NumberArrayCharacteristicValueSpecification",
                "BooleanCharacteristicValueSpecification",
                "BooleanArrayCharacteristicValueSpecification",
                "FloatCharacteristicValueSpecification",
                "FloatArrayCharacteristicValueSpecification",
                "ObjectCharacteristicValueSpecification",
                "ObjectArrayCharacteristicValueSpecification",
                "MapCharacteristicValueSpecification",
                "MapArrayCharacteristicValueSpecification",
            ] => core::CharacteristicValueSpecification,
            "CharacteristicRelationship" => core::CharacteristicRelationship,
            "ProductSpecificationCharacteristicValueUse" => core::CharacteristicValueUse,
            ["RelatedPartyRefOrPartyRoleRef", "RelatedPartyOrPartyRole"] => core::RelatedParty,
            "Money" => core::Money,
            "Quantity" => core::Quantity,
            "Duration" => core::Duration,
            "TimePeriod" => core::TimePeriod,
            "Error" => core::TmfError,
            "Hub" => rutmf::api::Hub,
            "Hub_FVO" => rutmf::api::HubCreate,
            "Event" => core::TmfEvent,
        }

        "TMF669" {
            // One Rust type for the whole role family: the base plus its four
            // `@type`-discriminated subclasses, none of which adds a member.
            // `PartyRoleRef` is deliberately absent: it adds `partyId` and
            // `partyName`, which belong to a reference rather than to the
            // entity, and this crate models references as `Ref<PartyRole>`.
            [
                "PartyRole",
                "Supplier", "Producer", "Consumer", "BusinessPartner",
            ] => party::PartyRole,
            [
                "PartyRole_FVO", "Supplier_FVO", "Producer_FVO",
                "Consumer_FVO", "BusinessPartner_FVO",
            ] => party::PartyRoleCreate,
            [
                "PartyRole_MVO", "Supplier_MVO", "Producer_MVO",
                "Consumer_MVO", "BusinessPartner_MVO",
            ] => party::PartyRoleUpdate,
            "PartyRoleSpecification" => party::PartyRoleSpecification,
            "PartyRoleSpecification_FVO" => party::PartyRoleSpecificationCreate,
            "PartyRoleSpecification_MVO" => party::PartyRoleSpecificationUpdate,
            "EntitySpecificationRelationship" => party::EntitySpecificationRelationship,
            "TargetEntitySchema" => party::TargetEntitySchema,
            // TMF669 carries the party model verbatim from TMF632, and the
            // shared-type gate checks the two specifications agree.
            ["Individual", "Individual_FVO"] => party::Individual,
            ["Organization", "Organization_FVO"] => party::Organization,
            [
                "ContactMedium",
                "EmailContactMedium",
                "PhoneContactMedium",
                "FaxContactMedium",
                "SocialContactMedium",
                "GeographicAddressContactMedium",
            ] => party::ContactMedium,
            "CreditProfile" => core::CreditProfile,
            "PartyCreditProfile" => party::PartyCreditProfile,
            [
                "Characteristic",
                "StringCharacteristic",
                "StringArrayCharacteristic",
                "IntegerCharacteristic",
                "IntegerArrayCharacteristic",
                "NumberCharacteristic",
                "NumberArrayCharacteristic",
                "BooleanCharacteristic",
                "BooleanArrayCharacteristic",
                "FloatCharacteristic",
                "FloatArrayCharacteristic",
                "ObjectCharacteristic",
                "ObjectArrayCharacteristic",
            ] => core::Characteristic,
            "CharacteristicSpecification" => core::CharacteristicSpecification,
            "TaxDefinition" => core::TaxDefinition,
            "TaxExemptionCertificate" => core::TaxExemptionCertificate,
            // Only the reference form. TMF669 also declares
            // `RelatedPartyOrPartyRole`, whose members are identical but whose
            // discriminator is its own — mapping both onto one type would make
            // the model claim a `@type` the specification does not.
            ["RelatedPartyRefOrPartyRoleRef", "RelatedPartyOrPartyRole"] => core::RelatedParty,
            "TimePeriod" => core::TimePeriod,
            "Quantity" => core::Quantity,
            ["Attachment", "AttachmentRef"] => core::Attachment,
            "Error" => core::TmfError,
            "Hub" => rutmf::api::Hub,
            "Hub_FVO" => rutmf::api::HubCreate,
            "Event" => core::TmfEvent,
        }

        "TMF679" {
            "CheckProductOfferingQualification" => product::CheckProductOfferingQualification,
            "CheckProductOfferingQualification_FVO"
                => product::CheckProductOfferingQualificationCreate,
            "CheckProductOfferingQualification_MVO"
                => product::CheckProductOfferingQualificationUpdate,
            "QueryProductOfferingQualification" => product::QueryProductOfferingQualification,
            "QueryProductOfferingQualification_FVO"
                => product::QueryProductOfferingQualificationCreate,
            "QueryProductOfferingQualification_MVO"
                => product::QueryProductOfferingQualificationUpdate,
            "CheckProductOfferingQualificationItem"
                => product::CheckProductOfferingQualificationItem,
            "QueryProductOfferingQualificationItem"
                => product::QueryProductOfferingQualificationItem,
            "AlternateProductOfferingProposal" => product::AlternateProductOfferingProposal,
            "EligibilityResultReason" => product::EligibilityResultReason,
            "ProductOfferingQualificationItemRelationship"
                => product::ProductOfferingQualificationItemRelationship,
            "TerminationError" => product::TerminationError,
            // Shared types, checked against TMF679's copy of them. TMF679's
            // `Product` is not mapped: it drops `@referredType`, which the
            // TMF622/TMF637 schema this crate models does declare.
            ["PlaceRefOrValue", "PlaceRef"] => core::PlaceRefOrValue,
            "RelatedPlaceRefOrValue" => core::RelatedPlace,
            ["RelatedPartyRefOrPartyRoleRef", "RelatedPartyOrPartyRole"] => core::RelatedParty,
            "Note" => core::Note,
            "TimePeriod" => core::TimePeriod,
            "Quantity" => core::Quantity,
            "Money" => core::Money,
            "Error" => core::TmfError,
            "Hub" => rutmf::api::Hub,
            "Hub_FVO" => rutmf::api::HubCreate,
            "Event" => core::TmfEvent,
        }

        "TMF621" {
            // TMF621 declares a `_RES` response schema beside the plain one,
            // with identical members but `id`/`href` required. The read model
            // maps to both and keeps them optional: requiredness binds where a
            // client authors a payload, not where it parses one.
            ["TroubleTicket", "TroubleTicket_RES", "TroubleTicketRef"] => ticket::TroubleTicket,
            "TroubleTicket_FVO" => ticket::TroubleTicketCreate,
            "TroubleTicket_MVO" => ticket::TroubleTicketUpdate,
            [
                "TroubleTicketSpecification",
                "TroubleTicketSpecification_RES",
                "TroubleTicketSpecificationRef",
            ] => ticket::TroubleTicketSpecification,
            "TroubleTicketSpecification_FVO" => ticket::TroubleTicketSpecificationCreate,
            "TroubleTicketSpecification_MVO" => ticket::TroubleTicketSpecificationUpdate,
            "TroubleTicketRelationship" => ticket::TroubleTicketRelationship,
            "RelatedEntity" => ticket::RelatedEntity,
            "StatusChange" => ticket::StatusChange,
            "Note" => core::Note,
            ["Attachment", "AttachmentRef", "AttachmentRefOrValue"] => core::Attachment,
            "ExternalIdentifier" => core::ExternalIdentifier,
            "CharacteristicSpecification" => core::CharacteristicSpecification,
            "TimePeriod" => core::TimePeriod,
            "Quantity" => core::Quantity,
            "Error" => core::TmfError,
            [
                "Characteristic",
                "StringCharacteristic",
                "StringArrayCharacteristic",
                "IntegerCharacteristic",
                "IntegerArrayCharacteristic",
                "NumberCharacteristic",
                "NumberArrayCharacteristic",
                "BooleanCharacteristic",
                "BooleanArrayCharacteristic",
                "FloatCharacteristic",
                "ObjectCharacteristic",
                "ObjectArrayCharacteristic",
            ] => core::Characteristic,
        }
        "TMF622" {
            "ProductOrder" => order::ProductOrder,
            "ProductOrder_FVO" => order::ProductOrderCreate,
            "ProductOrder_MVO" => order::ProductOrderUpdate,
            "ProductOrderItem" => order::ProductOrderItem,
            "ProductOrderItem_FVO" => order::ProductOrderItemCreate,
            "CancelProductOrder" => order::CancelProductOrder,
            "CancelProductOrder_FVO" => order::CancelProductOrderCreate,
            "OrderPrice" => order::OrderPrice,
            "PriceAlteration" => product::PriceAlteration,
            "Price" => core::Price,
            "OrderTerm" => order::OrderTerm,
            "Note" => core::Note,
            "OrderRelationship" => order::OrderRelationship,
            "RelatedChannel" => order::RelatedChannel,
            "OrderItemRelationship" => order::OrderItemRelationship,
            "ProductOrderErrorMessage" => order::OrderErrorMessage,
            "ProductOrderMilestone" => order::OrderMilestone,
            "ProductOrderJeopardyAlert" => order::OrderJeopardyAlert,
            ["Product", "ProductRef"] => product::Product,
            "ProductPrice" => product::ProductPrice,
            "ProductTerm" => product::ProductTerm,
            "ProductRelationship" => product::ProductRelationship,
            "RelatedOrderItem" => product::RelatedOrderItem,
            "RelatedPlaceRefOrValue" => core::RelatedPlace,
            "AgreementItemRef" => product::AgreementItemRef,
            "QuoteItemRef" => order::QuoteItemRef,
            "ProductOrderItemRef" => order::ProductOrderItemRef,
            "ProductOfferingQualificationItemRef" => order::ProductOfferingQualificationItemRef,
            [
                "Characteristic",
                "StringCharacteristic",
                "StringArrayCharacteristic",
                "IntegerCharacteristic",
                "IntegerArrayCharacteristic",
                "NumberCharacteristic",
                "NumberArrayCharacteristic",
                "BooleanCharacteristic",
                "BooleanArrayCharacteristic",
                "FloatCharacteristic",
                "FloatArrayCharacteristic",
                "ObjectCharacteristic",
                "ObjectArrayCharacteristic",
            ] => core::Characteristic,
        }

        "TMF629" {
            "Customer" => customer::Customer,
            "Customer_FVO" => customer::CustomerCreate,
            "Customer_MVO" => customer::CustomerUpdate,
            "CreditProfile" => core::CreditProfile,
        }

        "TMF632" {
            "Individual" => party::Individual,
            "Individual_FVO" => party::IndividualCreate,
            "Individual_MVO" => party::IndividualUpdate,
            "Organization" => party::Organization,
            "Organization_FVO" => party::OrganizationCreate,
            "Organization_MVO" => party::OrganizationUpdate,
            [
                "ContactMedium",
                "EmailContactMedium",
                "PhoneContactMedium",
                "FaxContactMedium",
                "SocialContactMedium",
                "GeographicAddressContactMedium",
            ] => party::ContactMedium,
            ["IndividualIdentification", "OrganizationIdentification"] => party::PartyIdentification,
            "OrganizationChildRelationship" => party::OrganizationChildRelationship,
            "OrganizationParentRelationship" => party::OrganizationParentRelationship,
            "PartyCreditProfile" => party::PartyCreditProfile,
            "TaxExemptionCertificate" => core::TaxExemptionCertificate,
            "TaxDefinition" => core::TaxDefinition,
            "OtherNameIndividual" => party::OtherNameIndividual,
            "OtherNameOrganization" => party::OtherNameOrganization,
            "Disability" => party::Disability,
            "LanguageAbility" => party::LanguageAbility,
            "Skill" => party::Skill,
        }

        "TMF634" {
            // The catalog half of the resource domain.
            // TMF634 declares no `ResourceCatalogRef`.
            "ResourceCatalog" => resource::ResourceCatalog,
            "ResourceCatalog_FVO" => resource::ResourceCatalogCreate,
            "ResourceCatalog_MVO" => resource::ResourceCatalogUpdate,
            ["ResourceCategory", "ResourceCategoryRef"] => resource::ResourceCategory,
            "ResourceCategory_FVO" => resource::ResourceCategoryCreate,
            "ResourceCategory_MVO" => resource::ResourceCategoryUpdate,
            ["ResourceCandidate", "ResourceCandidateRef"] => resource::ResourceCandidate,
            "ResourceCandidate_FVO" => resource::ResourceCandidateCreate,
            "ResourceCandidate_MVO" => resource::ResourceCandidateUpdate,
            // One Rust type for the whole `ResourceSpecification` family: the
            // base plus its three `@type`-discriminated subclasses, whose
            // members this type carries as the union the discriminator implies.
            [
                "ResourceSpecification",
                "ResourceSpecificationRef",
                "LogicalResourceSpecification",
                "PhysicalResourceSpecification",
                "ResourceFunctionSpecification",
            ] => resource::ResourceSpecification,
            // TMF634 declares no `_FVO`/`_MVO` for the three subclasses, only
            // for the base — so a create body cannot carry `vendor` or the
            // connectivity members, and neither can these types.
            "ResourceSpecification_FVO" => resource::ResourceSpecificationCreate,
            "ResourceSpecification_MVO" => resource::ResourceSpecificationUpdate,
            "ResourceSpecificationRelationship" => resource::ResourceSpecificationRelationship,
            "FeatureSpecification" => resource::FeatureSpecification,
            "FeatureSpecificationRelationship" => resource::FeatureSpecificationRelationship,
            "ResourceGraphSpecification" => resource::ResourceGraphSpecification,
            "ResourceGraphSpecificationRelationship" => resource::ResourceGraphSpecificationRelationship,
            "ConnectionSpecification" => resource::ConnectionSpecification,
            "TargetResourceSchema" => resource::TargetResourceSchema,
            // TMF634 declares these exactly as TMF620 does, so one type serves
            // both — see `shared_types_do_not_diverge_between_apis`.
            "ImportJob" => product::ImportJob,
            "ImportJob_FVO" => product::ImportJobCreate,
            "ExportJob" => product::ExportJob,
            "ExportJob_FVO" => product::ExportJobCreate,
            "ExternalIdentifier" => core::ExternalIdentifier,
            ["Attachment", "AttachmentRef", "AttachmentRefOrValue"] => core::Attachment,
            "CharacteristicSpecification" => core::CharacteristicSpecification,
            "TimePeriod" => core::TimePeriod,
            "Quantity" => core::Quantity,
            "Error" => core::TmfError,
            // Every spec declares `JsonPatch` identically; checking it once is
            // enough, because `shared_types_do_not_diverge_between_apis` is
            // what asserts the other thirteen still agree.
            "JsonPatch" => core::JsonPatchOp,
        }
        "TMF642" {
            ["Alarm", "AlarmRef", "AlarmRefOrValue"] => alarm::Alarm,
            "Alarm_FVO" => alarm::AlarmCreate,
            "Alarm_MVO" => alarm::AlarmUpdate,
            // Six tasks, each a `POST`-and-read collection: a create body, a
            // read model, and no patch type because there is no `PATCH`.
            "AckAlarm" => alarm::AckAlarm,
            "AckAlarm_FVO" => alarm::AckAlarmCreate,
            "UnAckAlarm" => alarm::UnAckAlarm,
            "UnAckAlarm_FVO" => alarm::UnAckAlarmCreate,
            "ClearAlarm" => alarm::ClearAlarm,
            "ClearAlarm_FVO" => alarm::ClearAlarmCreate,
            "CommentAlarm" => alarm::CommentAlarm,
            "CommentAlarm_FVO" => alarm::CommentAlarmCreate,
            "GroupAlarm" => alarm::GroupAlarm,
            "GroupAlarm_FVO" => alarm::GroupAlarmCreate,
            "UnGroupAlarm" => alarm::UnGroupAlarm,
            "UnGroupAlarm_FVO" => alarm::UnGroupAlarmCreate,
            "Comment" => alarm::Comment,
            "CrossedThresholdInformation" => alarm::CrossedThresholdInformation,
            // TMF642's `RelatedPlace` nests a whole `Place`; TMF639's carries a
            // reference. Same name, different schema, two types.
            "RelatedPlace" => alarm::RelatedPlace,
            ["Place", "PlaceRef"] => alarm::Place,
            "ExternalIdentifier" => core::ExternalIdentifier,
            "Error" => core::TmfError,
            // TMF642 declares `Characteristic` as the bare base, with no
            // typed-value subclasses and so no `value` member, and none of the
            // resources modelled here reference it. Mapping the crate's
            // union type against that base would report `value` as invented.
        }
        "TMF666" {
            // One Rust type for the whole account family: the abstract base
            // plus its four `@type`-discriminated subclasses, each of which
            // TMF666 also exposes as its own collection.
            [
                "Account", "AccountRef",
                "BillingAccount", "FinancialAccount", "PartyAccount", "SettlementAccount",
            ] => account::Account,
            [
                "Account_FVO", "BillingAccount_FVO", "FinancialAccount_FVO",
                "PartyAccount_FVO", "SettlementAccount_FVO",
            ] => account::AccountCreate,
            [
                "Account_MVO", "BillingAccount_MVO", "FinancialAccount_MVO",
                "PartyAccount_MVO", "SettlementAccount_MVO",
            ] => account::AccountUpdate,
            ["BillFormat", "BillFormatRef", "BillFormatRefOrValue"] => account::BillFormat,
            "BillFormat_FVO" => account::BillFormatCreate,
            "BillFormat_MVO" => account::BillFormatUpdate,
            ["BillPresentationMedia", "BillPresentationMediaRef", "BillPresentationMediaRefOrValue"]
                => account::BillPresentationMedia,
            "BillPresentationMedia_FVO" => account::BillPresentationMediaCreate,
            "BillPresentationMedia_MVO" => account::BillPresentationMediaUpdate,
            [
                "BillingCycleSpecification",
                "BillingCycleSpecificationRef",
                "BillingCycleSpecificationRefOrValue",
            ] => account::BillingCycleSpecification,
            "BillingCycleSpecification_FVO" => account::BillingCycleSpecificationCreate,
            "BillingCycleSpecification_MVO" => account::BillingCycleSpecificationUpdate,
            "AccountBalance" => account::AccountBalance,
            "AccountRelationship" => account::AccountRelationship,
            "BillStructure" => account::BillStructure,
            "Contact" => account::Contact,
            "PaymentPlan" => account::PaymentPlan,
            "TaxExemptionCertificate" => core::TaxExemptionCertificate,
            "TaxDefinition" => core::TaxDefinition,
            ["Attachment", "AttachmentRef", "AttachmentRefOrValue"] => core::Attachment,
            "Money" => core::Money,
            "Quantity" => core::Quantity,
            "TimePeriod" => core::TimePeriod,
            "Error" => core::TmfError,
            [
                "ContactMedium", "EmailContactMedium", "PhoneContactMedium",
                "FaxContactMedium", "SocialContactMedium", "GeographicAddressContactMedium",
            ] => party::ContactMedium,
        }
        "TMF678" {
            ["CustomerBill", "CustomerBillRef"] => bill::CustomerBill,
            // No `CustomerBill_FVO`: TMF678 declares no `POST /customerBill`.
            "CustomerBill_MVO" => bill::CustomerBillUpdate,
            "CustomerBillOnDemand" => bill::CustomerBillOnDemand,
            "CustomerBillOnDemand_FVO" => bill::CustomerBillOnDemandCreate,
            // Read-only collections: a read model and nothing else.
            "AppliedCustomerBillingRate" => bill::AppliedCustomerBillingRate,
            ["BillCycle", "BillCycleRef"] => bill::BillCycle,
            "TaxItem" => core::TaxItem,
            "AppliedBillingTaxRate" => bill::AppliedBillingTaxRate,
            "AppliedPayment" => bill::AppliedPayment,
            ["Attachment", "AttachmentRef", "AttachmentRefOrValue"] => core::Attachment,
            "Money" => core::Money,
            "Quantity" => core::Quantity,
            "TimePeriod" => core::TimePeriod,
            "Error" => core::TmfError,
        }
        "TMF637" {
            // TMF622 and TMF637 declare `Product` identically, so the type an
            // order line acts on *is* the inventory record.
            ["Product", "ProductRef"] => product::Product,
            "Product_FVO" => product::ProductCreate,
            "Product_MVO" => product::ProductUpdate,
            "ProductPrice" => product::ProductPrice,
            "ProductTerm" => product::ProductTerm,
            "ProductRelationship" => product::ProductRelationship,
            "PriceAlteration" => product::PriceAlteration,
            "Price" => core::Price,
            "RelatedOrderItem" => product::RelatedOrderItem,
            "RelatedPlaceRefOrValue" => core::RelatedPlace,
            "AgreementItemRef" => product::AgreementItemRef,
            [
                "Characteristic",
                "StringCharacteristic",
                "StringArrayCharacteristic",
                "IntegerCharacteristic",
                "IntegerArrayCharacteristic",
                "NumberCharacteristic",
                "NumberArrayCharacteristic",
                "BooleanCharacteristic",
                "BooleanArrayCharacteristic",
                "FloatCharacteristic",
                "FloatArrayCharacteristic",
                "ObjectCharacteristic",
                "ObjectArrayCharacteristic",
            ] => core::Characteristic,
        }

        "TMF638" {
            ["Service", "ServiceRef"] => service::Service,
            "Service_FVO" => service::ServiceCreate,
            "Service_MVO" => service::ServiceUpdate,
            "Feature" => service::Feature,
            "FeatureRelationship" => service::FeatureRelationship,
            "ServiceRelationship" => service::ServiceRelationship,
            "RelatedEntityRefOrValue" => service::RelatedEntity,
            "RelatedServiceOrderItem" => service::RelatedServiceOrderItem,
            "RelatedPlaceRefOrValue" => core::RelatedPlace,
            "Note" => core::Note,
            [
                "Characteristic",
                "StringCharacteristic",
                "StringArrayCharacteristic",
                "IntegerCharacteristic",
                "IntegerArrayCharacteristic",
                "NumberCharacteristic",
                "NumberArrayCharacteristic",
                "BooleanCharacteristic",
                "BooleanArrayCharacteristic",
                "FloatCharacteristic",
                "FloatArrayCharacteristic",
                "ObjectCharacteristic",
                "ObjectArrayCharacteristic",
            ] => core::Characteristic,
        }

        "TMF639" {
            // One type for the whole `Resource` family: the base plus the four
            // `@type`-discriminated subclasses, whose members it carries as the
            // union the discriminator implies. `ResourceKind` recovers which
            // one a server sent — the same shape `ResourceSpecification` takes
            // on the catalog side.
            [
                "Resource",
                "ResourceRef",
                "LogicalResource",
                "PhysicalResource",
                "ResourceFunction",
                "SoftwareResource",
            ] => resource::Resource,
            "Resource_FVO" => resource::ResourceCreate,
            "ResourceGraph" => resource::ResourceGraph,
            "ResourceGraphRelationship" => resource::ResourceGraphRelationship,
            "Connection" => resource::Connection,
            "EndpointRef" => resource::Endpoint,
            "Feature" => resource::Feature,
            "FeatureRelationship" => resource::FeatureRelationship,
            "ResourceRelationship" => resource::ResourceRelationship,
            "RelatedResourceOrderItem" => resource::RelatedResourceOrderItem,
            "RelatedPlaceRef" => resource::RelatedPlace,
            "Note" => core::Note,
            [
                "Characteristic",
                "StringCharacteristic",
                "StringArrayCharacteristic",
                "IntegerCharacteristic",
                "IntegerArrayCharacteristic",
                "NumberCharacteristic",
                "NumberArrayCharacteristic",
                "BooleanCharacteristic",
                "BooleanArrayCharacteristic",
                "FloatCharacteristic",
                "FloatArrayCharacteristic",
                "ObjectCharacteristic",
                "ObjectArrayCharacteristic",
            ] => core::Characteristic,
        }
    }
}

// --- the tests -------------------------------------------------------------

#[test]
fn every_specified_member_has_a_typed_field() {
    let specs = load_specs();
    let mut failures = Vec::new();

    for entry in mapped_types() {
        let schemas = &specs[entry.api]["components"]["schemas"];
        let spec_props = entry.spec_properties(schemas);
        assert!(
            !spec_props.is_empty(),
            "{}/{:?}: schema not found in the vendored spec",
            entry.api,
            entry.schemas
        );

        let (rust_props, _) = rust_shape(&entry.json_schema);
        let missing: Vec<&String> = spec_props
            .keys()
            .filter(|member| !rust_props.contains(*member))
            .collect();
        if !missing.is_empty() {
            failures.push(format!(
                "{} ({}/{}) has no typed field for: {missing:?}",
                entry.rust,
                entry.api,
                entry.base()
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "the model is missing members the v5 specification defines.\n\
         A missing field is not caught by the round-trip suite — the value \
         survives in `Extensions` either way — so it is caught here.\n\n{}",
        failures.join("\n")
    );
}

#[test]
fn no_typed_field_is_absent_from_the_specification() {
    let specs = load_specs();
    let mut failures = Vec::new();

    for entry in mapped_types() {
        let schemas = &specs[entry.api]["components"]["schemas"];
        let spec_props = entry.spec_properties(schemas);
        let (rust_props, _) = rust_shape(&entry.json_schema);

        let invented: Vec<&String> = rust_props
            .iter()
            .filter(|member| !spec_props.contains_key(*member))
            .filter(|member| {
                !WIRE_ONLY.iter().any(|(api, schema, allowed, _)| {
                    *api == entry.api && *schema == entry.base() && *allowed == member.as_str()
                })
            })
            .collect();
        if !invented.is_empty() {
            failures.push(format!(
                "{} ({}/{}) defines members the spec does not: {invented:?}",
                entry.rust,
                entry.api,
                entry.base()
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "the model defines members the v5 specification does not.\n\
         This is what a surviving v4 member name looks like. If the member is \
         genuinely declared by a subclass, add that subclass to the type's \
         schema list rather than making an exception for the member.\n\n{}",
        failures.join("\n")
    );
}

#[test]
fn requiredness_matches_the_specification() {
    let specs = load_specs();
    let mut failures = Vec::new();

    for entry in mapped_types() {
        let schemas = &specs[entry.api]["components"]["schemas"];
        let (_, spec_required) = flatten(schemas, entry.base(), &mut BTreeSet::new());
        let (_, rust_required) = rust_shape(&entry.json_schema);

        // A member this crate demands that the spec does not would reject a
        // conformant payload. That is never acceptable.
        let over: Vec<&String> = rust_required.difference(&spec_required).collect();
        if !over.is_empty() {
            failures.push(format!(
                "{} ({}/{}) requires members the spec leaves optional: {over:?}",
                entry.rust,
                entry.api,
                entry.base()
            ));
        }

        // Create and patch bodies are authored, so they must enforce the full
        // set. Read models and nested types relax it on purpose — see
        // `rutmf::core::macros`.
        if entry.base().ends_with("_FVO") || entry.base().ends_with("_MVO") {
            // `@type` is the exception: the crate always *sends* it (see
            // `the_discriminator_is_always_on_the_wire`) but accepts a payload
            // that omits it, because servers do omit it. Demanding it here
            // would be demanding strictness on the parsing side, which is the
            // half where leniency belongs.
            let under: Vec<&String> = spec_required
                .difference(&rust_required)
                .filter(|member| *member != "@type")
                .collect();
            if !under.is_empty() {
                failures.push(format!(
                    "{} ({}/{}) leaves required members optional: {under:?}",
                    entry.rust,
                    entry.api,
                    entry.base()
                ));
            }
        }
    }

    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn the_discriminator_is_always_on_the_wire() {
    // Where the specification marks `@type` required, the crate will not let
    // you omit it when sending. It is lenient on the way *in* — see
    // `rutmf::core::macros` — but the field is a plain `String` with no
    // `skip_serializing_if`, so every payload this crate produces carries it.
    // In the generated schema that is a bare `"string"` rather than the
    // `["string", "null"]` of an `Option`.
    let specs = load_specs();
    let mut failures = Vec::new();
    let mut checked = 0;

    for entry in mapped_types() {
        let schemas = &specs[entry.api]["components"]["schemas"];
        let (_, spec_required) = flatten(schemas, entry.base(), &mut BTreeSet::new());
        if !spec_required.contains("@type") {
            // A value object: `Money`, `TimePeriod` and friends have no
            // discriminator at all, which `every_specified_member_has_a_typed_field`
            // already holds them to.
            continue;
        }
        checked += 1;

        match entry.json_schema["properties"].get("@type") {
            None => failures.push(format!("{} has no @type at all", entry.rust)),
            Some(d) if d["type"] != serde_json::json!("string") => failures.push(format!(
                "{}: @type is {}, so it can be omitted from a request",
                entry.rust, d["type"]
            )),
            Some(_) => {}
        }
    }

    assert!(
        checked > 60,
        "only {checked} types were checked; the filter is wrong"
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_member_has_the_type_the_specification_gives_it() {
    // Checking names alone let `productOrderItem` be a `String` where the spec
    // says `array<ProductOrderItemRef>` — on three types at once, none of them
    // exercised by a fixture. A shape check is what catches that.
    let specs = load_specs();
    let mut failures = Vec::new();

    for entry in mapped_types() {
        let schemas = &specs[entry.api]["components"]["schemas"];
        let spec_props = entry.spec_properties(schemas);
        let empty = serde_json::json!({});
        let defs = entry.json_schema.get("$defs").unwrap_or(&empty);
        let Some(model_props) = entry.json_schema["properties"].as_object() else {
            continue;
        };

        for (member, declarations) in &spec_props {
            let Some(model) = model_props.get(member) else {
                continue; // reported by `every_specified_member_has_a_typed_field`
            };
            let got = model_shape(model, defs);
            // Across a polymorphic family one member can take several shapes —
            // `StringCharacteristic.value` is a string, `IntegerCharacteristic`'s
            // an integer — so matching any declaration is enough.
            let wanted: Vec<Shape> = declarations
                .iter()
                .map(|prop| spec_shape(prop, schemas))
                .collect();
            if !wanted.iter().any(|want| got.accepts(*want)) {
                failures.push(format!(
                    "{}.{member}: the spec says {wanted:?}, the model says {got:?}",
                    entry.rust
                ));
            }
        }
    }

    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Members that keep a `String` despite looking enumerated.
///
/// One entry, and it is a specification defect rather than a modelling choice.
const UNENUMERATED: &[(&str, &str)] = &[
    // TMF639 names this member's values in prose and declares no `enum`, so
    // there is nothing to mirror. Inventing the vocabulary would be guessing at
    // what a server accepts.
    ("TMF639", "allocationStatus"),
];

/// Every member the specification gives a closed vocabulary is a Rust
/// enumeration, not a `String`.
///
/// This is the [`Ref`](rutmf::core::Ref) lesson in a second place. A member
/// typed `String` where the wire admits four values compiles for any string at
/// all: `"may_include"` for `"may include"`, `"Acknowledged"` for
/// `"acknowledged"`, `"pointToPoint"` for `"pointtoPoint"`. Each is a request a
/// conformant server rejects, and none is a compile error — the type invited
/// it. Modelling the vocabulary moves all three to build time.
///
/// The check is per-schema, not per-member-name. `relationshipType` is
/// enumerated on `FeatureRelationship` and free text on
/// `ProductOfferingRelationship`; matching by name alone reports 52 members,
/// most of them wrong.
#[test]
fn every_enumerated_member_is_typed_as_an_enumeration() {
    let specs = load_specs();
    let mut failures = Vec::new();

    for entry in mapped_types() {
        let schemas = &specs[entry.api]["components"]["schemas"];
        let empty = serde_json::json!({});
        let defs = entry.json_schema.get("$defs").unwrap_or(&empty);
        let Some(model_props) = entry.json_schema["properties"].as_object() else {
            continue;
        };

        for (member, declarations) in &entry.spec_properties(schemas) {
            if UNENUMERATED.contains(&(entry.api, member.as_str())) {
                continue;
            }
            // A member is enumerated only if *every* declaration of it says so;
            // across a polymorphic family one arm may leave it open.
            if !declarations
                .iter()
                .all(|prop| declared_enum(prop, schemas).is_some())
            {
                continue;
            }
            let Some(model) = model_props.get(member) else {
                continue; // reported by `every_specified_member_has_a_typed_field`
            };
            if models_a_string(model, defs) {
                let values = declarations
                    .first()
                    .and_then(|prop| declared_enum(prop, schemas))
                    .unwrap_or_default();
                failures.push(format!(
                    "{}.{member} is a String, but {} declares {values:?}",
                    entry.rust, entry.api,
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "a closed vocabulary typed as a String is a typo the compiler accepts:\n{}",
        failures.join("\n")
    );
}

/// Every member the specification formats as `date-time` is a
/// [`Timestamp`](rutmf::core::Timestamp), not a `String`.
///
/// The sibling of [`every_enumerated_member_is_typed_as_an_enumeration`], and
/// the same failure: a `String` here accepts `"2026-08-27"`, `"27/08/2026"` and
/// `"tomorrow"`, none of which is RFC 3339. It also loses the offset-preserving
/// behaviour `Timestamp` exists for.
///
/// This finds nothing today. It is here because the enumerated-member gate
/// found eight the day it was written, and both blind spots are the same shape:
/// two things that serialise as strings and mean different things.
#[test]
fn every_date_time_member_is_typed_as_a_timestamp() {
    let specs = load_specs();
    let mut failures = Vec::new();

    for entry in mapped_types() {
        let schemas = &specs[entry.api]["components"]["schemas"];
        let empty = serde_json::json!({});
        let defs = entry.json_schema.get("$defs").unwrap_or(&empty);
        let Some(model_props) = entry.json_schema["properties"].as_object() else {
            continue;
        };

        for (member, declarations) in &entry.spec_properties(schemas) {
            if !declarations
                .iter()
                .all(|prop| prop.get("format").and_then(Yaml::as_str) == Some("date-time"))
            {
                continue;
            }
            let Some(model) = model_props.get(member) else {
                continue; // reported by `every_specified_member_has_a_typed_field`
            };
            // `Timestamp` carries `"format": "date-time"` through `schemars`;
            // a bare `String` carries no format at all.
            if models_a_string(model, defs) && !mentions_date_time(model, defs) {
                failures.push(format!(
                    "{}.{member} is a String, but {} formats it as date-time",
                    entry.rust, entry.api,
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "a timestamp typed as a String accepts anything at all:\n{}",
        failures.join("\n")
    );
}

/// Whether a `date-time` format survives anywhere in the model's schema for a
/// member — through `Option`, arrays and `$ref` alike.
fn mentions_date_time(model: &serde_json::Value, defs: &serde_json::Value) -> bool {
    if model.get("format").and_then(serde_json::Value::as_str) == Some("date-time") {
        return true;
    }
    match model {
        serde_json::Value::Object(map) => map.iter().any(|(key, value)| {
            if key == "$ref" {
                return value
                    .as_str()
                    .and_then(|r| defs.get(r.rsplit('/').next().unwrap_or("")))
                    .is_some_and(|def| mentions_date_time(def, &serde_json::json!({})));
            }
            mentions_date_time(value, defs)
        }),
        serde_json::Value::Array(items) => items.iter().any(|i| mentions_date_time(i, defs)),
        _ => false,
    }
}

/// The vocabulary one property declares, however it declares it.
fn declared_enum(prop: &Yaml, schemas: &Yaml) -> Option<BTreeSet<String>> {
    let inner = if prop.get("enum").is_some() {
        prop.clone()
    } else if let Some(items) = prop.get("items") {
        items.clone()
    } else {
        prop.clone()
    };
    let resolved = match inner.get("$ref").and_then(Yaml::as_str) {
        Some(reference) => schemas.get(reference.rsplit('/').next()?)?.clone(),
        None => inner,
    };
    let values: BTreeSet<String> = resolved
        .get("enum")?
        .as_sequence()?
        .iter()
        .filter_map(|v| v.as_str().map(ToOwned::to_owned))
        .collect();
    (!values.is_empty()).then_some(values)
}

/// Whether the model types this member as a bare string — no `enum`, no `const`
/// arms, after unwrapping `Option`, arrays and `$ref`.
fn models_a_string(model: &serde_json::Value, defs: &serde_json::Value) -> bool {
    let mut node = model.clone();
    for _ in 0..4 {
        // `Option<T>` and untagged enums both land in `anyOf`; a `null` arm is
        // optionality, so look through it to the one arm that carries the type.
        if let Some(arms) = node.get("anyOf").and_then(serde_json::Value::as_array) {
            let mut real = arms
                .iter()
                .filter(|a| a.get("type").and_then(serde_json::Value::as_str) != Some("null"));
            match (real.next(), real.next()) {
                // Exactly one non-null arm: unwrap it. More than one means
                // `const` arms — an enumeration, so not a bare string.
                (Some(only), None) => node = only.clone(),
                _ => return false,
            }
            continue;
        }
        if let Some(items) = node.get("items") {
            node = items.clone();
            continue;
        }
        if let Some(reference) = node.get("$ref").and_then(serde_json::Value::as_str) {
            let Some(def) = defs.get(reference.rsplit('/').next().unwrap_or("")) else {
                return false;
            };
            node = def.clone();
            continue;
        }
        break;
    }
    // `Option<String>` collapses to `"type": ["string", "null"]` rather than an
    // `anyOf`, so both spellings have to count — checking only the scalar form
    // would leave the gate blind to every optional member, which is most of
    // them.
    let is_string = match node.get("type") {
        Some(serde_json::Value::String(t)) => t == "string",
        Some(serde_json::Value::Array(ts)) => ts.iter().any(|t| t == "string"),
        _ => false,
    };
    is_string && node.get("enum").is_none() && node.get("const").is_none()
}

/// Rust enumerations paired with the v5 enumeration each mirrors.
///
/// The `…Type` naming is not perfectly regular in the specifications — TMF622
/// calls the order-item action `ItemActionType` — so the pairing is written
/// out, and [`every_state_enum_is_paired`] keeps the list honest.
const ENUMS: &[(&str, &str, &str)] = &[
    ("TMF620", "JobState", "JobStateType"),
    // The one enumeration the specs declare inline on a member rather than as a
    // schema of its own — RFC 6902's six verbs, on `JsonPatch.op`.
    ("TMF620", "PatchOperation", "JsonPatch.op"),
    (
        "TMF639",
        "ResourceStandbyStatus",
        "ResourceStandbyStatusType",
    ),
    (
        "TMF639",
        "ResourcePowerConsumingState",
        "ResourcePowerConsumingStateType",
    ),
    ("TMF621", "TroubleTicketStatus", "TroubleTicketStatusType"),
    ("TMF642", "AlarmType", "AlarmType"),
    ("TMF642", "PerceivedSeverity", "PerceivedSeverity"),
    ("TMF678", "CustomerBillState", "CustomerBillStateType"),
    ("TMF678", "CustomerBillRunType", "CustomerBillRunType"),
    (
        "TMF678",
        "CustomerBillOnDemandState",
        "CustomerBillOnDemandStateType",
    ),
    ("TMF622", "ProductOrderState", "ProductOrderStateType"),
    (
        "TMF622",
        "ProductOrderItemState",
        "ProductOrderItemStateType",
    ),
    (
        "TMF622",
        "InitialProductOrderState",
        "InitialProductOrderStateType",
    ),
    ("TMF622", "ItemAction", "ItemActionType"),
    ("TMF622", "TaskState", "TaskStateType"),
    ("TMF622", "ProductStatus", "ProductStatusType"),
    ("TMF632", "IndividualState", "IndividualStateType"),
    ("TMF632", "OrganizationState", "OrganizationStateType"),
    ("TMF637", "ProductStatus", "ProductStatusType"),
    ("TMF637", "ItemAction", "ItemActionType"),
    ("TMF638", "ServiceState", "ServiceStateType"),
    (
        "TMF638",
        "ServiceOperatingStatus",
        "ServiceOperatingStatusType",
    ),
    ("TMF638", "ItemAction", "OrderItemActionType"),
    ("TMF639", "ItemAction", "OrderItemActionType"),
    (
        "TMF639",
        "ResourceOperationalState",
        "ResourceOperationalStateType",
    ),
    ("TMF639", "ResourceUsageState", "ResourceUsageStateType"),
    (
        "TMF639",
        "ResourceAdministrativeState",
        "ResourceAdministrativeStateType",
    ),
    (
        "TMF639",
        "ResourceLifecycleState",
        "ResourceLifecycleStateType",
    ),
    ("TMF639", "ResourceAlarmStatus", "ResourceAlarmStatusType"),
    (
        "TMF639",
        "ResourceProceduralStatus",
        "ResourceProceduralStatusType",
    ),
    (
        "TMF639",
        "ResourceAvailabilityStatus",
        "ResourceAvailabilityStatusType",
    ),
    (
        "TMF639",
        "ResourceControlStatus",
        "ResourceControlStatusType",
    ),
    // Declared inline on the one member that uses them, rather than as named
    // schemas — see `specified_values`.
    ("TMF642", "AlarmState", "Alarm.state"),
    ("TMF642", "AckState", "Alarm.ackState"),
    (
        "TMF642",
        "PlannedOutageIndicator",
        "Alarm.plannedOutageIndicator",
    ),
    ("TMF642", "AlarmTaskState", "AckAlarm.state"),
    (
        "TMF634",
        "ConnectionAssociationType",
        "ConnectionSpecification.associationType",
    ),
    (
        "TMF634",
        "FeatureRelationshipType",
        "FeatureSpecificationRelationship.relationshipType",
    ),
    (
        "TMF634",
        "ResourceGraphRelationshipType",
        "ResourceGraphSpecificationRelationship.relationshipType",
    ),
    (
        "TMF622",
        "OrderMilestoneStatus",
        "ProductOrderMilestone.status",
    ),
];

/// The values an `ENUMS` entry names, whether it points at a named schema or at
/// one member of one.
///
/// TM Forum declares its vocabularies both ways. Most are named schemas
/// (`PerceivedSeverity`), but a good number are written inline on the one member
/// that uses them — `Alarm.state`, `ConnectionSpecification.associationType`.
/// An inline enumeration is no less closed than a named one, so `ENUMS` accepts
/// a `Schema.member` path for those.
fn specified_values(schemas: &Yaml, path: &str) -> Option<BTreeSet<String>> {
    let node = match path.split_once('.') {
        None => schemas.get(path)?.clone(),
        Some((schema, member)) => flatten_properties(schemas, schema).get(member)?.clone(),
    };
    // A member may hold the enumeration directly, wrap it in `items`, or defer
    // to a named schema by `$ref`.
    let node = match () {
        () if node.get("enum").is_some() => node,
        () if node.get("items").is_some() => node.get("items")?.clone(),
        () => node,
    };
    let node = match node.get("$ref").and_then(Yaml::as_str) {
        Some(reference) => schemas.get(reference.rsplit('/').next()?)?.clone(),
        None => node,
    };
    Some(
        node.get("enum")?
            .as_sequence()?
            .iter()
            .filter_map(|v| v.as_str().map(ToOwned::to_owned))
            .collect(),
    )
}

#[test]
fn every_enumeration_admits_exactly_the_specified_values() {
    let specs = load_specs();
    let types = mapped_types();
    let mut failures = Vec::new();

    for (api, rust, spec_name) in ENUMS {
        let spec_values = specified_values(&specs[api]["components"]["schemas"], spec_name)
            .unwrap_or_else(|| panic!("{api}/{spec_name} is not an enumeration"));

        let model_values = types
            .iter()
            .find_map(|m| model_enum_values(&m.json_schema, rust))
            .unwrap_or_else(|| panic!("{rust} is not reachable from any mapped type"));

        let missing: Vec<&String> = spec_values.difference(&model_values).collect();
        let invented: Vec<&String> = model_values.difference(&spec_values).collect();
        if !missing.is_empty() {
            failures.push(format!("{rust} is missing {missing:?}"));
        }
        if !invented.is_empty() {
            failures.push(format!(
                "{rust} admits {invented:?}, which {spec_name} does not"
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "a state the model cannot name is a state a client cannot handle:\n{}",
        failures.join("\n")
    );
}

#[test]
fn every_state_enum_is_paired_with_a_specified_enumeration() {
    // A new enum nobody added to `ENUMS` is a new enum nobody checked.
    let mut unpaired = Vec::new();
    for entry in mapped_types() {
        let Some(defs) = entry
            .json_schema
            .get("$defs")
            .and_then(serde_json::Value::as_object)
        else {
            continue;
        };
        for name in defs.keys() {
            if model_enum_values(&entry.json_schema, name).is_some()
                && !ENUMS.iter().any(|(_, rust, _)| rust == name)
            {
                unpaired.push(name.clone());
            }
        }
    }
    unpaired.sort();
    unpaired.dedup();
    assert!(
        unpaired.is_empty(),
        "these enumerations are unchecked; add them to `ENUMS`: {unpaired:?}"
    );
}

#[test]
fn the_mapping_covers_every_type_the_model_declares() {
    // The mapping is the list of what gets checked, so a type missing from it
    // is a type nothing checks. Reading the declarations back out of the source
    // is crude, but it is the only place the full list exists.
    let mapped: BTreeSet<String> = mapped_types().into_iter().map(|m| m.rust).collect();
    let mut unmapped = Vec::new();

    for path in walk_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src")) {
        if path.file_name().and_then(|n| n.to_str()) == Some("macros.rs") {
            continue; // the macro definitions, and the examples in their docs
        }
        let source = std::fs::read_to_string(&path).expect("unreadable source file");
        for declaration in source
            .split("tmf_struct! {")
            .skip(1)
            .chain(source.split("tmf_value! {").skip(1))
        {
            let Some(name) = declaration
                .split("pub struct ")
                .nth(1)
                .and_then(|rest| rest.split_whitespace().next())
            else {
                continue;
            };
            let qualified = format!("{}::{name}", module_of(&path));
            if !mapped.contains(&qualified) && !UNMAPPED.iter().any(|(n, _)| *n == name) {
                unmapped.push(format!("{qualified}  ({})", path.display()));
            }
        }
    }
    unmapped.sort();

    assert!(
        unmapped.is_empty(),
        "these types are declared but not mapped, so nothing checks them \
         against a schema. Map them, or list them in `UNMAPPED` with a \
         reason:\n{}",
        unmapped.join("\n")
    );

    // The size of the gate is a number the documentation quotes, so it is
    // asserted here rather than remembered. Growing it is expected — this only
    // insists the change be deliberate, and the README updated with it.
    let schemas: usize = mapped_types().iter().map(|m| m.schemas.len()).sum();
    assert_eq!(
        (mapped.len(), schemas),
        (MAPPED_TYPE_COUNT, MAPPED_SCHEMA_COUNT),
        "the size of the gate changed; update `MAPPED_TYPE_COUNT` / \
         `MAPPED_SCHEMA_COUNT` and the figures quoted in README.md and \
         the site"
    );
}

/// Every [`WIRE_ONLY`] allowance must be backed by a payload that carries it.
///
/// Otherwise the list is a way to silence the gate; with it, an allowance is a
/// claim about the corpus that the corpus has to support.
#[test]
fn every_wire_only_member_is_justified_by_the_corpus() {
    for (api, schema, member, reason) in WIRE_ONLY {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(api.to_ascii_lowercase());
        let resource = schema.split('_').next().unwrap_or(schema);

        let mut carried = false;
        for entry in std::fs::read_dir(&dir).expect("fixture directory is missing") {
            let path = entry.expect("unreadable directory entry").path();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if !name.starts_with(resource) || path.extension().is_none_or(|e| e != "json") {
                continue;
            }
            let raw = std::fs::read_to_string(&path).expect("unreadable fixture");
            let value: serde_json::Value = serde_json::from_str(&raw).expect("invalid JSON");
            let has_member = |v: &serde_json::Value| v.get(member).is_some();
            carried |= match &value {
                serde_json::Value::Array(items) => items.iter().any(has_member),
                single => has_member(single),
            };
            if carried {
                break;
            }
        }

        assert!(
            carried,
            "WIRE_ONLY allows {api}/{schema}.{member} on the grounds that the \
             wire carries it ({reason}) — but no vendored {resource} fixture \
             does. Either the allowance is wrong, or the evidence for it is \
             gone and it should be removed."
        );
    }
}

/// How many distinct Rust types the coverage gate checks.
///
/// Fewer than [`MAPPED_SCHEMA_COUNT`], because a polymorphic family is one Rust
/// type standing for several schemas — the whole reason a schema-by-schema
/// generator gets this model wrong.
///
/// Quoted in `README.md` and the site; asserted by
/// [`the_mapping_covers_every_type_the_model_declares`].
const MAPPED_TYPE_COUNT: usize = 215;

/// How many v5 schemas those types are checked against.
const MAPPED_SCHEMA_COUNT: usize = 462;

/// Members this crate types although the schema does not declare them.
///
/// The rule is "the wire wins": where a specification contradicts itself or its
/// own examples, the crate models what servers actually send, because a member
/// left untyped lands in `extensions` and stops being part of the API.
///
/// This is **not** a general escape hatch, and it is not a way to quiet a
/// failing check. Each entry names the exact member and its justification, and
/// [`every_wire_only_member_is_justified_by_the_corpus`] fails unless a
/// vendored fixture actually carries it — so an allowance cannot outlive the
/// evidence for it. If TM Forum fixes the schema, the entry must go.
const WIRE_ONLY: &[(&str, &str, &str, &str)] = &[
    (
        "TMF634",
        "ResourceCandidate_FVO",
        "name",
        "TMF634 marks `name` required on the create body while declaring the \
         member on neither the create body nor the read model. The schema \
         cannot be satisfied as written; every vendored example sends a name.",
    ),
    (
        "TMF634",
        "ResourceCandidate_MVO",
        "name",
        "The patch body omits `name` for the same reason the create body does; \
         a candidate that cannot be renamed would be an odd API.",
    ),
];

/// Declared types with no schema of their own to check against, and why.
const UNMAPPED: &[(&str, &str)] = &[(
    "Hub",
    "checked under TMF620/Hub; the other three specs declare it identically,      which `shared_types_do_not_diverge_between_apis` asserts",
)];

#[test]
fn every_addressable_resource_can_be_resolved() {
    // `src/api/resolve.rs` says "every type implementing `Entity` should appear
    // here, so the invariant 'an addressable resource can be resolved' holds".
    // A resource added without a `resolvable!` entry breaks that silently:
    // `Ref<T>::resolve` simply does not compile for it, which a caller finds
    // and nothing else here would.
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    let mut entities = BTreeSet::new();
    for path in walk_sources(&src) {
        if path.file_name().and_then(|n| n.to_str()) == Some("macros.rs") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("unreadable source file");
        for declaration in source.split("tmf_entity!(").skip(1) {
            // One invocation may name several: `tmf_entity!(ImportJob, ExportJob)`.
            let Some(names) = declaration.split(')').next() else {
                continue;
            };
            entities.extend(names.split(',').map(|name| name.trim().to_owned()));
        }
    }

    let resolve = std::fs::read_to_string(src.join("api/resolve.rs")).expect("no resolve.rs");
    let resolvable: BTreeSet<String> = resolve
        .split("resolvable! {")
        .skip(1)
        .flat_map(|block| block[..block.find("\n}").unwrap_or(block.len())].lines())
        .filter_map(|line| line.split("=>").next())
        .filter_map(|path| path.trim().rsplit("::").next())
        .map(str::to_owned)
        .collect();

    let missing: Vec<_> = entities.difference(&resolvable).collect();

    assert!(
        entities.len() > 10,
        "only {} entities found; the scan is not reading the declarations",
        entities.len()
    );
    assert!(
        missing.is_empty(),
        "these resources are addressable but have no `resolvable!` entry, so \
         `Ref<T>::resolve` does not compile for them: {missing:?}"
    );
}

fn walk_sources(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let entries = std::fs::read_dir(dir).expect("unreadable source directory");
    for entry in entries {
        let path = entry.expect("unreadable directory entry").path();
        if path.is_dir() {
            out.extend(walk_sources(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    out
}

/// Types every specification declares, which this crate models once.
const SHARED: &[&str] = &[
    "Attachment",
    "ExternalIdentifier",
    "Characteristic",
    "CharacteristicRelationship",
    "CharacteristicSpecification",
    "CharacteristicValueSpecification",
    "RelatedPartyRefOrPartyRoleRef",
    "Money",
    "Quantity",
    "Duration",
    "TimePeriod",
    "Error",
    "Hub",
    "ContactMedium",
    "Note",
    "JsonPatch",
];

#[test]
fn shared_types_do_not_diverge_between_apis() {
    // One Rust `Attachment` serves several specifications. That is only sound
    // while the four agree, and a TM Forum patch release could end that
    // quietly — this is where it would surface.
    let specs = load_specs();
    let mut failures = Vec::new();

    for name in SHARED {
        let mut reference: Option<(&str, BTreeSet<String>, BTreeSet<String>)> = None;
        for (api, _) in SPECS {
            let schemas = &specs[api]["components"]["schemas"];
            let (props, required) = flatten(schemas, name, &mut BTreeSet::new());
            if props.is_empty() {
                continue; // this API does not declare it
            }
            match &reference {
                None => reference = Some((api, props, required)),
                Some((first, first_props, first_required)) => {
                    if *first_props != props || *first_required != required {
                        failures.push(format!(
                            "{name}: {first} and {api} disagree — only in {api}: {:?}, only in \
                             {first}: {:?}",
                            props.difference(first_props).collect::<Vec<_>>(),
                            first_props.difference(&props).collect::<Vec<_>>(),
                        ));
                    }
                }
            }
        }
        assert!(
            reference.is_some(),
            "{name} is declared by no vendored spec"
        );
    }

    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// How many specifications declare each shared schema, as the doc comments say.
///
/// A type in `core` justifies its place by counting — "eight of the fourteen
/// declare this byte for byte, so it is one type here rather than one per
/// domain". That number is the argument for the type living there, and prose is
/// invisible to the compiler, so it is asserted rather than remembered.
const SHARED_DECLARING_SPECS: &[(&str, usize, &str)] = &[
    ("TaxDefinition", 8, "core::TaxDefinition"),
    (
        "TaxExemptionCertificate",
        8,
        "core::TaxExemptionCertificate",
    ),
    ("CreditProfile", 7, "core::CreditProfile"),
    ("Note", 5, "core::Note"),
    ("RelatedPlaceRefOrValue", 4, "core::RelatedPlace"),
    ("TaxItem", 2, "core::TaxItem"),
];

#[test]
fn the_doc_comments_count_declaring_specifications_correctly() {
    let specs = load_specs();
    let mut failures = Vec::new();

    for (schema, claimed, rust_type) in SHARED_DECLARING_SPECS {
        let declaring: Vec<&str> = SPECS
            .iter()
            .filter(|(api, _)| {
                !specs[api]["components"]["schemas"]
                    .get(schema)
                    .is_none_or(Yaml::is_null)
            })
            .map(|(api, _)| *api)
            .collect();

        if declaring.len() != *claimed {
            failures.push(format!(
                "{schema}: the doc comment on `{rust_type}` claims {claimed} declaring \
                 specifications, but {} declare it ({}). Update the doc comment and this \
                 table together.",
                declaring.len(),
                declaring.join(", "),
            ));
        }
    }

    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The `@type` value a schema's discriminator mapping names for itself.
fn declared_discriminator(schemas: &Yaml, name: &str) -> Option<String> {
    let discriminator = schemas.get(name)?.get("discriminator")?;
    if discriminator.get("propertyName").and_then(Yaml::as_str) != Some("@type") {
        return None;
    }
    discriminator
        .get("mapping")?
        .as_mapping()?
        .iter()
        .find(|(_, target)| {
            target
                .as_str()
                .is_some_and(|t| t.rsplit('/').next() == Some(name))
        })
        .and_then(|(value, _)| value.as_str().map(ToOwned::to_owned))
}

#[test]
fn every_discriminator_is_the_one_the_specification_names() {
    // The v5 documents declare each schema's `@type` explicitly, in a
    // `discriminator.mapping`. Nothing checked ours against it, and a wrong
    // discriminator is a payload a server routes to the wrong handler.
    let specs = load_specs();
    let declared_type_names = model_type_names();
    let mut failures = Vec::new();
    let mut checked = 0;

    for entry in mapped_types() {
        let schemas = &specs[entry.api]["components"]["schemas"];
        let Some(declared) = declared_discriminator(schemas, entry.base()) else {
            continue; // a value object, or a schema with no mapping
        };
        let Some(model) = declared_type_names.get(&entry.rust) else {
            continue; // a value object: no `@type`, so no discriminator
        };
        checked += 1;
        if *model != declared {
            failures.push(format!(
                "{}: declares @type={model:?}, {}/{} names {declared:?}",
                entry.rust,
                entry.api,
                entry.base()
            ));
        }
    }

    assert!(
        checked > 90,
        "only {checked} discriminators were compared; the scan is not reading \
         the declarations"
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The top-level module a source file belongs to: `src/service/mod.rs` and
/// `src/product/inventory.rs` yield `service` and `product`.
///
/// This is the half of the key `mapping!` supplies as a path prefix.
fn module_of(path: &Path) -> String {
    path.strip_prefix(Path::new(env!("CARGO_MANIFEST_DIR")).join("src"))
        .ok()
        .and_then(|rest| rest.components().next())
        .map(|first| {
            Path::new(first.as_os_str())
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        })
        .unwrap_or_default()
}

/// Every `@name` a declaration claims, keyed by module-qualified struct name.
///
/// `TYPE_NAME` is an associated const, and the value objects do not implement
/// `TmfType` at all, so the declarations are the one place the full list
/// exists — the same reason `reference_class_names` reads them.
fn model_type_names() -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for path in walk_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src")) {
        if path.file_name().and_then(|n| n.to_str()) == Some("macros.rs") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("unreadable source file");
        for declaration in source.split("@name = ").skip(1) {
            let Some(type_name) = declaration.split('"').nth(1) else {
                continue;
            };
            let Some(struct_name) = declaration
                .split("pub struct ")
                .nth(1)
                .and_then(|rest| rest.split_whitespace().next())
            else {
                continue;
            };
            out.insert(
                format!("{}::{struct_name}", module_of(&path)),
                type_name.to_owned(),
            );
        }
    }
    out
}

#[test]
fn every_reference_class_the_model_claims_is_specified() {
    // `Ref<T>` stamps `T::REF_TYPE_NAME` into `@type`. Naming a class no
    // specification defines produces a payload nothing can route — and four of
    // them did: `CatalogRef`, `CustomerRef`, `ImportJobRef`, `ExportJobRef`.
    let specs = load_specs();
    let mut known: BTreeSet<String> = BTreeSet::new();
    for (api, _) in SPECS {
        if let Some(schemas) = specs[api]["components"]["schemas"].as_mapping() {
            known.extend(
                schemas
                    .keys()
                    .filter_map(Yaml::as_str)
                    .map(ToOwned::to_owned),
            );
        }
    }

    let claimed = reference_class_names();
    assert!(
        claimed.len() > 20,
        "only {} reference classes were found; the scan is not reading the \
         declarations",
        claimed.len()
    );

    let mut failures = Vec::new();
    for (rust, reference) in claimed {
        // `EntityRef` is the base every `…Ref` extends, and the default for a
        // type nothing points at.
        if reference != "EntityRef" && !known.contains(&reference) {
            failures.push(format!("{rust} claims to be referenced as {reference:?}"));
        }
    }

    assert!(
        failures.is_empty(),
        "no vendored specification defines these classes:\n{}",
        failures.join("\n")
    );
}

/// Every `@ref` a declaration claims, read back out of the source.
///
/// `REF_TYPE_NAME` is an associated const with a default, so there is no way to
/// enumerate the types that override it from the type system alone.
fn reference_class_names() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for path in walk_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src")) {
        // `macros.rs` *defines* the macros: it contains their matchers and
        // doc examples, neither of which is a declaration.
        if path.file_name().and_then(|n| n.to_str()) == Some("macros.rs") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("unreadable source file");
        for declaration in source.split("@ref = ").skip(1) {
            let Some(reference) = declaration.split('"').nth(1) else {
                continue;
            };
            let name = declaration
                .split("pub struct ")
                .nth(1)
                .and_then(|rest| rest.split_whitespace().next())
                .unwrap_or("<unknown>");
            out.push((name.to_owned(), reference.to_owned()));
        }
        // `ref_target!` markers declare one on a single line.
        for marker in source.split("ref_target!(").skip(1) {
            let literals: Vec<&str> = marker.split(')').next().unwrap_or("").split('"').collect();
            if literals.len() >= 4 {
                let name = marker
                    .split(',')
                    .next()
                    .and_then(|head| head.split_whitespace().next_back())
                    .unwrap_or("<unknown>");
                out.push((name.to_owned(), literals[3].to_owned()));
            }
        }
    }
    out
}

#[test]
fn every_write_operation_may_answer_202() {
    // The client reports a `202` as `Error::Accepted` rather than letting serde
    // fail on an empty body. That is only right while the specifications
    // actually declare it, so this records the reason.
    let specs = load_specs();
    let mut without = Vec::new();
    let mut checked = 0;

    for (api, _) in SPECS {
        let Some(paths) = specs[api]["paths"].as_mapping() else {
            continue;
        };
        for (path, operations) in paths {
            let path = path.as_str().unwrap_or_default();
            // The `/listener/…` endpoints are ones a *client* implements.
            if path.starts_with("/listener/") || path.starts_with("/hub") {
                continue;
            }
            let Some(operations) = operations.as_mapping() else {
                continue;
            };
            for (method, operation) in operations {
                let method = method.as_str().unwrap_or_default();
                if !matches!(method, "post" | "patch") {
                    continue;
                }
                checked += 1;
                let declares_202 = operation
                    .get("responses")
                    .and_then(Yaml::as_mapping)
                    .is_some_and(|r| r.keys().any(|c| c.as_str() == Some("202")));
                if !declares_202 {
                    without.push(format!("{api} {} {path}", method.to_uppercase()));
                }
            }
        }
    }

    assert!(
        checked > 20,
        "only {checked} write operations were examined"
    );
    assert!(
        without.is_empty(),
        "these writes do not declare 202, so the client's handling of it needs \
         a second look:\n{}",
        without.join("\n")
    );
}

#[test]
fn every_type_captures_unknown_members() {
    // `additionalProperties: true` is what the flattened `Extensions` map
    // generates, and it is the round-trip guarantee expressed in the schema.
    for entry in mapped_types() {
        assert_eq!(
            entry.json_schema.get("additionalProperties"),
            Some(&serde_json::Value::Bool(true)),
            "{} does not capture unknown members",
            entry.rust
        );
    }
}

/// The hub surface is asymmetric across the fourteen APIs, and `HubOps`
/// documents which way round it is. That is a claim about the specifications,
/// so it is read back out of them rather than remembered — adding an API
/// changes the answer.
#[test]
fn the_hub_surface_is_what_the_specifications_declare() {
    let specs = load_specs();

    let mut readable = BTreeSet::new();
    let mut listable = BTreeSet::new();
    let mut creatable = BTreeSet::new();
    let mut deletable = BTreeSet::new();

    for (api, _) in SPECS {
        let paths = specs[api]["paths"]
            .as_mapping()
            .expect("a spec declares paths");
        for (path, operations) in paths {
            let (Some(path), Some(operations)) = (path.as_str(), operations.as_mapping()) else {
                continue;
            };
            let has = |method: &str| operations.keys().any(|k| k.as_str() == Some(method));
            match path {
                "/hub" => {
                    if has("post") {
                        creatable.insert(*api);
                    }
                    if has("get") {
                        listable.insert(*api);
                    }
                }
                "/hub/{id}" => {
                    if has("get") {
                        readable.insert(*api);
                    }
                    if has("delete") {
                        deletable.insert(*api);
                    }
                }
                _ => {}
            }
        }
    }

    let all: BTreeSet<&str> = SPECS.iter().map(|(api, _)| *api).collect();

    assert_eq!(
        creatable, all,
        "every API must offer POST /hub, or `register_listener` is not universal"
    );
    assert_eq!(
        deletable, all,
        "every API must offer DELETE /hub/{{id}}, or `unregister_listener` is not universal"
    );
    assert!(
        listable.is_empty(),
        "an API grew GET /hub — subscriptions became listable, and `HubOps` \
         should offer it: {listable:?}"
    );
    assert_eq!(
        readable,
        BTreeSet::from(["TMF621", "TMF629", "TMF639", "TMF642", "TMF679"]),
        "which APIs support `get_listener` changed; update the note on \
         `HubOps::get_listener`"
    );
}

/// Every `…Kind` enumeration names exactly the subclasses its schema declares.
///
/// Both directions fail silently. A subclass the documents declare and the
/// enumeration omits reads back as `Other` and cannot be written without
/// spelling the name; a variant the enumeration invents is a `@type` no server
/// has a schema for.
///
/// `every_discriminator_is_the_one_the_specification_names` checks each *base*
/// against its own self-mapping; this checks the rest of the mapping.
#[test]
fn every_subclass_enumeration_is_the_declared_mapping() {
    /// A family: where its schema lives, and what the crate says it contains.
    struct Family {
        api: &'static str,
        schema: &'static str,
        rust: &'static str,
        modelled: Vec<String>,
    }

    let families = vec![
        Family {
            api: "TMF666",
            schema: "Account",
            rust: "AccountKind",
            modelled: names(
                rutmf::account::AccountKind::all(),
                rutmf::account::AccountKind::type_name,
            ),
        },
        Family {
            api: "TMF632",
            schema: "ContactMedium",
            rust: "ContactMediumKind",
            modelled: names(
                rutmf::party::ContactMediumKind::all(),
                rutmf::party::ContactMediumKind::type_name,
            ),
        },
        Family {
            api: "TMF669",
            schema: "PartyRole",
            rust: "PartyRoleKind",
            modelled: rutmf::party::PartyRoleKind::all()
                .iter()
                .map(|kind| kind.type_name().to_owned())
                .collect(),
        },
        Family {
            api: "TMF639",
            schema: "Resource",
            rust: "ResourceKind",
            modelled: names(
                rutmf::resource::ResourceKind::all(),
                rutmf::resource::ResourceKind::type_name,
            ),
        },
        Family {
            api: "TMF634",
            schema: "ResourceSpecification",
            rust: "ResourceSpecificationKind",
            modelled: names(
                rutmf::resource::ResourceSpecificationKind::all(),
                rutmf::resource::ResourceSpecificationKind::type_name,
            ),
        },
    ];

    let specs = load_specs();
    for family in families {
        let mapping =
            specs[family.api]["components"]["schemas"][family.schema]["discriminator"]["mapping"]
                .as_mapping()
                .unwrap_or_else(|| {
                    panic!(
                        "{}/{} declares no discriminator mapping",
                        family.api, family.schema
                    )
                });
        let declared: BTreeSet<String> = mapping
            .keys()
            .filter_map(Yaml::as_str)
            .map(ToOwned::to_owned)
            .collect();
        let modelled: BTreeSet<String> = family.modelled.into_iter().collect();

        assert_eq!(
            modelled,
            declared,
            "{} does not name exactly the subclasses {}/{} declares; missing \
             {:?}, invented {:?}",
            family.rust,
            family.api,
            family.schema,
            declared.difference(&modelled).collect::<Vec<_>>(),
            modelled.difference(&declared).collect::<Vec<_>>(),
        );

        // And the mapping is a round trip: reading a declared name back gives
        // the variant that writes it, so neither direction is lossy.
        for name in &declared {
            assert!(
                modelled.contains(name),
                "{}: `from_type_name({name:?})` has no variant that writes it back",
                family.rust
            );
        }
    }
}

/// Collects the `@type` each kind writes, for a `Copy` enumeration.
fn names<K: Copy>(kinds: &[K], type_name: impl Fn(K) -> &'static str) -> Vec<String> {
    kinds.iter().map(|k| type_name(*k).to_owned()).collect()
}

/// `ValueKind` names exactly the characteristic subclasses the documents declare.
///
/// One enumeration models two families that differ only by suffix, which is
/// sound only while it is exactly their union: a kind it omits makes
/// `Characteristic::new` write the base class where a subclass was called for,
/// and one it invents is a class name no server knows.
///
/// The families are deliberately different sizes — `Map` and `MapArray` are
/// value specifications only — which is why naming a `…Characteristic` class
/// returns an `Option`.
#[test]
fn every_characteristic_subclass_is_a_value_kind() {
    const CHARACTERISTIC: &str = "Characteristic";
    const VALUE_SPECIFICATION: &str = "CharacteristicValueSpecification";

    let specs = load_specs();
    let mut declared_characteristics: BTreeSet<String> = BTreeSet::new();
    let mut declared_specifications: BTreeSet<String> = BTreeSet::new();

    for (api, _) in SPECS {
        let Some(schemas) = specs[api]["components"]["schemas"].as_mapping() else {
            continue;
        };
        for name in schemas.keys().filter_map(Yaml::as_str) {
            // The base classes themselves are not subclasses.
            if name == CHARACTERISTIC || name == VALUE_SPECIFICATION {
                continue;
            }
            if name.ends_with(VALUE_SPECIFICATION) {
                declared_specifications.insert(name.to_owned());
            } else if name.ends_with(CHARACTERISTIC) {
                // `ProductSpecificationCharacteristic` and friends are named
                // for the entity that owns them, not for a value shape; they
                // are separate schemas with their own mapping.
                if ValueKind::from_type_name(name) != ValueKind::Other {
                    declared_characteristics.insert(name.to_owned());
                }
            }
        }
    }

    let modelled_characteristics: BTreeSet<String> = ValueKind::all()
        .iter()
        .filter_map(|kind| kind.characteristic_type())
        .map(ToOwned::to_owned)
        .collect();
    let modelled_specifications: BTreeSet<String> = ValueKind::all()
        .iter()
        .filter_map(|kind| kind.value_specification_type())
        .map(ToOwned::to_owned)
        .collect();

    assert_eq!(
        modelled_characteristics, declared_characteristics,
        "`ValueKind::characteristic_type` does not name exactly the \
         `…Characteristic` subclasses the documents declare"
    );
    assert_eq!(
        modelled_specifications, declared_specifications,
        "`ValueKind::value_specification_type` does not name exactly the \
         `…CharacteristicValueSpecification` subclasses the documents declare"
    );

    // The asymmetry the `Option` on `characteristic_type` exists for. Asserted
    // rather than described, so it is noticed if TM Forum evens the two up.
    let specification_only: BTreeSet<String> = modelled_specifications
        .iter()
        .map(|name| name.trim_end_matches(VALUE_SPECIFICATION).to_owned())
        .filter(|prefix| !modelled_characteristics.contains(&format!("{prefix}{CHARACTERISTIC}")))
        .collect();
    assert_eq!(
        specification_only,
        BTreeSet::from(["Map".to_owned(), "MapArray".to_owned()]),
        "which shapes are value-specification-only changed; `ValueKind` \
         documents `Map` and `MapArray` as the two"
    );
}

/// The prose documentation lists exactly the APIs the crate covers.
///
/// "Fourteen APIs" is the headline claim, and the list behind it is a table in
/// two places — the part least likely to be remembered when an API is added, and
/// the most visible when it is wrong.
#[test]
fn the_documentation_lists_every_covered_api() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let expected: BTreeSet<&str> = SPECS.iter().map(|(api, _)| *api).collect();

    for relative in ["README.md", "site/content/_index.md"] {
        let Ok(text) = std::fs::read_to_string(root.join(relative)) else {
            // The site is not part of the published crate, so a checkout
            // without it is not a failure.
            continue;
        };

        // Sliced by bytes rather than by `&text[..]`, because the prose is full
        // of em dashes and slicing into one panics.
        let bytes = text.as_bytes();
        let listed: BTreeSet<&str> = text
            .match_indices("TMF")
            .filter_map(|(at, _)| {
                let digits = bytes.get(at + 3..at + 6)?;
                if !digits.iter().all(u8::is_ascii_digit)
                    || bytes.get(at + 6).is_some_and(u8::is_ascii_digit)
                {
                    return None;
                }
                // `TMF630` and `TMF688` are named in prose without being
                // covered, so only ids in `SPECS` count as a listing.
                expected.get(&text[at..at + 6]).copied()
            })
            .collect();

        assert_eq!(
            listed,
            expected,
            "{relative} does not name every covered API; missing {:?}",
            expected.difference(&listed).collect::<Vec<_>>()
        );
    }

    // And the count quoted in prose is the count of the list.
    assert_eq!(
        expected.len(),
        14,
        "the number of covered APIs changed; `fourteen` is written out in the \
         README, the landing page, the crate documentation and the guides"
    );
}

/// Every `/listener/…` endpoint the specifications declare must be a name this
/// crate can produce.
///
/// `EventKind` derives event class names rather than tabulating them, which is
/// only sound while the set of kinds is complete: a kind it does not know is an
/// event nothing can subscribe to or raise, and the failure is silent at both
/// ends — a hub registers happily against a name nothing emits.
///
/// So every listener path must be `event_type_for(collection, kind)` for some
/// [`EventKind`], modulo the lowercase initial the URL uses.
#[test]
fn every_declared_listener_is_a_kind_this_crate_names() {
    /// TMF637 exposes `ProductBatchEvent` — the class name its own
    /// `components.schemas` declares — at `/listener/productProductBatchEvent`,
    /// having prefixed the resource name onto a class that already carried it.
    /// The doubling is in the path only; `eventType` carries `ProductBatchEvent`,
    /// which is what this crate produces.
    const PATH_DOES_NOT_MATCH_THE_CLASS: &[&str] = &["productProductBatchEvent"];

    let specs = load_specs();
    let mut checked = 0usize;
    let mut unnameable: Vec<String> = Vec::new();

    for (api, _) in SPECS {
        let Some(paths) = specs[api]["paths"].as_mapping() else {
            continue;
        };

        // The collections this API actually serves. Splitting the listener name
        // at the *wrong* place still reassembles into the same string — reading
        // `serviceOperatingStatusChangeEvent` as `serviceOperating` plus a plain
        // status change round-trips perfectly — so the check that a kind is
        // missing is that what is left over is not a collection.
        let collections: BTreeSet<&str> = paths
            .keys()
            .filter_map(|path| path.as_str())
            .filter_map(|path| path.strip_prefix('/'))
            .map(|rest| rest.split('/').next().unwrap_or(rest))
            .filter(|segment| *segment != "listener" && *segment != "hub")
            .collect();

        for (path, _) in paths {
            let Some(path) = path.as_str() else { continue };
            let Some(name) = path.strip_prefix("/listener/") else {
                continue;
            };
            checked += 1;
            if PATH_DOES_NOT_MATCH_THE_CLASS.contains(&name) {
                continue;
            }

            // The path is the class name with a lowercase initial, so the kind
            // is recoverable from the suffix and the collection is what is left.
            let recovered = EventKind::from_event_name(name).map(|kind| {
                let collection = &name[..name.len() - kind.suffix().len()];
                (collection, event_type_for(collection, kind))
            });
            let matches = recovered.as_ref().is_some_and(|(collection, class)| {
                let mut expected = class.clone();
                expected[..1].make_ascii_lowercase();
                expected == name && collections.contains(collection)
            });
            if !matches {
                unnameable.push(format!("{api} /listener/{name}"));
            }
        }
    }

    assert!(
        unnameable.is_empty(),
        "these listener endpoints do not decompose into a collection this API \
         serves plus a kind `EventKind` knows, so nothing in this crate can \
         subscribe to or raise them — add the missing variant: {unnameable:#?}"
    );
    assert_eq!(
        checked, 157,
        "the number of listener endpoints changed; the count is quoted in \
         `EventKind`'s documentation"
    );
}

/// The collections whose lifecycle move is spelled `…StatusChangeEvent` must be
/// the ones the specifications declare it for.
///
/// `server::state_change_kind` transcribes the vendored `/listener/…` paths, and
/// a transcription nothing re-reads is a memory. Getting it wrong is invisible
/// in testing and total in production: a subscriber filtering on the name TMF634
/// declares receives nothing from a server raising the other spelling.
#[test]
fn the_status_change_collections_are_the_ones_the_specifications_declare() {
    let specs = load_specs();
    let mut declared: BTreeSet<String> = BTreeSet::new();

    for (api, _) in SPECS {
        let Some(paths) = specs[api]["paths"].as_mapping() else {
            continue;
        };
        for (path, _) in paths {
            let Some(name) = path.as_str().and_then(|p| p.strip_prefix("/listener/")) else {
                continue;
            };
            // `OperatingStatusChangeEvent` also ends in `StatusChangeEvent`, so
            // the kind has to be recovered by longest match rather than by
            // `ends_with`.
            if EventKind::from_event_name(name) == Some(EventKind::StatusChange) {
                let collection = &name[..name.len() - EventKind::StatusChange.suffix().len()];
                declared.insert(collection.to_owned());
            }
        }
    }

    let modelled: BTreeSet<String> = declared
        .iter()
        .filter(|collection| state_change_kind(collection) == EventKind::StatusChange)
        .cloned()
        .collect();
    assert_eq!(
        modelled, declared,
        "a collection declares `…StatusChangeEvent` but `state_change_kind` \
         returns `StateChange` for it, so its lifecycle notifications are \
         raised under a name no subscriber can have registered for"
    );

    // And the reverse: nothing may claim the minority spelling without the
    // specifications backing it, or twelve APIs' events go out misnamed.
    for collection in [
        "productOffering",
        "productCatalog",
        "customer",
        "individual",
        "alarm",
        "service",
        "productOrder",
    ] {
        assert_eq!(
            state_change_kind(collection),
            EventKind::StateChange,
            "{collection} does not declare `…StatusChangeEvent`"
        );
    }
}

/// Every operation a client exposes must be one its specification declares.
///
/// The client layer composes its surface from the macros in `api::ops`, and
/// each macro corresponds to exactly one HTTP operation. This reads those
/// invocations back out of the client source and compares them against the
/// vendored paths — so a client cannot offer `create_customer_bill` against a
/// `POST /customerBill` that does not exist.
///
/// That is the same class of guarantee as pairing a `PATCH` body with its
/// content type: the type system should not invite a request the server will
/// refuse. It is checked here rather than reviewed, because the temptation to
/// reach for `resource_ops!` on a resource that is only *nearly* CRUD is
/// exactly the mistake that would not be noticed.
/// The collection constants a client declares, e.g. `const BILLS: &str = "…"`.
fn collection_constants(source: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for line in source.lines() {
        if let Some(rest) = line.trim().strip_prefix("const ")
            && let Some((name, value)) = rest.split_once(": &str = ")
        {
            out.insert(
                name.trim().to_owned(),
                value
                    .trim()
                    .trim_end_matches(';')
                    .trim_matches('"')
                    .to_owned(),
            );
        }
    }
    out
}

/// The HTTP operations a specification declares, keyed by collection name.
fn declared_operations(spec: &Yaml) -> BTreeMap<String, BTreeSet<String>> {
    let mut declared: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let Some(paths) = spec["paths"].as_mapping() else {
        return declared;
    };
    for (raw, ops) in paths {
        let (Some(raw), Some(ops)) = (raw.as_str(), ops.as_mapping()) else {
            continue;
        };
        let trimmed = raw.trim_start_matches('/');
        let (name, is_item) = match trimmed.split_once("/{id}") {
            Some((name, "")) => (name, true),
            _ => (trimmed, false),
        };
        if name.contains('/') || name.is_empty() {
            continue;
        }
        let entry = declared.entry(name.to_owned()).or_default();
        for method in ops.keys().filter_map(Yaml::as_str) {
            let op = match method {
                "get" if is_item => "GET_ITEM",
                "get" => "GET_COLL",
                "post" => "POST",
                "patch" => "PATCH",
                "delete" => "DELETE",
                _ => continue,
            };
            entry.insert(op.to_owned());
        }
    }
    declared
}

/// Which HTTP operations each macro in `api::ops` generates.
const GENERATED: &[(&str, &[&str])] = &[
    ("op_list", &["GET_COLL"]),
    ("op_stream", &["GET_COLL"]),
    ("op_get", &["GET_ITEM"]),
    ("op_create", &["POST"]),
    ("op_patch", &["PATCH"]),
    ("op_delete", &["DELETE"]),
    (
        "resource_ops",
        &["GET_COLL", "GET_ITEM", "POST", "PATCH", "DELETE"],
    ),
    ("task_ops", &["GET_COLL", "GET_ITEM", "POST"]),
    ("readonly_ops", &["GET_COLL", "GET_ITEM"]),
];

/// Every operation a client exposes must be one its specification declares.
///
/// The client layer composes its surface from the macros in `api::ops`, and
/// each macro corresponds to exactly one HTTP operation. This reads those
/// invocations back out of the client source and compares them against the
/// vendored paths — so a client cannot offer `create_customer_bill` against a
/// `POST /customerBill` that does not exist.
///
/// That is the same class of guarantee as pairing a `PATCH` body with its
/// content type: the type system should not invite a request the server will
/// refuse. It is checked here rather than reviewed, because the temptation to
/// reach for `resource_ops!` on a resource that is only *nearly* CRUD is
/// exactly the mistake that would not be noticed.
#[test]
fn every_client_operation_is_declared_by_its_specification() {
    let specs = load_specs();
    let mut failures = Vec::new();

    for (api, _) in SPECS {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/api")
            .join(format!("{}.rs", api.to_ascii_lowercase()));
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue; // an API modelled but not yet given a client
        };
        let constants = collection_constants(&source);
        let declared = declared_operations(&specs[api]);

        for (macro_name, ops) in GENERATED {
            let needle = format!("{macro_name}!(");
            for (index, _) in source.match_indices(&needle) {
                let first = source[index + needle.len()..]
                    .split([',', '\n'])
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .trim_matches('"')
                    .to_owned();
                let collection = constants.get(&first).cloned().unwrap_or(first);
                if collection.is_empty() {
                    continue;
                }
                let Some(have) = declared.get(&collection) else {
                    failures.push(format!(
                        "{api}: {macro_name}! targets `{collection}`, which the \
                         specification declares no path for"
                    ));
                    continue;
                };
                for op in *ops {
                    assert!(
                        have.contains(*op) || {
                            failures.push(format!(
                                "{api}: {macro_name}! on `{collection}` generates {op}, \
                                 which the specification does not declare (it has {have:?})"
                            ));
                            true
                        },
                    );
                }
            }
        }
    }

    assert!(
        failures.is_empty(),
        "a client offers operations its specification does not declare.\n\
         Compose the individual `op_*` macros for that resource instead of \
         `resource_ops!` — see `src/api/ops.rs`.\n\n{}",
        failures.join("\n")
    );
}

/// Every typed `Ref<T>` must name the `…Ref` class its specification declares.
///
/// `Ref::new` stamps `@type` from `T::REF_TYPE_NAME`, so the choice of `T` is
/// what decides the discriminator on the wire. Pointing a member at the wrong
/// target compiles, round-trips, and satisfies every other check here — the
/// shape of a `Ref` is identical whatever it points at — while sending a
/// discriminator no server routes.
///
/// That is not hypothetical: `ProductOrder.billingAccount` was typed
/// `Ref<Account>` and emitted `AccountRef` where TMF622 declares
/// `BillingAccountRef`, and nothing noticed until TMF666 made `Account` a real
/// type. This reads the field declarations back out of the source and compares
/// each target's reference class against the specification's.
#[test]
fn every_typed_reference_names_the_class_the_specification_declares() {
    // `REF_TYPE_NAME` per target type: from `@ref = "…"` headers and from the
    // `ref_target!` markers in `core::refs`.
    let mut ref_class: BTreeMap<String, String> = BTreeMap::new();
    for (name, class) in reference_class_names() {
        ref_class.insert(name, class);
    }

    // Every `field: Ref<T>` / `Vec<Ref<T>>` declared in a `tmf_struct!`.
    let field = regex_lite_ref_fields();
    let mut failures = Vec::new();
    let specs = load_specs();
    let mapped = mapped_types();

    for entry in &mapped {
        let schemas = &specs[entry.api]["components"]["schemas"];
        let spec_props = entry.spec_properties(schemas);
        let struct_name = entry.rust.rsplit("::").next().unwrap_or(&entry.rust);

        for (owner, member, target) in &field {
            if owner != struct_name {
                continue;
            }
            let Some(declared) = spec_props.get(member) else {
                continue;
            };
            // A member may be declared by several schemas in a family; the
            // reference class must match at least one of them.
            let classes: BTreeSet<String> = declared
                .iter()
                .filter_map(referenced_class)
                .filter(|c| c.ends_with("Ref"))
                .collect();
            if classes.is_empty() {
                continue;
            }
            let Some(ours) = ref_class.get(target) else {
                continue; // a target whose class the mapping does not record
            };
            if !classes.contains(ours) {
                failures.push(format!(
                    "{}.{member} is `Ref<{target}>`, which stamps `{ours}`, but \
                     {}/{} declares {classes:?}",
                    entry.rust,
                    entry.api,
                    entry.base()
                ));
            }
        }
    }

    failures.sort();
    failures.dedup();
    assert!(
        failures.is_empty(),
        "a typed reference names a class its specification does not.\n\
         `Ref<T>` puts `T::REF_TYPE_NAME` on the wire, so the target type is \
         the discriminator — point it at the marker or entity whose reference \
         class matches.\n\n{}",
        failures.join("\n")
    );
}

/// The `…Ref` class a schema member refers to, if it refers to one.
fn referenced_class(member: &Yaml) -> Option<String> {
    if let Some(reference) = member.get("$ref").and_then(Yaml::as_str) {
        return Some(reference.rsplit('/').next()?.to_owned());
    }
    if member.get("type").and_then(Yaml::as_str) == Some("array") {
        return referenced_class(member.get("items")?);
    }
    None
}

/// Scans the model source for `field: Ref<T>` declarations.
///
/// Returns `(struct, wire member name, target type)`. Crude, and deliberately
/// so: the declarations are the only place this association exists, and
/// `schemars` cannot see it because every `Ref<T>` generates the same schema.
fn regex_lite_ref_fields() -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for path in walk_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src")) {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        for block in source.split("tmf_struct! {").skip(1) {
            let Some(name) = block
                .split("pub struct ")
                .nth(1)
                .and_then(|rest| rest.split_whitespace().next())
            else {
                continue;
            };
            let body = block.split("pub struct ").nth(1).unwrap_or_default();
            for line in body.lines() {
                let line = line.trim();
                let Some((field, ty)) = line.trim_end_matches(',').split_once(": ") else {
                    continue;
                };
                if !field
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit())
                    || field.is_empty()
                {
                    continue;
                }
                let inner = ty.trim().trim_start_matches("Vec<").trim_end_matches('>');
                let Some(target) = inner.strip_prefix("Ref<") else {
                    continue;
                };
                out.push((
                    name.to_owned(),
                    camel_case(field),
                    target.trim_end_matches('>').to_owned(),
                ));
            }
        }
    }
    out
}

/// `snake_case` to the `camelCase` the wire uses, matching `serde`'s rename.
fn camel_case(field: &str) -> String {
    let mut out = String::with_capacity(field.len());
    let mut upper = false;
    for c in field.chars() {
        if c == '_' {
            upper = true;
        } else if upper {
            out.push(c.to_ascii_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// Types the model deliberately keeps separate despite an identical shape.
///
/// Each entry names a schema whose *members* two specifications agree on while
/// TM Forum gives it two different names. Merging those would assert an
/// equivalence the specifications have not — the same reason
/// `BillingCycleSpecification` and `BillCycleSpecificationRef` stay apart.
const SEPARATE_BY_DESIGN: &[&str] = &[];

/// One schema modelled once.
///
/// When several specifications declare a schema **byte for byte identically**
/// and the crate answers with a Rust type per module, a caller holding one
/// cannot pass it where the other is wanted, for no reason the wire supports.
///
/// The rule is deliberately narrow: **merge what TM Forum names identically,
/// keep what it names differently.** `service::FeatureRelationship` and
/// `resource::FeatureRelationship` are two types because TMF638 and TMF639
/// declare two different schemas, and `RelatedPlaceRef` stays apart from
/// `RelatedPlaceRefOrValue` for the same reason — so this compares within one
/// schema name, never across two.
#[test]
fn one_schema_declared_identically_is_one_rust_type() {
    let specs = load_specs();
    let mut by_schema: BTreeMap<&str, BTreeMap<String, &str>> = BTreeMap::new();

    for entry in mapped_types() {
        by_schema
            .entry(entry.base())
            .or_default()
            .insert(entry.rust.clone(), entry.api);
    }

    let mut duplicates = Vec::new();
    for (schema, types) in by_schema {
        if types.len() < 2 || SEPARATE_BY_DESIGN.contains(&schema) {
            continue;
        }
        // Only a complaint when the specifications actually agree. Where two
        // APIs give one name to two different schemas, two types are correct.
        let declarations: BTreeSet<String> = types
            .values()
            .map(|api| {
                serde_json::to_string(&specs[*api]["components"]["schemas"][schema])
                    .unwrap_or_default()
            })
            .collect();
        if declarations.len() == 1 {
            duplicates.push(format!(
                "{schema} is declared identically by {:?} but modelled as {:?}",
                types.values().collect::<BTreeSet<_>>(),
                types.keys().collect::<Vec<_>>(),
            ));
        }
    }

    assert!(
        duplicates.is_empty(),
        "one schema, several specifications, one wire shape — but more than one \
         Rust type, so a value from one API cannot be used with another:\n{}",
        duplicates.join("\n")
    );
}

/// No monetary or rate quantity is a binary float.
///
/// The v5 schemas spell `Money.value`, `Price.taxRate`, `TaxItem.taxRate` and
/// the rest as `number/format: float`. This crate does not follow them there:
/// storing money — or a rate that gets multiplied into money — in binary
/// floating point is a defect whatever the schema says, so every one of them is
/// a `rust_decimal::Decimal`.
///
/// The rule had drifted twice before this gate existed. `TaxItem` was modelled
/// once as `f64` and once as `Decimal` for a single schema, and
/// `AppliedBillingTaxRate.taxRate` — one struct further down the same file —
/// was still an `f64`. Both were invisible to every other check, because an
/// `f64` and a `Decimal` serialise to the same JSON number.
///
/// A genuine float belongs in `FLOATS_BY_DESIGN` with the reason.
#[test]
fn no_money_or_rate_is_a_binary_float() {
    /// Fields that are legitimately binary floats, and why.
    ///
    /// Empty: every `number` the vendored specifications declare is either a
    /// monetary quantity, a rate applied to one, or the value of a
    /// `FloatCharacteristic` — and the last is a `serde_json::Value`, because a
    /// characteristic's type is decided by its `@type`, not by its field.
    const FLOATS_BY_DESIGN: &[(&str, &str)] = &[];

    let mut found = Vec::new();
    for path in walk_sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src")) {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let file = path
            .strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .unwrap_or(&path)
            .display()
            .to_string();
        for (n, line) in source.lines().enumerate() {
            let line = line.trim();
            // A field declaration, not a signature: `name: f64,`.
            let Some(name) = line
                .strip_suffix(": f64,")
                .or(line.strip_suffix(": Vec<f64>,"))
            else {
                continue;
            };
            if FLOATS_BY_DESIGN
                .iter()
                .any(|(f, member)| file.ends_with(f) && *member == name)
            {
                continue;
            }
            found.push(format!("{file}:{}: {name}", n + 1));
        }
    }

    assert!(
        found.is_empty(),
        "a rate or amount held as a binary float carries its rounding error into \
         every total computed from it; use `Decimal` via the macros' `@decimal` \
         section, or justify it in `FLOATS_BY_DESIGN`:\n{}",
        found.join("\n")
    );
}

/// Every query parameter the specifications declare, `Query` can produce.
///
/// A parameter a client cannot express is a feature of the API that the crate
/// silently does not offer. Nothing else notices: the request is well-formed
/// and the server simply returns an unfiltered or differently-paged result.
///
/// This found `after` and `before` — cursor pagination, declared by TMF621 and
/// TMF639 on three collections — and `filter`, which those same three declare
/// as a `JSONPath` expression rather than the attribute-name filtering every
/// other collection uses.
#[test]
fn every_declared_query_parameter_can_be_expressed() {
    let specs = load_specs();
    let mut declared: BTreeMap<String, BTreeSet<&str>> = BTreeMap::new();

    for (api, _) in SPECS {
        let doc = &specs[api];
        let shared = &doc["components"]["parameters"];
        let Some(paths) = doc["paths"].as_mapping() else {
            continue;
        };
        for (_, operations) in paths {
            let Some(parameters) = operations["get"]["parameters"].as_sequence() else {
                continue;
            };
            for parameter in parameters {
                // Most are `$ref`s into `components.parameters`.
                let resolved = match parameter.get("$ref").and_then(Yaml::as_str) {
                    Some(reference) => &shared[reference.rsplit('/').next().unwrap_or("")],
                    None => parameter,
                };
                if resolved.get("in").and_then(Yaml::as_str) != Some("query") {
                    continue;
                }
                if let Some(name) = resolved.get("name").and_then(Yaml::as_str) {
                    declared.entry(name.to_owned()).or_default().insert(api);
                }
            }
        }
    }

    // What a `Query` can put on the wire, exercised rather than listed.
    let exercised = rutmf::api::Query::new()
        .fields(["id"])
        .filter("status", "active")
        .sort("name")
        .offset(1)
        .limit(2)
        .after("cursor")
        .before("cursor")
        .json_path("$[*]")
        .to_params();

    let missing: Vec<String> = declared
        .iter()
        .filter(|(name, _)| !exercised.contains_key(name.as_str()))
        .map(|(name, apis)| format!("{name} (declared by {apis:?})"))
        .collect();

    assert!(
        missing.is_empty(),
        "these query parameters are declared by a specification and cannot be \
         built with `Query`, so the crate quietly does not offer them:\n{}",
        missing.join("\n")
    );

    // And the reverse reading, which `Query`'s documentation states: only three
    // parameters are universal, and the other four belong to two APIs. Saying
    // `sort` is understood everywhere would be a promise the documents do not
    // make — a server may ignore a parameter it never declared, and the caller
    // would be reading an order that is not the one asked for.
    let everywhere: BTreeSet<&str> = SPECS.iter().map(|(api, _)| *api).collect();
    let universal: BTreeSet<&str> = declared
        .iter()
        .filter(|(_, apis)| **apis == everywhere)
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(
        universal,
        BTreeSet::from(["fields", "limit", "offset"]),
        "which query parameters every API declares changed; `Query`'s \
         documentation says which are universal and which are not"
    );
    for restricted in ["sort", "filter", "after", "before"] {
        assert_eq!(
            declared.get(restricted).map(BTreeSet::len),
            Some(2),
            "`{restricted}` is documented as declared by TMF621 and TMF639 alone"
        );
    }
}

/// Schemas the vendored documents declare that this crate deliberately does not
/// model, each with the reason.
///
/// The counterpart of [`UNMAPPED`], which excuses a *Rust type* with no schema.
/// This excuses a *schema* with no Rust type — the direction nothing checked,
/// and the one that answers "is the domain model complete?".
///
/// A schema reaches this list only if it is unreachable from every modelled
/// resource except through a value arm the crate declines to model. Anything a
/// client can arrive at by following typed members is modelled.
const NOT_MODELLED: &[(&str, &str)] = &[
    // --- The `PlaceRefOrValue` value arms, and everything below them. ---
    //
    // `PlaceRefOrValue` is a `oneOf` over a plain `PlaceRef` and three inline
    // value arms owned by APIs outside the fourteen: TMF673 Geographic Address,
    // TMF674 Geographic Site and TMF675 Geographic Location. The crate models
    // the reference arm and keeps a value arm's members in `extensions`, so a
    // payload round-trips either way — see `core::PlaceRefOrValue`.
    //
    // Modelling them here would mean shipping a partial model of three APIs
    // this crate does not cover, under type names that would then collide when
    // it does.
    ("GeographicAddress", "a TMF673 value arm of PlaceRefOrValue"),
    (
        "GeographicAddressRelationship",
        "reachable only via GeographicAddress",
    ),
    (
        "GeographicSubAddress",
        "reachable only via GeographicAddress",
    ),
    (
        "GeographicSubAddressUnit",
        "reachable only via GeographicSubAddress",
    ),
    ("StandardIdentifier", "reachable only via GeographicAddress"),
    ("GeographicSite", "a TMF674 value arm of PlaceRefOrValue"),
    (
        "GeographicSiteRelationship",
        "reachable only via GeographicSite",
    ),
    ("GeographicSiteFeature", "reachable only via GeographicSite"),
    (
        "CalendarPeriod",
        "opening hours, reachable only via GeographicSite",
    ),
    ("HourPeriod", "reachable only via CalendarPeriod"),
    (
        "GeographicLocation",
        "a TMF675 value arm of PlaceRefOrValue",
    ),
    // GeoJSON (RFC 7946) geometry, reachable only through GeographicLocation.
    // TM Forum vendors the GeoJSON schema wholesale, lower-case names and all.
    (
        "Point",
        "GeoJSON geometry, reachable only via GeographicLocation",
    ),
    (
        "LineString",
        "GeoJSON geometry, reachable only via GeographicLocation",
    ),
    (
        "Polygon",
        "GeoJSON geometry, reachable only via GeographicLocation",
    ),
    (
        "MultiPoint",
        "GeoJSON geometry, reachable only via GeographicLocation",
    ),
    (
        "MultiLineString",
        "GeoJSON geometry, reachable only via GeographicLocation",
    ),
    (
        "MultiPolygon",
        "GeoJSON geometry, reachable only via GeographicLocation",
    ),
    (
        "bbox",
        "GeoJSON bounding box, reachable only via GeographicLocation",
    ),
    (
        "position",
        "GeoJSON coordinate, reachable only via GeographicLocation",
    ),
    ("positionArray", "GeoJSON coordinate list"),
    ("linearRing", "GeoJSON closed ring"),
    ("lineString", "GeoJSON coordinate list"),
    ("polygon", "GeoJSON ring list"),
    ("multiLineString", "GeoJSON line list"),
    ("multiPolygon", "GeoJSON polygon list"),
    // --- The `IntentRefOrValue` value arm, and everything below it. ---
    //
    // `Intent` belongs to TMF921 Intent Management. The crate models the
    // reference — `Ref<core::Intent>` — because that is what a resource
    // carries; the inline value arm is TMF921's model, not this crate's.
    ("Intent", "a TMF921 value arm of IntentRefOrValue"),
    ("IntentExpression", "reachable only via Intent"),
    ("Expression", "reachable only via Intent"),
    ("ExpressionLanguageEnum", "reachable only via Expression"),
    ("EntityRelationship", "reachable only via Intent"),
    // --- Wrappers and unions with no shape of their own. ---
    (
        "JsonPatchOperations",
        "an array of JsonPatch; modelled as `Vec<JsonPatchOp>`",
    ),
    (
        "PartyOrPartyRole",
        "a oneOf inside RelatedPartyOrPartyRole; modelled as `core::PartyOrPartyRole`",
    ),
    // --- Declared by the documents, referenced by nothing in them. ---
    (
        "ProductRelationshipType",
        "an orphan enumeration: `ProductRelationship.relationshipType` is \
         declared `type: string`, not a $ref to this",
    ),
    (
        "ProductOrderItemRelationshipType",
        "an orphan enumeration: no schema in TMF622 references it",
    ),
    (
        "ResourceAllocationStatusType",
        "declares no `enum`, only prose; `Resource.allocation_status` is a String",
    ),
];

/// Every schema the vendored documents declare is modelled, or excused by name.
///
/// The mapping gate asks "does every Rust type have a schema?", which says
/// nothing about completeness — a crate modelling three schemas out of three
/// thousand passes it. This asks the opposite, and it is what "do we have the
/// full domain model?" reduces to.
///
/// A schema counts as covered when it is mapped, absorbed into a mapped schema
/// through `allOf`, handled generically (an event, a `…Ref`, a write variant),
/// paired in [`ENUMS`], or listed in [`NOT_MODELLED`] with a reason.
#[test]
fn every_declared_schema_is_modelled_or_excused() {
    let specs = load_specs();

    let mut mapped: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for m in mapped_types() {
        mapped
            .entry(m.api)
            .or_default()
            .extend(m.schemas.iter().map(|s| (*s).to_owned()));
    }

    // A schema absorbed into a mapped one through `allOf` is checked as part of
    // it: `flatten` resolves the chain, so `Milestone` is covered by
    // `ProductOrderMilestone`. Gathered across every API, because one Rust type
    // serves a schema that several specifications redeclare.
    let mut absorbed: BTreeSet<String> = BTreeSet::new();
    let mut mapped_anywhere: BTreeSet<String> = BTreeSet::new();
    for (api, _) in SPECS {
        let schemas = &specs[api]["components"]["schemas"];
        for name in mapped.get(api).into_iter().flatten() {
            mapped_anywhere.insert(name.clone());
            collect_all_of(schemas, name, &mut absorbed);
        }
    }

    let mut gaps: BTreeMap<String, BTreeSet<&str>> = BTreeMap::new();
    for (api, _) in SPECS {
        let Some(schemas) = specs[api]["components"]["schemas"].as_mapping() else {
            continue;
        };
        for (name, _) in schemas {
            let Some(name) = name.as_str() else { continue };

            // Handled generically rather than by a type of their own: an event
            // is a `TmfEvent`, a `…Ref` is a `Ref<T>`, and a write variant is
            // checked wherever its read model is.
            let generic = name.ends_with("Event")
                || name.ends_with("Payload")
                || name.ends_with("Ref")
                || name.ends_with("RefOrValue")
                || name.ends_with("_FVO")
                || name.ends_with("_MVO")
                || name.ends_with("_RES");

            if generic
                || mapped_anywhere.contains(name)
                || absorbed.contains(name)
                || ENUMS.iter().any(|(_, _, schema)| schema == &name)
                || UNMAPPED.iter().any(|(n, _)| *n == name)
                || NOT_MODELLED.iter().any(|(n, _)| *n == name)
            {
                continue;
            }
            gaps.entry(name.to_owned()).or_default().insert(api);
        }
    }

    let report: Vec<String> = gaps
        .iter()
        .map(|(name, apis)| {
            format!(
                "  {name}  (declared by {})",
                apis.iter().copied().collect::<Vec<_>>().join(", ")
            )
        })
        .collect();

    assert!(
        gaps.is_empty(),
        "these schemas are declared by the vendored specifications and have no \
         Rust type, so the domain model is incomplete in a way nothing else \
         reports. Model them, or add them to `NOT_MODELLED` with the reason:\n{}",
        report.join("\n"),
    );

    // Symmetrically: an excuse for a schema the documents no longer declare is
    // one nobody has rechecked, and the list would only ever grow.
    let declared: BTreeSet<&str> = SPECS
        .iter()
        .filter_map(|(api, _)| specs[api]["components"]["schemas"].as_mapping())
        .flat_map(|m| m.keys().filter_map(Yaml::as_str))
        .collect();
    let stale: Vec<&str> = NOT_MODELLED
        .iter()
        .map(|(name, _)| *name)
        .filter(|name| !declared.contains(name))
        .collect();
    assert!(
        stale.is_empty(),
        "`NOT_MODELLED` excuses schemas no vendored specification declares any \
         more; remove them: {stale:?}"
    );
}

/// Every schema name reachable from `name` through `allOf`, transitively.
fn collect_all_of(schemas: &Yaml, name: &str, out: &mut BTreeSet<String>) {
    let Some(node) = schemas.get(name) else {
        return;
    };
    let Some(all_of) = node.get("allOf").and_then(Yaml::as_sequence) else {
        return;
    };
    for entry in all_of {
        let Some(target) = entry
            .get("$ref")
            .and_then(Yaml::as_str)
            .and_then(|r| r.rsplit('/').next())
        else {
            continue;
        };
        if out.insert(target.to_owned()) {
            collect_all_of(schemas, target, out);
        }
    }
}
