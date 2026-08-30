//! Characteristics and characteristic specifications.
//!
//! Characteristics are how TM Forum models "everything else": the attributes of
//! a product, service or resource that are not part of the fixed schema.

use serde_json::Value;

use super::macros::tmf_struct;
use super::reference::Ref;
use super::value::TimePeriod;

tmf_struct! {
    @name = "Characteristic";
    /// A name/value pair describing one attribute of an entity.
    ///
    /// The v5 OAS gives each value shape its own subclass —
    /// `StringCharacteristic`, `IntegerArrayCharacteristic`, twelve in all. This
    /// is one struct instead: the shared members stay typed, `value` holds any
    /// JSON, and `@type` names the subclass, so a vendor subclass never fails to
    /// parse and [`value_kind`] reads the class back.
    ///
    /// # Build with [`new`]
    ///
    /// The subclass follows from the value, so [`new`] derives it. The builder
    /// cannot — it sets `@type` before it has seen the value — so use it only
    /// for members [`new`] does not set, and follow it with [`with_value`].
    ///
    /// ```
    /// use rutmf::core::{Characteristic, ValueKind};
    ///
    /// let speed = Characteristic::new("downstreamSpeed", 100);
    /// assert_eq!(speed.type_name(), "IntegerCharacteristic");
    /// assert_eq!(speed.value_kind(), ValueKind::Integer);
    ///
    /// let bare = Characteristic::builder().name("downstreamSpeed").value(100).build();
    /// assert_eq!(bare.type_name(), "Characteristic", "the builder cannot know");
    /// ```
    ///
    /// `valueType` is *not* derived: it looks like a JSON type name and is not
    /// one. The corpus uses it for `Quantity` and `Slice5G JSON descriptor` as
    /// readily as for `string`, so it is a domain label only the caller knows.
    ///
    /// [`new`]: Characteristic::new
    /// [`with_value`]: Characteristic::with_value
    /// [`value_kind`]: Characteristic::value_kind
    pub struct Characteristic {
        /// Unique identifier of the characteristic.
        id: String,
        /// Name of the characteristic.
        name: String,
        /// Data type of the value, e.g. `string`, `integer`.
        value_type: String,
        /// The value itself, shaped according to `@type`.
        value: Value,
        /// Relationships to other characteristics.
        characteristic_relationship: Vec<CharacteristicRelationship>,
    }
}

impl Characteristic {
    /// A named characteristic whose class follows from the value.
    ///
    /// ```
    /// use rutmf::core::Characteristic;
    ///
    /// assert_eq!(Characteristic::new("tier", "gold").type_name(), "StringCharacteristic");
    /// assert_eq!(Characteristic::new("enabled", true).type_name(), "BooleanCharacteristic");
    /// assert_eq!(
    ///     Characteristic::new("bands", vec!["n78", "n1"]).type_name(),
    ///     "StringArrayCharacteristic",
    /// );
    /// ```
    ///
    /// A value whose shape names no subclass — `null`, an empty array, an array
    /// of mixed types — leaves the base class rather than guessing at one.
    pub fn new(name: impl Into<String>, value: impl Into<Value>) -> Self {
        Self::builder().name(name).build().with_value(value)
    }

    /// Sets the value, and the `@type` that value implies.
    ///
    /// The class stays as it was when the value names no subclass, so this
    /// never replaces a subclass a server sent with the bare base.
    #[must_use]
    pub fn with_value(mut self, value: impl Into<Value>) -> Self {
        let value = value.into();
        if let Some(class) = ValueKind::of_value(&value).characteristic_type() {
            class.clone_into(&mut self.at_type);
        }
        self.value = Some(value);
        self
    }

    /// Sets the `valueType` label.
    ///
    /// Free text — what the value *means*, not what shape it has, which is
    /// [`value_kind`](Self::value_kind)'s question.
    #[must_use]
    pub fn with_value_type(mut self, value_type: impl Into<String>) -> Self {
        self.value_type = Some(value_type.into());
        self
    }

