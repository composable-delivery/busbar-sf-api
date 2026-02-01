use tracing::instrument;

use crate::error::Result;
use crate::types::ApexCodeCoverageAggregate;

impl super::ToolingClient {
    /// Get code coverage for all Apex classes and triggers.
    #[instrument(skip(self))]
    pub async fn get_code_coverage(&self) -> Result<Vec<ApexCodeCoverageAggregate>> {
        self.query_all(
            "SELECT Id, ApexClassOrTriggerId, ApexClassOrTrigger.Name, NumLinesCovered, NumLinesUncovered, Coverage FROM ApexCodeCoverageAggregate"
        ).await
    }

    /// Get overall org-wide code coverage percentage.
    #[instrument(skip(self))]
    pub async fn get_org_wide_coverage(&self) -> Result<f64> {
        let coverage = self.get_code_coverage().await?;

        let mut total_covered = 0i64;
        let mut total_uncovered = 0i64;

        for item in coverage {
            total_covered += item.num_lines_covered as i64;
            total_uncovered += item.num_lines_uncovered as i64;
        }

        let total_lines = total_covered + total_uncovered;
        if total_lines == 0 {
            return Ok(0.0);
        }

        Ok((total_covered as f64 / total_lines as f64) * 100.0)
    }
}
