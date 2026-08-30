//! The notification envelope every TM Forum API delivers to a listener.
//!
//! This lives in `core` rather than beside the hub client because it is
//! I/O-free wire data: a server implementation, a queue consumer or a webhook
//! handler needs the envelope without needing an HTTP client. Registering a
//! subscription is the client's job, and lives in `api::HubOps`.

use serde_json::Value;

use super::characteristic::Characteristic;
use super::extensible::TmfType;
use super::macros::tmf_struct;
use super::party::RelatedParty;
use super::reference::Ref;
use super::refs::AnyEntity;
use super::value::Timestamp;

tmf_struct! {
    @name = "Event";
    /// An event delivered to a registered listener.
    ///
    /// The `event` member carries the resource the event is about, whose shape
    /// depends on `event_type`; [`payload`] deserialises it once you know which
    /// resource to expect.
    ///
    /// ```
    /// use rutmf::core::TmfEvent;
    /// use rutmf::product::ProductOffering;
    ///
    /// let json = r#"{
    ///   "eventId": "e1",
    ///   "eventType": "ProductOfferingCreateEvent",
    ///   "event": {"productOffering": {"id": "7655", "@type": "ProductOffering"}},
    ///   "@type": "ProductOfferingCreateEvent"
    /// }"#;
    /// let event: TmfEvent = serde_json::from_str(json).unwrap();
    ///
    /// let offering: ProductOffering = event.payload("productOffering").unwrap().unwrap();
    /// assert_eq!(offering.id.as_deref(), Some("7655"));
    /// ```
    ///
    /// [`payload`]: TmfEvent::payload
    pub struct TmfEvent {
        /// Server-assigned identifier of the event resource.
        id: String,
        /// Canonical URI of the event resource.
        href: String,
        /// Identifier of this event.
        event_id: String,
        /// The kind of event, e.g. `ProductOfferingCreateEvent`.
        event_type: String,
        /// When the event occurred.
        event_time: Timestamp,
        /// When the event was raised, where the API distinguishes the two.
        time_occurred: Timestamp,
        /// Correlation identifier, for tracing a chain of events.
        correlation_id: String,
        /// The domain the event belongs to.
        domain: String,
        /// Short title.
        title: String,
        /// Narrative description.
        description: String,
        /// Priority assigned by the emitter.
        priority: String,
        /// The event payload: an object wrapping the affected resource.
        event: Value,
        /// Characteristics carried for analytics.
        analytic_characteristic: Vec<Characteristic>,
        /// Parties related to the event.
        related_party: Vec<RelatedParty>,
        /// The system that raised the event.
        source: Ref<AnyEntity>,
        /// The system that reported the event, where it differs from the source.
        reporting_system: Ref<AnyEntity>,
    }
}

/// What happened to the resource an event is about.
///
/// TM Forum names every notification `{Resource}{Kind}Event`, so the names are
/// derived rather than tabulated and a subscription cannot be misspelled into
/// silence.
///
/// These are exactly the kinds the fourteen vendored specifications declare,
/// checked by `every_declared_listener_is_a_kind_this_crate_names` in
/// `tests/coverage.rs`. Two belong to one API each:
/// [`OperatingStatusChange`](Self::OperatingStatusChange) to TMF638 and
/// [`Batch`](Self::Batch) to TMF637.
///
/// Which spelling a lifecycle move gets is the *collection's* property, not the
/// member's: TMF621 and TMF634 raise `…StatusChangeEvent` where the rest raise
/// `…StateChangeEvent`. See
/// [`state_change_kind`](crate::server::state_change_kind).
///
/// One listener path is not its event name: TMF637 exposes `ProductBatchEvent`
/// at `/listener/productProductBatchEvent`. The class name is what goes in
/// `eventType`, and that is what this produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EventKind {
    /// The resource was created.
    Create,
    /// The resource was deleted.
    Delete,
    /// The resource moved to a new lifecycle state.
    StateChange,
    /// An attribute of the resource changed.
    AttributeValueChange,
    /// Fulfilment is blocked pending information — TMF622.
    InformationRequired,
    /// An error was raised while fulfilling an order — TMF622.
    ErrorMessage,
    /// An order may miss its committed dates — TMF622.
    JeopardyAlert,
    /// An order reached a milestone — TMF622.
    Milestone,
    /// A trouble ticket was resolved — TMF621.
    Resolved,
    /// A resource moved to a new *status* — TMF621 and TMF634.
    ///
    /// The same change as [`EventKind::StateChange`], spelled differently by
    /// those two APIs. See
    /// [`state_change_kind`](crate::server::state_change_kind).
    StatusChange,
    /// A service's `operatingStatus` moved — TMF638.
    ///
    /// TMF638's `Service` is the only resource of the fourteen carrying both an
    /// administrative `state` and an operational `operatingStatus`, with a
    /// listener for each.
    OperatingStatusChange,
    /// Several resources changed at once — TMF637.
    ///
    /// A `ProductBatchEvent` payload carries an *array*, so
    /// [`TmfEvent::resource`] does not apply; read it with
    /// [`TmfEvent::payload::<Vec<_>>`](TmfEvent::payload).
    Batch,
}

