//! Conformance: the crate's types against the official v5 examples.
//!
//! Fixtures under `tests/fixtures/` are the `components.examples` values
//! vendored verbatim from the TM Forum API repositories (Apache-2.0). They are
//! the closest thing to a wire-level ground truth that does not require a live
//! server.
//!
//! Two properties are asserted for **every** fixture:
//!
//! 1. **It parses.** A payload a conformant server may send must not fail.
//! 2. **It round-trips by value.** Re-serialising reproduces every member of
//!    the input, including vendor extensions this crate has no field for.
//!
//! # Every fixture, not a chosen subset
//!
//! An earlier version of this suite named the fixtures it checked, and covered
//! about a quarter of the corpus: the event envelopes and the JSON Patch bodies
//! were vendored but never asserted against anything. So the mapping here is
//! derived from the file-naming convention instead, and
//! [`every_fixture_is_covered`] fails if any file falls through it. Adding a
//! fixture cannot silently add an untested one.
//!
//! # What round-tripping does *not* prove
//!
//! Anything the model has no typed field for survives in `Extensions`, so a
//! payload round-trips whether or not the model understands a single member of
//! it. `coverage.rs` is what checks the model against the schemas; this file
//! checks it against the examples. Both are needed, and neither substitutes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use rutmf::account::{
    Account, AccountCreate, AccountUpdate, BillFormat, BillFormatCreate, BillFormatUpdate,
    BillPresentationMedia, BillPresentationMediaCreate, BillPresentationMediaUpdate,
    BillingCycleSpecification, BillingCycleSpecificationCreate, BillingCycleSpecificationUpdate,
};
use rutmf::alarm::{
    AckAlarm, AckAlarmCreate, Alarm, AlarmCreate, AlarmUpdate, ClearAlarm, ClearAlarmCreate,
    CommentAlarm, CommentAlarmCreate, GroupAlarm, GroupAlarmCreate, UnAckAlarm, UnAckAlarmCreate,
    UnGroupAlarm, UnGroupAlarmCreate,
};
use rutmf::bill::{
    AppliedCustomerBillingRate, BillCycle, CustomerBill, CustomerBillOnDemand,
    CustomerBillOnDemandCreate, CustomerBillUpdate,
};
use rutmf::core::{JsonPatchOp, TmfEvent};
use rutmf::customer::{Customer, CustomerCreate, CustomerUpdate};
use rutmf::order::{
    CancelProductOrder, CancelProductOrderCreate, ProductOrder, ProductOrderCreate,
    ProductOrderUpdate,
};
use rutmf::party::{
    Individual, IndividualCreate, IndividualUpdate, Organization, OrganizationCreate,
    OrganizationUpdate, PartyRole, PartyRoleCreate, PartyRoleSpecification,
    PartyRoleSpecificationCreate, PartyRoleSpecificationUpdate, PartyRoleUpdate,
};
use rutmf::product::{
    Category, CategoryCreate, CategoryUpdate, CheckProductOfferingQualification,
    CheckProductOfferingQualificationCreate, CheckProductOfferingQualificationUpdate, ExportJob,
    ExportJobCreate, ImportJob, ImportJobCreate, Product, ProductCatalog, ProductCatalogCreate,
    ProductCatalogUpdate, ProductCreate, ProductOffering, ProductOfferingCreate,
    ProductOfferingPrice, ProductOfferingPriceCreate, ProductOfferingPriceUpdate,
    ProductOfferingUpdate, ProductSpecification, ProductSpecificationCreate,
    ProductSpecificationUpdate, ProductUpdate, QueryProductOfferingQualification,
    QueryProductOfferingQualificationCreate, QueryProductOfferingQualificationUpdate,
};
use rutmf::resource::{
    Resource, ResourceCandidate, ResourceCandidateCreate, ResourceCandidateUpdate, ResourceCatalog,
    ResourceCatalogCreate, ResourceCatalogUpdate, ResourceCategory, ResourceCategoryCreate,
    ResourceCategoryUpdate, ResourceCreate, ResourceSpecification, ResourceSpecificationCreate,
    ResourceSpecificationUpdate, ResourceUpdate,
};
use rutmf::service::{Service, ServiceCreate, ServiceUpdate};
use rutmf::ticket::{
    TroubleTicket, TroubleTicketCreate, TroubleTicketSpecification,
    TroubleTicketSpecificationCreate, TroubleTicketSpecificationUpdate, TroubleTicketUpdate,
};

