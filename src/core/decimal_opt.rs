//! serde adapter for `Option<Decimal>` on the TMF wire format.
//!
//! Accepts a JSON number *or* a numeric string, and re-emits a JSON number
//! keeping integers integral. Public so that code defining its own TMF-shaped
//! types can reuse it:
//!
//! ```
//! use rust_decimal::Decimal;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize)]
//! struct Charge {
//!     #[serde(default, with = "rutmf::core::decimal_opt")]
//!     amount: Option<Decimal>,
//! }
//!
//! let c: Charge = serde_json::from_str(r#"{"amount":"9.99"}"#).unwrap();
//! assert_eq!(serde_json::to_string(&c).unwrap(), r#"{"amount":9.99}"#);
//! ```
//!
//! # Precision
//!
//! JSON numbers and numeric strings are decoded in base ten. An amount outside
//! the exact range of [`Decimal`] is rejected. Writers emit a JSON number.

use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Number, value::RawValue};

/// Serialises an optional decimal as an exact JSON number.
pub fn serialize<S: Serializer>(value: &Option<Decimal>, serializer: S) -> Result<S::Ok, S::Error> {
    value
        .map(|amount| amount.normalize().to_string().parse::<Number>())
        .transpose()
        .map_err(serde::ser::Error::custom)?
        .serialize(serializer)
}

/// Reads a JSON number or numeric string without binary floating point.
pub fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Decimal>, D::Error> {
    let raw = Box::<RawValue>::deserialize(JsonToken(deserializer))?;
    let token = raw.get();
    if token == "null" {
        return Ok(None);
    }
    let decoded: String;
    let text = if token.starts_with('"') {
        decoded = serde_json::from_str(token).map_err(serde::de::Error::custom)?;
        &decoded
    } else {
        token
    };
    Parts::parse(text)
        .and_then(|parts| parts.decimal())
        .map(Some)
        .ok_or_else(|| serde::de::Error::custom("expected an exactly representable decimal amount"))
}

// RawValue asks the format for a newtype and receives a map containing the
// original JSON token. Serde's buffered formats (including untagged enums)
// instead hand the newtype visitor their buffered value. Let serde_json
// reconstruct the token in that case, using its arbitrary-precision Number.
struct JsonToken<D>(D);

impl<'de, D: Deserializer<'de>> Deserializer<'de> for JsonToken<D> {
    type Error = D::Error;

    fn deserialize_newtype_struct<V: serde::de::Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.0
            .deserialize_newtype_struct(name, TokenVisitor { name, visitor })
    }

    fn deserialize_any<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.0.deserialize_any(visitor)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct seq tuple tuple_struct map struct
        enum identifier ignored_any
    }
}

struct TokenVisitor<V> {
    name: &'static str,
    visitor: V,
}

impl<'de, V: serde::de::Visitor<'de>> serde::de::Visitor<'de> for TokenVisitor<V> {
    type Value = V::Value;

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.visitor.expecting(f)
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        self.visitor.visit_map(map)
    }

    fn visit_newtype_struct<D: Deserializer<'de>>(self, de: D) -> Result<Self::Value, D::Error> {
        serde_json::Value::deserialize(de)?
            .deserialize_newtype_struct(self.name, self.visitor)
            .map_err(serde::de::Error::custom)
    }
}

// The significant digits and their power of ten, kept separate so a short
// exponent never expands into an enormous coefficient.
struct Parts<'a> {
    negative: bool,
    digits: String,
    exponent: &'a str,
    shift: i64,
}

impl<'a> Parts<'a> {
    fn parse(text: &'a str) -> Option<Self> {
        let (base, exponent) = text.split_once(['e', 'E']).unwrap_or((text, "0"));
        let unsigned = base.strip_prefix(['-', '+']).unwrap_or(base);
        let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
        let mut digits = [whole, fraction].concat();
        let exp_digits = exponent.strip_prefix(['-', '+']).unwrap_or(exponent);
        if [digits.as_str(), exp_digits]
            .into_iter()
            .any(|s| s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()))
        {
            return None;
        }
        let end = digits.trim_end_matches('0').len();
        let trailing = digits.len() - end;
        digits.truncate(end);
        let shift = i64::try_from(trailing).ok()? - i64::try_from(fraction.len()).ok()?;
        let leading = digits.len() - digits.trim_start_matches('0').len();
        digits.drain(..leading);
        Some(Self {
            negative: base.starts_with('-') && !digits.is_empty(),
            digits,
            exponent,
            shift,
        })
    }

    fn decimal(&self) -> Option<Decimal> {
        if self.digits.is_empty() {
            return Some(Decimal::ZERO);
        }
        let power = self.shift.checked_add(self.exponent.parse().ok()?)?;
        if !(-28..=28).contains(&power) {
            return None;
        }
        let mut mantissa: i128 = self.digits.parse().ok()?;
        if power > 0 {
            mantissa = mantissa.checked_mul(10_i128.pow(u32::try_from(power).ok()?))?;
        }
        if self.negative {
            mantissa = -mantissa;
        }
        Decimal::try_from_i128_with_scale(mantissa, u32::try_from((-power).max(0)).ok()?).ok()
    }
}

/// Compare JSON numbers including vendor data outside the monetary `Decimal` range.
#[cfg(feature = "server")]
pub(crate) fn compare(left: &str, right: &str) -> Option<std::cmp::Ordering> {
    use num_bigint::BigInt;
    let a = Parts::parse(left)?;
    let b = Parts::parse(right)?;
    let sign = |n: &Parts<'_>| {
        if n.negative {
            -1
        } else {
            i8::from(!n.digits.is_empty())
        }
    };
    let signs = sign(&a).cmp(&sign(&b));
    if !signs.is_eq() || a.digits.is_empty() {
        return Some(signs);
    }
    // First compare the position of the leading digit, then the significant
    // digits lexically. Trailing zeroes have already been removed.
    let magnitude =
        |p: &Parts<'_>| Some(p.exponent.parse::<BigInt>().ok()? + p.shift + p.digits.len());
    let order = magnitude(&a)?
        .cmp(&magnitude(&b)?)
        .then_with(|| a.digits.cmp(&b.digits));
    Some(if a.negative { order.reverse() } else { order })
}
