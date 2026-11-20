//! `ProductUsage` and its status — TMF635.

use serde::{Deserialize, Serialize};

use crate::core::macros::{tmf_entity, tmf_patch_body, tmf_struct};
use crate::core::{
    Characteristic, ExternalIdentifier, ProductUsageSpecification, Ref, RelatedParty, ServiceUsage,
    Timestamp,
};
use crate::product::{Product, ProductPrice};

/// How far a [`ProductUsage`] has got — the v5 `ProductUsageStatusType`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum ProductUsageStatus {
    /// Recorded, not yet checked.
    #[serde(rename = "initialized")]
    Initialized,
    /// Refused by validation.
    #[serde(rename = "rejected")]
    Rejected,
    /// Checked and ready to rate.
    #[serde(rename = "validated")]
    Validated,
    /// Priced.
    #[serde(rename = "rated")]
    Rated,
    /// A record that corrects an earlier one, priced.
    #[serde(rename = "corrected")]
    Corrected,
    /// Replaced by a corrected record.
    #[serde(rename = "superseded")]
    Superseded,
    /// Deliberately not priced.
    #[serde(rename = "notRated")]
    NotRated,
    /// Rating failed.
    #[serde(rename = "failed")]
    Failed,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

impl ProductUsageStatus {
    /// Whether the record carries prices a bill may use: `rated` or
    /// `corrected`.
    #[must_use]
    pub fn is_rated(&self) -> bool {
        matches!(self, Self::Rated | Self::Corrected)
    }

    /// Whether the record has stopped moving.
    ///
    /// An unrecognised status is **not** final, so a process waiting for a
    /// record to settle keeps waiting rather than acting on a state it does not
    /// know.
    #[must_use]
    pub fn is_final(&self) -> bool {
        matches!(
            self,
            Self::Rejected | Self::Superseded | Self::NotRated | Self::Failed
        )
    }
}

tmf_struct! {
    // TMF635 declares no `ProductUsageRef`, so a reference to a usage record is
    // the generic `EntityRef` naming `ProductUsage` as its `@referredType`.
    @name = "ProductUsage", @ref = "EntityRef";
    /// One usage record against a product — a call, a session, an event — and,
    /// once rated, its prices.
    ///
    /// ```
    /// use rutmf::usage::{ProductUsage, ProductUsageStatus};
    ///
    /// let json = r#"{"@type":"ProductUsage","status":"rated",
    ///   "usageProductPrice":[{"@type":"ProductPrice","priceType":"usage",
    ///     "price":{"@type":"Price","taxIncludedAmount":{"unit":"EUR","value":0.1}}}]}"#;
    /// let usage: ProductUsage = serde_json::from_str(json).unwrap();
    ///
    /// assert!(usage.status.as_ref().unwrap().is_rated());
    /// // Money is an exact decimal: 0.1 stays 0.1.
    /// assert!(serde_json::to_string(&usage).unwrap().contains(r#""value":0.1"#));
    /// ```
    pub struct ProductUsage {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this record.
        href: String,
        /// When the record was created.
        creation_date: Timestamp,
        /// When the record was last changed.
        last_update: Timestamp,
        /// How far the record has got.
        status: ProductUsageStatus,
        /// What kind of usage this is — voice, data, an event.
        usage_type: String,
        /// The measured quantities and attributes of the usage.
        usage_characteristic: Vec<Characteristic>,
        /// The service-level records this one was built from.
        service_usage: Vec<Ref<ServiceUsage>>,
        /// Parties related to the usage.
        related_party: Vec<RelatedParty>,
        /// The prices rating assigned, without applied taxes unless stated.
        usage_product_price: Vec<ProductPrice>,
        /// The product the usage was consumed on.
        product: Ref<Product>,
        /// The specification the record conforms to.
        usage_specification: Ref<ProductUsageSpecification>,
        /// Identifiers the record has in other systems — a CDR id, say.
        external_identifier: Vec<ExternalIdentifier>,
    }
}

