//! Event notifications: the hub/listener pattern.
//!
//! Every v5 API exposes a `/hub` collection. You `POST` a callback URL, the
//! server records a subscription, and it then `POST`s events to
//! `{callback}/listener/{eventName}` as they occur.
//!
//! Note the version skew: the generic **Event Management API (TMF688) is still
//! v4 only**, so what v5 actually ships is this per-API hub, not a central event
//! bus. This module models the per-API pattern; it does not pretend a v5 TMF688
//! exists.

use crate::core::macros::tmf_struct;
use crate::core::{EventKind, TmfType};

use super::client::TmfClient;
use super::error::Result;
use super::query::Query;

const HUB: &str = "hub";

tmf_struct! {
    @name = "Hub";
    /// A registered notification subscription.
    pub struct Hub {
        /// Server-assigned identifier of the subscription.
        id: String,
        /// URI of the subscription.
        href: String,
        /// The callback URL events are delivered to.
        callback: String,
        /// Optional filter narrowing which events are delivered.
        ///
        /// Servers interpret this as a TMF630 filter expression, e.g.
        /// `eventType=ProductOfferingCreateEvent`.
        query: String,
    }
}

tmf_struct! {
    @name = "Hub";
    /// Body of a `POST /hub`.
    pub struct HubCreate {
        @required {
            /// The callback URL events should be delivered to. **Required.**
            callback: String,
        }
        /// Optional filter narrowing which events are delivered.
        query: String,
    }
}

impl HubCreate {
    /// Subscribes to every event the API emits.
    pub fn to(callback: impl Into<String>) -> Self {
        Self::builder().callback(callback).build()
    }

    /// Subscribes to one kind of change to one resource.
    ///
    /// The event class name is derived from the type, so a subscription cannot
    /// be misspelled into silence — which is how a wrong `eventType` filter
    /// fails: the hub registers happily and never delivers anything.
    ///
    /// ```
    /// use rutmf::api::HubCreate;
    /// use rutmf::core::EventKind;
    /// use rutmf::product::ProductOffering;
    ///
    /// let sub = HubCreate::for_resource::<ProductOffering>(
    ///     "https://me/callback",
    ///     EventKind::Create,
    /// );
    /// assert_eq!(
    ///     sub.query.as_deref(),
    ///     Some("eventType=ProductOfferingCreateEvent"),
    /// );
    /// ```
    pub fn for_resource<T: TmfType>(callback: impl Into<String>, kind: EventKind) -> Self {
        Self::for_event(callback, &kind.name_for::<T>())
    }

    /// Subscribes to a named event type.
    ///
    /// Prefer [`for_resource`](Self::for_resource), which derives the name.
    /// This is for an event class this crate does not model — a vendor's own.
    ///
    /// ```
    /// use rutmf::api::HubCreate;
    ///
    /// let sub = HubCreate::for_event("https://me/callback", "VendorAuditEvent");
    /// assert_eq!(sub.query.as_deref(), Some("eventType=VendorAuditEvent"));
    /// ```
    pub fn for_event(callback: impl Into<String>, event_type: &str) -> Self {
        Self::builder()
            .callback(callback)
            .query(format!("eventType={event_type}"))
            .build()
    }
}

/// Hub operations, shared by every per-API client.
///
/// Implemented by each client so subscription management reads the same way
/// across APIs; you only need the client itself in scope.
#[allow(
    async_fn_in_trait,
    reason = "clients are used directly, not as trait objects"
)]
pub trait HubOps {
    /// The generic client this API dispatches through.
    fn hub_client(&self) -> &TmfClient;

    /// Registers a listener, returning the created subscription.
    async fn register_listener(&self, hub: &HubCreate) -> Result<Hub> {
        self.hub_client().create(HUB, hub).await
    }

    /// Removes a subscription by id.
    async fn unregister_listener(&self, id: &str) -> Result<()> {
        self.hub_client().delete(HUB, id).await
    }

    /// Retrieves a subscription by id.
    ///
    /// Rarely available: of the fourteen APIs this crate covers, only
    /// **TMF621, TMF629, TMF639, TMF642 and TMF679** define `GET /hub/{id}`.
    /// The other nine expose `POST /hub` and `DELETE /hub/{id}` and nothing
    /// else, so expect a `405` from them.
    ///
    /// None of the fourteen lets you *list* subscriptions at all — there is no
    /// `GET /hub`. Keep the id you were given at registration; it is the only
    /// handle on the subscription you will get.
    ///
    /// Which APIs those are is not a remembered fact:
    /// `the_hub_surface_is_what_the_specifications_declare` in
    /// `tests/coverage.rs` reads it back out of the vendored documents.
    async fn get_listener(&self, id: &str) -> Result<Hub> {
        self.hub_client().get(HUB, id, &Query::new()).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hub_round_trips_unknown_members() {
        let json = r#"{"callback":"https://me/cb","@type":"Hub","x-secret":"s"}"#;
        let hub: Hub = serde_json::from_str(json).unwrap();
        assert_eq!(hub.callback.as_deref(), Some("https://me/cb"));
        assert_eq!(
            serde_json::to_value(&hub).unwrap(),
            serde_json::from_str::<serde_json::Value>(json).unwrap()
        );
    }
}
