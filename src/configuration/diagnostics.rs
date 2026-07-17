use std::path::Path;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuredError {
    summary: String,
    technical_details: &'static str,
    affected_path: Option<String>,
}

impl StructuredError {
    /// Technical details are static diagnostic labels, so credentials cannot be interpolated.
    pub fn new(summary: impl Into<String>, technical_details: &'static str) -> Self {
        Self {
            summary: summary.into(),
            technical_details,
            affected_path: None,
        }
    }

    pub fn at_path(mut self, path: &Path) -> Self {
        self.affected_path = path.to_str().map(ToOwned::to_owned);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticReport {
    pub application_version: String,
    pub operating_system: String,
    pub errors: Vec<StructuredError>,
}

impl DiagnosticReport {
    pub fn redacted_text(&self, include_paths: bool) -> String {
        let mut report = format!(
            "SyncPak {}\nOperating system: {}",
            self.application_version, self.operating_system
        );
        for error in &self.errors {
            report.push_str(&format!(
                "\nError: {}\nTechnical details: {}",
                error.summary, error.technical_details
            ));
            if include_paths && let Some(path) = &error.affected_path {
                report.push_str(&format!("\nPath: {path}"));
            }
        }
        report
    }
}

#[cfg(test)]
mod tests {
    use super::{DiagnosticReport, StructuredError};
    use std::path::Path;

    #[test]
    fn paths_are_excluded_unless_requested() {
        let report = DiagnosticReport {
            application_version: "0.1.0".to_owned(),
            operating_system: "test".to_owned(),
            errors: vec![
                StructuredError::new("Could not save", "permission denied")
                    .at_path(Path::new("/private/file")),
            ],
        };
        assert!(!report.redacted_text(false).contains("/private/file"));
        assert!(report.redacted_text(true).contains("/private/file"));
    }
}