    /// The value shape this characteristic's `@type` declares.
    ///
    /// Reads the subclass, so it reports what the sender *said*. Compare with
    /// [`ValueKind::of_value`], which reports what the value *is*, to catch a
    /// payload where the two disagree.
    ///
    /// ```
    /// use rutmf::core::{Characteristic, ValueKind};
    ///
    /// let odd: Characteristic =
    ///     serde_json::from_str(r#"{"@type":"IntegerCharacteristic","value":"12"}"#).unwrap();
    /// assert_eq!(odd.value_kind(), ValueKind::Integer);
    /// assert_eq!(
    ///     odd.value.as_ref().map(ValueKind::of_value),
    ///     Some(ValueKind::String),
    /// );
    /// ```
    #[must_use]
    pub fn value_kind(&self) -> ValueKind {
        ValueKind::from_type_name(self.type_name())
    }
}

tmf_struct! {
    @name = "CharacteristicRelationship";
    /// A relationship between two characteristics.
    pub struct CharacteristicRelationship {
        /// Identifier of the related characteristic.
        id: String,
        /// Kind of relationship, e.g. `dependency`, `exclusivity`.
        relationship_type: String,
    }
}

tmf_struct! {
    @name = "CharacteristicSpecification";
    /// Defines a characteristic that instances of a specification may carry.
    pub struct CharacteristicSpecification {
        /// Unique identifier for the characteristic.
        id: String,
        /// Name distinguishing this characteristic specification from others.
        name: String,
        /// The kind of value the characteristic can take on.
        value_type: String,
        /// A narrative explaining the characteristic specification.
        description: String,
        /// Whether the target characteristic is configurable.
        configurable: bool,
        /// Whether the characteristic value is unique across instances.
        is_unique: bool,
        /// Minimum number of instances the value can take on.
        min_cardinality: i64,
        /// Maximum number of instances the value can take on.
        max_cardinality: i64,
        /// Regular expression constraining the value.
        regex: String,
        /// Whether the characteristic is extensible.
        extensible: bool,
        /// Period during which this specification is applicable.
        valid_for: TimePeriod,
        /// Relationships to characteristic specifications on other entities.
        char_spec_relationship: Vec<CharacteristicSpecificationRelationship>,
        /// The permissible values.
        characteristic_value_specification: Vec<CharacteristicValueSpecification>,
        @renamed {
            /// A URI to a JSON-Schema file defining the *value* of this
            /// characteristic, as distinct from `@schemaLocation`, which
            /// describes the specification object itself.
            "@valueSchemaLocation" at_value_schema_location: String,
        }
    }
}

tmf_struct! {
    @name = "CharacteristicSpecificationRelationship";
    /// A relationship between characteristic specifications, possibly across
    /// two different specifications.
    pub struct CharacteristicSpecificationRelationship {
        /// Kind of relationship, e.g. `dependency`, `aggregation`.
        relationship_type: String,
        /// Name of the target characteristic specification.
        name: String,
        /// Identifier of the target characteristic specification.
        characteristic_specification_id: String,
        /// Identifier of the specification the target belongs to.
        parent_specification_id: String,
        /// URI of the specification the target belongs to.
        parent_specification_href: String,
        /// Period during which the relationship holds.
        valid_for: TimePeriod,
    }
}

tmf_struct! {
    @name = "CharacteristicValueSpecification";
    /// One permissible value for a [`CharacteristicSpecification`].
    ///
    /// The v5 OAS models this as an abstract base with fourteen strongly-typed
    /// subclasses discriminated by `@type`
    /// (`StringCharacteristicValueSpecification`,
    /// `IntegerArrayCharacteristicValueSpecification`, …). Rather than expose
    /// fourteen near-identical Rust structs, this crate keeps the shared
    /// attributes typed and the `value` as JSON, with [`value_kind`] recovering
    /// the subclass.
    ///
    /// [`value_kind`]: CharacteristicValueSpecification::value_kind
    pub struct CharacteristicValueSpecification {
        /// The kind of value, e.g. `numeric`, `text`.
        value_type: String,
        /// Whether this is the default value for the characteristic.
        is_default: bool,
        /// Unit of measure for the value.
        unit_of_measure: String,
        /// Period during which the value is applicable.
        valid_for: TimePeriod,
        /// Lower bound of a permitted range.
        value_from: i64,
        /// Upper bound of a permitted range.
        value_to: i64,
        /// Whether the range bounds are open or closed, e.g. `closedBottom`.
        range_interval: String,
        /// Regular expression constraining the value.
        regex: String,
        /// The value itself, shaped according to `@type`.
        ///
        /// Declared by each concrete subclass rather than by the abstract base,
        /// which is why the base schema does not list it.
        value: Value,
    }
}