/// The vendored corpora, one directory per API: the API, the specification
/// version it was taken from, and how many examples it contributes.
///
/// The count is exact rather than a lower bound. A `>=` threshold cannot tell
/// "the corpus grew because TM Forum published more examples" from "half of it
/// stopped being loaded" — and the second is precisely the silent failure this
/// suite exists to prevent. It is also what keeps the totals quoted in the
/// documentation honest: they are asserted here, not remembered.
const APIS: &[(&str, &str, usize)] = &[
    ("tmf620", "5.0.0", 94),
    ("tmf621", "5.0.1", 37),
    ("tmf622", "5.0.0", 34),
    ("tmf629", "5.0.1", 16),
    ("tmf632", "5.0.0", 32),
    ("tmf634", "5.0.0", 82),
    ("tmf642", "5.0.1", 42),
    ("tmf666", "5.0.0", 104),
    ("tmf669", "5.0.0", 34),
    ("tmf679", "5.0.0", 36),
    ("tmf678", "5.0.0", 22),
    ("tmf637", "5.0.0", 21),
    ("tmf638", "5.0.0", 21),
    ("tmf639", "5.0.0", 16),
];

/// Fixtures that are not valid payloads, and the defect in each.
///
/// TM Forum's examples are generated alongside the specifications and are
/// occasionally wrong. Where an example contradicts the RFC the member is
/// governed by, the crate does **not** bend its model to accept it: doing so
/// would let the type build requests no conformant server applies. The example
/// is excluded here instead, with the defect named.
///
/// This is not a way to silence a failing fixture.
/// [`known_bad_fixtures_are_still_bad`] re-checks every entry and fails if one
/// starts parsing — so when TM Forum fixes an example upstream, the exclusion
/// must be removed rather than quietly outliving the bug it documents.
const KNOWN_BAD: &[(&str, &str)] = &[
    (
        "tmf634/ResourceSpecification_Update_example_JSON-PATCH__request.json",
        "writes `\"/path\": \"lifecycleStatus\"` where RFC 6902 §4 requires \
         `\"path\": \"/lifecycleStatus\"` — the slash is inside the member name. \
         Accepting it would mean making `path` optional on every operation.",
    ),
    (
        "tmf678/Customer_Bill_Update_Implied_Merge_response.json",
        "sends `billDocument[].size` as a bare number where TMF678's own \
         `Attachment` schema types it as a `Quantity` object — the same shape \
         TMF620 gives it — and names the state member `status` where the \
         schema says `state`. Loosening `size` would break the one `Attachment` \
         this crate shares across twelve specifications.",
    ),
    (
        "tmf678/Customer_Bill_Update_JSON_Patch_Query_response.json",
        "sends `billDocument[].size` as a bare number where TMF678's own \
         `Attachment` schema types it as a `Quantity` object — the same shape \
         TMF620 gives it — and names the state member `status` where the \
         schema says `state`. Loosening `size` would break the one `Attachment` \
         this crate shares across twelve specifications.",
    ),
    (
        "tmf678/Customer_Bill_Update_JSON_Patch_response.json",
        "sends `billDocument[].size` as a bare number where TMF678's own \
         `Attachment` schema types it as a `Quantity` object — the same shape \
         TMF620 gives it — and names the state member `status` where the \
         schema says `state`. Loosening `size` would break the one `Attachment` \
         this crate shares across twelve specifications.",
    ),
    (
        "tmf678/Customer_Bill_Update_Patch_Merge_response.json",
        "sends `billDocument[].size` as a bare number where TMF678's own \
         `Attachment` schema types it as a `Quantity` object — the same shape \
         TMF620 gives it — and names the state member `status` where the \
         schema says `state`. Loosening `size` would break the one `Attachment` \
         this crate shares across twelve specifications.",
    ),
    (
        "tmf678/BillCycle_retrieve_example_response.json",
        "carries `\"endDateTime\": \"2020-01-00T00:00:00.000Z\"` — there is no \
         day zero, so this is not a date any RFC 3339 parser accepts. (The same \
         example also puts its period end before its start.) Accepting it would \
         mean parsing timestamps loosely enough to accept impossible ones.",
    ),
];

/// Every vendored example, across all fourteen APIs.
///
/// Quoted in `README.md` and the crate documentation; asserted by
/// [`fixture_corpora_are_present_and_documented`].
const TOTAL_FIXTURES: usize = 591;

/// Which of a resource's three shapes a fixture is an example of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// A `GET` response, or the resource a write returned.
    Read,
    /// A `POST` body — the `_FVO`.
    Create,
    /// A merge or implicit `PATCH` body — the `_MVO`.
    Update,
    /// A notification envelope.
    Event,
    /// An RFC 6902 operation list.
    PatchOps,
}

