//! TM Forum value objects: [`Money`], [`Quantity`], [`Duration`] and
//! [`TimePeriod`].
//!
//! The v5 OAS defines these four without `@type` and without an extension
//! point, which makes them the only types in the crate that
//! [`tmf_value!`](super::macros) rather than `tmf_struct!` declares.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::attachment::Attachment;
use super::macros::{tmf_struct, tmf_value};

tmf_value! {
    /// A monetary amount in a given currency.
    ///
    /// # Precision
    ///
    /// The v5 OAS types `Money.value` as `number/float`. Storing money in
    /// binary floating point is a defect regardless of what the schema says, so
    /// this crate parses into [`Decimal`] and re-emits a JSON number. Values are
    /// accepted from a JSON number *or* a string, which covers servers that
    /// (correctly) send `"12.34"` to dodge the float problem.
    ///
    /// ```
    /// use rutmf::core::Money;
    /// use rust_decimal::Decimal;
    /// use std::str::FromStr;
    ///
    /// let m: Money = serde_json::from_str(r#"{"unit":"EUR","value":12.34}"#).unwrap();
    /// assert_eq!(m.value, Some(Decimal::from_str("12.34").unwrap()));
    /// assert_eq!(serde_json::to_string(&m).unwrap(), r#"{"unit":"EUR","value":12.34}"#);
    /// ```
    pub struct Money {
        /// Currency, as an ISO 4217 three-letter code.
        unit: String,
        @decimal {
            /// A signed amount; the meaning of the sign depends on the using API.
            value: Decimal,
        }
    }
}

impl Money {
    /// Creates a monetary amount.
    pub fn new(unit: impl Into<String>, value: Decimal) -> Self {
        Self::builder().unit(unit).value(value).build()
    }
}

tmf_struct! {
    @name = "Price";
    /// A priced amount, before and after tax.
    ///
    /// Distinct from [`Money`]: that is an amount, this is what something
    /// *costs* — the duty-free and tax-included figures carried separately.
    ///
    /// TMF622 and TMF637 declare this schema identically, so an order line and
    /// an inventory record price the same way.
    pub struct Price {
        /// The amount before tax.
        duty_free_amount: Money,
        /// The amount including tax.
        tax_included_amount: Money,
        @decimal {
            /// Tax rate applied, as a percentage.
            tax_rate: Decimal,
            /// A percentage applied instead of an absolute amount.
            percentage: Decimal,
        }
    }
}

tmf_value! {
    /// An amount in a given unit, e.g. `5 GB`.
    pub struct Quantity {
        /// The unit of measure.
        units: String,
        @decimal {
            /// Numeric value in the given unit.
            amount: Decimal,
        }
    }
}

tmf_value! {
    /// A time interval expressed as a count of some unit.
    ///
    /// Distinct from [`TimePeriod`], which is an absolute start/end pair.
    pub struct Duration {
        /// Number of units (seconds, minutes, hours, …).
        amount: i64,
        /// Unit of time.
        units: String,
    }
}

tmf_value! {
    /// A period of time: a start, an end, or both.
    ///
    /// ```
    /// use rutmf::core::TimePeriod;
    ///
    /// let p: TimePeriod = serde_json::from_str(
    ///     r#"{"startDateTime":"2020-09-23T00:00:00Z"}"#,
    /// ).unwrap();
    /// assert!(p.end_date_time.is_none());
    /// ```
    pub struct TimePeriod {
        /// Start of the period (RFC 3339).
        start_date_time: Timestamp,
        /// End of the period (RFC 3339).
        end_date_time: Timestamp,
    }
}

impl TimePeriod {
    /// A period starting at `start` with no defined end.
    pub fn starting(start: impl Into<Timestamp>) -> Self {
        Self::builder().start_date_time(start.into()).build()
    }

    /// A period ending at `end` with no defined start (a deadline).
    pub fn until(end: impl Into<Timestamp>) -> Self {
        Self::builder().end_date_time(end.into()).build()
    }

    /// A closed period.
    pub fn between(start: impl Into<Timestamp>, end: impl Into<Timestamp>) -> Self {
        Self::builder()
            .start_date_time(start.into())
            .end_date_time(end.into())
            .build()
    }