impl CharacteristicValueSpecification {
    /// A permitted value whose class follows from the value itself.
    ///
    /// The counterpart of [`Characteristic::new`], and it exists for the same
    /// reason: the builder has to set `@type` before it has seen the value, so
    /// it leaves the base class where every v5 example names a subclass.
    ///
    /// ```
    /// use rutmf::core::{CharacteristicValueSpecification, ValueKind};
    ///
    /// let allowed = CharacteristicValueSpecification::new("gold");
    /// assert_eq!(allowed.type_name(), "StringCharacteristicValueSpecification");
    /// assert_eq!(allowed.value_kind(), ValueKind::String);
    /// ```
    pub fn new(value: impl Into<Value>) -> Self {
        Self::builder().build().with_value(value)
    }

    /// Sets the value, and the `@type` that value implies.
    ///
    /// The class stays as it was when the value names no subclass — an empty
    /// array, a `null`, or a JSON object where the sender meant a map.
    #[must_use]
    pub fn with_value(mut self, value: impl Into<Value>) -> Self {
        let value = value.into();
        if let Some(class) = ValueKind::of_value(&value).value_specification_type() {
            class.clone_into(&mut self.at_type);
        }
        self.value = Some(value);
        self
    }

    /// Recovers the value kind implied by `@type`.
    #[must_use]
    pub fn value_kind(&self) -> ValueKind {
        ValueKind::from_type_name(self.type_name())
    }
}

/// The shape of a characteristic's value.
///
/// TMF v5 gives the value shape its own class in **two** families that differ
/// only by suffix: `StringCharacteristic` carries an actual value, and
/// `StringCharacteristicValueSpecification` describes a permitted one. This is
/// the shape they share, so reading a subclass or naming one is the same
/// question asked twice rather than two enumerations.
///
/// ```
/// use rutmf::core::ValueKind;
///
/// assert_eq!(ValueKind::from_type_name("IntegerArrayCharacteristic"), ValueKind::IntegerArray);
/// assert_eq!(
///     ValueKind::from_type_name("IntegerArrayCharacteristicValueSpecification"),
///     ValueKind::IntegerArray,
/// );
/// ```
///
/// Naming a class returns an `Option` because the families differ in size:
/// [`Map`](Self::Map) and [`MapArray`](Self::MapArray) are declared for value
/// specifications only. [`Other`](Self::Other) is what an unrecognised subclass
/// becomes, so a vendor's own never fails a parse.
///
/// Both families are checked against the vendored documents by
/// `every_characteristic_subclass_is_a_value_kind` in `tests/coverage.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ValueKind {
    /// A JSON string.
    String,
    /// An array of JSON strings.
    StringArray,
    /// A JSON integer.
    Integer,
    /// An array of JSON integers.
    IntegerArray,
    /// A JSON number.
    Number,
    /// An array of JSON numbers.
    NumberArray,
    /// A JSON boolean.
    Boolean,
    /// An array of JSON booleans.
    BooleanArray,
    /// A JSON number, as distinct from `Number` in the v5 mapping.
    Float,
    /// An array of JSON floats.
    FloatArray,
    /// A JSON object.
    Object,
    /// An array of JSON objects.
    ObjectArray,
    /// A JSON object treated as a map. Value specifications only.
    Map,
    /// An array of maps. Value specifications only.
    MapArray,
    /// A subclass this crate does not know, or a payload naming the base class.
    Other,
}