/// Classifies a fixture, from its content first and its name second.
///
/// Content decides what a name only hints at. TMF638 ships its notification
/// examples as `Create_request.json` and `StateChange_request.json`, with no
/// `Event` in the name at all, and its JSON Patch bodies as
/// `Service_partialupdate_example_11_request.json`, with no `json_patch` in the
/// name either. A rule that read only the name would misfile all five and then
/// pass, having checked the wrong type — so the two unambiguous shapes are
/// recognised by what they *are*: an operation list is a JSON array, and a
/// notification carries `eventType`.
///
/// The name still settles read-vs-create-vs-update, which the payload cannot:
/// a create body and a patch body are both just objects.
fn shape_of(file: &str, value: &Value) -> Option<Shape> {
    // An RFC 6902 operation list: an array of objects that each carry an `op`.
    // A *list* response is an array too, which is why the `op` matters.
    if let Some(items) = value.as_array() {
        return items
            .iter()
            .all(|item| item.get("op").is_some())
            .then_some(Shape::PatchOps)
            .or(Some(Shape::Read));
    }
    // The notification envelope, whatever the file is called.
    if value.get("eventType").is_some() {
        return Some(Shape::Event);
    }

    let name = file.to_ascii_lowercase().replace(['-', ' '], "_");
    if name.ends_with("_request.json") {
        if name.contains("create") {
            return Some(Shape::Create);
        }
        if name.contains("update") {
            return Some(Shape::Update);
        }
        return None;
    }
    if name.ends_with("_response.json") {
        return Some(Shape::Read);
    }
    None
}

/// Identifies which resource a fixture is about, from its name.
///
/// Order matters: `productOfferingPrice` must be tested before
/// `productOffering`, and `cancelProductOrder` before `productOrder`.
fn resource_of(api: &str, file: &str) -> Option<&'static str> {
    let name = file.to_ascii_lowercase().replace(['_', '-', ' '], "");
    let candidates: &[&'static str] = match api {
        "tmf620" => &[
            "productofferingprice",
            "productoffering",
            "productspecification",
            "productcatalog",
            "category",
            "importjob",
            "exportjob",
        ],
        // `troubleTicketSpecification` must be tested before `troubleTicket`.
        "tmf621" => &["troubleticketspecification", "troubleticket"],
        "tmf622" => &["cancelproductorder", "productorder"],
        "tmf629" => &["customer"],
        "tmf632" => &["individual", "organization"],
        // Order matters: `resourceCandidate` and `resourceCategory` both
        // contain `resourcec…`, and `resourceSpecification` must be tested
        // before the bare `resource` a file name may fall back to.
        "tmf634" => &[
            "resourcecatalog",
            "resourcecategory",
            "resourcecandidate",
            "resourcespecification",
            "importjob",
            "exportjob",
        ],
        // Longest first: `unAckAlarm` and `unGroupAlarm` contain `ackAlarm`
        // and `groupAlarm`, and every task name contains `alarm`.
        "tmf642" => &[
            "ungroupalarm",
            "unackalarm",
            "groupalarm",
            "ackalarm",
            "clearalarm",
            "commentalarm",
            "alarm",
        ],
        // `customerBillOnDemand` before `customerBill`; the rate and cycle
        // collections have their own names.
        "tmf678" => &[
            "customerbillondemand",
            "customerbill",
            "appliedcustomerbillingrate",
            "billcycle",
        ],
        // Longest first: the four account collections all end in `account`,
        // and `billingCycleSpecification` contains neither `billformat` nor
        // `billpresentationmedia`.
        "tmf666" => &[
            "billingcyclespecification",
            "billpresentationmedia",
            "billformat",
            "billingaccount",
            "financialaccount",
            "partyaccount",
            "settlementaccount",
            "account",
        ],
        // Longest first: `partyRole` is a prefix of `partyRoleSpecification`.
        "tmf669" => &["partyrolespecification", "partyrole"],
        // TMF679 names half its examples by initialism. `QPOC` is an upstream
        // typo for `QPOQ`, in two files; it is matched rather than corrected,
        // because the corpus is vendored verbatim.
        "tmf679" => &[
            "checkproductofferingqualification",
            "queryproductofferingqualification",
            "cpoq",
            "qpoq",
            "qpoc",
        ],
        "tmf637" => &["product"],
        // TMF638 names four of its examples `Create_request.json`,
        // `Delete_request.json` and so on, with the resource left implicit.
        // The API has exactly one, so there is nothing to disambiguate.
        "tmf638" => &["service", ""],
        "tmf639" => &["resource"],
        _ => &[],
    };
    candidates.iter().copied().find(|c| name.contains(c))
}

/// Asserts the fixture parses as `$read`/`$create`/`$update` for its shape.
macro_rules! dispatch {
    ($shape:expr, $label:expr, $value:expr, $read:ty, $create:ty $(, $update:ty)?) => {
        match $shape {
            Shape::Read => assert_fixture_round_trips::<$read>($label, $value),
            Shape::Create => assert_fixture_round_trips::<$create>($label, $value),
            $(Shape::Update => assert_fixture_round_trips::<$update>($label, $value),)?
            other => panic!("{}: no {other:?} type for this resource", $label),
        }
    };
}

