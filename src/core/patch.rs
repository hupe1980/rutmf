//! RFC 6902 JSON Patch operations.
//!
//! An operation list is I/O-free wire data, so it lives in `core` next to the
//! rest of the model: a server applying a patch, or a test asserting on one,
//! needs the type without needing an HTTP client. Which of the four v5 `PATCH`
//! content types a list is sent under is the client's concern, and lives in
//! `api::Patch`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::extensible::Extensions;

/// Which RFC 6902 operation a [`JsonPatchOp`] performs.
///
/// RFC 6902 §4 defines exactly six, and every vendored v5 specification repeats
/// the same closed list on `JsonPatch.op`. A `String` here would accept
/// `"replaces"` and let the server explain the mistake — which is the class of
/// error this crate spends its type system on.
///
/// ```
/// use rutmf::core::{JsonPatchOp, PatchOperation};
///
/// assert_eq!(JsonPatchOp::remove("/x").op, PatchOperation::Remove);
///
/// // An unknown verb still parses, so relaying a payload never fails on one.
/// let odd: JsonPatchOp = serde_json::from_str(r#"{"op":"merge","path":"/x"}"#).unwrap();
/// assert_eq!(odd.op, PatchOperation::Other("merge".into()));
/// assert_eq!(serde_json::to_value(&odd).unwrap()["op"], "merge");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum PatchOperation {
    /// Insert a value, creating the location or shifting an array along.
    #[serde(rename = "add")]
    Add,
    /// Delete the value at the location, which must exist.
    #[serde(rename = "remove")]
    Remove,
    /// Overwrite a value in place; the location must already exist.
    #[serde(rename = "replace")]
    Replace,
    /// Relocate a value, removing it from its old location.
    #[serde(rename = "move")]
    Move,
    /// Duplicate a value, leaving the original in place.
    #[serde(rename = "copy")]
    Copy,
    /// Assert a value, failing the whole patch when it does not hold.
    #[serde(rename = "test")]
    Test,
    /// A verb outside RFC 6902, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

/// One RFC 6902 patch operation.
///
/// ```
/// use rutmf::core::{JsonPatchOp, PatchOperation};
///
/// let ops = vec![
///     JsonPatchOp::replace("/lifecycleStatus", "Retired"),
///     JsonPatchOp::remove("/description"),
/// ];
/// assert_eq!(ops[0].op, PatchOperation::Replace);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub struct JsonPatchOp {
    /// Which operation this is.
    pub op: PatchOperation,
    /// A JSON Pointer (RFC 6901), or a `JSONPath` for
    /// [`Patch::Query`](crate::api::Patch::Query).
    pub path: String,
    /// The value for `add`, `replace` and `test`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    /// The source pointer for `move` and `copy`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// Members not covered by the typed model, kept in document order.
    ///
    /// An operation list is a payload like any other, so the crate's
    /// round-trip guarantee applies to it too. That is not hypothetical:
    /// TMF634's own `ResourceCatalog` patch example writes the new value in a
    /// member named after the field — `{"op":"replace","path":"/relatedParty",
    /// "relatedParty":[…]}` — instead of under `value`. Without this, relaying
    /// that request would silently drop the thing being written.
    #[serde(flatten, default, skip_serializing_if = "Extensions::is_empty")]
    pub extensions: Extensions,
}

impl JsonPatchOp {
    /// An `add` operation.
    pub fn add(path: impl Into<String>, value: impl Into<Value>) -> Self {
        Self {
            op: PatchOperation::Add,
            path: path.into(),
            value: Some(value.into()),
            from: None,
            extensions: Extensions::new(),
        }
    }

    /// A `replace` operation.
    pub fn replace(path: impl Into<String>, value: impl Into<Value>) -> Self {
        Self {
            op: PatchOperation::Replace,
            path: path.into(),
            value: Some(value.into()),
            from: None,
            extensions: Extensions::new(),
        }
    }

    /// A `remove` operation.
    pub fn remove(path: impl Into<String>) -> Self {
        Self {
            op: PatchOperation::Remove,
            path: path.into(),
            value: None,
            from: None,
            extensions: Extensions::new(),
        }
    }

    /// A `test` operation, asserting the value at `path`.
    ///
    /// RFC 6902 makes a patch all-or-nothing, so a leading `test` is how you
    /// get a conditional update: the rest of the list applies only if the
    /// assertion holds.
    pub fn test(path: impl Into<String>, value: impl Into<Value>) -> Self {
        Self {
            op: PatchOperation::Test,
            path: path.into(),
            value: Some(value.into()),
            from: None,
            extensions: Extensions::new(),
        }
    }

    /// A `move` operation.
    pub fn move_from(from: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            op: PatchOperation::Move,
            path: path.into(),
            value: None,
            extensions: Extensions::new(),
            from: Some(from.into()),
        }
    }

    /// A `copy` operation.
    pub fn copy_from(from: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            op: PatchOperation::Copy,
            path: path.into(),
            value: None,
            extensions: Extensions::new(),
            from: Some(from.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_op_omits_value() {
        let json = serde_json::to_string(&JsonPatchOp::remove("/description")).unwrap();
        assert_eq!(json, r#"{"op":"remove","path":"/description"}"#);
    }

    #[test]
    fn move_and_copy_carry_a_source_pointer() {
        let moved = JsonPatchOp::move_from("/a", "/b");
        assert_eq!(moved.from.as_deref(), Some("/a"));
        assert_eq!(moved.path, "/b");
        assert!(moved.value.is_none());
    }
}