impl ValueKind {
    /// Every kind the v5 documents declare a class for.
    ///
    /// Excludes [`Other`](Self::Other), which is the absence of a kind.
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[
            Self::String,
            Self::StringArray,
            Self::Integer,
            Self::IntegerArray,
            Self::Number,
            Self::NumberArray,
            Self::Boolean,
            Self::BooleanArray,
            Self::Float,
            Self::FloatArray,
            Self::Object,
            Self::ObjectArray,
            Self::Map,
            Self::MapArray,
        ]
    }

    /// The kind a `@type` names, in either family.
    ///
    /// Unknown names become [`Other`](Self::Other), so a vendor subclass never
    /// fails a parse.
    #[must_use]
    pub fn from_type_name(name: &str) -> Self {
        // The value-specification suffix ends in the characteristic one, so it
        // has to be tried first or every class reads as a bare characteristic.
        let prefix = name
            .strip_suffix("CharacteristicValueSpecification")
            .or_else(|| name.strip_suffix("Characteristic"))
            .unwrap_or("");

        match prefix {
            "String" => Self::String,
            "StringArray" => Self::StringArray,
            "Integer" => Self::Integer,
            "IntegerArray" => Self::IntegerArray,
            "Number" => Self::Number,
            "NumberArray" => Self::NumberArray,
            "Boolean" => Self::Boolean,
            "BooleanArray" => Self::BooleanArray,
            "Float" => Self::Float,
            "FloatArray" => Self::FloatArray,
            "Object" => Self::Object,
            "ObjectArray" => Self::ObjectArray,
            "Map" => Self::Map,
            "MapArray" => Self::MapArray,
            _ => Self::Other,
        }
    }

    /// The shape a JSON value actually has, as against what a `@type` claims.
    ///
    /// ```
    /// use rutmf::core::ValueKind;
    /// use serde_json::json;
    ///
    /// assert_eq!(ValueKind::of_value(&json!("gold")), ValueKind::String);
    /// assert_eq!(ValueKind::of_value(&json!(100)), ValueKind::Integer);
    /// assert_eq!(ValueKind::of_value(&json!(1.5)), ValueKind::Number);
    /// assert_eq!(ValueKind::of_value(&json!(["a", "b"])), ValueKind::StringArray);
    /// ```
    ///
    /// An integer reads as [`Integer`](Self::Integer) rather than
    /// [`Number`](Self::Number): an integer satisfies both schemas, so the
    /// narrower answer is right either way.
    ///
    /// A shape that names no subclass is [`Other`](Self::Other): `null`, an
    /// empty array, and an array of mixed types.
    ///
    /// Two kinds are never *returned*, because JSON does not distinguish them
    /// from ones that are. [`Map`](Self::Map) is an object, and
    /// [`Float`](Self::Float) is a number — both real v5 classes, and both a
    /// statement about intent rather than about the value. Set `@type` yourself
    /// when you mean one:
    ///
    /// ```
    /// use rutmf::core::{Characteristic, ValueKind};
    ///
    /// let rate = Characteristic::builder()
    ///     .name("errorRate")
    ///     .value(0.001)
    ///     .at_type(ValueKind::Float.characteristic_type().unwrap())
    ///     .build();
    /// assert_eq!(rate.value_kind(), ValueKind::Float);
    /// ```
    #[must_use]
    pub fn of_value(value: &Value) -> Self {
        match value {
            Value::String(_) => Self::String,
            Value::Bool(_) => Self::Boolean,
            Value::Number(n) => {
                if n.is_i64() || n.is_u64() {
                    Self::Integer
                } else {
                    Self::Number
                }
            }
            Value::Object(_) => Self::Object,
            Value::Array(items) => match items.split_first() {
                // Every element must have the same shape, or no `…ArrayCharacteristic`
                // describes it.
                Some((first, rest)) => {
                    let kind = Self::of_value(first);
                    if rest.iter().all(|item| Self::of_value(item) == kind) {
                        kind.array_of()
                    } else {
                        Self::Other
                    }
                }
                None => Self::Other,
            },
            Value::Null => Self::Other,
        }
    }

    /// The array kind whose elements are this kind.
    #[must_use]
    fn array_of(self) -> Self {
        match self {
            Self::String => Self::StringArray,
            Self::Integer => Self::IntegerArray,
            Self::Number => Self::NumberArray,
            Self::Boolean => Self::BooleanArray,
            Self::Float => Self::FloatArray,
            Self::Object => Self::ObjectArray,
            Self::Map => Self::MapArray,
            // An array of arrays names no v5 class.
            _ => Self::Other,
        }
    }

    /// Whether this kind is one of the array shapes.
    #[must_use]
    pub fn is_array(self) -> bool {
        matches!(
            self,
            Self::StringArray
                | Self::IntegerArray
                | Self::NumberArray
                | Self::BooleanArray
                | Self::FloatArray
                | Self::ObjectArray
                | Self::MapArray
        )
    }

    /// The `…Characteristic` class name for this kind, if the v5 documents
    /// declare one.
    ///
    /// `None` for [`Map`](Self::Map) and [`MapArray`](Self::MapArray), which are
    /// declared only as value specifications, and for
    /// [`Other`](Self::Other).
    #[must_use]
    pub fn characteristic_type(self) -> Option<&'static str> {
        Some(match self {
            Self::String => "StringCharacteristic",
            Self::StringArray => "StringArrayCharacteristic",
            Self::Integer => "IntegerCharacteristic",
            Self::IntegerArray => "IntegerArrayCharacteristic",
            Self::Number => "NumberCharacteristic",
            Self::NumberArray => "NumberArrayCharacteristic",
            Self::Boolean => "BooleanCharacteristic",
            Self::BooleanArray => "BooleanArrayCharacteristic",
            Self::Float => "FloatCharacteristic",
            Self::FloatArray => "FloatArrayCharacteristic",
            Self::Object => "ObjectCharacteristic",
            Self::ObjectArray => "ObjectArrayCharacteristic",
            Self::Map | Self::MapArray | Self::Other => return None,
        })
    }

    /// The `…CharacteristicValueSpecification` class name for this kind.
    ///
    /// `None` only for [`Other`](Self::Other): this is the larger of the two
    /// families and declares a class for every kind.
    #[must_use]
    pub fn value_specification_type(self) -> Option<&'static str> {
        Some(match self {
            Self::String => "StringCharacteristicValueSpecification",
            Self::StringArray => "StringArrayCharacteristicValueSpecification",
            Self::Integer => "IntegerCharacteristicValueSpecification",
            Self::IntegerArray => "IntegerArrayCharacteristicValueSpecification",
            Self::Number => "NumberCharacteristicValueSpecification",
            Self::NumberArray => "NumberArrayCharacteristicValueSpecification",
            Self::Boolean => "BooleanCharacteristicValueSpecification",
            Self::BooleanArray => "BooleanArrayCharacteristicValueSpecification",
            Self::Float => "FloatCharacteristicValueSpecification",
            Self::FloatArray => "FloatArrayCharacteristicValueSpecification",
            Self::Object => "ObjectCharacteristicValueSpecification",
            Self::ObjectArray => "ObjectArrayCharacteristicValueSpecification",
            Self::Map => "MapCharacteristicValueSpecification",
            Self::MapArray => "MapArrayCharacteristicValueSpecification",
            Self::Other => return None,
        })
    }
}