#[allow(
    clippy::too_many_lines,
    reason = "one arm per resource across fourteen APIs is the point"
)]
fn check_fixture(api: &str, file: &str, value: &Value) {
    let label = format!("{api}/{file}");
    let shape = shape_of(file, value).unwrap_or_else(|| panic!("{label}: unclassifiable"));

    match shape {
        Shape::Event => return assert_fixture_round_trips::<TmfEvent>(&label, value),
        Shape::PatchOps => return assert_round_trips::<Vec<JsonPatchOp>>(&label, value),
        _ => {}
    }

    let resource =
        resource_of(api, file).unwrap_or_else(|| panic!("{label}: unrecognised resource"));

    match (api, resource) {
        ("tmf620", "productoffering") => dispatch!(
            shape,
            &label,
            value,
            ProductOffering,
            ProductOfferingCreate,
            ProductOfferingUpdate
        ),
        ("tmf620", "productofferingprice") => dispatch!(
            shape,
            &label,
            value,
            ProductOfferingPrice,
            ProductOfferingPriceCreate,
            ProductOfferingPriceUpdate
        ),
        ("tmf620", "productspecification") => dispatch!(
            shape,
            &label,
            value,
            ProductSpecification,
            ProductSpecificationCreate,
            ProductSpecificationUpdate
        ),
        ("tmf620", "productcatalog") => dispatch!(
            shape,
            &label,
            value,
            ProductCatalog,
            ProductCatalogCreate,
            ProductCatalogUpdate
        ),
        ("tmf620", "category") => {
            dispatch!(
                shape,
                &label,
                value,
                Category,
                CategoryCreate,
                CategoryUpdate
            );
        }
        // The v5 job resources have no patch schema: a running job is not
        // edited, it is polled.
        // TMF620 and TMF634 declare the job resources identically, which is
        // why one pair of types serves both.
        ("tmf620" | "tmf634", "importjob") => {
            dispatch!(shape, &label, value, ImportJob, ImportJobCreate);
        }
        ("tmf620" | "tmf634", "exportjob") => {
            dispatch!(shape, &label, value, ExportJob, ExportJobCreate);
        }
        ("tmf622", "productorder") => dispatch!(
            shape,
            &label,
            value,
            ProductOrder,
            ProductOrderCreate,
            ProductOrderUpdate
        ),
        // Cancellation is a task: you create one and read it, never patch it.
        ("tmf622", "cancelproductorder") => dispatch!(
            shape,
            &label,
            value,
            CancelProductOrder,
            CancelProductOrderCreate
        ),
        ("tmf621", "troubleticket") => dispatch!(
            shape,
            &label,
            value,
            TroubleTicket,
            TroubleTicketCreate,
            TroubleTicketUpdate
        ),
        ("tmf621", "troubleticketspecification") => dispatch!(
            shape,
            &label,
            value,
            TroubleTicketSpecification,
            TroubleTicketSpecificationCreate,
            TroubleTicketSpecificationUpdate
        ),
        ("tmf629", "customer") => {
            dispatch!(
                shape,
                &label,
                value,
                Customer,
                CustomerCreate,
                CustomerUpdate
            );
        }
        ("tmf669", "partyrole") => {
            dispatch!(
                shape,
                &label,
                value,
                PartyRole,
                PartyRoleCreate,
                PartyRoleUpdate
            );
        }
        ("tmf669", "partyrolespecification") => {
            dispatch!(
                shape,
                &label,
                value,
                PartyRoleSpecification,
                PartyRoleSpecificationCreate,
                PartyRoleSpecificationUpdate
            );
        }
        ("tmf679", "checkproductofferingqualification" | "cpoq") => {
            dispatch!(
                shape,
                &label,
                value,
                CheckProductOfferingQualification,
                CheckProductOfferingQualificationCreate,
                CheckProductOfferingQualificationUpdate
            );
        }
        ("tmf679", "queryproductofferingqualification" | "qpoq" | "qpoc") => {
            dispatch!(
                shape,
                &label,
                value,
                QueryProductOfferingQualification,
                QueryProductOfferingQualificationCreate,
                QueryProductOfferingQualificationUpdate
            );
        }
        ("tmf632", "individual") => dispatch!(
            shape,
            &label,
            value,
            Individual,
            IndividualCreate,
            IndividualUpdate
        ),
        ("tmf632", "organization") => dispatch!(
            shape,
            &label,
            value,
            Organization,
            OrganizationCreate,
            OrganizationUpdate
        ),
        ("tmf634", "resourcecatalog") => dispatch!(
            shape,
            &label,
            value,
            ResourceCatalog,
            ResourceCatalogCreate,
            ResourceCatalogUpdate
        ),
        ("tmf634", "resourcecategory") => dispatch!(
            shape,
            &label,
            value,
            ResourceCategory,
            ResourceCategoryCreate,
            ResourceCategoryUpdate
        ),
        ("tmf634", "resourcecandidate") => dispatch!(
            shape,
            &label,
            value,
            ResourceCandidate,
            ResourceCandidateCreate,
            ResourceCandidateUpdate
        ),
        ("tmf634", "resourcespecification") => dispatch!(
            shape,
            &label,
            value,
            ResourceSpecification,
            ResourceSpecificationCreate,
            ResourceSpecificationUpdate
        ),
        ("tmf637", "product") => {
            dispatch!(shape, &label, value, Product, ProductCreate, ProductUpdate);
        }
        ("tmf642", "alarm") => dispatch!(shape, &label, value, Alarm, AlarmCreate, AlarmUpdate),
        // The six tasks are POST-and-read: a create body, a read model, no patch.
        ("tmf642", "ackalarm") => dispatch!(shape, &label, value, AckAlarm, AckAlarmCreate),
        ("tmf642", "unackalarm") => dispatch!(shape, &label, value, UnAckAlarm, UnAckAlarmCreate),
        ("tmf642", "clearalarm") => dispatch!(shape, &label, value, ClearAlarm, ClearAlarmCreate),
        ("tmf642", "commentalarm") => {
            dispatch!(shape, &label, value, CommentAlarm, CommentAlarmCreate);
        }
        ("tmf642", "groupalarm") => dispatch!(shape, &label, value, GroupAlarm, GroupAlarmCreate),
        ("tmf642", "ungroupalarm") => {
            dispatch!(shape, &label, value, UnGroupAlarm, UnGroupAlarmCreate);
        }
        // TMF678 has no create body for a bill and no update for the rest,
        // so each arm names only the shapes that exist.
        ("tmf678", "customerbill") => match shape {
            Shape::Read => assert_fixture_round_trips::<CustomerBill>(&label, value),
            Shape::Update => assert_fixture_round_trips::<CustomerBillUpdate>(&label, value),
            other => panic!("{label}: no {other:?} type for a customer bill"),
        },
        ("tmf678", "customerbillondemand") => dispatch!(
            shape,
            &label,
            value,
            CustomerBillOnDemand,
            CustomerBillOnDemandCreate
        ),
        ("tmf678", "appliedcustomerbillingrate") => match shape {
            Shape::Read => {
                assert_fixture_round_trips::<AppliedCustomerBillingRate>(&label, value);
            }
            other => panic!("{label}: {other:?} on a read-only collection"),
        },
        ("tmf678", "billcycle") => match shape {
            Shape::Read => assert_fixture_round_trips::<BillCycle>(&label, value),
            other => panic!("{label}: {other:?} on a read-only collection"),
        },
        // One type serves the whole account family and all four collections.
        (
            "tmf666",
            "account" | "billingaccount" | "financialaccount" | "partyaccount"
            | "settlementaccount",
        ) => dispatch!(shape, &label, value, Account, AccountCreate, AccountUpdate),
        ("tmf666", "billformat") => dispatch!(
            shape,
            &label,
            value,
            BillFormat,
            BillFormatCreate,
            BillFormatUpdate
        ),
        ("tmf666", "billpresentationmedia") => dispatch!(
            shape,
            &label,
            value,
            BillPresentationMedia,
            BillPresentationMediaCreate,
            BillPresentationMediaUpdate
        ),
        ("tmf666", "billingcyclespecification") => dispatch!(
            shape,
            &label,
            value,
            BillingCycleSpecification,
            BillingCycleSpecificationCreate,
            BillingCycleSpecificationUpdate
        ),
        ("tmf638", "service" | "") => {
            dispatch!(shape, &label, value, Service, ServiceCreate, ServiceUpdate);
        }
        // TMF639 declares no `Resource_MVO`: its patch body is the read model,
        // which is what `ResourceUpdate` aliases.
        ("tmf639", "resource") => dispatch!(
            shape,
            &label,
            value,
            Resource,
            ResourceCreate,
            ResourceUpdate
        ),
        _ => panic!("{label}: no type mapped for {api}/{resource}"),
    }
}

