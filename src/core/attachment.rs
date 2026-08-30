//! Attachments and external identifiers, shared across domains.

use super::macros::tmf_struct;
use super::value::{Quantity, TimePeriod};

tmf_struct! {
    @name = "Attachment", @ref = "AttachmentRef";
    /// A file or document attached to an entity, either inline or by URL.
    ///
    /// The v5 schemas type this member as `AttachmentRefOrValue`, a `oneOf`
    /// over the full attachment and a bare reference to one. Both arms are
    /// structurally a subset of this type, so one Rust struct covers them:
    /// `id`/`href` alone is the reference form.
    ///
    /// `attachmentType` and `mimeType` are required by the *create* schema and
    /// optional on the base — nested types follow the base, so both are
    /// [`Option`] here. See [`crate::core`] for why that rule exists.
    pub struct Attachment {
        /// Unique identifier of the attachment.
        id: String,
        /// URI of the attachment.
        href: String,
        /// Friendly name.
        name: String,
        /// Narrative describing the attachment.
        description: String,
        /// Kind of attachment, e.g. `video`, `picture`.
        attachment_type: String,
        /// The attachment MIME type.
        mime_type: String,
        /// URL where the attachment can be retrieved.
        url: String,
        /// Inline base64-encoded content.
        content: String,
        /// Size of the attachment.
        size: Quantity,
        /// Period during which the attachment is valid.
        valid_for: TimePeriod,
        @renamed {
            /// The concrete class of the attachment, when this is the reference
            /// form: `AttachmentRefOrValue` is a `oneOf` over the whole
            /// attachment and a bare reference to one, and only the reference
            /// carries a `@referredType`.
            "@referredType" referred_type: String,
        }
    }
}

tmf_struct! {
    @name = "ExternalIdentifier";
    /// An identifier for the entity in an external system.
    pub struct ExternalIdentifier {
        /// The identifier in the external system.
        id: String,
        /// The system that owns the identifier.
        owner: String,
        /// The kind of external identifier.
        external_identifier_type: String,
    }
}