tmf_struct! {
    @name = "ProductUsage";
    /// Body of a `POST /productUsage` — the v5 `ProductUsage_FVO`.
    ///
    /// The product and the usage specification are **required**: a usage
    /// record that says neither what was used nor what it conforms to is not
    /// one a server can rate. There is no `status` and no `creationDate`; the
    /// server owns both.
    ///
    /// ```
    /// use rutmf::core::Ref;
    /// use rutmf::usage::ProductUsageCreate;
    ///
    /// let usage = ProductUsageCreate::builder()
    ///     .product(Ref::new("P-1"))
    ///     .usage_specification(Ref::new("voice-call"))
    ///     .usage_type("voice")
    ///     .build();
    /// assert_eq!(usage.usage_type.as_deref(), Some("voice"));
    /// ```
    ///
    /// Leaving either out does not compile:
    ///
    /// ```compile_fail
    /// use rutmf::core::Ref;
    /// use rutmf::usage::ProductUsageCreate;
    ///
    /// let usage = ProductUsageCreate::builder().product(Ref::new("P-1")).build();
    /// ```
    pub struct ProductUsageCreate {
        @required {
            /// The product the usage was consumed on. **Required on create.**
            product: Ref<Product>,
            /// The specification the record conforms to. **Required on create.**
            usage_specification: Ref<ProductUsageSpecification>,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Client-supplied URI, where the server permits one.
        href: String,
        /// When the record was last changed.
        last_update: Timestamp,
        /// What kind of usage this is.
        usage_type: String,
        /// The measured quantities and attributes of the usage.
        usage_characteristic: Vec<Characteristic>,
        /// The service-level records this one was built from.
        service_usage: Vec<Ref<ServiceUsage>>,
        /// Parties related to the usage.
        related_party: Vec<RelatedParty>,
        /// Prices, when the record arrives already rated.
        usage_product_price: Vec<ProductPrice>,
        /// Identifiers the record has in other systems.
        external_identifier: Vec<ExternalIdentifier>,
    }
}

tmf_struct! {
    @name = "ProductUsage";
    /// Body of a `PATCH /productUsage/{id}` — the v5 `ProductUsage_MVO`.
    ///
    /// To correct a rated record, prefer recording a new one with status
    /// [`Corrected`](ProductUsageStatus::Corrected) and patching only the old
    /// record's status to [`Superseded`](ProductUsageStatus::Superseded): the
    /// record a bill was built from stays as it was.
    pub struct ProductUsageUpdate {
        /// Identifier, as the `_MVO` declares it.
        id: String,
        /// URI, as the `_MVO` declares it.
        href: String,
        /// When the record was last changed.
        last_update: Timestamp,
        /// The status to move the record to.
        status: ProductUsageStatus,
        /// What kind of usage this is.
        usage_type: String,
        /// The measured quantities and attributes of the usage.
        usage_characteristic: Vec<Characteristic>,
        /// The service-level records this one was built from.
        service_usage: Vec<Ref<ServiceUsage>>,
        /// Parties related to the usage.
        related_party: Vec<RelatedParty>,
        /// The prices rating assigned.
        usage_product_price: Vec<ProductPrice>,
        /// The product the usage was consumed on.
        product: Ref<Product>,
        /// The specification the record conforms to.
        usage_specification: Ref<ProductUsageSpecification>,
        /// Identifiers the record has in other systems.
        external_identifier: Vec<ExternalIdentifier>,
    }
}

tmf_struct! {
    @name = "ExportJob";
    /// Body of a `POST /exportJob` on TMF635 — its own `ExportJob_FVO`.
    ///
    /// Not [`ExportJobCreate`](crate::product::ExportJobCreate): TMF620 and
    /// TMF634 require the `url` and accept a status and timestamps, while TMF635
    /// requires nothing and accepts only the job's description. The job that
    /// comes back is the shared [`ExportJob`](crate::product::ExportJob).
    pub struct ProductUsageExportJobCreate {
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Client-supplied URI, where the server permits one.
        href: String,
        /// URL of the file to write to.
        url: String,
        /// Path within the file, where the format supports one.
        path: String,
        /// MIME type of the file.
        content_type: String,
        /// TMF630 filter selecting which usage records to export.
        query: String,
    }
}

tmf_entity!(ProductUsage);
tmf_patch_body!(ProductUsageUpdate);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn round_trip(value: &Value) -> Value {
        let usage: ProductUsage = serde_json::from_value(value.clone()).expect("parses");
        serde_json::to_value(&usage).expect("serialises")
    }

    /// product-usage/SR-001: rated prices survive exactly — no float, and an
    /// integral amount stays integral.
    #[test]
    fn usage_prices_round_trip_exactly() {
        let record = json!({
            "@type": "ProductUsage",
            "id": "U1",
            "status": "rated",
            "usageProductPrice": [{
                "@type": "ProductPrice",
                "priceType": "usage",
                "price": {
                    "@type": "Price",
                    "dutyFreeAmount": {"unit": "EUR", "value": 0.1},
                    "taxIncludedAmount": {"unit": "EUR", "value": 50}
                }
            }]
        });
        let back = round_trip(&record);
        let text = serde_json::to_string(&back).unwrap();
        assert!(text.contains(r#""value":0.1}"#), "{text}");
        assert!(text.contains(r#""value":50}"#), "{text}");
        assert_eq!(back, record);
    }

    /// product-usage/SR-002: a status and a member the model does not know are
    /// written back unchanged.
    #[test]
    fn unknown_status_and_members_survive() {
        let record = json!({
            "@type": "ProductUsage",
            "id": "U2",
            "status": "reRated",
            "x-cdr": {"switch": "MSC-7", "seq": 991}
        });
        let usage: ProductUsage = serde_json::from_value(record.clone()).unwrap();
        assert_eq!(
            usage.status,
            Some(ProductUsageStatus::Other("reRated".into()))
        );
        assert!(!usage.status.as_ref().unwrap().is_final());
        assert_eq!(serde_json::to_value(&usage).unwrap(), record);
    }

    /// TMF635 declares no `ProductUsageRef`: the reference is the generic one.
    #[test]
    fn a_usage_reference_is_the_generic_entity_ref() {
        let reference: Ref<ProductUsage> = Ref::new("U1");
        let json = serde_json::to_value(&reference).unwrap();
        assert_eq!(json["@type"], "EntityRef");
        assert_eq!(json["@referredType"], "ProductUsage");
        assert_eq!(json["id"], "U1");
    }

    #[test]
    fn rated_and_final_follow_the_lifecycle() {
        assert!(ProductUsageStatus::Corrected.is_rated());
        assert!(!ProductUsageStatus::Validated.is_rated());
        assert!(ProductUsageStatus::Superseded.is_final());
        assert!(!ProductUsageStatus::Rated.is_final());
    }
}
