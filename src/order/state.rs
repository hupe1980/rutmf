//! Order lifecycle states.
//!
//! TMF622 defines three overlapping state enumerations. Modelling them as
//! distinct Rust types stops an item state being assigned to an order, and
//! stops a client asking a server to create an order in a state that only the
//! server can reach.

use serde::{Deserialize, Serialize};

/// The state of a whole [`ProductOrder`](super::ProductOrder).
///
/// [`ProductOrderState::Other`] preserves a value outside the v5 enumeration
/// rather than failing the parse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum ProductOrderState {
    /// Being prepared by the buyer; not yet submitted.
    #[serde(rename = "draft")]
    Draft,
    /// Received and validated by the provider.
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
    /// Being fulfilled.
    #[serde(rename = "inProgress")]
    InProgress,
    /// Fulfilment accepted, a sub-state of `inProgress`.
    #[serde(rename = "inProgress.accepted")]
    InProgressAccepted,
    /// A cancellation request is being evaluated.
    #[serde(rename = "assessingCancellation")]
    AssessingCancellation,
    /// Cancellation approved but not yet applied.
    #[serde(rename = "pendingCancellation")]
    PendingCancellation,
    /// Cancelled.
    #[serde(rename = "cancelled")]
    Cancelled,
    /// Some items completed, others did not.
    #[serde(rename = "partial")]
    Partial,
    /// Fulfilled in full.
    #[serde(rename = "completed")]
    Completed,
    /// Fulfilment failed.
    #[serde(rename = "failed")]
    Failed,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

impl ProductOrderState {
    /// Whether the order has reached a state it will not leave.
    ///
    /// An unrecognised state counts as *not* terminal: treating a state you do
    /// not understand as final is how orders get abandoned mid-flight.
    ///
    /// # `partial` is terminal
    ///
    /// TMF622 lists the states without saying which are final, so this is the
    /// crate's reading. `partial` means fulfilment finished with some items done
    /// and others not: the order will not be worked again, and the failed items
    /// are a new order's problem. Excluding it would make
    /// `while !state.is_terminal() { poll().await }` never end.
    ///
    /// ```
    /// use rutmf::order::ProductOrderState;
    ///
    /// let state = ProductOrderState::Partial;
    /// assert!(state.is_terminal(), "the order will not move again");
    /// assert!(!state.is_success(), "but it did not all get done");
    /// ```
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Cancelled | Self::Rejected | Self::Failed | Self::Partial
        )
    }

    /// Whether the order finished with everything asked for delivered.
    ///
    /// Narrower than [`is_terminal`](Self::is_terminal), which four other states
    /// also satisfy — including [`Partial`](Self::Partial), where the order is
    /// over and the outcome is not what was ordered.
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Completed)
    }

    /// Whether a cancellation request is in flight.
    #[must_use]
    pub fn is_cancelling(&self) -> bool {
        matches!(
            self,
            Self::AssessingCancellation | Self::PendingCancellation
        )
    }
}

/// The state a client may ask for when submitting an order.
///
/// TMF622 restricts this to two values — the remaining states are reached by
/// the provider, not requested by the buyer. A separate type makes that
/// unrepresentable rather than merely documented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum InitialProductOrderState {
    /// Submit for processing.
    #[serde(rename = "acknowledged")]
    Acknowledged,
    /// Save without submitting.
    #[serde(rename = "draft")]
    Draft,
}

/// The state of a single [`ProductOrderItem`](super::ProductOrderItem).
///
/// The same values as [`ProductOrderState`] minus the order-only `draft` and
/// `inProgress.accepted`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum ProductOrderItemState {
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
    /// Being fulfilled.
    #[serde(rename = "inProgress")]
    InProgress,
    /// A cancellation request is being evaluated.
    #[serde(rename = "assessingCancellation")]
    AssessingCancellation,
    /// Cancellation approved but not yet applied.
    #[serde(rename = "pendingCancellation")]
    PendingCancellation,
    /// Cancelled.
    #[serde(rename = "cancelled")]
    Cancelled,
    /// Partially fulfilled.
    #[serde(rename = "partial")]
    Partial,
    /// Fulfilled in full.
    #[serde(rename = "completed")]
    Completed,
    /// Fulfilment failed.
    #[serde(rename = "failed")]
    Failed,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

impl ProductOrderItemState {
    /// Whether the item has reached a state it will not leave.
    ///
    /// The same reading as [`ProductOrderState::is_terminal`], including
    /// [`Partial`](Self::Partial).
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Cancelled | Self::Rejected | Self::Failed | Self::Partial
        )
    }

    /// Whether the item finished with everything asked for delivered.
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Completed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dotted_state_keeps_its_spec_spelling() {
        let state: ProductOrderState = serde_json::from_str(r#""inProgress.accepted""#).unwrap();
        assert_eq!(state, ProductOrderState::InProgressAccepted);
        assert_eq!(
            serde_json::to_string(&state).unwrap(),
            r#""inProgress.accepted""#
        );
    }

    #[test]
    fn unknown_state_is_preserved_and_not_terminal() {
        let state: ProductOrderState = serde_json::from_str(r#""awaitingSurvey""#).unwrap();
        assert_eq!(state, ProductOrderState::Other("awaitingSurvey".into()));
        assert!(!state.is_terminal(), "an unknown state must not look final");
        assert_eq!(
            serde_json::to_string(&state).unwrap(),
            r#""awaitingSurvey""#
        );
    }

    #[test]
    fn terminal_and_cancelling_states_are_distinguished() {
        assert!(ProductOrderState::Completed.is_terminal());
        assert!(ProductOrderState::Cancelled.is_terminal());
        assert!(!ProductOrderState::InProgress.is_terminal());
        assert!(ProductOrderState::PendingCancellation.is_cancelling());
        assert!(!ProductOrderState::Cancelled.is_cancelling());
    }

    #[test]
    fn a_partial_order_has_finished_even_though_it_did_not_succeed() {
        // `while !state.is_terminal() { poll() }` is the loop everyone writes.
        // Leaving `partial` out of the terminal set makes it never end.
        for state in [ProductOrderState::Partial, ProductOrderState::Failed] {
            assert!(state.is_terminal(), "{state:?} will not move again");
            assert!(!state.is_success(), "{state:?} is not what was ordered");
        }
        assert!(ProductOrderState::Completed.is_success());

        assert!(ProductOrderItemState::Partial.is_terminal());
        assert!(!ProductOrderItemState::Partial.is_success());
        assert!(!ProductOrderItemState::InProgress.is_terminal());
    }

    #[test]
    fn initial_state_is_restricted_to_what_a_client_may_request() {
        // The v5 InitialProductOrderStateType admits only these two.
        assert_eq!(
            serde_json::to_string(&InitialProductOrderState::Draft).unwrap(),
            r#""draft""#
        );
        assert!(serde_json::from_str::<InitialProductOrderState>(r#""completed""#).is_err());
    }
}