fn fixture_dir(api: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(api)
}

fn load_from(api: &str, name: &str) -> Value {
    let path = fixture_dir(api).join(name);
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("fixture {} is not valid JSON: {e}", path.display()))
}

/// Every fixture of one API, keyed by file name.
fn all_fixtures(api: &str) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    for entry in std::fs::read_dir(fixture_dir(api)).expect("fixture directory is missing") {
        let path = entry.expect("unreadable directory entry").path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_owned();
        if name.starts_with('_') || path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        out.insert(name.clone(), load_from(api, &name));
    }
    assert!(
        !out.is_empty(),
        "no fixtures found — vendoring step did not run"
    );
    out
}

/// Asserts that `T` parses `value` and re-serialises to an equal document.
///
/// Compared after [`canonicalise`], which is the one documented normalisation:
/// a timestamp's fractional seconds are re-emitted in SI groups. Nothing else
/// is smoothed over — member values, extension ordering and the presence of
/// every member are compared exactly.
fn assert_round_trips<T: DeserializeOwned + Serialize>(label: &str, value: &Value) {
    let parsed: T = serde_json::from_value(value.clone())
        .unwrap_or_else(|e| panic!("{label}: failed to parse: {e}\n{value:#}"));

    let reserialised = serde_json::to_value(&parsed)
        .unwrap_or_else(|e| panic!("{label}: failed to re-serialise: {e}"));

    similar_asserts::assert_eq!(
        expected: canonicalise(value),
        actual: canonicalise(&reserialised),
        "{label}: round-trip lost or altered data"
    );
}

