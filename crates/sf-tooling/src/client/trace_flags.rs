use tracing::instrument;

use crate::error::Result;
use crate::types::*;

impl super::ToolingClient {
    /// Get all active trace flags.
    #[instrument(skip(self))]
    pub async fn get_trace_flags(&self) -> Result<Vec<TraceFlag>> {
        self.query_all(
            "SELECT Id, TracedEntityId, LogType, DebugLevelId, StartDate, ExpirationDate FROM TraceFlag"
        ).await
    }

    /// Get all debug levels.
    #[instrument(skip(self))]
    pub async fn get_debug_levels(&self) -> Result<Vec<DebugLevel>> {
        self.query_all(
            "SELECT Id, DeveloperName, MasterLabel, ApexCode, ApexProfiling, Callout, Database, System, Validation, Visualforce, Workflow FROM DebugLevel"
        ).await
    }
}
