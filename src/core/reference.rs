//! Typed entity references.

use std::fmt;
use std::marker::PhantomData;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::extensible::{Extensions, TmfType};

/// A reference to an entity of type `T`.
///
/// TM Forum payloads are dense with `…Ref` objects: a `ProductOffering` points
/// at a `ProductSpecificationRef`, which points at a `ProductSpecification`
/// living in another API. On the wire they are all the same shape, so most
/// libraries model them as one stringly-typed struct and lose track of what a
/// reference actually points at. `Ref<T>` keeps that in the type system at zero
/// runtime cost — the target is a [`PhantomData`], so a reference is the same
/// size as the members it carries.
///
/// The `@type` and `@referredType` members are filled in from `T` when you
/// construct a reference in Rust, and preserved verbatim when one is parsed
/// from a server payload.
///
/// ```
/// use rutmf::core::Ref;
/// use rutmf::product::ProductSpecification;
///
/// let r: Ref<ProductSpecification> = Ref::new("9881").with_name("Robotics999");
/// let json = serde_json::to_value(&r).unwrap();
/// assert_eq!(json["@type"], "ProductSpecificationRef");
/// assert_eq!(json["@referredType"], "ProductSpecification");
/// ```
pub struct Ref<T: ?Sized> {
    /// Identifier of the referred entity.
    pub id: String,
    /// URI of the referred entity.
    pub href: Option<String>,
    /// Name of the referred entity.
    pub name: Option<String>,
    /// Version of the referred entity.
    ///
    /// Eight of the v5 `…Ref` schemas add this to the common `EntityRef` shape
    /// — `CategoryRef`, `ProductOfferingRef`, `ProductSpecificationRef` and the
    /// rest of the catalog family — and TM Forum's own examples carry it on 124
    /// references. A catalog is a versioned thing, so a reference into one
    /// usually says which version it means.
    ///
    /// Absent on the `…Ref` schemas that do not define it, where it simply
    /// stays `None`.
    pub version: Option<String>,
    /// The actual type of the target instance, when needed for disambiguation.
    ///
    /// Defaults to `T::TYPE_NAME`, but a server may report a more specific
    /// subclass (a `PlaceRef` whose `@referredType` is `GeographicAddress`).
    pub referred_type: Option<String>,
    /// The `@type` of this reference object itself, e.g. `ProductOfferingRef`.
    ///
    /// Empty when the payload declared none; see [`Ref::type_name`].
    pub at_type: String,
    /// The `@baseType` of this reference object, when sub-classed.
    pub at_base_type: Option<String>,
    /// A URI to a JSON-Schema file defining additional attributes.
    pub at_schema_location: Option<String>,
    /// Vendor extensions, preserved in document order.
    pub extensions: Extensions,
    marker: PhantomData<fn() -> T>,
}

impl<T: TmfType + ?Sized> Ref<T> {
    /// Creates a reference to the entity with the given id.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            href: None,
            name: None,
            version: None,
            referred_type: Some(T::TYPE_NAME.to_owned()),
            at_type: T::REF_TYPE_NAME.to_owned(),
            at_base_type: None,
            at_schema_location: None,
            extensions: Extensions::new(),
            marker: PhantomData,
        }
    }

    /// Sets the human-readable name of the referred entity.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the URI of the referred entity.
    #[must_use]
    pub fn with_href(mut self, href: impl Into<String>) -> Self {
        self.href = Some(href.into());
        self
    }

    /// Pins the reference to a version of the referred entity.
    #[must_use]
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Overrides `@referredType` — use when the target is a subclass of `T`.
    #[must_use]
    pub fn with_referred_type(mut self, ty: impl Into<String>) -> Self {
        self.referred_type = Some(ty.into());
        self
    }

    /// The reference class this payload declares, defaulting to
    /// `T::REF_TYPE_NAME`.
    ///
    /// Prefer this to reading `at_type` directly: a reference that declared no
    /// `@type` leaves that field empty, and its class is still known.
    #[must_use]
    pub fn type_name(&self) -> &str {
        if self.at_type.is_empty() {
            T::REF_TYPE_NAME
        } else {
            &self.at_type
        }
    }
}

