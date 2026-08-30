//! The four PATCH flavours a TMF v5 endpoint accepts.

use crate::core::{JsonPatchOp, PatchBody};

/// A `PATCH` request body, paired with the semantics it must be sent under.
///
/// # Why this is one type and not two arguments
///
/// TMF620 v5 declares four request content types for `PATCH`, and the body
/// schema is **not** the same for all of them. The specification pairs them:
///
/// | `Content-Type` | Body schema |
/// |---|---|
/// | `application/merge-patch+json` | the resource's `_MVO` |
/// | `application/json` | the resource's `_MVO` |
/// | `application/json-patch+json` | an array of RFC 6902 operations |
/// | `application/json-patch-query+json` | an array of RFC 6902 operations |
///
/// Taking the body and the content type as separate arguments would let you
/// send an `_MVO` body labelled `application/json-patch+json` — a request every
/// conformant server rejects, and one the compiler is in a position to prevent.
///
/// So `Patch` carries the body *inside* the variant, and the pairing cannot
/// come apart. The common cases need no ceremony, because the two ordinary
/// bodies convert into it — an `…Update` type is a
/// [`PatchBody`](crate::core::PatchBody) and an operation list is not, which is
/// what keeps the two conversions apart:
///
/// ```no_run
/// # async fn demo(
/// #     client: rutmf::api::tmf620::ProductCatalogClient,
/// #     update: rutmf::product::ProductOfferingUpdate,
/// # ) -> rutmf::api::Result<()> {
/// use rutmf::api::{JsonPatchOp, Patch};
///
/// // A merge patch — the safe default.
/// client.update_product_offering("42", &update).await?;
///
/// // An RFC 6902 operation list, to change one array element in place.
/// let ops = [JsonPatchOp::replace("/productOfferingPrice/0/name", "Promo")];
/// client.update_product_offering("42", &ops).await?;
///
/// // The TM Forum `JSONPath` dialect, to target by predicate instead of index.
/// let ops = [JsonPatchOp::replace("/place[?(@.id=='9989')]/name", "Berlin")];
/// client.update_product_offering("42", Patch::Query(&ops)).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Patch<'a, U> {
    /// `application/merge-patch+json` — RFC 7386, and the safe default.
    ///
    /// Members present in the body replace their counterparts, and an explicit
    /// `null` deletes one. This is what `&update` converts into.
    Merge(&'a U),

    /// `application/json` — an implicit merge.
    ///
    /// Behaves like a merge patch on most servers, but the semantics are the
    /// API's own rather than an RFC's. Prefer [`Patch::Merge`] unless a server
    /// specifically wants this.
    Implicit(&'a U),

    /// `application/json-patch+json` — an RFC 6902 operation list.
    ///
    /// Needed to modify a single array element without resending the array.
    /// This is what `&ops` converts into, for a slice, array or `Vec`.
    Operations(&'a [JsonPatchOp]),

    /// `application/json-patch-query+json` — the TM Forum `JSONPath` extension.
    ///
    /// Like RFC 6902, but `path` accepts a `JSONPath` expression, so an operation
    /// can target array elements by predicate rather than by index.
    Query(&'a [JsonPatchOp]),
}

impl<U> Patch<'_, U> {
    /// The `Content-Type` this body must be sent with.
    #[must_use]
    pub fn content_type(&self) -> &'static str {
        match self {
            Self::Merge(_) => "application/merge-patch+json",
            Self::Implicit(_) => "application/json",
            Self::Operations(_) => "application/json-patch+json",
            Self::Query(_) => "application/json-patch-query+json",
        }
    }
}

impl<U: PatchBody> Patch<'_, U> {
    /// Serialises the body this patch carries.
    ///
    /// # Errors
    ///
    /// Returns the serde error if the body cannot be encoded.
    pub fn to_json(&self) -> Result<Vec<u8>, serde_json::Error> {
        match self {
            Self::Merge(body) | Self::Implicit(body) => serde_json::to_vec(body),
            Self::Operations(ops) | Self::Query(ops) => serde_json::to_vec(ops),
        }
    }
}

// `U: PatchBody` is what makes these four coexist. Without it, `From<&U>`
// and `From<&Vec<JsonPatchOp>>` overlap at `U = Vec<JsonPatchOp>`; with it,
// they cannot, because an operation list is not a patch *body*.
impl<'a, U: PatchBody> From<&'a U> for Patch<'a, U> {
    fn from(body: &'a U) -> Self {
        Self::Merge(body)
    }
}

impl<'a, U: PatchBody> From<&'a [JsonPatchOp]> for Patch<'a, U> {
    fn from(ops: &'a [JsonPatchOp]) -> Self {
        Self::Operations(ops)
    }
}

impl<'a, U: PatchBody> From<&'a Vec<JsonPatchOp>> for Patch<'a, U> {
    fn from(ops: &'a Vec<JsonPatchOp>) -> Self {
        Self::Operations(ops)
    }
}

impl<'a, U: PatchBody, const N: usize> From<&'a [JsonPatchOp; N]> for Patch<'a, U> {
    fn from(ops: &'a [JsonPatchOp; N]) -> Self {
        Self::Operations(ops)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Serialize)]
    struct Update {
        name: &'static str,
    }
    impl PatchBody for Update {}

    #[test]
    fn content_types_match_the_v5_spec() {
        let update = Update { name: "n" };
        let ops = [JsonPatchOp::remove("/x")];

        assert_eq!(
            Patch::Merge(&update).content_type(),
            "application/merge-patch+json"
        );
        assert_eq!(Patch::Implicit(&update).content_type(), "application/json");
        assert_eq!(
            Patch::<Update>::Operations(&ops).content_type(),
            "application/json-patch+json"
        );
        assert_eq!(
            Patch::<Update>::Query(&ops).content_type(),
            "application/json-patch-query+json"
        );
    }

    #[test]
    fn a_body_converts_to_a_merge_patch_and_ops_to_an_operation_list() {
        let update = Update { name: "n" };
        assert!(matches!(Patch::from(&update), Patch::Merge(_)));

        // A slice, an array and a `Vec` all convert; a patch body cannot be
        // mistaken for an operation list, or the reverse.
        let ops = vec![JsonPatchOp::remove("/x")];
        assert!(matches!(Patch::<Update>::from(&ops), Patch::Operations(_)));
        assert!(matches!(
            Patch::<Update>::from(&ops[..]),
            Patch::Operations(_)
        ));

        let array = [JsonPatchOp::remove("/x")];
        assert!(matches!(
            Patch::<Update>::from(&array),
            Patch::Operations(_)
        ));
    }

    #[test]
    fn each_variant_serialises_the_body_it_carries() {
        let update = Update { name: "n" };
        assert_eq!(Patch::Merge(&update).to_json().unwrap(), br#"{"name":"n"}"#);

        let ops = [JsonPatchOp::remove("/description")];
        assert_eq!(
            Patch::<Update>::Query(&ops).to_json().unwrap(),
            br#"[{"op":"remove","path":"/description"}]"#
        );
    }

    #[test]
    fn remove_op_omits_value() {
        let json = serde_json::to_string(&JsonPatchOp::remove("/description")).unwrap();
        assert_eq!(json, r#"{"op":"remove","path":"/description"}"#);
    }
}
