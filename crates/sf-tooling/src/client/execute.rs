use tracing::instrument;

use crate::error::{Error, ErrorKind, Result};
use crate::types::ExecuteAnonymousResult;

impl super::ToolingClient {
    /// Execute anonymous Apex code.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let result = client.execute_anonymous("System.debug('Hello World');").await?;
    /// if result.success {
    ///     println!("Execution successful");
    /// } else if let Some(err) = result.compile_problem {
    ///     println!("Compilation error: {}", err);
    /// }
    /// ```
    #[instrument(skip(self))]
    pub async fn execute_anonymous(&self, apex_code: &str) -> Result<ExecuteAnonymousResult> {
        let encoded = urlencoding::encode(apex_code);
        let url = format!(
            "{}/services/data/v{}/tooling/executeAnonymous/?anonymousBody={}",
            self.client.instance_url(),
            self.client.api_version(),
            encoded
        );

        let result: ExecuteAnonymousResult = self.client.get_json(&url).await?;

        // Check for compilation or execution errors
        if !result.compiled {
            if let Some(ref problem) = result.compile_problem {
                return Err(Error::new(ErrorKind::ApexCompilation(problem.clone())));
            }
        }

        if !result.success {
            if let Some(ref message) = result.exception_message {
                return Err(Error::new(ErrorKind::ApexExecution(message.clone())));
            }
        }

        Ok(result)
    }
}