/// Rewrites every RFC 3339 timestamp to the lexical form `chrono` emits.
///
/// `chrono` writes fractional seconds in SI groups — none, milli, micro or
/// nano — so an input of `12:15:59.96747` (five digits) comes back as
/// `12:15:59.967470`. Same instant, different spelling, and the only difference
/// this crate's round trip permits. The *offset* is preserved exactly; see
/// [`rutmf::core::Timestamp`].
fn canonicalise(value: &Value) -> Value {
    match value {
        Value::String(raw) => match raw.parse::<rutmf::core::Timestamp>() {
            Ok(t) => Value::String(t.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)),
            Err(_) => value.clone(),
        },
        Value::Array(items) => Value::Array(items.iter().map(canonicalise).collect()),
        Value::Object(members) => Value::Object(
            members
                .iter()
                .map(|(k, v)| (k.clone(), canonicalise(v)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Applies [`assert_round_trips`] to a fixture holding one resource or a list.
fn assert_fixture_round_trips<T: DeserializeOwned + Serialize>(label: &str, value: &Value) {
    match value {
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                assert_round_trips::<T>(&format!("{label}[{index}]"), item);
            }
        }
        single => assert_round_trips::<T>(label, single),
    }
}

#[test]
fn every_vendored_example_parses_and_round_trips() {
    let mut checked = 0;
    for (api, _, _) in APIS {
        for (file, value) in all_fixtures(api) {
            let label = format!("{api}/{file}");
            if KNOWN_BAD.iter().any(|(bad, _)| *bad == label) {
                continue;
            }
            check_fixture(api, &file, &value);
            checked += 1;
        }
    }
    assert_eq!(
        checked,
        TOTAL_FIXTURES - KNOWN_BAD.len(),
        "the whole corpus must be checked, not most of it"
    );
}

/// Each excluded fixture must still exhibit the defect it was excluded for.
///
/// Without this, an exclusion outlives its reason: TM Forum fixes the example,
/// the fixture becomes checkable, and nothing notices that it stopped being
/// checked.
#[test]
fn known_bad_fixtures_are_still_bad() {
    for (label, defect) in KNOWN_BAD {
        let (api, file) = label.split_once('/').expect("api/file");
        let value = load_from(api, file);
        let outcome = std::panic::catch_unwind(|| check_fixture(api, file, &value));
        assert!(
            outcome.is_err(),
            "{label} now parses cleanly — the defect it was excluded for \
             ({defect}) appears to be fixed upstream. Remove it from KNOWN_BAD."
        );
    }
}

#[test]
fn every_fixture_is_covered_by_the_naming_rules() {
    // A fixture the rules cannot classify would be silently skipped, which is
    // exactly the failure mode this suite exists to avoid.
    let mut unclassified = Vec::new();
    for (api, _, _) in APIS {
        for (file, value) in &all_fixtures(api) {
            match shape_of(file, value) {
                None => unclassified.push(format!("{api}/{file}: no shape")),
                Some(Shape::Event | Shape::PatchOps) => {}
                Some(_) if resource_of(api, file).is_none() => {
                    unclassified.push(format!("{api}/{file}: no resource"));
                }
                Some(_) => {}
            }
        }
    }
    assert!(unclassified.is_empty(), "{}", unclassified.join("\n"));
}

/// The vendored corpora must stay in place; a silently empty fixture directory
/// would make every test above vacuously pass.
#[test]
fn fixture_corpora_are_present_and_documented() {
    let mut total = 0;
    for (api, version, expected) in APIS {
        let fixtures = all_fixtures(api);
        total += fixtures.len();

        let manifest = load_from(api, "_manifest.json");
        assert_eq!(manifest["license"], "Apache-2.0", "{api}");
        assert_eq!(
            manifest["spec_version"], *version,
            "{api} spec version drifted"
        );
        assert_eq!(
            manifest["examples"].as_object().map(serde_json::Map::len),
            Some(fixtures.len()),
            "{api}: manifest and fixture directory disagree"
        );
        assert_eq!(
            fixtures.len(),
            *expected,
            "{api} contributes a different number of examples than APIS records; \
             if TM Forum published more, update the count deliberately"
        );
    }
    assert_eq!(
        total, TOTAL_FIXTURES,
        "the documented corpus total is wrong"
    );
}

/// Unknown members must survive a round trip, in order — the property that
/// makes this crate safe to put in an integration path.
#[test]
fn vendor_extensions_survive_in_order() {
    let json = serde_json::json!({
        "id": "7655",
        "name": "Basic Firewall for Business",
        "@type": "ProductOffering",
        "zzz-vendor": {"nested": [1, 2, 3]},
        "aaa-vendor": "second",
    });

    let parsed: ProductOffering = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(parsed.extensions.len(), 2);

    // Insertion order, not sorted order.
    let keys: Vec<&String> = parsed.extensions.iter().map(|(k, _)| k).collect();
    assert_eq!(keys, ["zzz-vendor", "aaa-vendor"]);

    similar_asserts::assert_eq!(
        expected: json,
        actual: serde_json::to_value(&parsed).unwrap()
    );
}

/// A payload omitting the spec-mandatory `@type` parses, keeps its shape, and
/// still knows what class it is.
///
/// TMF630 requires `@type`, but servers omit it and so do TM Forum's own
/// examples. Adding one back would make this crate unusable for relaying a
/// payload unchanged, so the absence is preserved and the class comes from the
/// type instead.
#[test]
fn a_missing_type_is_preserved_not_invented() {
    let json = r#"{"id":"1","name":"n"}"#;
    let parsed: ProductOffering =
        serde_json::from_str(json).expect("should tolerate a missing @type");

    assert_eq!(parsed.type_name(), "ProductOffering");
    assert_eq!(serde_json::to_string(&parsed).unwrap(), json);
}

/// Anything this crate *builds* declares its class, because a request without
/// `@type` is the one a conformant server rejects.
#[test]
fn a_constructed_payload_declares_its_type() {
    let built = ProductOffering::builder().id("1").build();
    assert_eq!(
        serde_json::to_string(&built).unwrap(),
        r#"{"id":"1","@type":"ProductOffering"}"#
    );
}

/// A timestamp keeps the offset the server sent.
///
/// TMF620's own examples carry `-04:00`; normalising that to `Z` is the same
/// instant and a different document, which shows up as a spurious diff in any
/// middleware that compares payloads.
#[test]
fn timestamps_keep_their_offset() {
    let json = serde_json::json!({
        "@type": "ProductOffering",
        "lastUpdate": "2020-09-23T16:42:23-04:00",
    });
    let parsed: ProductOffering = serde_json::from_value(json.clone()).unwrap();
    similar_asserts::assert_eq!(
        expected: json,
        actual: serde_json::to_value(&parsed).unwrap()
    );
}

/// Each client must report the specification version its corpus was taken from.
///
/// `TMF_VERSION` says `v5` for the whole crate, but the covered APIs sit on
/// different patch releases. A consumer that logs or asserts which contract it
/// is speaking needs the precise one, and a constant nobody checks is a
/// constant that drifts — so this reads it back out of the vendored manifests.
#[cfg(all(
    feature = "api-tmf620",
    feature = "api-tmf621",
    feature = "api-tmf622",
    feature = "api-tmf629",
    feature = "api-tmf632",
    feature = "api-tmf634",
    feature = "api-tmf637",
    feature = "api-tmf638",
    feature = "api-tmf639",
    feature = "api-tmf642",
    feature = "api-tmf666",
    feature = "api-tmf678"
))]
#[test]
fn every_client_reports_the_version_it_was_modelled_from() {
    use rutmf::api::{
        tmf620, tmf621, tmf622, tmf629, tmf632, tmf634, tmf637, tmf638, tmf639, tmf642, tmf666,
        tmf669, tmf678, tmf679,
    };

    let declared = [
        ("tmf620", tmf620::SPEC_VERSION),
        ("tmf621", tmf621::SPEC_VERSION),
        ("tmf622", tmf622::SPEC_VERSION),
        ("tmf629", tmf629::SPEC_VERSION),
        ("tmf632", tmf632::SPEC_VERSION),
        ("tmf634", tmf634::SPEC_VERSION),
        ("tmf637", tmf637::SPEC_VERSION),
        ("tmf638", tmf638::SPEC_VERSION),
        ("tmf639", tmf639::SPEC_VERSION),
        ("tmf642", tmf642::SPEC_VERSION),
        ("tmf666", tmf666::SPEC_VERSION),
        ("tmf669", tmf669::SPEC_VERSION),
        ("tmf679", tmf679::SPEC_VERSION),
        ("tmf678", tmf678::SPEC_VERSION),
    ];
    assert_eq!(
        declared.len(),
        APIS.len(),
        "every vendored API needs a client constant, and the reverse"
    );

    for (api, version) in declared {
        let expected = APIS.iter().find(|(name, _, _)| *name == api).map_or_else(
            || panic!("{api} is not in APIS"),
            |(_, version, _)| *version,
        );
        assert_eq!(
            version, expected,
            "{api}::SPEC_VERSION disagrees with the vendored corpus"
        );
        // And the corpus itself agrees with its manifest.
        assert_eq!(load_from(api, "_manifest.json")["spec_version"], expected);
    }
}