impl EventKind {
    /// The suffix this kind contributes to an event class name.
    #[must_use]
    pub fn suffix(self) -> &'static str {
        match self {
            Self::Create => "CreateEvent",
            Self::Delete => "DeleteEvent",
            Self::StateChange => "StateChangeEvent",
            Self::AttributeValueChange => "AttributeValueChangeEvent",
            Self::InformationRequired => "InformationRequiredEvent",
            Self::ErrorMessage => "ErrorMessageEvent",
            Self::JeopardyAlert => "JeopardyAlertEvent",
            Self::Milestone => "MilestoneEvent",
            Self::Resolved => "ResolvedEvent",
            Self::StatusChange => "StatusChangeEvent",
            Self::OperatingStatusChange => "OperatingStatusChangeEvent",
            Self::Batch => "BatchEvent",
        }
    }

    /// Every kind, for iterating a resource's notifications.
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[
            Self::Create,
            Self::Delete,
            Self::StateChange,
            Self::AttributeValueChange,
            Self::InformationRequired,
            Self::ErrorMessage,
            Self::JeopardyAlert,
            Self::Milestone,
            Self::Resolved,
            Self::StatusChange,
            Self::OperatingStatusChange,
            Self::Batch,
        ]
    }

    /// The event class name for this kind of change to `T`.
    ///
    /// ```
    /// use rutmf::core::EventKind;
    /// use rutmf::product::ProductOffering;
    ///
    /// assert_eq!(
    ///     EventKind::Create.name_for::<ProductOffering>(),
    ///     "ProductOfferingCreateEvent",
    /// );
    /// ```
    #[must_use]
    pub fn name_for<T: TmfType>(self) -> String {
        format!("{}{}", T::TYPE_NAME, self.suffix())
    }

    /// The kind an event class name denotes, if this crate knows it.
    ///
    /// Matched **longest suffix first**, because the names nest:
    /// `OperatingStatusChangeEvent` also ends in `StatusChangeEvent`, and every
    /// one ends in `Event`. Shortest-first would read a
    /// `ServiceOperatingStatusChangeEvent` as a plain status change on a
    /// resource called `ServiceOperating`.
    #[must_use]
    pub fn from_event_name(name: &str) -> Option<Self> {
        // `all()` in longest-suffix-first order, which
        // `a_nested_suffix_never_shadows_a_longer_one` is what keeps honest —
        // sorting here would re-sort on every delivered event.
        const BY_LENGTH: &[EventKind] = &[
            EventKind::OperatingStatusChange,
            EventKind::AttributeValueChange,
            EventKind::InformationRequired,
            EventKind::JeopardyAlert,
            EventKind::StatusChange,
            EventKind::ErrorMessage,
            EventKind::StateChange,
            EventKind::Milestone,
            EventKind::Resolved,
            EventKind::Create,
            EventKind::Delete,
            EventKind::Batch,
        ];
        BY_LENGTH
            .iter()
            .copied()
            .find(|k| name.ends_with(k.suffix()))
    }
}