impl<T: ?Sized> Ref<T> {
    /// The `href`, when the server supplied one that is an absolute HTTP URL.
    ///
    /// TM Forum servers put all three of an absolute URL, a root-relative path
    /// and nothing at all in this member. Only the first names a location a
    /// client can dispatch to directly; the other two are resolved against the
    /// API the reference came from, which is what
    /// [`resolve`](crate::api::ResolveRef::resolve) falls back to.
    ///
    /// ```
    /// use rutmf::core::Ref;
    /// use rutmf::product::ProductSpecification;
    ///
    /// let absolute: Ref<ProductSpecification> =
    ///     Ref::new("9881").with_href("https://host/tmf-api/x/v5/productSpecification/9881");
    /// assert!(absolute.absolute_href().is_some());
    ///
    /// let relative: Ref<ProductSpecification> =
    ///     Ref::new("9881").with_href("/productSpecification/9881");
    /// assert_eq!(relative.absolute_href(), None, "not dispatchable on its own");
    /// ```
    #[must_use]
    pub fn absolute_href(&self) -> Option<&str> {
        self.href
            .as_deref()
            .filter(|href| href.starts_with("http://") || href.starts_with("https://"))
    }

    /// Reinterprets the reference as pointing at `U`, keeping every wire member.
    ///
    /// Only the compile-time target changes; `@type` and `@referredType` are
    /// left exactly as they were. This is how a `oneOf` over structurally
    /// identical reference shapes is resolved after reading the discriminator.
    #[must_use]
    pub fn retarget<U: ?Sized>(self) -> Ref<U> {
        Ref {
            id: self.id,
            href: self.href,
            name: self.name,
            version: self.version,
            referred_type: self.referred_type,
            at_type: self.at_type,
            at_base_type: self.at_base_type,
            at_schema_location: self.at_schema_location,
            extensions: self.extensions,
            marker: PhantomData,
        }
    }
}

// Manual trait impls: deriving would add spurious `T: Clone`-style bounds,
// since `PhantomData<fn() -> T>` carries none of them.
impl<T: ?Sized> Clone for Ref<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            href: self.href.clone(),
            name: self.name.clone(),
            version: self.version.clone(),
            referred_type: self.referred_type.clone(),
            at_type: self.at_type.clone(),
            at_base_type: self.at_base_type.clone(),
            at_schema_location: self.at_schema_location.clone(),
            extensions: self.extensions.clone(),
            marker: PhantomData,
        }
    }
}

impl<T: ?Sized> fmt::Debug for Ref<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ref")
            .field("id", &self.id)
            .field("href", &self.href)
            .field("name", &self.name)
            .field("version", &self.version)
            .field("referred_type", &self.referred_type)
            .field("at_type", &self.at_type)
            .field("extensions", &self.extensions)
            .finish_non_exhaustive()
    }
}

impl<T: ?Sized> PartialEq for Ref<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.href == other.href
            && self.name == other.name
            && self.version == other.version
            && self.referred_type == other.referred_type
            && self.at_type == other.at_type
            && self.at_base_type == other.at_base_type
            && self.at_schema_location == other.at_schema_location
            && self.extensions == other.extensions
    }
}

impl<T: ?Sized> Eq for Ref<T> {}

