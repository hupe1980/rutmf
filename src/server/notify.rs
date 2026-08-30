//! Notifications: naming an event, and finding who asked for it.
//!
//! The half of the event story a *server* owes. `api::HubOps` registers a
//! subscription and `core::TmfEvent` reads one; this is what happens in between,
//! on the server, when a resource actually changes.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::core::EventKind;

use super::semantics::matches_filters;

/// The collection a `hub` subscription is stored in.
///
/// TMF v5 gives every API a `/hub` collection, so the handler serves it like
/// any other and a store needs no special case for it.
pub const HUB_COLLECTION: &str = "hub";

/// A registered subscription an event should be delivered to.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Listener {
    /// Identifier of the `hub` resource that registered this callback.
    pub hub_id: String,
    /// The URL the event should be `POST`ed to.
    ///
    /// TMF630 delivers to `{callback}/listener/{eventName}`; see
    /// [`Listener::delivery_url`].
    pub callback: String,
}

impl Listener {
    /// Where this listener expects `event_type` to be delivered.
    ///
    /// TM Forum appends `/listener/{eventName}` to the registered callback, so
    /// one subscription serves every event class the client asked for and the
    /// receiver dispatches on the path.
    ///
    /// ```
    /// use rutmf::server::{Listener, matching_listeners};
    /// use serde_json::json;
    ///
    /// let hubs = vec![json!({"id": "h1", "callback": "https://me/tmf"})];
    /// let event = json!({"eventType": "ProductOfferingCreateEvent"});
    ///
    /// let listener = &matching_listeners(&hubs, &event)[0];
    /// assert_eq!(
    ///     listener.delivery_url("ProductOfferingCreateEvent"),
    ///     "https://me/tmf/listener/productOfferingCreateEvent",
    /// );
    /// ```
    #[must_use]
    pub fn delivery_url(&self, event_type: &str) -> String {
        let mut member = event_type.to_owned();
        if let Some(first) = member.get_mut(..1) {
            first.make_ascii_lowercase();
        }
        format!("{}/listener/{member}", self.callback.trim_end_matches('/'))
    }
}

/// The registered subscriptions that want `event`.
///
/// A hub's `query` is the TMF630 filter it looks like, so
/// `eventType=ProductOfferingCreateEvent` selects that class alone and a hub
/// registered with no query receives everything.
///
/// ```
/// use rutmf::server::matching_listeners;
/// use serde_json::json;
///
/// let hubs = vec![
///     json!({"id": "a", "callback": "https://me/created",
///            "query": "eventType=ProductOfferingCreateEvent"}),
///     json!({"id": "b", "callback": "https://me/everything"}),
/// ];
///
/// let created = json!({"eventType": "ProductOfferingCreateEvent"});
/// assert_eq!(matching_listeners(&hubs, &created).len(), 2);
///
/// let deleted = json!({"eventType": "ProductOfferingDeleteEvent"});
/// assert_eq!(matching_listeners(&hubs, &deleted).len(), 1, "only the catch-all");
/// ```
#[must_use]
pub fn matching_listeners(hubs: &[Value], event: &Value) -> Vec<Listener> {
    hubs.iter()
        .filter(|hub| {
            hub.get("query")
                .and_then(Value::as_str)
                .is_none_or(|query| matches_filters(event, &parse_filter(query)))
        })
        .filter_map(|hub| {
            Some(Listener {
                hub_id: hub.get("id")?.as_str()?.to_owned(),
                callback: hub.get("callback")?.as_str()?.to_owned(),
            })
        })
        .collect()
}

/// Reads a hub's `query` string into the filter map it stands for.
fn parse_filter(query: &str) -> BTreeMap<String, String> {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
}

/// The event class name for a change of `kind` to a resource in `collection`.
///
/// TM Forum names every notification `{Resource}{Kind}Event`, and the
/// collection segment of the URL *is* the resource name with a lowercase
/// initial — so `productOffering` plus [`EventKind::Create`] is
/// `ProductOfferingCreateEvent`, with no table to keep in step.
///
/// ```
/// use rutmf::core::EventKind;
/// use rutmf::server::event_type_for;
///
/// assert_eq!(
///     event_type_for("productOffering", EventKind::Create),
///     "ProductOfferingCreateEvent",
/// );
/// ```
#[must_use]
pub fn event_type_for(collection: &str, kind: EventKind) -> String {
    let mut class = collection.to_owned();
    if let Some(first) = class.get_mut(..1) {
        first.make_ascii_uppercase();
    }
    format!("{class}{}", kind.suffix())
}

/// The collections whose lifecycle move TM Forum spells `…StatusChangeEvent`.
///
/// Transcribed from the vendored `/listener/…` paths, and re-checked against
/// them by `the_status_change_collections_are_the_ones_the_specifications_declare`
/// in `tests/coverage.rs`.
const STATUS_CHANGE_COLLECTIONS: &[&str] = &[
    // TMF621.
    "troubleTicket",
    "troubleTicketSpecification",
    // TMF634 — which spells the member `lifecycleStatus`, as TMF620 does.
    "resourceCandidate",
    "resourceCatalog",
    "resourceCategory",
    "resourceSpecification",
];

