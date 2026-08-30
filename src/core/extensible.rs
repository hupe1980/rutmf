//! TMF630 polymorphism (`@type`, `@baseType`, `@schemaLocation`) and the
//! round-trip-safe extension mechanism.
//!
//! Every TM Forum entity is *extensible*: a server may add vendor-specific
//! attributes to any payload, and TMF630 v5 requires each entity to declare its
//! concrete class in `@type`.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A TM Forum type that knows its own `@type` discriminator value.
///
/// The value is taken verbatim from the `components.schemas` key of the v5
/// specification document, which is what servers match on for polymorphic
/// payloads.
pub trait TmfType {
    /// The concrete `@type` value for this entity, e.g. `"ProductOffering"`.
    const TYPE_NAME: &'static str;

    /// The `@type` of a *reference to* this entity, e.g. `"ProductOfferingRef"`.
    ///
    /// Types that are never referenced may leave the default.
    const REF_TYPE_NAME: &'static str = "EntityRef";
}

/// serde `default` helper producing the concrete `@type` for `T`.
///
/// Used as `#[serde(default = "crate::core::default_type::<ProductOffering>")]`
/// so that payloads from servers that omit the (spec-required) `@type` still
/// deserialize, and are normalised to the correct value on the way out.
#[must_use]
pub fn default_type<T: TmfType>() -> String {
    T::TYPE_NAME.to_owned()
}

/// serde `default` helper producing the `@type` of a *reference* to `T`.
#[must_use]
pub fn default_ref_type<T: TmfType>() -> String {
    T::REF_TYPE_NAME.to_owned()
}

/// Vendor extensions: any JSON members not covered by the typed model.
///
/// This is the mechanism behind the crate's round-trip guarantee. Unknown
/// members are captured here **in document order** (backed by [`IndexMap`] and
/// `serde_json/preserve_order`), so re-serialising a payload reproduces both
/// the values *and* their ordering.
///
/// ```
/// use rutmf::core::Extensions;
///
/// let mut ext = Extensions::new();
/// ext.insert("x-vendor-flag", serde_json::json!(true));
/// assert_eq!(ext.get("x-vendor-flag"), Some(&serde_json::json!(true)));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Extensions(IndexMap<String, Value>);

impl Extensions {
    /// Creates an empty extension map.
    #[must_use]
    pub fn new() -> Self {
        Self(IndexMap::new())
    }

    /// Returns `true` when no vendor extensions are present.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Number of extension members.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Looks up an extension member.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.get(key)
    }

    /// Inserts an extension member, returning the previous value if any.
    pub fn insert(&mut self, key: impl Into<String>, value: Value) -> Option<Value> {
        self.0.insert(key.into(), value)
    }

    /// Removes an extension member, preserving the order of the rest.
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.0.shift_remove(key)
    }

    /// Iterates over extension members in document order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.0.iter()
    }

    /// Deserialises an extension member into a concrete type.
    ///
    /// Returns `Ok(None)` when the member is absent.
    pub fn get_as<T: serde::de::DeserializeOwned>(
        &self,
        key: &str,
    ) -> Result<Option<T>, serde_json::Error> {
        self.0
            .get(key)
            .cloned()
            .map(serde_json::from_value)
            .transpose()
    }
}

impl<K: Into<String>> FromIterator<(K, Value)> for Extensions {
    fn from_iter<I: IntoIterator<Item = (K, Value)>>(iter: I) -> Self {
        Self(iter.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }
}

impl<'a> IntoIterator for &'a Extensions {
    type Item = (&'a String, &'a Value);
    type IntoIter = indexmap::map::Iter<'a, String, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