/// Wire representation shared by every `…Ref` schema in the v5 OAS.
#[derive(Serialize, Deserialize)]
struct RefWire {
    id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    href: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(
        rename = "@referredType",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    referred_type: Option<String>,
    #[serde(rename = "@type", default, skip_serializing_if = "String::is_empty")]
    at_type: String,
    #[serde(rename = "@baseType", default, skip_serializing_if = "Option::is_none")]
    at_base_type: Option<String>,
    #[serde(
        rename = "@schemaLocation",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    at_schema_location: Option<String>,
    #[serde(flatten, default, skip_serializing_if = "Extensions::is_empty")]
    extensions: Extensions,
}

impl<T: ?Sized> Serialize for Ref<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        RefWire {
            id: self.id.clone(),
            href: self.href.clone(),
            name: self.name.clone(),
            version: self.version.clone(),
            referred_type: self.referred_type.clone(),
            at_type: self.at_type.clone(),
            at_base_type: self.at_base_type.clone(),
            at_schema_location: self.at_schema_location.clone(),
            extensions: self.extensions.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de, T: TmfType + ?Sized> Deserialize<'de> for Ref<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // `@type` is required by TMF630 v5, but real servers omit it, so a
        // missing one must not fail the parse.
        #[derive(Deserialize)]
        struct Lenient {
            id: String,
            #[serde(default)]
            href: Option<String>,
            #[serde(default)]
            name: Option<String>,
            #[serde(default)]
            version: Option<String>,
            #[serde(rename = "@referredType", default)]
            referred_type: Option<String>,
            #[serde(rename = "@type", default)]
            at_type: Option<String>,
            #[serde(rename = "@baseType", default)]
            at_base_type: Option<String>,
            #[serde(rename = "@schemaLocation", default)]
            at_schema_location: Option<String>,
            #[serde(flatten, default)]
            extensions: Extensions,
        }

        let w = Lenient::deserialize(deserializer)?;
        Ok(Self {
            id: w.id,
            href: w.href,
            name: w.name,
            version: w.version,
            referred_type: w.referred_type,
            // Absent stays absent: re-emitting a member the server did not send
            // would make this crate unusable for relaying payloads unchanged.
            at_type: w.at_type.unwrap_or_default(),
            at_base_type: w.at_base_type,
            at_schema_location: w.at_schema_location,
            extensions: w.extensions,
            marker: PhantomData,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Dummy;
    impl TmfType for Dummy {
        const TYPE_NAME: &'static str = "Dummy";
        const REF_TYPE_NAME: &'static str = "DummyRef";
    }

    #[test]
    fn round_trips_unknown_members_in_order() {
        let json =
            r#"{"id":"1","name":"n","@referredType":"Sub","@type":"DummyRef","zzz":1,"aaa":2}"#;
        let r: Ref<Dummy> = serde_json::from_str(json).unwrap();
        assert_eq!(r.extensions.len(), 2);
        assert_eq!(serde_json::to_string(&r).unwrap(), json);
    }

    #[test]
    fn an_absent_type_stays_absent_but_is_still_known() {
        let json = r#"{"id":"1"}"#;
        let r: Ref<Dummy> = serde_json::from_str(json).unwrap();

        assert_eq!(r.type_name(), "DummyRef", "the class is known from `T`");
        assert_eq!(
            serde_json::to_string(&r).unwrap(),
            json,
            "but relaying the payload must not add a member to it"
        );
    }

    #[test]
    fn a_version_on_a_reference_is_typed_not_swept_into_extensions() {
        // TM Forum's own examples carry `version` on 124 references, so it is a
        // member the catalog family genuinely uses.
        let json = r#"{"id":"1","version":"2.0","@type":"CategoryRef"}"#;
        let r: Ref<Dummy> = serde_json::from_str(json).unwrap();
        assert_eq!(r.version.as_deref(), Some("2.0"));
        assert!(r.extensions.is_empty());
        assert_eq!(serde_json::to_string(&r).unwrap(), json);
    }

    #[test]
    fn a_constructed_reference_declares_its_type() {
        let r: Ref<Dummy> = Ref::new("1");
        assert_eq!(
            serde_json::to_string(&r).unwrap(),
            r#"{"id":"1","@referredType":"Dummy","@type":"DummyRef"}"#
        );
    }
}
