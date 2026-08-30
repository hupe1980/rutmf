//! Property-based tests for the round-trip guarantee.
//!
//! The conformance suite proves fidelity for the payloads TM Forum publishes.
//! These prove it for payloads nobody has written down: arbitrary vendor
//! extensions, arbitrary decimals, arbitrary nesting.

use proptest::prelude::*;
use serde_json::{Map, Value, json};

use rutmf::core::{Money, Ref, TimePeriod};
use rutmf::product::{ProductOffering, ProductSpecification};

/// JSON values shallow enough to keep shrinking fast, deep enough to be real.
fn arb_json() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::from),
        any::<i32>().prop_map(Value::from),
        "[a-zA-Z0-9 _-]{0,24}".prop_map(Value::from),
    ];
    leaf.prop_recursive(3, 24, 4, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..4).prop_map(Value::Array),
            prop::collection::hash_map("[a-z][a-z0-9_]{0,10}", inner, 0..4)
                .prop_map(|m| Value::Object(m.into_iter().collect())),
        ]
    })
}

/// Extension member names that cannot collide with a modelled field.
fn arb_extension_key() -> impl Strategy<Value = String> {
    "x-[a-z][a-z0-9-]{0,12}".prop_map(String::from)
}

proptest! {
    /// Arbitrary vendor extensions on a resource survive decode → encode.
    #[test]
    fn extensions_survive_round_trip(
        extensions in prop::collection::vec((arb_extension_key(), arb_json()), 0..6),
        name in "[a-zA-Z ]{0,30}",
        id in "[0-9]{1,8}",
    ) {
        let mut object = Map::new();
        object.insert("id".into(), Value::String(id));
        object.insert("name".into(), Value::String(name));
        object.insert("@type".into(), Value::String("ProductOffering".into()));
        for (key, value) in extensions {
            object.insert(key, value);
        }
        let original = Value::Object(object);

        let parsed: ProductOffering = serde_json::from_value(original.clone())?;
        let reserialised = serde_json::to_value(&parsed)?;

        prop_assert_eq!(original, reserialised);
    }

    /// An explicitly empty array is distinct from an absent one, in both
    /// directions — the property that forced list members to be `Option<Vec<_>>`.
    #[test]
    fn empty_and_absent_arrays_stay_distinct(present in any::<bool>()) {
        let mut object = Map::new();
        object.insert("id".into(), Value::String("1".into()));
        object.insert("@type".into(), Value::String("ProductSpecification".into()));
        if present {
            object.insert("attachment".into(), Value::Array(vec![]));
        }
        let original = Value::Object(object);

        let parsed: ProductSpecification = serde_json::from_value(original.clone())?;
        prop_assert_eq!(parsed.attachment.is_some(), present);
        prop_assert_eq!(original, serde_json::to_value(&parsed)?);
    }

    /// Money keeps its exact decimal value and its integral-vs-fractional form.
    #[test]
    fn money_round_trips_exactly(units in -1_000_000i64..1_000_000, cents in 0u32..100) {
        let literal = if cents == 0 {
            json!(units)
        } else {
            json!(format!("{units}.{cents:02}").parse::<f64>().unwrap())
        };
        let original = json!({"unit": "EUR", "value": literal});

        let parsed: Money = serde_json::from_value(original.clone())?;
        prop_assert_eq!(original, serde_json::to_value(&parsed)?);
    }

    /// A typed reference preserves every member it was given.
    #[test]
    fn references_round_trip(
        id in "[0-9]{1,8}",
        name in prop::option::of("[a-zA-Z ]{1,20}"),
        referred in prop::option::of("[A-Z][a-zA-Z]{1,20}"),
    ) {
        let mut object = Map::new();
        object.insert("id".into(), Value::String(id));
        if let Some(name) = name {
            object.insert("name".into(), Value::String(name));
        }
        if let Some(referred) = referred {
            object.insert("@referredType".into(), Value::String(referred));
        }
        object.insert("@type".into(), Value::String("ProductSpecificationRef".into()));
        let original = Value::Object(object);

        let parsed: Ref<ProductSpecification> = serde_json::from_value(original.clone())?;
        prop_assert_eq!(original, serde_json::to_value(&parsed)?);
    }

    /// The presence or absence of `@type` survives untouched, whatever else the
    /// payload carries.
    ///
    /// The crate used to add one when the server omitted it, which is a member
    /// invented in something being relayed. TM Forum's own examples omit it.
    #[test]
    fn the_discriminator_is_neither_invented_nor_dropped(
        declared in prop::option::of("[A-Z][a-zA-Z]{1,20}"),
        id in "[0-9]{1,8}",
    ) {
        let mut object = Map::new();
        object.insert("id".into(), Value::String(id));
        if let Some(declared) = &declared {
            object.insert("@type".into(), Value::String(declared.clone()));
        }
        let original = Value::Object(object);

        let parsed: ProductOffering = serde_json::from_value(original.clone())?;
        prop_assert_eq!(&original, &serde_json::to_value(&parsed)?);

        // Absent or not, the class is still known.
        prop_assert_eq!(
            parsed.type_name(),
            declared.as_deref().unwrap_or("ProductOffering")
        );
    }

    /// A characteristic's declared class always agrees with the value it carries.
    ///
    /// The subclass is derived from the value, so the two cannot disagree by
    /// construction — but "cannot" is a claim about a `match` arm over eight
    /// JSON shapes and their array forms, which is exactly the sort of thing a
    /// generator finds a hole in. The payload must also round-trip, because
    /// deriving `@type` is a member the crate *writes*.
    #[test]
    fn a_derived_characteristic_class_matches_its_value(
        name in "[a-zA-Z][a-zA-Z0-9]{0,12}",
        value in arb_json(),
    ) {
        use rutmf::core::{Characteristic, ValueKind};

        let built = Characteristic::new(&name, value.clone());
        let shape = ValueKind::of_value(&value);

        if let Some(class) = shape.characteristic_type() {
            // The value names a subclass: the built characteristic declares it,
            // and reading it back gives the shape it was derived from.
            prop_assert_eq!(built.type_name(), class);
            prop_assert_eq!(built.value_kind(), shape);
        } else {
            // It does not: the base class stays, rather than a guess going on
            // the wire that the value contradicts.
            prop_assert_eq!(built.type_name(), "Characteristic");
        }

        // And whatever was derived, the payload round-trips by value — except
        // for the documented case of an explicit `null` on a modelled member,
        // which is read as absence. `null` is also the one value that names no
        // subclass, so the two exceptions are the same input.
        let encoded = serde_json::to_value(&built)?;
        let decoded: Characteristic = serde_json::from_value(encoded.clone())?;
        if value.is_null() {
            prop_assert_eq!(decoded.value, None);
        } else {
            prop_assert_eq!(&encoded, &serde_json::to_value(&decoded)?);
            prop_assert_eq!(decoded.value.as_ref(), Some(&value));
        }
    }

    /// An explicit `null` is read as absence on a modelled member, and survives
    /// on one the crate does not model.
    ///
    /// This is the single exception to the round-trip guarantee, so it is pinned
    /// rather than left to be rediscovered: `Option<T>` has two states where this
    /// needs three, and the alternative costs every caller on every field. What
    /// must not drift is *which* members it applies to.
    #[test]
    fn an_explicit_null_is_absence_on_a_modelled_member_and_survives_elsewhere(
        vendor_key in arb_extension_key(),
        id in "[0-9]{1,8}",
    ) {
        let original = json!({
            "id": id,
            "name": Value::Null,
            "@type": "ProductOffering",
            vendor_key.clone(): Value::Null,
        });

        let parsed: ProductOffering = serde_json::from_value(original)?;
        let out = serde_json::to_value(&parsed)?;

        prop_assert_eq!(parsed.name, None, "a modelled member reads as absent");
        prop_assert!(out.get("name").is_none(), "and is not re-emitted");
        prop_assert_eq!(
            out.get(&vendor_key),
            Some(&Value::Null),
            "an unmodelled one keeps its null, because `Extensions` can hold it"
        );
    }

    /// A merge patch can remove a member, which is half of what RFC 7386 does.
    #[test]
    fn a_patch_body_can_delete_a_member(
        name in "[a-zA-Z ]{1,20}",
        member in prop::sample::select(vec!["description", "lifecycleStatus", "version"]),
    ) {
        use rutmf::product::ProductOfferingUpdate;

        let update = ProductOfferingUpdate::builder()
            .name(name.clone())
            .build()
            .deleting(member);

        prop_assert!(update.deletes(member));

        let body = serde_json::to_value(&update)?;
        prop_assert_eq!(body[member].clone(), Value::Null);
        prop_assert_eq!(body["name"].clone(), Value::String(name));
    }

    /// A timestamp keeps the UTC offset it arrived with.
    ///
    /// Parsing into `DateTime<Utc>` would rewrite `-04:00` to `Z`: the same
    /// instant, a different document, and a spurious diff in any middleware
    /// that compares payloads.
    /// Zero offset is the one exception, and it is a spelling rather than a
    /// value: `+00:00` comes back as the `Z` RFC 3339 prefers.
    #[test]
    fn timestamps_keep_their_offset(
        hours in prop_oneof![-12i32..=-1, 1i32..=14],
        minutes in prop_oneof![Just(0), Just(30), Just(45)],
    ) {
        let sign = if hours < 0 { '-' } else { '+' };
        let offset = format!("{sign}{:02}:{minutes:02}", hours.abs());
        let original = json!({
            "@type": "ProductOffering",
            "lastUpdate": format!("2020-09-23T16:42:23{offset}"),
        });

        let parsed: ProductOffering = serde_json::from_value(original.clone())?;
        prop_assert_eq!(original, serde_json::to_value(&parsed)?);
    }

    /// A time period containing an instant must not also exclude it.
    #[test]
    fn time_period_containment_is_consistent(
        start_offset in -10_000i64..10_000,
        length in 1i64..10_000,
        probe in -20_000i64..20_000,
    ) {
        use chrono::{DateTime, TimeDelta, Utc};

        let epoch = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let start = epoch + TimeDelta::seconds(start_offset);
        let end = start + TimeDelta::seconds(length);
        let at = epoch + TimeDelta::seconds(probe);

        let closed = TimePeriod::between(start, end);
        prop_assert_eq!(closed.contains(at), at >= start && at < end);

        // An open period is unbounded on the missing side.
        prop_assert!(TimePeriod::starting(start).contains(at) == (at >= start));
        prop_assert!(TimePeriod::until(end).contains(at) == (at < end));
    }
}

/// `+00:00` and `Z` are the same offset; RFC 3339 prefers the latter, and so
/// does `chrono`. It is the only re-spelling a timestamp undergoes.
#[test]
fn a_zero_offset_is_written_as_z() {
    let parsed: ProductOffering = serde_json::from_value(json!({
        "@type": "ProductOffering",
        "lastUpdate": "2020-09-23T16:42:23+00:00",
    }))
    .unwrap();

    assert_eq!(
        serde_json::to_value(&parsed).unwrap()["lastUpdate"],
        json!("2020-09-23T16:42:23Z")
    );
}

/// A payload nesting extensions inside a modelled sub-object still round-trips.
#[test]
fn extensions_nested_in_sub_objects_survive() {
    let original = json!({
        "id": "1",
        "@type": "ProductOffering",
        "productSpecification": {
            "id": "9881",
            "@type": "ProductSpecificationRef",
            "x-nested-vendor": {"deep": [1, {"deeper": true}]},
        },
        "validFor": {"startDateTime": "2020-09-23T00:00:00Z"},
    });

    let parsed: ProductOffering = serde_json::from_value(original.clone()).unwrap();
    assert_eq!(serde_json::to_value(&parsed).unwrap(), original);
}