    /// Whether `at` falls inside the period.
    ///
    /// Half-open: the start is included, the end is not, so adjacent periods
    /// tile without overlapping. Absent bounds are unbounded. Comparison is by
    /// instant, so a bound written in one offset and an `at` in another still
    /// order correctly.
    pub fn contains(&self, at: impl Into<Timestamp>) -> bool {
        let at = at.into();
        self.start_date_time.is_none_or(|s| at >= s) && self.end_date_time.is_none_or(|e| at < e)
    }
}

/// A TM Forum timestamp: an RFC 3339 instant that keeps the offset it arrived
/// with.
///
/// TMF payloads carry timestamps as RFC 3339 strings, and TM Forum's own
/// examples use offsets other than `Z` — `2020-09-23T16:42:23-04:00` appears in
/// the TMF620 v5 examples. Parsing those into `Timestamp` would re-emit
/// them as `2020-09-23T20:42:23Z`: the same instant, a different document, and
/// a spurious diff in any middleware that compares payloads. So the model keeps
/// the offset.
///
/// Construction stays ergonomic, because every builder setter takes
/// `impl Into<_>` and chrono converts:
///
/// ```
/// use chrono::Utc;
/// use rutmf::product::ProductOfferingCreate;
///
/// let body = ProductOfferingCreate::builder()
///     .name("Business Internet")
///     .lifecycle_status("Active")
///     .last_update(Utc::now()) // `Timestamp` converts on the way in
///     .build();
/// ```
///
/// To compare or do arithmetic against UTC, call
/// [`to_utc`](chrono::DateTime::to_utc):
///
/// ```
/// use chrono::Utc;
/// use rutmf::core::Timestamp;
///
/// let t: Timestamp = "2020-09-23T16:42:23-04:00".parse().unwrap();
/// assert!(t.to_utc() < Utc::now());
/// ```
pub type Timestamp = chrono::DateTime<chrono::FixedOffset>;

tmf_struct! {
    @name = "TaxDefinition";
    /// One tax a [`TaxExemptionCertificate`] applies to.
    ///
    /// Eight of the fourteen vendored specifications declare this schema, byte
    /// for byte, so it is one type here rather than one per domain.
    pub struct TaxDefinition {
        /// Identifier of the tax.
        id: String,
        /// Name of the tax.
        name: String,
        /// Kind of tax, e.g. `VAT`.
        tax_type: String,
        /// Level of the jurisdiction levying it, e.g. `national`.
        jurisdiction_level: String,
        /// Name of the jurisdiction levying it.
        jurisdiction_name: String,
        /// Period during which the tax applies.
        valid_for: TimePeriod,
    }
}

tmf_struct! {
    @name = "TaxExemptionCertificate";
    /// A certificate exempting a party from a tax.
    ///
    /// Declared identically by the same eight specifications as
    /// [`TaxDefinition`].
    pub struct TaxExemptionCertificate {
        /// Identifier of the certificate.
        id: String,
        /// The certificate number.
        certificate_number: String,
        /// Jurisdiction granting the exemption.
        issuing_jurisdiction: String,
        /// Why the exemption was granted.
        reason: String,
        /// The taxes the certificate exempts the party from.
        tax_definition: Vec<TaxDefinition>,
        /// Period during which the certificate is valid.
        valid_for: TimePeriod,
        /// A scan of the certificate.
        attachment: Attachment,
    }
}

tmf_struct! {
    @name = "RelatedPlaceRefOrValue";
    /// A place in a named role relative to a resource.
    ///
    /// TMF622, TMF637, TMF638 and TMF679 declare `RelatedPlaceRefOrValue`
    /// identically, so this is one type rather than four. TMF639 and TMF642
    /// each declare a
    /// *differently named* schema for the same idea — `RelatedPlaceRef` and
    /// `RelatedPlace` — and those stay separate, in
    /// [`resource`](crate::resource) and [`alarm`](crate::alarm): merging them
    /// would assert an equivalence TM Forum has not.
    pub struct RelatedPlace {
        /// The place being referred to, or carried inline.
        place: PlaceRefOrValue,
        /// The role the place plays, e.g. `installationAddress`.
        role: String,
    }
}