impl TmfEvent {
    /// What kind of change this event reports, if this crate knows it.
    #[must_use]
    pub fn kind(&self) -> Option<EventKind> {
        EventKind::from_event_name(self.event_type.as_deref().unwrap_or(self.type_name()))
    }

    /// The member of `event` that carries the affected resource.
    ///
    /// Derived from the event class name: a `ProductOfferingCreateEvent` wraps
    /// its resource under `productOffering`. Where that member is absent but
    /// the payload holds exactly one, that one is used instead — which covers
    /// `CancelProductOrderStateChangeEvent`, whose v5 payload member is spelled
    /// `canccelProductOrder`.
    #[must_use]
    pub fn resource_key(&self) -> Option<&str> {
        let payload = self.event.as_ref()?.as_object()?;
        let name = self.event_type.as_deref().unwrap_or(self.type_name());

        if let Some(kind) = EventKind::from_event_name(name) {
            let resource = &name[..name.len() - kind.suffix().len()];
            let mut derived = resource.to_owned();
            if let Some(first) = derived.get_mut(..1) {
                first.make_ascii_lowercase();
            }
            if let Some((key, _)) = payload.get_key_value(&derived) {
                return Some(key);
            }
        }

        match payload.len() {
            1 => payload.keys().next().map(String::as_str),
            _ => None,
        }
    }

    /// Deserialises the resource this event is about.
    ///
    /// The payload member is derived from the event class name, so there is no
    /// string to get wrong:
    ///
    /// ```
    /// use rutmf::core::TmfEvent;
    /// use rutmf::product::ProductOffering;
    ///
    /// let json = r#"{
    ///   "eventId": "e1",
    ///   "eventType": "ProductOfferingCreateEvent",
    ///   "event": {"productOffering": {"id": "7655"}}
    /// }"#;
    /// let event: TmfEvent = serde_json::from_str(json).unwrap();
    ///
    /// let offering: ProductOffering = event.resource().unwrap().unwrap();
    /// assert_eq!(offering.id.as_deref(), Some("7655"));
    /// ```
    ///
    /// # Errors
    ///
    /// Returns the serde error if the payload is present but is not a `T`.
    pub fn resource<T: serde::de::DeserializeOwned>(
        &self,
    ) -> std::result::Result<Option<T>, serde_json::Error> {
        match self.resource_key() {
            Some(key) => self.payload(key),
            None => Ok(None),
        }
    }