tmf_struct! {
    @name = "ProductSpecificationCharacteristicValueUse";
    /// Reference to a characteristic specification defined on another entity,
    /// narrowing which values are usable in this context.
    pub struct CharacteristicValueUse {
        /// Unique identifier of the use.
        id: String,
        /// Name of the characteristic being narrowed.
        name: String,
        /// Description of the use.
        description: String,
        /// The kind of value.
        value_type: String,
        /// Minimum cardinality in this context.
        min_cardinality: i64,
        /// Maximum cardinality in this context.
        max_cardinality: i64,
        /// Period during which this use is applicable.
        valid_for: TimePeriod,
        /// The permissible values in this context.
        product_spec_characteristic_value: Vec<CharacteristicValueSpecification>,
        /// The specification the characteristic is defined on.
        product_specification: Ref<SpecificationTarget>,
    }
}

/// Marker for a reference to whichever specification owns a characteristic.
///
/// Used as the target of [`CharacteristicValueUse::product_specification`],
/// which the v5 OAS types as `ProductSpecificationRef`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpecificationTarget;

impl super::extensible::TmfType for SpecificationTarget {
    const TYPE_NAME: &'static str = "ProductSpecification";
    const REF_TYPE_NAME: &'static str = "ProductSpecificationRef";
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unknown_value_subclass_parses_as_other() {
        let json = r#"{"@type":"VendorSpecialCharacteristicValueSpecification","value":1}"#;
        let v: CharacteristicValueSpecification = serde_json::from_str(json).unwrap();
        assert_eq!(v.value_kind(), ValueKind::Other);

        // Round-trip is by value, not by byte: known members re-serialise in
        // declaration order, so compare the parsed documents.
        let before: serde_json::Value = serde_json::from_str(json).unwrap();
        let after = serde_json::to_value(&v).unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn known_value_subclass_maps_to_kind() {
        let json = r#"{"@type":"StringCharacteristicValueSpecification","value":"x"}"#;
        let v: CharacteristicValueSpecification = serde_json::from_str(json).unwrap();
        assert_eq!(v.value_kind(), ValueKind::String);
        assert_eq!(
            ValueKind::String.value_specification_type(),
            Some(v.at_type.as_str())
        );
    }

    #[test]
    fn a_built_characteristic_declares_its_subclass_not_the_base() {
        // Of the characteristics in the vendored corpus that carry a `@type`,
        // every one names a subclass. Emitting the base class is what a server
        // that validates the discriminator rejects.
        for (value, class) in [
            (json!("gold"), "StringCharacteristic"),
            (json!(100), "IntegerCharacteristic"),
            (json!(1.5), "NumberCharacteristic"),
            (json!(true), "BooleanCharacteristic"),
            (json!({"a": 1}), "ObjectCharacteristic"),
            (json!(["a", "b"]), "StringArrayCharacteristic"),
            (json!([1, 2]), "IntegerArrayCharacteristic"),
            (json!([{"a": 1}]), "ObjectArrayCharacteristic"),
        ] {
            let c = Characteristic::new("x", value.clone());
            assert_eq!(c.type_name(), class, "for {value}");
            assert_eq!(c.value.as_ref(), Some(&value));
        }
    }

    #[test]
    fn a_shape_that_names_no_subclass_leaves_the_base_class() {
        // Guessing would put a `@type` on the wire that the value contradicts,
        // which is worse than the honest base class.
        for value in [json!(null), json!([]), json!([1, "two"]), json!([[1]])] {
            assert_eq!(
                Characteristic::new("x", value.clone()).type_name(),
                "Characteristic",
                "for {value}"
            );
        }
    }

    #[test]
    fn a_type_names_the_same_kind_in_both_families() {
        // The value-specification suffix ends in the characteristic one, so a
        // naive `strip_suffix` order reads every specification as a bare
        // characteristic and loses `Map` entirely.
        for kind in ValueKind::all() {
            if let Some(name) = kind.characteristic_type() {
                assert_eq!(ValueKind::from_type_name(name), *kind, "{name}");
            }
            let name = kind
                .value_specification_type()
                .expect("every kind is a value specification");
            assert_eq!(ValueKind::from_type_name(name), *kind, "{name}");
        }

        assert_eq!(ValueKind::Map.characteristic_type(), None, "no such class");
        assert_eq!(ValueKind::Other.value_specification_type(), None);
        assert_eq!(
            ValueKind::from_type_name("Characteristic"),
            ValueKind::Other
        );
    }

    #[test]
    fn what_the_sender_said_and_what_the_value_is_stay_separable() {
        let odd: Characteristic =
            serde_json::from_str(r#"{"@type":"IntegerCharacteristic","value":"12"}"#).unwrap();
        assert_eq!(odd.value_kind(), ValueKind::Integer, "what it claims");
        assert_eq!(
            odd.value.as_ref().map(ValueKind::of_value),
            Some(ValueKind::String),
            "what it is"
        );
    }

    #[test]
    fn array_kinds_report_themselves_as_arrays() {
        assert!(ValueKind::StringArray.is_array());
        assert!(!ValueKind::String.is_array());
        assert!(!ValueKind::Other.is_array());
    }

    #[test]
    fn value_schema_location_is_distinct_from_schema_location() {
        let json = r#"{"@type":"CharacteristicSpecification","@schemaLocation":"a","@valueSchemaLocation":"b"}"#;
        let c: CharacteristicSpecification = serde_json::from_str(json).unwrap();
        assert_eq!(c.at_schema_location.as_deref(), Some("a"));
        assert_eq!(c.at_value_schema_location.as_deref(), Some("b"));
        assert!(c.extensions.is_empty(), "both must be typed, not captured");
    }
}
