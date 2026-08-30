//! `ProductCatalog` and `Category` — how offerings are organised.
//!
//! Note the v5 rename: the resource is `productCatalog` (v4 called it
//! `catalog`), which is why the client exposes `list_product_catalogs`.

use crate::core::macros::{tmf_entity, tmf_patch_body, tmf_struct};
use crate::core::{Ref, RelatedParty, TimePeriod, Timestamp};

use super::ProductOffering;

tmf_struct! {
    @name = "ProductCatalog";
    /// A collection of categories and offerings published together.
    pub struct ProductCatalog {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this catalog.
        href: String,
        /// Name of the catalog.
        name: String,
        /// Narrative description.
        description: String,
        /// Version of the catalog.
        version: String,
        /// Kind of catalog, e.g. `ProductCatalog`.
        catalog_type: String,
        /// When the catalog was last changed.
        last_update: Timestamp,
        /// Lifecycle status.
        lifecycle_status: String,
        /// Period during which the catalog is valid.
        valid_for: TimePeriod,
        /// Top-level categories in this catalog.
        category: Vec<Ref<Category>>,
        /// Parties related to the catalog, e.g. the publisher.
        related_party: Vec<RelatedParty>,
    }
}

tmf_struct! {
    @name = "ProductCatalog";
    /// Body of a `POST /productCatalog` — the v5 `ProductCatalog_FVO`.
    pub struct ProductCatalogCreate {
        @required {
            /// Name of the catalog. **Required on create.**
            name: String,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Narrative description.
        description: String,
        /// Version of the catalog.
        version: String,
        /// Kind of catalog.
        catalog_type: String,
        /// When the catalog was last changed.
        last_update: Timestamp,
        /// Lifecycle status.
        lifecycle_status: String,
        /// Period during which the catalog is valid.
        valid_for: TimePeriod,
        /// Top-level categories in this catalog.
        category: Vec<Ref<Category>>,
        /// Parties related to the catalog.
        related_party: Vec<RelatedParty>,
    }
}

tmf_struct! {
    @name = "ProductCatalog";
    /// Body of a `PATCH /productCatalog/{id}` — the v5 `_MVO` schema.
    pub struct ProductCatalogUpdate {
        /// New name.
        name: String,
        /// New description.
        description: String,
        /// New version.
        version: String,
        /// New catalog type.
        catalog_type: String,
        /// New last-changed timestamp.
        last_update: Timestamp,
        /// New lifecycle status.
        lifecycle_status: String,
        /// New validity period.
        valid_for: TimePeriod,
        /// Replacement category list.
        category: Vec<Ref<Category>>,
        /// Replacement related parties.
        related_party: Vec<RelatedParty>,
    }
}

tmf_struct! {
    @name = "Category", @ref = "CategoryRef";
    /// A node in the catalog hierarchy, grouping offerings and sub-categories.
    pub struct Category {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this category.
        href: String,
        /// Name of the category.
        name: String,
        /// Narrative description.
        description: String,
        /// Version of the category.
        version: String,
        /// When the category was last changed.
        last_update: Timestamp,
        /// Lifecycle status.
        lifecycle_status: String,
        /// Whether this is a root category of its catalog.
        is_root: bool,
        /// The parent category.
        ///
        /// v5 carries a full `CategoryRef` here; v4's `parentId` string is gone.
        parent: Ref<Category>,
        /// Period during which the category is valid.
        valid_for: TimePeriod,
        /// Child categories.
        sub_category: Vec<Ref<Category>>,
        /// Offerings filed directly under this category.
        product_offering: Vec<Ref<ProductOffering>>,
    }
}

tmf_entity!(ProductCatalog, Category);

tmf_struct! {
    @name = "Category";
    /// Body of a `POST /category` — the v5 `Category_FVO`.
    pub struct CategoryCreate {
        @required {
            /// Name of the category. **Required on create.**
            name: String,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Narrative description.
        description: String,
        /// Version of the category.
        version: String,
        /// When the category was last changed.
        last_update: Timestamp,
        /// Lifecycle status.
        lifecycle_status: String,
        /// Whether this is a root category.
        is_root: bool,
        /// The parent category.
        parent: Ref<Category>,
        /// Period during which the category is valid.
        valid_for: TimePeriod,
        /// Child categories.
        sub_category: Vec<Ref<Category>>,
        /// Offerings filed directly under this category.
        product_offering: Vec<Ref<ProductOffering>>,
    }
}

tmf_struct! {
    @name = "Category";
    /// Body of a `PATCH /category/{id}` — the v5 `Category_MVO`.
    pub struct CategoryUpdate {
        /// New name.
        name: String,
        /// New description.
        description: String,
        /// New version.
        version: String,
        /// New lifecycle status.
        lifecycle_status: String,
        /// New root flag.
        is_root: bool,
        /// New parent category.
        parent: Ref<Category>,
        /// New validity period.
        valid_for: TimePeriod,
        /// Replacement sub-category list.
        sub_category: Vec<Ref<Category>>,
        /// Replacement offering list.
        product_offering: Vec<Ref<ProductOffering>>,
    }
}

tmf_patch_body!(ProductCatalogUpdate, CategoryUpdate);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_v5_parent_is_a_reference_not_an_id() {
        let json = r#"{"@type":"Category","parent":{"id":"12","@type":"CategoryRef"}}"#;
        let category: Category = serde_json::from_str(json).unwrap();
        assert_eq!(category.parent.as_ref().unwrap().id, "12");
        assert!(
            category.extensions.is_empty(),
            "v4's parentId is gone; `parent` must be typed"
        );
    }
}