/// Which event a lifecycle move on `collection` raises.
///
/// Twelve of the fourteen vendored APIs raise `…StateChangeEvent`; TMF621 and
/// TMF634 raise `…StatusChangeEvent` for the same change, over a member TMF634
/// still spells `lifecycleStatus`. The difference is not visible in the
/// resource, so it has to be known — and a subscriber filters on the name, so a
/// server raising the other spelling delivers nothing to a correctly registered
/// hub.
///
/// ```
/// use rutmf::core::EventKind;
/// use rutmf::server::state_change_kind;
///
/// assert_eq!(state_change_kind("productOffering"), EventKind::StateChange);
/// assert_eq!(state_change_kind("resourceCatalog"), EventKind::StatusChange);
/// assert_eq!(state_change_kind("troubleTicket"), EventKind::StatusChange);
/// ```
///
/// An unknown collection gets [`EventKind::StateChange`], which is the majority
/// spelling and the one a vendor extension almost certainly follows.
#[must_use]
pub fn state_change_kind(collection: &str) -> EventKind {
    if STATUS_CHANGE_COLLECTIONS.contains(&collection) {
        EventKind::StatusChange
    } else {
        EventKind::StateChange
    }
}

/// Builds the TMF notification envelope for a change to `resource`.
///
/// The payload member is the collection name, which is what the v5 event
/// schemas declare: a `ProductOfferingCreateEvent` wraps its resource under
/// `productOffering`. [`TmfEvent::resource`] reads it back by the same rule, so
/// a server built here and a client built here agree by construction.
///
/// `event_id` is the server's own identifier for the notification — the same
/// [`IdGenerator`](super::IdGenerator) that names resources is a reasonable
/// source.
///
/// ```
/// use rutmf::core::{EventKind, TmfEvent};
/// use rutmf::product::ProductOffering;
/// use rutmf::server::change_event;
/// use serde_json::json;
///
/// let resource = json!({"id": "7655", "name": "Firewall", "@type": "ProductOffering"});
/// let raw = change_event("productOffering", EventKind::Create, &resource, "e-1");
///
/// // It is a TMF event, and the client half reads it without being told how.
/// let event: TmfEvent = serde_json::from_value(raw).unwrap();
/// assert_eq!(event.event_type.as_deref(), Some("ProductOfferingCreateEvent"));
///
/// let offering: ProductOffering = event.resource().unwrap().unwrap();
/// assert_eq!(offering.id.as_deref(), Some("7655"));
/// ```
///
/// [`TmfEvent::resource`]: crate::core::TmfEvent::resource
#[must_use]
pub fn change_event(collection: &str, kind: EventKind, resource: &Value, event_id: &str) -> Value {
    let event_type = event_type_for(collection, kind);

    let mut payload = Map::new();
    payload.insert(collection.to_owned(), resource.clone());

    let mut event = Map::new();
    event.insert("eventId".to_owned(), Value::String(event_id.to_owned()));
    event.insert("eventType".to_owned(), Value::String(event_type.clone()));
    event.insert("event".to_owned(), Value::Object(payload));
    event.insert("@type".to_owned(), Value::String(event_type));
    Value::Object(event)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_event_name_comes_from_the_collection() {
        assert_eq!(
            event_type_for("productOffering", EventKind::Create),
            "ProductOfferingCreateEvent"
        );
        assert_eq!(
            event_type_for("troubleTicket", EventKind::StatusChange),
            "TroubleTicketStatusChangeEvent"
        );
        // A collection whose name is already capitalised is left alone rather
        // than double-capitalised into nonsense.
        assert_eq!(
            event_type_for("Alarm", EventKind::Delete),
            "AlarmDeleteEvent"
        );
        assert_eq!(event_type_for("", EventKind::Create), "CreateEvent");
    }

    #[test]
    fn a_lifecycle_move_is_named_the_way_the_owning_api_names_it() {
        // A subscriber filters on the class name, so raising
        // `ResourceCatalogStateChangeEvent` against a spec that declares
        // `ResourceCatalogStatusChangeEvent` delivers nothing to anyone.
        assert_eq!(
            event_type_for("resourceCatalog", state_change_kind("resourceCatalog")),
            "ResourceCatalogStatusChangeEvent"
        );
        assert_eq!(
            event_type_for("troubleTicket", state_change_kind("troubleTicket")),
            "TroubleTicketStatusChangeEvent"
        );
        // And the majority spelling is still the majority spelling.
        assert_eq!(
            event_type_for("productOffering", state_change_kind("productOffering")),
            "ProductOfferingStateChangeEvent"
        );
        assert_eq!(
            event_type_for("service", state_change_kind("service")),
            "ServiceStateChangeEvent"
        );
    }

    #[test]
    fn a_change_event_round_trips_through_the_client_half() {
        // The point of deriving both ends from the collection name: what the
        // server writes is exactly what `TmfEvent::resource` looks for.
        let resource = json!({"id": "1", "@type": "Alarm"});
        let raw = change_event("alarm", EventKind::StateChange, &resource, "e-9");

        let event: crate::core::TmfEvent =
            serde_json::from_value(raw).expect("a well-formed TMF event");
        assert_eq!(event.kind(), Some(EventKind::StateChange));
        assert_eq!(event.resource_key(), Some("alarm"));
        assert_eq!(event.resource::<Value>().unwrap(), Some(resource));
    }

    #[test]
    fn a_hub_without_an_id_or_callback_is_not_a_listener() {
        // A malformed hub row must not become a delivery to nowhere.
        let hubs = vec![
            json!({"callback": "https://me/x"}),
            json!({"id": "no-callback"}),
            json!({"id": "ok", "callback": "https://me/y"}),
        ];
        let listeners = matching_listeners(&hubs, &json!({"eventType": "AnyEvent"}));
        assert_eq!(listeners.len(), 1);
        assert_eq!(listeners[0].hub_id, "ok");
    }

    #[test]
    fn a_callback_keeps_one_separator_however_it_was_registered() {
        let listener = Listener {
            hub_id: "h".into(),
            callback: "https://me/tmf/".into(),
        };
        assert_eq!(
            listener.delivery_url("AlarmCreateEvent"),
            "https://me/tmf/listener/alarmCreateEvent"
        );
    }
}
