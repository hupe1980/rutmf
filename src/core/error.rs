//! The TMF630 error representation.

use super::macros::tmf_struct;

tmf_struct! {
    @name = "Error";
    /// The error body TM Forum APIs return with a 4xx or 5xx response.
    ///
    /// TMF630 marks `code` and `reason` mandatory. They are [`Option`] here
    /// anyway, because this is a payload the client *receives*: a gateway that
    /// truncates the body should cost you the missing member, not the whole
    /// error. See [`crate::core::macros`] for the rule.
    ///
    /// ```
    /// use rutmf::core::TmfError;
    ///
    /// let e: TmfError = serde_json::from_str(
    ///     r#"{"code":"40001","reason":"Missing name","status":"400","@type":"Error"}"#,
    /// ).unwrap();
    /// assert_eq!(e.http_status(), Some(400));
    /// ```
    pub struct TmfError {
        /// Application-relevant detail, defined in the API or a common list.
        code: String,
        /// Explanation of the reason for the error, safe to show to a user.
        reason: String,
        /// Further detail and corrective actions.
        message: String,
        /// The HTTP status code, carried in the body as a string.
        status: String,
        /// URI of documentation describing the error.
        reference_error: String,
    }
}

impl TmfError {
    /// The `status` member parsed as an HTTP status code.
    #[must_use]
    pub fn http_status(&self) -> Option<u16> {
        self.status.as_ref()?.parse().ok()
    }

    /// Whether this looks like a TMF630 error body rather than an arbitrary
    /// JSON object that happened to deserialize.
    ///
    /// Every member is optional, so *any* JSON object parses into this type.
    /// The client layer uses this to decide whether a failing response carried
    /// a usable error body or should be reported as a raw status.
    #[must_use]
    pub fn is_populated(&self) -> bool {
        self.code.is_some() || self.reason.is_some()
    }
}

impl std::fmt::Display for TmfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (&self.code, &self.reason) {
            (Some(code), Some(reason)) => write!(f, "{code}: {reason}")?,
            (Some(code), None) => write!(f, "{code}")?,
            (None, Some(reason)) => write!(f, "{reason}")?,
            (None, None) => f.write_str("unspecified error")?,
        }
        if let Some(message) = &self.message {
            write!(f, " ({message})")?;
        }
        Ok(())
    }
}

impl std::error::Error for TmfError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_body_without_the_mandatory_type_still_parses() {
        let json = r#"{"code":"40001","reason":"nope"}"#;
        let e: TmfError = serde_json::from_str(json).unwrap();

        assert!(e.is_populated());
        assert_eq!(e.type_name(), "Error", "the class is known regardless");
        assert_eq!(
            serde_json::to_string(&e).unwrap(),
            json,
            "and relaying the body does not add a member to it"
        );
    }

    #[test]
    fn an_unrelated_json_object_is_not_a_populated_error() {
        let e: TmfError = serde_json::from_str(r#"{"detail":"gateway timeout"}"#).unwrap();
        assert!(!e.is_populated());
    }

    #[test]
    fn unknown_members_survive() {
        let json = r#"{"code":"40001","reason":"r","@type":"Error","x-trace":"abc"}"#;
        let e: TmfError = serde_json::from_str(json).unwrap();
        assert_eq!(e.extensions.get("x-trace").unwrap(), "abc");
        assert_eq!(
            serde_json::to_value(&e).unwrap(),
            serde_json::from_str::<serde_json::Value>(json).unwrap()
        );
    }
}
