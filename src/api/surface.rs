//! What a TM Forum API declares: its collections and the operations on each.
//!
//! Every v5 specification is the same shape — flat `/{collection}` and
//! `/{collection}/{id}` paths — but no two declare the same *operations*.
//! TMF678 lets a client read a customer bill and move it through its lifecycle,
//! never create or delete one; TMF642's `ackAlarm` is a task, created and read
//! back but never changed. [`Api`] writes that down as data, once per API, so
//! the server can refuse what the specification does not declare and the
//! clients can be checked against the same table.
//!
//! Each `rutmf::api::tmfXXX` module publishes one as `API`, and a test compares
//! every one of them with its pinned specification document in both directions.
//! An API this crate does not cover can be declared the same way:
//!
//! ```
//! use rutmf::api::{Api, Collection, Operation};
//!
//! /// A vendor API: read-only usage records, and a task to request a re-rating.
//! const USAGE: Api = Api::new(
//!     "ACME-USAGE",
//!     "tmf-api/acmeUsage/v1",
//!     "1.0.0",
//!     &[
//!         Collection::read_only("usageRecord"),
//!         Collection::task("rerateRequest"),
//!         Collection::new("hub", &[Operation::Create, Operation::Delete]),
//!     ],
//! );
//!
//! let records = USAGE.collection("usageRecord").unwrap();
//! assert!(records.allows(&http::Method::GET, true));
//! assert!(!records.allows(&http::Method::DELETE, true));
//! assert_eq!(records.allowed_methods(true), [http::Method::GET]);
//! ```

use http::Method;

/// One operation a specification can declare on a collection.
///
/// TMF630 defines five, each one HTTP method on one of the two path forms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Operation {
    /// `GET /{collection}` — list, filter, page.
    List,
    /// `POST /{collection}` — create a resource, or submit a task.
    Create,
    /// `GET /{collection}/{id}` — read one resource.
    Retrieve,
    /// `PATCH /{collection}/{id}` — partially update one resource.
    Update,
    /// `DELETE /{collection}/{id}` — remove one resource.
    Delete,
}

impl Operation {
    /// Every operation, in the order an `Allow` header lists their methods.
    pub const ALL: [Self; 5] = [
        Self::List,
        Self::Retrieve,
        Self::Create,
        Self::Update,
        Self::Delete,
    ];

    /// The HTTP method this operation is sent with.
    #[must_use]
    pub const fn method(self) -> Method {
        match self {
            Self::List | Self::Retrieve => Method::GET,
            Self::Create => Method::POST,
            Self::Update => Method::PATCH,
            Self::Delete => Method::DELETE,
        }
    }

    /// Whether this operation addresses one resource (`/{collection}/{id}`)
    /// rather than the collection.
    #[must_use]
    pub const fn on_item(self) -> bool {
        matches!(self, Self::Retrieve | Self::Update | Self::Delete)
    }

    /// The operation `method` performs on the collection path (`on_item ==
    /// false`) or the item path, if there is one.
    #[must_use]
    pub fn from_method(method: &Method, on_item: bool) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|op| op.on_item() == on_item && op.method() == method)
    }
}

/// A collection and the operations its specification declares on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Collection {
    /// The path segment, e.g. `customerBill`.
    pub name: &'static str,
    /// The declared operations, in no particular order.
    pub operations: &'static [Operation],
}

impl Collection {
    /// A collection declaring exactly `operations`.
    #[must_use]
    pub const fn new(name: &'static str, operations: &'static [Operation]) -> Self {
        Self { name, operations }
    }

    /// All five operations — what `resource_ops!` generates on the client.
    #[must_use]
    pub const fn crud(name: &'static str) -> Self {
        Self::new(
            name,
            &[
                Operation::List,
                Operation::Create,
                Operation::Retrieve,
                Operation::Update,
                Operation::Delete,
            ],
        )
    }

    /// List and retrieve only: a collection the server populates and the
    /// client reads, such as TMF678's `appliedCustomerBillingRate`.
    #[must_use]
    pub const fn read_only(name: &'static str) -> Self {
        Self::new(name, &[Operation::List, Operation::Retrieve])
    }

    /// A task: submitted, then read back, never changed or removed — TMF642's
    /// `ackAlarm`, TMF622's `cancelProductOrder`.
    #[must_use]
    pub const fn task(name: &'static str) -> Self {
        Self::new(
            name,
            &[Operation::List, Operation::Create, Operation::Retrieve],
        )
    }

