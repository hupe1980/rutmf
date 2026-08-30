//! The declaration macros every TM Forum entity in this crate is built from.
//!
//! The macros themselves are crate-internal; this module is public because its
//! rules are the ones a reader needs to understand the model — why `@type` is a
//! `String` that can be empty, and where requiredness binds.
//!
//! # Why a macro
//!
//! TMF v5 defines each resource three times — the read model, an `_FVO` schema
//! for `POST` and an `_MVO` schema for `PATCH` — and this crate mirrors that in
//! three Rust types (see [`crate::core`]). Hand-written, each field costs four
//! to six lines of identical serde and builder attributes, and every type ends
//! with the same twenty-line `@type` / `@baseType` / `@schemaLocation` /
//! `extensions` tail. Across fourteen APIs that is thousands of lines of
//! near-duplicate code, and three copies of a field list do not stay in step.
//!
//! `tmf_struct!` collapses a field to a single line and derives the tail from
//! the type, so the TMF630 `Extensible` contract holds by construction rather
//! than by review. What a macro cannot check — that the field *list* matches
//! the specification — is checked by `tests/coverage.rs`, which reads the
//! vendored OAS and fails on any member without a typed field.
//!
//! # Syntax
//!
//! ```ignore
//! tmf_struct! {
//!     @name = "ProductOffering", @ref = "ProductOfferingRef";
//!     /// A product offering: what a service provider actually sells.
//!     pub struct ProductOffering {
//!         @required {
//!             /// Name of the offering.
//!             name: String,
//!         }
//!         /// Server-assigned identifier.
//!         id: String,
//!         /// Prices at which the offering is sold.
//!         product_offering_price: Vec<ProductOfferingPrice>,
//!         @decimal {
//!             /// Percentage applied instead of an absolute amount.
//!             percentage: Decimal,
//!         }
//!         @renamed {
//!             /// The actual type of the referred instance.
//!             "@referredType" referred_type: String,
//!         }
//!     }
//! }
//! ```
//!
//! Plain fields become `Option<T>`, are renamed to camelCase for the wire, are
//! omitted when `None`, and take `impl Into<_>` in the builder. Three optional
//! sections change that: `@required` comes before the plain fields, `@decimal`
//! and `@renamed` after.
//!
//! | Section | Effect |
//! |---|---|
//! | `@required { … }` | plain `T`, always serialised: the members v5 marks required |
//! | `@decimal { … }` | `Option<Decimal>` through [`decimal_opt`](crate::core::decimal_opt) |
//! | `@renamed { "wire" name: T, … }` | an explicit wire name, for the `@`-prefixed members |
//!
//! The header carries the discriminator; only `@name` is mandatory.
//!
//! | Item | Meaning |
//! |---|---|
//! | `@name = "…"` | the `@type` discriminator, and `TmfType::TYPE_NAME` |
//! | `@ref = "…"` | `TmfType::REF_TYPE_NAME`, for types something points at |
//!
//! `tmf_entity!` adds the [`Entity`](crate::core::Entity) implementation for
//! resources addressable by `id` and `href`, and `tmf_value!` declares the
//! handful of v5 value objects the schemas give no `@type` at all. `tmf_value!`
//! accepts the same `@decimal` and `@renamed` sections, because a value object
//! can still carry an `@`-prefixed member — TMF634's `TargetResourceSchema` is
//! nothing but two of them.
//!
//! # Where requiredness is enforced, and why
//!
//! The v5 schemas mark members required in three places: on create bodies
//! (`_FVO`), on a few patch bodies (`_MVO`), and on nested types such as
//! `ProductOfferingRelationship.id`. This crate enforces it in the first two
//! and not the third, under one rule:
//!
//! > **Requiredness binds where the client authors the payload, and relaxes
//! > where the client parses one.**
//!
//! A create body is something you construct, so `@required` there turns a
//! request a conformant server would reject into a compile error — the whole
//! point of the `_FVO` / `_MVO` split. A nested type inside a `GET` response is
//! something a server hands you, and refusing to parse an entire catalogue
//! because one relationship omitted its `id` serves nobody. That is Postel's
//! law, applied where each half belongs; `tests/coverage.rs` checks the
//! division against the OAS so it cannot drift into carelessness.
//!
//! `@required` therefore appears only on `…Create` and `…Update` types, and it
//! carries two consequences:
//!
//! - **Types without it** get `#[serde(default)]` and a [`Default`] whose
//!   `@type` is the declared discriminator. Deriving `Default` would leave
//!   `@type` empty, so `Default::default()` would produce `{"@type":""}` — a
//!   payload no conformant server accepts. It also means a response omitting
//!   the spec-mandatory `@type` still parses, normalised on the way out.
//!   (Every type is `#[non_exhaustive]`, so downstream code builds these with
//!   `T::builder()` rather than struct-update syntax; `Default` is for the
//!   serde path and for `let mut patch = T::default();`.)
//! - **Types with it** get neither, so `@type` and every required member must
//!   be present to deserialize. Those are bodies a client authors, where the
//!   specification's demands are exactly what you want checked.

