//! `TroubleTicket` and its specification — TMF621.

use serde::{Deserialize, Serialize};

use crate::core::macros::{tmf_entity, tmf_patch_body, tmf_struct};
use crate::core::{
    AnyEntity, Attachment, Channel, Characteristic, CharacteristicSpecification,
    ExternalIdentifier, Note, Ref, RelatedParty, TimePeriod, Timestamp,
};

/// Where a [`TroubleTicket`] has got to.
///
/// [`TroubleTicketStatus::Other`] preserves a value outside the v5 enumeration
/// rather than failing the parse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum TroubleTicketStatus {
    /// Received and validated.
    #[serde(rename = "acknowledged")]
    Acknowledged,
    /// Refused at validation.
    #[serde(rename = "rejected")]
    Rejected,
    /// Waiting on something before work can start.
    #[serde(rename = "pending")]
    Pending,
    /// Deliberately paused.
    #[serde(rename = "held")]
    Held,
    /// Being worked on.
    #[serde(rename = "inProgress")]
    InProgress,
    /// Withdrawn before resolution.
    #[serde(rename = "cancelled")]
    Cancelled,
    /// Closed out; no further work will happen.
    #[serde(rename = "closed")]
    Closed,
    /// The reported trouble has been fixed.
    #[serde(rename = "resolved")]
    Resolved,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

impl TroubleTicketStatus {
    /// Whether the ticket has stopped moving.
    ///
    /// An unknown status is reported as **not** terminal: a client that polls
    /// until a ticket finishes should keep polling rather than stop on a state
    /// this crate does not recognise.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Closed | Self::Cancelled | Self::Rejected)
    }
}

tmf_struct! {
    @name = "TroubleTicket", @ref = "TroubleTicketRef";
    /// A reported problem, and the record of what is being done about it.
    ///
    /// ```
    /// use rutmf::ticket::{TroubleTicket, TroubleTicketStatus};
    ///
    /// let json = r#"{"@type":"TroubleTicket","status":"inProgress"}"#;
    /// let ticket: TroubleTicket = serde_json::from_str(json).unwrap();
    ///
    /// assert_eq!(ticket.status, Some(TroubleTicketStatus::InProgress));
    /// assert!(!ticket.status.unwrap().is_terminal());
    /// ```
    pub struct TroubleTicket {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this ticket.
        href: String,
        /// Short name for the ticket.
        name: String,
        /// What the trouble is.
        description: String,
        /// The kind of ticket, as the provider classifies it.
        ticket_type: String,
        /// How badly the trouble affects the customer.
        severity: String,
        /// How urgently it should be worked.
        priority: String,
        /// Where the ticket has got to.
        status: TroubleTicketStatus,
        /// When the status last changed.
        status_change_date: Timestamp,
        /// Why the status last changed.
        status_change_reason: String,
        /// Every status this ticket has passed through.
        status_change_history: Vec<StatusChange>,
        /// When the ticket was raised.
        creation_date: Timestamp,
        /// When the ticket was last changed.
        last_update: Timestamp,
        /// When the provider expects to resolve it.
        expected_resolution_date: Timestamp,
        /// When the requester asked for it to be resolved.
        requested_resolution_date: Timestamp,
        /// When it was actually resolved.
        resolution_date: Timestamp,
        /// The channel the ticket arrived through.
        channel: Ref<Channel>,
        /// Files supporting the report — logs, screenshots, photographs.
        attachment: Vec<Attachment>,
        /// Identifiers this ticket is known by in other systems.
        external_identifier: Vec<ExternalIdentifier>,
        /// Free-text notes added as the ticket is worked.
        note: Vec<Note>,
        /// What the trouble is about — a product, a service, a resource.
        related_entity: Vec<RelatedEntity>,
        /// Parties involved: who reported it, who owns it.
        related_party: Vec<RelatedParty>,
        /// Provider-defined attributes of this ticket.
        trouble_ticket_characteristic: Vec<Characteristic>,
        /// Links to other tickets — duplicates, dependencies, parents.
        trouble_ticket_relationship: Vec<TroubleTicketRelationship>,
        /// The specification this ticket was raised against.
        trouble_ticket_specification: Ref<TroubleTicketSpecification>,
        @renamed {
            /// The concrete class a `TroubleTicketRef` points at.
            "@referredType" referred_type: String,
        }
    }
}

