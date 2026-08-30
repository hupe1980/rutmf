//! Bulk import and export jobs — the asynchronous side of TMF620.
//!
//! Catalog data is rarely moved one offering at a time. TMF620 v5 exposes
//! `importJob` and `exportJob` collections: you `POST` a job pointing at a file
//! URL, then poll it until [`JobState::is_finished`].

use serde::{Deserialize, Serialize};

use crate::core::Timestamp;
use crate::core::macros::{tmf_entity, tmf_struct};

/// The lifecycle state of an import or export job.
///
/// The v5 `JobStateType` enumeration, with [`JobState::Other`] preserving any
/// value outside it rather than failing the parse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum JobState {
    /// Accepted but not yet begun.
    #[serde(rename = "Not Started")]
    NotStarted,
    /// In progress.
    #[serde(rename = "Running")]
    Running,
    /// Finished successfully.
    #[serde(rename = "Succeeded")]
    Succeeded,
    /// Finished with an error; see `error_log`.
    #[serde(rename = "Failed")]
    Failed,
    /// A value outside the v5 enumeration, preserved verbatim.
    #[serde(untagged)]
    Other(String),
}

impl JobState {
    /// Whether the job has stopped, successfully or not.
    ///
    /// An unrecognised state counts as *not* finished: polling a job the server
    /// describes in its own terms is safer than declaring it done.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed)
    }

    /// Whether the job finished successfully.
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Succeeded)
    }
}

tmf_struct! {
    @name = "ImportJob";
    /// A job loading catalog data from a file into the catalog.
    pub struct ImportJob {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this job.
        href: String,
        /// URL of the file to read from.
        url: String,
        /// Path within the file, where the format supports one.
        path: String,
        /// MIME type of the file.
        content_type: String,
        /// Current state of the job.
        status: JobState,
        /// When the job was created.
        creation_date: Timestamp,
        /// When the job finished.
        completion_date: Timestamp,
        /// Why the job failed, when it did.
        error_log: String,
    }
}

tmf_struct! {
    @name = "ImportJob";
    /// Body of a `POST /importJob` — the v5 `ImportJob_FVO`.
    ///
    /// `url` is the only required member; the state members exist on the create
    /// schema too, but a server owns them.
    pub struct ImportJobCreate {
        @required {
            /// URL of the file to read from. **Required on create.**
            url: String,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Path within the file, where the format supports one.
        path: String,
        /// MIME type of the file.
        content_type: String,
        /// Initial state, where a server accepts one.
        status: JobState,
        /// Creation timestamp, where a server accepts one.
        creation_date: Timestamp,
        /// Completion timestamp, where a server accepts one.
        completion_date: Timestamp,
        /// Error log, where a server accepts one.
        error_log: String,
    }
}

tmf_struct! {
    @name = "ExportJob";
    /// A job writing catalog data out to a file.
    pub struct ExportJob {
        /// Server-assigned identifier.
        id: String,
        /// Canonical URI of this job.
        href: String,
        /// URL of the file to write to.
        url: String,
        /// Path within the file, where the format supports one.
        path: String,
        /// MIME type of the file.
        content_type: String,
        /// TMF630 filter selecting which resources to export.
        query: String,
        /// Current state of the job.
        status: JobState,
        /// When the job was created.
        creation_date: Timestamp,
        /// When the job finished.
        completion_date: Timestamp,
        /// Why the job failed, when it did.
        error_log: String,
    }
}

tmf_struct! {
    @name = "ExportJob";
    /// Body of a `POST /exportJob` — the v5 `ExportJob_FVO`.
    pub struct ExportJobCreate {
        @required {
            /// URL of the file to write to. **Required on create.**
            url: String,
        }
        /// Client-supplied identifier, where the server permits one.
        id: String,
        /// Path within the file, where the format supports one.
        path: String,
        /// MIME type of the file.
        content_type: String,
        /// TMF630 filter selecting which resources to export.
        query: String,
        /// Initial state, where a server accepts one.
        status: JobState,
        /// Creation timestamp, where a server accepts one.
        creation_date: Timestamp,
        /// Completion timestamp, where a server accepts one.
        completion_date: Timestamp,
        /// Error log, where a server accepts one.
        error_log: String,
    }
}

tmf_entity!(ImportJob, ExportJob);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_state_round_trips_spec_spellings_with_spaces() {
        let job: ImportJob = serde_json::from_str(r#"{"status":"Not Started"}"#).unwrap();
        assert_eq!(job.status, Some(JobState::NotStarted));
        assert!(!job.status.as_ref().unwrap().is_finished());
        assert_eq!(serde_json::to_value(&job).unwrap()["status"], "Not Started");
    }

    #[test]
    fn unknown_job_state_is_preserved_and_not_finished() {
        let job: ImportJob = serde_json::from_str(r#"{"status":"Paused"}"#).unwrap();
        assert_eq!(job.status, Some(JobState::Other("Paused".into())));
        assert!(!job.status.as_ref().unwrap().is_finished());
    }

    #[test]
    fn export_job_carries_a_query_but_import_does_not() {
        let export = ExportJobCreate::builder()
            .url("https://files/out.json")
            .query("lifecycleStatus=Active")
            .build();
        assert_eq!(export.query.as_deref(), Some("lifecycleStatus=Active"));
    }
}