    /// Whether `operation` is declared.
    #[must_use]
    pub fn declares(&self, operation: Operation) -> bool {
        self.operations.contains(&operation)
    }

    /// Whether `method` is declared on the collection path (`on_item ==
    /// false`) or on the item path.
    #[must_use]
    pub fn allows(&self, method: &Method, on_item: bool) -> bool {
        Operation::from_method(method, on_item).is_some_and(|op| self.declares(op))
    }

    /// The methods declared on one path form, in the order an `Allow` header
    /// lists them: `GET`, `POST`, `PATCH`, `DELETE`.
    ///
    /// Empty when the specification declares that path form not at all.
    #[must_use]
    pub fn allowed_methods(&self, on_item: bool) -> Vec<Method> {
        Operation::ALL
            .into_iter()
            .filter(|op| op.on_item() == on_item && self.declares(*op))
            .map(Operation::method)
            .collect()
    }
}

/// One TM Forum API as a server exposes it.
///
/// Published as `API` by every `rutmf::api::tmfXXX` module, and what
/// [`TmfHandler`](crate::server::TmfHandler) is built for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Api {
    /// The specification's identifier, e.g. `TMF678`.
    pub name: &'static str,
    /// The conventional API root, appended to a host to form the base URL,
    /// e.g. `tmf-api/customerBillManagement/v5`.
    pub path: &'static str,
    /// The specification version, e.g. `5.0.0`.
    pub version: &'static str,
    /// Every collection the specification declares, including `hub` and those
    /// this crate does not model.
    pub collections: &'static [Collection],
}

impl Api {
    /// Declares an API.
    #[must_use]
    pub const fn new(
        name: &'static str,
        path: &'static str,
        version: &'static str,
        collections: &'static [Collection],
    ) -> Self {
        Self {
            name,
            path,
            version,
            collections,
        }
    }

    /// The collection called `name`, if the API declares one.
    #[must_use]
    pub fn collection(&self, name: &str) -> Option<&Collection> {
        self.collections.iter().find(|c| c.name == name)
    }

    /// The base URL of this API on `host`: `{host}/{path}`.
    #[must_use]
    pub fn url_on(&self, host: &str) -> String {
        format!("{}/{}", host.trim_end_matches('/'), self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BILLS: Api = Api::new(
        "TMF678",
        "tmf-api/customerBillManagement/v5",
        "5.0.0",
        &[
            Collection::new(
                "customerBill",
                &[Operation::List, Operation::Retrieve, Operation::Update],
            ),
            Collection::read_only("appliedCustomerBillingRate"),
        ],
    );

    #[test]
    fn allowed_methods_follow_the_declaration_in_a_stable_order() {
        let bills = BILLS.collection("customerBill").unwrap();
        assert_eq!(bills.allowed_methods(false), [Method::GET]);
        assert_eq!(bills.allowed_methods(true), [Method::GET, Method::PATCH]);
        let crud = Collection::crud("x");
        assert_eq!(
            crud.allowed_methods(true),
            [Method::GET, Method::PATCH, Method::DELETE]
        );
        assert_eq!(crud.allowed_methods(false), [Method::GET, Method::POST]);
    }

    #[test]
    fn a_method_maps_to_an_operation_only_on_its_own_path_form() {
        assert_eq!(
            Operation::from_method(&Method::POST, false),
            Some(Operation::Create)
        );
        assert_eq!(Operation::from_method(&Method::POST, true), None);
        assert_eq!(Operation::from_method(&Method::PUT, true), None);
        assert_eq!(Operation::from_method(&Method::PATCH, false), None);
    }

    #[test]
    fn undeclared_collections_and_methods_are_not_allowed() {
        assert!(BILLS.collection("listener").is_none());
        let bills = BILLS.collection("customerBill").unwrap();
        assert!(!bills.allows(&Method::POST, false));
        assert!(!bills.allows(&Method::DELETE, true));
        assert!(bills.allows(&Method::PATCH, true));
    }

    #[test]
    fn the_base_url_joins_host_and_path_once() {
        assert_eq!(
            BILLS.url_on("https://host/"),
            "https://host/tmf-api/customerBillManagement/v5"
        );
    }
}