tmf_struct! {
    @name = "TroubleTicket";
    /// Body of a `POST /troubleTicket` — the v5 `TroubleTicket_FVO`.
    pub struct TroubleTicketCreate {
        @required {
            /// What the trouble is. **Required on create.**
            description: String,
            /// How badly it affects the customer. **Required on create.**
            severity: String,
            /// The kind of ticket. **Required on create.**
            ticket_type: String,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Canonical URI, where the client supplies one.
        href: String,
        /// Short name for the ticket.
        name: String,
        /// How urgently it should be worked.
        priority: String,
        /// The status to open the ticket in, where the provider allows a choice.
        status: TroubleTicketStatus,
        /// When the status last changed.
        status_change_date: Timestamp,
        /// Why the status last changed.
        status_change_reason: String,
        /// When the provider expects to resolve it.
        expected_resolution_date: Timestamp,
        /// When the requester asks for it to be resolved.
        requested_resolution_date: Timestamp,
        /// When it was resolved.
        resolution_date: Timestamp,
        /// The channel the ticket arrived through.
        channel: Ref<Channel>,
        /// Files supporting the report.
        attachment: Vec<Attachment>,
        /// Identifiers this ticket is known by in other systems.
        external_identifier: Vec<ExternalIdentifier>,
        /// Free-text notes.
        note: Vec<Note>,
        /// What the trouble is about.
        related_entity: Vec<RelatedEntity>,
        /// Parties involved.
        related_party: Vec<RelatedParty>,
        /// Provider-defined attributes.
        trouble_ticket_characteristic: Vec<Characteristic>,
        /// Links to other tickets.
        trouble_ticket_relationship: Vec<TroubleTicketRelationship>,
        /// The specification to raise this ticket against.
        trouble_ticket_specification: Ref<TroubleTicketSpecification>,
    }
}

tmf_struct! {
    @name = "TroubleTicket";
    /// Body of a `PATCH /troubleTicket/{id}` — the v5 `TroubleTicket_MVO`.
    ///
    /// Unusually, this carries `id` and `href`. Every other `_MVO` in this
    /// crate drops the server-owned members; TMF621's declares them, and the
    /// model follows the schema rather than the house style.
    pub struct TroubleTicketUpdate {
        /// Identifier, which TMF621 leaves on the patch body.
        id: String,
        /// Canonical URI, likewise.
        href: String,
        /// New name.
        name: String,
        /// New description.
        description: String,
        /// New ticket type.
        ticket_type: String,
        /// New severity.
        severity: String,
        /// New priority.
        priority: String,
        /// New status.
        status: TroubleTicketStatus,
        /// Why the status changed.
        status_change_reason: String,
        /// Replacement status history.
        status_change_history: Vec<StatusChange>,
        /// New expected resolution date.
        expected_resolution_date: Timestamp,
        /// New requested resolution date.
        requested_resolution_date: Timestamp,
        /// New resolution date.
        resolution_date: Timestamp,
        /// New channel.
        channel: Ref<Channel>,
        /// Replacement attachments.
        attachment: Vec<Attachment>,
        /// Replacement external identifiers.
        external_identifier: Vec<ExternalIdentifier>,
        /// Replacement notes.
        note: Vec<Note>,
        /// Replacement related entities.
        related_entity: Vec<RelatedEntity>,
        /// Replacement related parties.
        related_party: Vec<RelatedParty>,
        /// Replacement characteristics.
        trouble_ticket_characteristic: Vec<Characteristic>,
        /// Replacement relationships.
        trouble_ticket_relationship: Vec<TroubleTicketRelationship>,
        /// New specification reference.
        trouble_ticket_specification: Ref<TroubleTicketSpecification>,
    }
}

tmf_struct! {
    @name = "TroubleTicketSpecification", @ref = "TroubleTicketSpecificationRef";
    /// The template a ticket is raised against: what a class of trouble means.
    pub struct TroubleTicketSpecification {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this specification.
        href: String,
        /// Name of the specification.
        name: String,
        /// Narrative description.
        description: String,
        /// Version of the specification.
        version: String,
        /// When it was created.
        creation_date: Timestamp,
        /// When it was last changed.
        last_update: Timestamp,
        /// Lifecycle status.
        lifecycle_status: String,
        /// Period during which the specification is valid.
        valid_for: TimePeriod,
        /// Parties related to the specification.
        related_party: Vec<RelatedParty>,
        /// Characteristics a ticket of this class carries.
        spec_characteristic: Vec<CharacteristicSpecification>,
        @renamed {
            /// The concrete class a `TroubleTicketSpecificationRef` points at.
            "@referredType" referred_type: String,
        }
    }
}

tmf_struct! {
    @name = "TroubleTicketSpecification";
    /// Body of a `POST /troubleTicketSpecification` — the v5 `_FVO` schema.
    pub struct TroubleTicketSpecificationCreate {
        @required {
            /// Name of the specification. **Required on create.**
            name: String,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Canonical URI, where the client supplies one.
        href: String,
        /// Narrative description.
        description: String,
        /// Version of the specification.
        version: String,
        /// Lifecycle status.
        lifecycle_status: String,
        /// Period during which the specification is valid.
        valid_for: TimePeriod,
        /// Parties related to the specification.
        related_party: Vec<RelatedParty>,
        /// Characteristics a ticket of this class carries.
        spec_characteristic: Vec<CharacteristicSpecification>,
    }
}

tmf_struct! {
    @name = "TroubleTicketSpecification";
    /// Body of a `PATCH /troubleTicketSpecification/{id}` — the v5 `_MVO`.
    pub struct TroubleTicketSpecificationUpdate {
        /// Identifier, which TMF621 leaves on the patch body.
        id: String,
        /// Canonical URI, likewise.
        href: String,
        /// New name.
        name: String,
        /// New description.
        description: String,
        /// New version.
        version: String,
        /// New lifecycle status.
        lifecycle_status: String,
        /// New validity period.
        valid_for: TimePeriod,
        /// Replacement related parties.
        related_party: Vec<RelatedParty>,
        /// Replacement characteristics.
        spec_characteristic: Vec<CharacteristicSpecification>,
    }
}

tmf_struct! {
    @name = "TroubleTicketRelationship";
    /// A typed link from one trouble ticket to another.
    pub struct TroubleTicketRelationship {
        /// Identifier of the referenced ticket.
        id: String,
        /// URI of the referenced ticket.
        href: String,
        /// Name of the referenced ticket.
        name: String,
        /// What kind of link this is, e.g. `duplicate`, `dependent`.
        relationship_type: String,
        @renamed {
            /// The concrete class of the referenced ticket.
            "@referredType" referred_type: String,
        }
    }
}

tmf_struct! {
    @name = "RelatedEntity";
    /// Whatever the trouble is about, in a named role.
    ///
    /// Deliberately untyped in the specification: a ticket may be raised
    /// against a product, a service, a resource or something this crate has no
    /// model for, so the target is an `EntityRef` rather than a typed
    /// reference.
    pub struct RelatedEntity {
        /// The entity the ticket concerns.
        entity: Ref<AnyEntity>,
        /// The role it plays — what it is to this ticket.
        role: String,
    }
}

tmf_struct! {
    @name = "StatusChange";
    /// One step in a ticket's status history.
    pub struct StatusChange {
        /// The status moved to.
        status: TroubleTicketStatus,
        /// When the move happened.
        status_change_date: Timestamp,
        /// Why it happened.
        status_change_reason: String,
    }
}

tmf_entity!(TroubleTicket, TroubleTicketSpecification);
tmf_patch_body!(TroubleTicketUpdate, TroubleTicketSpecificationUpdate);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unknown_status_is_preserved_and_not_terminal() {
        let json = r#"{"@type":"TroubleTicket","status":"awaitingFieldVisit"}"#;
        let ticket: TroubleTicket = serde_json::from_str(json).unwrap();

        assert_eq!(
            ticket.status,
            Some(TroubleTicketStatus::Other("awaitingFieldVisit".into()))
        );
        assert!(
            !ticket.status.as_ref().unwrap().is_terminal(),
            "a poller must not stop on a status it does not recognise"
        );
        assert_eq!(
            serde_json::to_value(&ticket).unwrap(),
            serde_json::from_str::<serde_json::Value>(json).unwrap()
        );
    }

    #[test]
    fn resolved_is_not_terminal_but_closed_is() {
        // A resolved ticket can still be reopened or disputed; a closed one is
        // the end of the road. Conflating them would stop a poller too early.
        assert!(!TroubleTicketStatus::Resolved.is_terminal());
        assert!(TroubleTicketStatus::Closed.is_terminal());
        assert!(TroubleTicketStatus::Cancelled.is_terminal());
    }

    #[test]
    fn the_patch_body_keeps_the_members_tmf621_leaves_on_it() {
        // Unlike every other `_MVO` in this crate.
        let patch = TroubleTicketUpdate::builder()
            .id("42")
            .status(TroubleTicketStatus::Resolved)
            .build();
        let json = serde_json::to_value(&patch).unwrap();
        assert_eq!(json["id"], "42");
        assert_eq!(json["status"], "resolved");
    }
}