    /// Deserialises the resource carried under `key` in the event payload.
    ///
    /// Prefer [`resource`](Self::resource), which derives the key. Reach for
    /// this when an event carries more than one member — a TMF622
    /// `…InformationRequiredEvent` wraps both the order and what is needed.
    ///
    /// Returns `Ok(None)` when the payload is absent or carries no such member,
    /// which is what a filtered or notification-only event looks like.
    ///
    /// # Errors
    ///
    /// Returns the serde error if the member is present but is not a `T`.
    pub fn payload<T: serde::de::DeserializeOwned>(
        &self,
        key: &str,
    ) -> std::result::Result<Option<T>, serde_json::Error> {
        self.event
            .as_ref()
            .and_then(|event| event.get(key))
            .cloned()
            .map(serde_json::from_value)
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_payload_is_absent_not_an_error() {
        let event = TmfEvent::builder().event_type("X").build();
        let missing: Option<Value> = event.payload("nope").unwrap();
        assert!(missing.is_none());
    }

    #[test]
    fn the_payload_member_is_derived_from_the_event_name() {
        let event: TmfEvent = serde_json::from_str(
            r#"{"eventType":"ProductOfferingCreateEvent","event":{"productOffering":{"id":"1"}}}"#,
        )
        .unwrap();
        assert_eq!(event.kind(), Some(EventKind::Create));
        assert_eq!(event.resource_key(), Some("productOffering"));
    }

    /// `from_event_name` tries a hand-written order, so this is what stops a
    /// new kind being appended where a shorter suffix it ends with would shadow
    /// it — `OperatingStatusChangeEvent` behind `StatusChangeEvent`, say, which
    /// would silently never match and would report the resource as
    /// `ServiceOperating`.
    #[test]
    fn a_nested_suffix_never_shadows_a_longer_one() {
        for kind in EventKind::all() {
            assert_eq!(
                EventKind::from_event_name(&kind.name_for::<TmfEvent>()),
                Some(*kind),
                "{} is shadowed by a shorter suffix it ends with",
                kind.suffix()
            );
        }

        // The nesting this exists to catch, spelled out: two real v5 event
        // classes whose names end in another kind's whole suffix.
        assert_eq!(
            EventKind::from_event_name("ServiceOperatingStatusChangeEvent"),
            Some(EventKind::OperatingStatusChange)
        );
        assert_eq!(
            EventKind::from_event_name("TroubleTicketStatusChangeEvent"),
            Some(EventKind::StatusChange)
        );
    }

    #[test]
    fn the_two_single_api_kinds_name_what_their_specifications_declare() {
        // TMF638 is the only resource separating an operational move from an
        // administrative one, and TMF637 the only one batching.
        assert_eq!(
            EventKind::OperatingStatusChange.suffix(),
            "OperatingStatusChangeEvent"
        );
        assert_eq!(EventKind::Batch.suffix(), "BatchEvent");

        // A batch payload is an array, so `resource()` is the wrong reader for
        // it and `payload()` is the right one.
        let event: TmfEvent = serde_json::from_str(
            r#"{"eventType":"ProductBatchEvent","event":{"product":[{"id":"1"},{"id":"2"}]}}"#,
        )
        .unwrap();
        assert_eq!(event.kind(), Some(EventKind::Batch));
        assert_eq!(event.resource_key(), Some("product"));
        let products: Vec<Value> = event.payload("product").unwrap().unwrap();
        assert_eq!(products.len(), 2);
    }

    #[test]
    fn the_longest_matching_suffix_wins() {
        // `AttributeValueChangeEvent` also ends in `ChangeEvent` and `Event`.
        assert_eq!(
            EventKind::from_event_name("CategoryAttributeValueChangeEvent"),
            Some(EventKind::AttributeValueChange)
        );
        assert_eq!(
            EventKind::from_event_name("CategoryStateChangeEvent"),
            Some(EventKind::StateChange)
        );
        assert_eq!(EventKind::from_event_name("SomethingElse"), None);
    }

    #[test]
    fn a_single_member_payload_survives_a_misspelled_member() {
        // TMF622 v5 spells this payload member `canccelProductOrder`.
        let event: TmfEvent = serde_json::from_str(
            r#"{"eventType":"CancelProductOrderStateChangeEvent","event":{"canccelProductOrder":{"id":"1"}}}"#,
        )
        .unwrap();
        assert_eq!(event.resource_key(), Some("canccelProductOrder"));
    }

    #[test]
    fn an_unrecognised_event_with_several_members_has_no_single_resource() {
        let event: TmfEvent =
            serde_json::from_str(r#"{"eventType":"VendorThing","event":{"a":{},"b":{}}}"#).unwrap();
        assert_eq!(event.resource_key(), None);
        assert_eq!(event.resource::<Value>().unwrap(), None);
    }

    #[test]
    fn an_event_round_trips_with_its_payload_intact() {
        let json = r#"{"eventId":"e1","eventType":"ProductOfferingCreateEvent","event":{"productOffering":{"id":"7655"}},"@type":"ProductOfferingCreateEvent"}"#;
        let event: TmfEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.type_name(), "ProductOfferingCreateEvent");
        assert_eq!(
            serde_json::to_value(&event).unwrap(),
            serde_json::from_str::<Value>(json).unwrap()
        );
    }
}
