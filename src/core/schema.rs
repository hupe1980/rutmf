//! Hand-written [`schemars`] implementations for the types whose serde
//! behaviour is hand-written too.
//!
//! [`Ref<T>`] and [`Extensions`] implement `Serialize`/`Deserialize` manually,
//! so the derive cannot infer their shape. These impls describe the exact wire
//! form the manual codecs produce, which is what keeps a generated JSON Schema
//! honest.

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};

use super::extensible::Extensions;
use super::party::PartyOrPartyRole;
use super::reference::Ref;

impl JsonSchema for Extensions {
    fn schema_name() -> Cow<'static, str> {
        "Extensions".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "rutmf::core::Extensions".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "object",
            "description": "Vendor extensions: any members not covered by the typed model.",
            "additionalProperties": true,
        })
    }
}

impl<T: ?Sized> JsonSchema for Ref<T> {
    fn schema_name() -> Cow<'static, str> {
        "EntityRef".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "rutmf::core::Ref".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "object",
            "description": "Reference to a TM Forum entity (the v5 `EntityRef` shape). \
                            `@type` is required by TMF630 but omitted by real servers, \
                            so it is not required here.",
            "required": ["id"],
            "properties": {
                "id": {"type": "string", "description": "Identifier of the referred entity."},
                "href": {"type": "string", "description": "URI of the referred entity."},
                "name": {"type": "string", "description": "Name of the referred entity."},
                "version": {"type": "string", "description": "Version of the referred entity."},
                "@referredType": {"type": "string", "description": "Actual type of the target."},
                "@type": {"type": "string", "description": "Type of this reference object."},
                "@baseType": {"type": "string"},
                "@schemaLocation": {"type": "string"},
            },
            "additionalProperties": true,
        })
    }
}

impl JsonSchema for PartyOrPartyRole {
    fn schema_name() -> Cow<'static, str> {
        "PartyRefOrPartyRoleRef".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "rutmf::core::PartyOrPartyRole".into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        // Both arms share the `EntityRef` shape and are told apart by `@type`,
        // which is exactly what the v5 `oneOf` + discriminator expresses.
        let reference = <Ref<()> as JsonSchema>::json_schema(generator);
        json_schema!({
            "description": "Either a PartyRef or a PartyRoleRef, discriminated by @type.",
            "oneOf": [reference],
            "discriminator": {"propertyName": "@type"},
        })
    }
}