/// Declares a TM Forum entity: the struct and its `TmfType` implementation.
///
/// See the [module documentation](self) for the syntax.
#[allow(
    unused_macros,
    reason = "not every feature combination declares an entity"
)]
macro_rules! tmf_struct {
    // --- With required members: no `Default`, because there is nothing to
    // default them to, and a strict `@type` for the same reason — these are the
    // create bodies, and v5 requires the discriminator on them.
    (
        @name = $type_name:literal $(, @ref = $ref_name:literal)? ;
        $(#[$meta:meta])*
        pub struct $name:ident {
            @required { $($(#[$rmeta:meta])* $rfield:ident : $rty:ty,)+ }
            $($(#[$ometa:meta])* $ofield:ident : $oty:ty,)*
            $(@decimal { $($(#[$dmeta:meta])* $dfield:ident : $dty:ty,)+ })?
            $(@renamed { $($(#[$nmeta:meta])* $wire:literal $nfield:ident : $nty:ty,)+ })?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, ::serde::Serialize, ::serde::Deserialize, ::bon::Builder)]
        #[serde(rename_all = "camelCase")]
        #[cfg_attr(feature = "schemars", derive(::schemars::JsonSchema))]
        #[non_exhaustive]
        pub struct $name {
            $(
                $(#[$rmeta])*
                #[builder(into)]
                pub $rfield: $rty,
            )+
            $(
                $(#[$ometa])*
                #[serde(default, skip_serializing_if = "Option::is_none")]
                #[builder(into)]
                pub $ofield: Option<$oty>,
            )*
            $($(
                $(#[$dmeta])*
                #[serde(default, with = "crate::core::decimal_opt", skip_serializing_if = "Option::is_none")]
                #[cfg_attr(feature = "schemars", schemars(with = "Option<f64>"))]
                #[builder(into)]
                pub $dfield: Option<$dty>,
            )+)?
            $($(
                $(#[$nmeta])*
                #[serde(rename = $wire, default, skip_serializing_if = "Option::is_none")]
                #[builder(into)]
                pub $nfield: Option<$nty>,
            )+)?
            /// The `@type` discriminator naming this entity's class.
            ///
            /// Empty when the payload declared none — TMF630 v5 marks it
            /// required, but servers omit it and so do TM Forum's own examples.
            /// An absent discriminator stays absent, so relaying a payload does
            /// not silently add a member to it; anything this crate *builds*
            /// carries the class name. Read it through
            /// [`type_name`](Self::type_name), which resolves the empty case.
            #[serde(rename = "@type", default, skip_serializing_if = "String::is_empty")]
            #[builder(default = crate::core::default_type::<$name>(), into)]
            pub at_type: String,
            /// The `@baseType`, naming the supertype where this class is sub-typed.
            #[serde(rename = "@baseType", default, skip_serializing_if = "Option::is_none")]
            #[builder(into)]
            pub at_base_type: Option<String>,
            /// A URI to a JSON-Schema file defining additional attributes.
            #[serde(rename = "@schemaLocation", default, skip_serializing_if = "Option::is_none")]
            #[builder(into)]
            pub at_schema_location: Option<String>,
            /// Members not covered by the typed model, kept in document order.
            ///
            /// See the crate documentation for the round-trip guarantee this
            /// underwrites.
            #[serde(flatten, default, skip_serializing_if = "crate::core::Extensions::is_empty")]
            #[builder(default)]
            pub extensions: crate::core::Extensions,
        }

        tmf_struct!(@tmf_type $name, $type_name $(, $ref_name)?);
    };

    // --- No required members: the type gets a correct `Default`, and tolerates
    // a server that omits the spec-mandatory `@type`.
    (
        @name = $type_name:literal $(, @ref = $ref_name:literal)? ;
        $(#[$meta:meta])*
        pub struct $name:ident {
            $($(#[$ometa:meta])* $ofield:ident : $oty:ty,)*
            $(@decimal { $($(#[$dmeta:meta])* $dfield:ident : $dty:ty,)+ })?
            $(@renamed { $($(#[$nmeta:meta])* $wire:literal $nfield:ident : $nty:ty,)+ })?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, ::serde::Serialize, ::serde::Deserialize, ::bon::Builder)]
        #[serde(rename_all = "camelCase")]
        #[cfg_attr(feature = "schemars", derive(::schemars::JsonSchema))]
        #[non_exhaustive]
        pub struct $name {
            $(
                $(#[$ometa])*
                #[serde(default, skip_serializing_if = "Option::is_none")]
                #[builder(into)]
                pub $ofield: Option<$oty>,
            )*
            $($(
                $(#[$dmeta])*
                #[serde(default, with = "crate::core::decimal_opt", skip_serializing_if = "Option::is_none")]
                #[cfg_attr(feature = "schemars", schemars(with = "Option<f64>"))]
                #[builder(into)]
                pub $dfield: Option<$dty>,
            )+)?
            $($(
                $(#[$nmeta])*
                #[serde(rename = $wire, default, skip_serializing_if = "Option::is_none")]
                #[builder(into)]
                pub $nfield: Option<$nty>,
            )+)?
            /// The `@type` discriminator naming this entity's class.
            ///
            /// Empty when the payload declared none — TMF630 v5 marks it
            /// required, but servers omit it and so do TM Forum's own examples.
            /// An absent discriminator stays absent, so relaying a payload does
            /// not silently add a member to it; anything this crate *builds*
            /// carries the class name. Read it through
            /// [`type_name`](Self::type_name), which resolves the empty case.
            #[serde(rename = "@type", default, skip_serializing_if = "String::is_empty")]
            #[builder(default = crate::core::default_type::<$name>(), into)]
            pub at_type: String,
            /// The `@baseType`, naming the supertype where this class is sub-typed.
            #[serde(rename = "@baseType", default, skip_serializing_if = "Option::is_none")]
            #[builder(into)]
            pub at_base_type: Option<String>,
            /// A URI to a JSON-Schema file defining additional attributes.
            #[serde(rename = "@schemaLocation", default, skip_serializing_if = "Option::is_none")]
            #[builder(into)]
            pub at_schema_location: Option<String>,
            /// Members not covered by the typed model, kept in document order.
            ///
            /// See the crate documentation for the round-trip guarantee this
            /// underwrites.
            #[serde(flatten, default, skip_serializing_if = "crate::core::Extensions::is_empty")]
            #[builder(default)]
            pub extensions: crate::core::Extensions,
        }

        tmf_struct!(@tmf_type $name, $type_name $(, $ref_name)?);

        // Deriving `Default` would leave `@type` empty, which is a payload no
        // conformant server accepts. Going through the builder keeps the
        // discriminator, and the container `#[serde(default)]` above reuses it
        // for a response that omits the member.
        impl Default for $name {
            fn default() -> Self {
                Self::builder().build()
            }
        }
    };

    (@tmf_type $name:ident, $type_name:literal) => {
        impl crate::core::TmfType for $name {
            const TYPE_NAME: &'static str = $type_name;
        }
        tmf_struct!(@type_name $name, $type_name);
    };
    (@tmf_type $name:ident, $type_name:literal, $ref_name:literal) => {
        impl crate::core::TmfType for $name {
            const TYPE_NAME: &'static str = $type_name;
            const REF_TYPE_NAME: &'static str = $ref_name;
        }
        tmf_struct!(@type_name $name, $type_name);
    };

    (@type_name $name:ident, $type_name:literal) => {
        impl $name {
            #[doc = concat!("The class this payload declares, defaulting to `", $type_name, "`.")]
            ///
            /// Prefer this to reading `at_type` directly: a payload that
            /// declared no `@type` leaves that field empty, and the class is
            /// still known — it is what the schema this type models says.
            #[must_use]
            pub fn type_name(&self) -> &str {
                if self.at_type.is_empty() {
                    $type_name
                } else {
                    &self.at_type
                }
            }
        }
    };
}

/// Implements [`Entity`](crate::core::Entity) for resources addressable by
/// `id` and `href`.
///
/// Kept out of `tmf_struct!` because only top-level resources are
/// addressable, and that is worth stating per type rather than hiding in a
/// header flag.
#[allow(
    unused_macros,
    reason = "not every feature combination declares an entity"
)]
macro_rules! tmf_entity {
    ($($name:ident),+ $(,)?) => {
        $(impl crate::core::Entity for $name {
            fn id(&self) -> Option<&str> {
                self.id.as_deref()
            }

            fn href(&self) -> Option<&str> {
                self.href.as_deref()
            }
        })+
    };
}

/// Marks the `…Update` types as [`PatchBody`](crate::core::PatchBody), and gives
/// each of them RFC 7386's other half: [`deleting`].
///
/// Listed rather than derived from the declaration, because "this is the `_MVO`
/// of a resource" is not something the shape of a struct reveals — and getting
/// it wrong would let a `PATCH` accept a body the endpoint has no schema for.
///
/// Deletion is only meaningful on a body being *sent*, which is why it is here
/// and not on every entity. It is carried in `extensions`, which serialises
/// flattened, so the `null` lands under the member's own name.
///
/// [`deleting`]: crate::product::ProductOfferingUpdate::deleting
#[allow(
    unused_macros,
    reason = "not every feature combination declares an entity"
)]
macro_rules! tmf_patch_body {
    ($($name:ident),+ $(,)?) => {
        $(
            impl crate::core::PatchBody for $name {}

            impl $name {
                /// Marks a member of the target for deletion by this merge patch.
                ///
                /// RFC 7386 §2 removes a member by naming it with `null`.
                /// Setting the field to `None` cannot say that: `None` means the
                /// patch does not mention the member, which leaves it unchanged.
                ///
                /// ```
                /// use rutmf::product::ProductOfferingUpdate;
                ///
                /// let update = ProductOfferingUpdate::builder()
                ///     .name("Business Internet")
                ///     .build()
                ///     .deleting("description");
                ///
                /// assert_eq!(
                ///     serde_json::to_value(&update).unwrap()["description"],
                ///     serde_json::Value::Null,
                /// );
                /// ```
                ///
                /// `member` is the **wire** name — `lifecycleStatus`, not
                /// `lifecycle_status` — and is not checked, so name one the
                /// schema declares.
                ///
                /// Under [`Patch::Operations`](crate::api::Patch::Operations) the
                /// same edit is
                /// [`JsonPatchOp::remove`](crate::core::JsonPatchOp::remove),
                /// which fails against a member that is not there rather than
                /// silently doing nothing.
                #[must_use]
                pub fn deleting(mut self, member: impl Into<String>) -> Self {
                    self.extensions.insert(member, ::serde_json::Value::Null);
                    self
                }

                /// Whether this patch removes `member`.
                #[must_use]
                pub fn deletes(&self, member: &str) -> bool {
                    self.extensions.get(member).is_some_and(::serde_json::Value::is_null)
                }
            }
        )+
    };
}

/// Declares a v5 *value object*: a type the schemas define with no `@type` and
/// no polymorphism attributes.
///
/// Extensions are still captured. The v5 schemas give `Money`, `Duration` and
/// friends no extension point at all, yet TM Forum's own TMF622 examples send
/// `"@type": "Duration"` on them — so modelling the schema literally would drop
/// data the specification's own examples contain. Fidelity is owed to the wire.
#[allow(
    unused_macros,
    reason = "not every feature combination declares an entity"
)]
macro_rules! tmf_value {
    (
        $(#[$meta:meta])*
        pub struct $name:ident {
            $($(#[$ometa:meta])* $ofield:ident : $oty:ty,)*
            $(@decimal { $($(#[$dmeta:meta])* $dfield:ident : $dty:ty,)+ })?
            $(@renamed { $($(#[$nmeta:meta])* $wire:literal $nfield:ident : $nty:ty,)+ })?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Default, PartialEq, ::serde::Serialize, ::serde::Deserialize, ::bon::Builder)]
        #[serde(rename_all = "camelCase")]
        #[cfg_attr(feature = "schemars", derive(::schemars::JsonSchema))]
        #[non_exhaustive]
        pub struct $name {
            $(
                $(#[$ometa])*
                #[serde(default, skip_serializing_if = "Option::is_none")]
                #[builder(into)]
                pub $ofield: Option<$oty>,
            )*
            $($(
                $(#[$dmeta])*
                #[serde(default, with = "crate::core::decimal_opt", skip_serializing_if = "Option::is_none")]
                #[cfg_attr(feature = "schemars", schemars(with = "Option<f64>"))]
                #[builder(into)]
                pub $dfield: Option<$dty>,
            )+)?
            $($(
                $(#[$nmeta])*
                #[serde(rename = $wire, default, skip_serializing_if = "Option::is_none")]
                #[builder(into)]
                pub $nfield: Option<$nty>,
            )+)?
            /// Members not defined by the v5 schema, kept in document order.
            ///
            /// The schemas give this type no `@type` and no extension point,
            /// yet TM Forum's own examples send one; capturing it here keeps a
            /// payload lossless instead of silently dropping members.
            #[serde(flatten, default, skip_serializing_if = "crate::core::Extensions::is_empty")]
            #[builder(default)]
            pub extensions: crate::core::Extensions,
        }
    };
}

// Which of these is used depends on which domain features are on: a build with
// only `api` or only `server` enabled declares no entities at all, and an
// unused macro is a warning that CI escalates to an error. The alternative —
// a `#[cfg(any(feature = "party", feature = "product", …))]` listing every
// domain on every macro — would need updating for each new API and would fail
// in exactly the same way when someone forgot.
#[allow(
    unused_imports,
    reason = "not every feature combination declares an entity"
)]
pub(crate) use {tmf_entity, tmf_patch_body, tmf_struct, tmf_value};