tmf_struct! {
    @name = "CreditProfile";
    /// A point-in-time credit assessment of a customer relationship.
    ///
    /// Seven of the vendored specifications declare this schema identically. It
    /// lives here rather than in [`customer`](crate::customer) because
    /// [`party`](crate::party) needs it too, and `party` sits below `customer`.
    ///
    /// Distinct from [`PartyCreditProfile`](crate::party::PartyCreditProfile),
    /// which records a rating held by an external agency about a *party*; this
    /// is the provider's own assessment of a *customer relationship*.
    pub struct CreditProfile {
        /// Identifier of the profile.
        id: String,
        /// URI of the profile.
        href: String,
        /// When the assessment was made.
        credit_profile_date: Timestamp,
        /// The assessed score.
        credit_score: i64,
        /// The assessed risk rating.
        credit_risk_rating: i64,
        /// Period during which the assessment holds.
        valid_for: TimePeriod,
    }
}

tmf_struct! {
    @name = "TaxItem";
    /// One tax rate and category applied to a charge.
    ///
    /// TMF620 and TMF678 declare this schema identically. Both spell `taxRate`
    /// as `number/float`; it is a [`Decimal`] here for the same reason
    /// [`Money`] is — a rate multiplied into a monetary amount inherits every
    /// rounding error the binary float brought with it.
    pub struct TaxItem {
        /// Kind of tax, e.g. `VAT`.
        tax_category: String,
        /// The tax amount.
        tax_amount: Money,
        @decimal {
            /// Tax rate as a percentage.
            tax_rate: Decimal,
        }
    }
}

/// How far a task-shaped request has got.
///
/// TMF622 and TMF679 declare `TaskStateType` identically, so this is one type:
/// it is the state of a `CancelProductOrder` and of a product-offering
/// qualification alike.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum TaskState {
    /// Received.
    #[serde(rename = "acknowledged")]
    Acknowledged,
    /// Refused.
    #[serde(rename = "rejected")]
    Rejected,
    /// Being processed.
    #[serde(rename = "inProgress")]
    InProgress,
    /// Withdrawn.
    #[serde(rename = "cancelled")]
    Cancelled,
    /// Finished.
    #[serde(rename = "done")]
    Done,
    /// Finished unsuccessfully.
    #[serde(rename = "terminatedWithError")]
    TerminatedWithError,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

impl TaskState {
    /// Whether the task has stopped moving.
    ///
    /// The task state TMF622 and TMF679 share, so a `CancelProductOrder` and a
    /// qualification are polled the way every other task-shaped resource is:
    ///
    /// ```
    /// use rutmf::core::TaskState;
    ///
    /// assert!(TaskState::Done.is_finished());
    /// assert!(TaskState::Rejected.is_finished());
    /// assert!(!TaskState::InProgress.is_finished());
    /// ```
    ///
    /// An unrecognised state is **not** finished, so a client polling a task
    /// keeps polling rather than giving up on a state it does not know.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        matches!(
            self,
            Self::Done | Self::Rejected | Self::Cancelled | Self::TerminatedWithError
        )
    }

    /// Whether the task finished by doing what was asked.
    ///
    /// Narrower than [`is_finished`](Self::is_finished), which a rejected,
    /// cancelled or errored task also satisfies.
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Done)
    }
}

tmf_struct! {
    @name = "PlaceRefOrValue", @ref = "PlaceRef";
    /// A place, as a reference *or* as the thing itself.
    ///
    /// TMF622, TMF637, TMF638 and TMF679 all type `RelatedPlace.place` as
    /// `PlaceRefOrValue`, a `oneOf` over `GeographicLocation`,
    /// `GeographicSite`, `GeographicAddress` and a plain `PlaceRef`. Only the
    /// last of those carries an `id`, so a [`Ref<Place>`](crate::core::Ref)
    /// — which requires one — can parse just one arm in four. TMF679's corpus
    /// is where that showed up: it sends an inline `GeographicAddress` with no
    /// `id` at all.
    ///
    /// The three value arms belong to APIs this crate does not model (TMF673,
    /// TMF674, TMF675), so their members are not invented here — they arrive in
    /// [`extensions`](Self::extensions) and round-trip untouched. Ask
    /// [`type_name`](Self::type_name) which arm a server sent.
    pub struct PlaceRefOrValue {
        /// Identifier of the place. Absent when a value arm carries no `id`.
        id: String,
        /// Canonical URI of the place, when it has one.
        href: String,
        /// Name of the place.
        name: String,
        @renamed {
            /// The concrete class of the place — `GeographicAddress`,
            /// `GeographicSite`, `GeographicLocation` or `PlaceRef`.
            "@referredType" referred_type: String,
        }
    }
}