/// Each client's `API_PATH` matches the path the corpus actually uses.
///
/// A wrong root is the most expensive kind of quiet mistake: `from_host` builds
/// a URL that `404`s against every operation, and no schema or round-trip check
/// can see it, because the path never appears in a payload's *body*.
///
/// It appears in the `href`s, though, so that is the evidence. Two were wrong
/// when this was written — TMF634 was `resourceCatalogManagement` where its own
/// `servers` block and all 132 of its example `href`s say `resourceCatalog`,
/// and TMF639 was `resourceInventory` where seventy `href`s across three
/// specifications say `resourceInventoryManagement`.
#[cfg(all(
    feature = "api-tmf620",
    feature = "api-tmf621",
    feature = "api-tmf622",
    feature = "api-tmf629",
    feature = "api-tmf632",
    feature = "api-tmf634",
    feature = "api-tmf637",
    feature = "api-tmf638",
    feature = "api-tmf639",
    feature = "api-tmf642",
    feature = "api-tmf666",
    feature = "api-tmf669",
    feature = "api-tmf678",
    feature = "api-tmf679"
))]
#[test]
fn every_client_uses_the_api_path_the_corpus_uses() {
    use rutmf::api::{
        tmf620, tmf621, tmf622, tmf629, tmf632, tmf634, tmf637, tmf638, tmf639, tmf642, tmf666,
        tmf669, tmf678, tmf679,
    };

    /// APIs whose own examples disagree with the path the crate uses, and why
    /// the crate is right anyway.
    const EXCEPTIONS: &[(&str, &str)] = &[
        // TMF632's own examples say `party/v5`, but its `servers` block says
        // `partyManagement/v5` and 223 references from nine other
        // specifications agree with the servers block. Its examples are the
        // outlier, not the crate.
        (
            "tmf632",
            "TMF632's own examples are outvoted by its servers block \
                    and 223 cross-references",
        ),
        // TMF678's example hrefs carry `Customer_Bill_Management` — the
        // document's *title*, underscored, which is no TM Forum path
        // convention. The only real URL evidence is TMF621's nine references
        // to `customerBillManagement`.
        (
            "tmf678",
            "TMF678's examples carry the document title, not a path",
        ),
        // TMF669's examples are carried over from v4 and still say `/v4/`.
        // The path segment matches; only the version is stale.
        ("tmf669", "TMF669's examples are stale at v4"),
    ];

    let declared = [
        ("tmf620", tmf620::API_PATH),
        ("tmf621", tmf621::API_PATH),
        ("tmf622", tmf622::API_PATH),
        ("tmf629", tmf629::API_PATH),
        ("tmf632", tmf632::API_PATH),
        ("tmf634", tmf634::API_PATH),
        ("tmf637", tmf637::API_PATH),
        ("tmf638", tmf638::API_PATH),
        ("tmf639", tmf639::API_PATH),
        ("tmf642", tmf642::API_PATH),
        ("tmf666", tmf666::API_PATH),
        ("tmf669", tmf669::API_PATH),
        ("tmf679", tmf679::API_PATH),
        ("tmf678", tmf678::API_PATH),
    ];

    let mut failures = Vec::new();
    for (api, path) in declared {
        if EXCEPTIONS.iter().any(|(name, _)| *name == api) {
            continue;
        }
        let Some(observed) = api_path_in_corpus(api) else {
            panic!("{api}: no example carries a self-href to read a path from");
        };
        let expected = format!("tmf-api/{observed}");
        if path != expected {
            failures.push(format!(
                "{api}: the client says `{path}`, its own examples say `{expected}`"
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "a wrong API root makes every request 404, and nothing else notices:\n{}",
        failures.join("\n")
    );
}

/// The `{name}/v{n}` segment an API's own top-level `href`s use, if they agree.
///
/// Only *self*-hrefs count — the `href` of a fixture's root object. A nested
/// `href` usually points into another API, which says nothing about this one.
#[cfg(feature = "api-tmf620")]
fn api_path_in_corpus(api: &str) -> Option<String> {
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for (_, value) in all_fixtures(api) {
        let items = match &value {
            Value::Array(items) => items.clone(),
            other => vec![other.clone()],
        };
        for item in items {
            let Some(href) = item.get("href").and_then(Value::as_str) else {
                continue;
            };
            if let Some(segment) = path_segment(href) {
                *counts.entry(segment).or_default() += 1;
            }
        }
    }
    counts.into_iter().max_by_key(|(_, n)| *n).map(|(s, _)| s)
}

/// Pulls `{name}/v{n}` out of an href, without a regex crate.
#[cfg(feature = "api-tmf620")]
fn path_segment(href: &str) -> Option<String> {
    let parts: Vec<&str> = href.split('/').collect();
    parts.windows(2).find_map(|pair| {
        let (name, version) = (pair[0], pair[1]);
        let is_version = version
            .strip_prefix('v')
            .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
        let is_name = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphabetic());
        (is_version && is_name).then(|| format!("{name}/{version}"))
    })
}
