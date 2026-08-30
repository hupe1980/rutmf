//! Lifecycle vocabulary shared across domains.

use serde::{Deserialize, Serialize};

/// What an item asks the provider to do.
///
/// The v5 `ItemActionType`, declared identically by TMF622 and TMF637: an order
/// line says what to do, and the inventory record remembers what was done.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum ItemAction {
    /// Provide a new product.
    #[serde(rename = "add")]
    Add,
    /// Change an existing product.
    #[serde(rename = "modify")]
    Modify,
    /// Cease an existing product.
    #[serde(rename = "delete")]
    Delete,
    /// Leave a product unchanged, e.g. when it is carried along by a bundle.
    #[serde(rename = "noChange")]
    NoChange,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}