/// How one feature relates to another.
///
/// TMF634, TMF638 and TMF639 each declare this vocabulary, byte for byte, on
/// their own feature-relationship schema. The three *schemas* differ — TMF638's
/// is a bare object, TMF639's extends `EntityRef` — but the set of values does
/// not, so the enumeration lives here rather than three times over.
///
/// Note [`MayInclude`](Self::MayInclude): the wire value contains a space.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum FeatureRelationshipType {
    /// The two features cannot both be present.
    #[serde(rename = "excluded")]
    Excluded,
    /// Selecting this feature brings the other with it.
    #[serde(rename = "includes")]
    Includes,
    /// The other feature is optional alongside this one.
    #[serde(rename = "may include")]
    MayInclude,
    /// This feature cannot be selected without the other.
    #[serde(rename = "requires")]
    Requires,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

/// serde adapter for `Option<Decimal>` on the TMF wire format.
///
/// Accepts a JSON number *or* a numeric string, and re-emits a JSON number
/// keeping integers integral. Public so that code defining its own TMF-shaped
/// types can reuse it:
///
/// ```
/// use rust_decimal::Decimal;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct Charge {
///     #[serde(default, with = "rutmf::core::decimal_opt")]
///     amount: Option<Decimal>,
/// }
///
/// let c: Charge = serde_json::from_str(r#"{"amount":"9.99"}"#).unwrap();
/// assert_eq!(serde_json::to_string(&c).unwrap(), r#"{"amount":9.99}"#);
/// ```
///
/// # Precision
///
/// A [`Decimal`] carries up to 28 significant digits; a JSON number that
/// `serde_json` can represent carries about 17. Serialising an integral value
/// is exact, and so is any value `f64` can round-trip — which covers every
/// monetary amount. Beyond that the emitted number is the nearest `f64`. Send
/// such values as strings if a server accepts them; the decoder here reads both.
pub mod decimal_opt {
    use std::fmt;
    use std::str::FromStr;

    use rust_decimal::Decimal;
    use rust_decimal::prelude::ToPrimitive;
    use serde::de::{self, Visitor};
    use serde::{Deserializer, Serializer};

    /// Serialises an optional decimal as a JSON number.
    pub fn serialize<S: Serializer>(
        value: &Option<Decimal>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let Some(d) = value else {
            return serializer.serialize_none();
        };

        // Emitted as a JSON number to match the OAS (`type: number`). An
        // integral amount is written as an integer so that a payload carrying
        // `50` does not come back as `50.0` — round-trip fidelity is asserted
        // over the vendored spec examples, which use both forms.
        if d.is_integer() {
            if let Some(i) = d.to_i64() {
                return serializer.serialize_i64(i);
            }
            if let Some(u) = d.to_u64() {
                return serializer.serialize_u64(u);
            }
        }
        serializer.serialize_f64(
            d.to_string()
                .parse::<f64>()
                .map_err(serde::ser::Error::custom)?,
        )
    }

    /// Deserialises an optional decimal from a JSON number or string.
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Decimal>, D::Error> {
        struct V;

        impl<'de> Visitor<'de> for V {
            type Value = Option<Decimal>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a number or numeric string")
            }

            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
                // `to_string` gives the shortest round-trip representation, so
                // 12.34_f64 becomes exactly 12.34 rather than 12.339999….
                Decimal::from_str(&v.to_string())
                    .map(Some)
                    .map_err(de::Error::custom)
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(Some(Decimal::from(v)))
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(Some(Decimal::from(v)))
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Decimal::from_str(v).map(Some).map_err(de::Error::custom)
            }

            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }

            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }

            fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
                d.deserialize_any(V)
            }
        }

        deserializer.deserialize_option(V)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn a_feature_relationship_type_keeps_the_space_the_wire_has() {
        // `may include` is the one v5 enumeration value containing a space. A
        // Rust-shaped guess — `may_include`, `mayInclude` — is a value no
        // conformant server accepts, which is the whole reason this member is
        // not a `String`.
        let value = FeatureRelationshipType::MayInclude;
        assert_eq!(serde_json::to_string(&value).unwrap(), r#""may include""#);
        assert_eq!(
            serde_json::from_str::<FeatureRelationshipType>(r#""may include""#).unwrap(),
            value
        );

        // An unknown value still round-trips rather than failing the parse.
        let unknown: FeatureRelationshipType = serde_json::from_str(r#""supersedes""#).unwrap();
        assert_eq!(unknown, FeatureRelationshipType::Other("supersedes".into()));
        assert_eq!(serde_json::to_string(&unknown).unwrap(), r#""supersedes""#);
    }

    #[test]
    fn money_accepts_number_and_string() {
        let a: Money = serde_json::from_str(r#"{"unit":"EUR","value":12.34}"#).unwrap();
        let b: Money = serde_json::from_str(r#"{"unit":"EUR","value":"12.34"}"#).unwrap();
        assert_eq!(a.value, b.value);
        assert_eq!(a.value, Some(Decimal::from_str("12.34").unwrap()));
    }

    #[test]
    fn money_emits_json_number() {
        let m = Money::new("EUR", Decimal::from_str("12.34").unwrap());
        assert_eq!(
            serde_json::to_string(&m).unwrap(),
            r#"{"unit":"EUR","value":12.34}"#
        );
    }

    #[test]
    fn integral_money_stays_integral() {
        let m: Money = serde_json::from_str(r#"{"unit":"EUR","value":50}"#).unwrap();
        assert_eq!(
            serde_json::to_string(&m).unwrap(),
            r#"{"unit":"EUR","value":50}"#,
            "50 must not come back as 50.0"
        );
    }

    #[test]
    fn value_objects_keep_the_type_their_examples_send() {
        // The v5 schemas give `Duration` no `@type`, yet TMF622's own examples
        // do send one. It must survive rather than being dropped.
        let json = r#"{"amount":30,"units":"day","@type":"Duration"}"#;
        let d: Duration = serde_json::from_str(json).unwrap();
        assert_eq!(d.extensions.get("@type").unwrap(), "Duration");
        assert_eq!(
            serde_json::to_value(&d).unwrap(),
            serde_json::from_str::<serde_json::Value>(json).unwrap()
        );
    }

    #[test]
    fn a_timestamp_keeps_the_offset_it_arrived_with() {
        // TM Forum's own TMF620 examples carry `-04:00`. Normalising it to `Z`
        // would be the same instant and a different document.
        let json = r#""2020-09-23T16:42:23-04:00""#;
        let t: Timestamp = serde_json::from_str(json).unwrap();
        assert_eq!(serde_json::to_string(&t).unwrap(), json);
    }

    #[test]
    fn a_task_state_answers_the_same_question_its_siblings_do() {
        // Every other task-shaped enum in the crate has `is_finished`; this is
        // the one TMF622 and TMF679 share, so a `CancelProductOrder` is polled
        // the way an alarm task is.
        for state in [
            TaskState::Done,
            TaskState::Rejected,
            TaskState::Cancelled,
            TaskState::TerminatedWithError,
        ] {
            assert!(state.is_finished(), "{state:?}");
        }
        for state in [TaskState::Acknowledged, TaskState::InProgress] {
            assert!(!state.is_finished(), "{state:?}");
        }

        // An unknown state keeps a poller polling rather than abandoning a task
        // it does not understand.
        let unknown: TaskState = serde_json::from_str(r#""awaitingApproval""#).unwrap();
        assert_eq!(unknown, TaskState::Other("awaitingApproval".into()));
        assert!(!unknown.is_finished());

        assert!(TaskState::Done.is_success());
        assert!(!TaskState::Cancelled.is_success());
    }

    #[test]
    fn time_period_containment_is_half_open() {
        let start = Timestamp::from_str("2020-01-01T00:00:00Z").unwrap();
        let end = Timestamp::from_str("2021-01-01T00:00:00Z").unwrap();
        let p = TimePeriod::between(start, end);
        assert!(p.contains(start));
        assert!(!p.contains(end));
    }
}

tmf_struct! {
    @name = "Note";
    /// A dated, authored remark attached to a resource.
    ///
    /// Lives in `core` because it is one schema, not five: TMF621, TMF622,
    /// TMF638, TMF639 and TMF679 declare `Note` byte for byte identically, so a
    /// note written against an order is the same shape as one written against a
    /// trouble ticket. Modelling it once is the same call as the one `Product`
    /// gets — and `shared_types_do_not_diverge_between_apis` is what keeps it
    /// honest if a future release makes them differ.
    pub struct Note {
        /// Identifier of the note.
        id: String,
        /// Who wrote it.
        author: String,
        /// When it was written.
        date: Timestamp,
        /// The remark itself.
        text: String,
    }
}

impl Note {
    /// A note with the given text.
    pub fn new(text: impl Into<String>) -> Self {
        Self::builder().text(text).build()
    }
}
