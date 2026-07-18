use std::path::Path;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuredError {
    summary: &'static str,
    technical_details: &'static str,
    affected_path: Option<String>,
}

impl StructuredError {
    /// Both labels are static so provider credentials cannot enter diagnostics by interpolation.
    pub fn new(summary: &'static str, technical_details: &'static str) -> Self {
        Self {
            summary,
            technical_details,
            affected_path: None,
        }
    }

    pub fn at_path(mut self, path: &Path) -> Self {
        self.affected_path = path.to_str().map(ToOwned::to_owned);
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DiagnosticLog {
    errors: Vec<StructuredError>,
}

impl DiagnosticLog {
    pub fn record(&mut self, error: StructuredError) {
        self.errors.push(error);
    }

    pub fn report(&self) -> DiagnosticReport {
        DiagnosticReport {
            application_version: env!("CARGO_PKG_VERSION").to_owned(),
            operating_system: std::env::consts::OS.to_owned(),
            errors: self.errors.clone(),
        }
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
        if self.errors.is_empty() {
            report.push_str("\nNo errors have been recorded in this session.");
        }
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
    use super::{DiagnosticLog, DiagnosticReport, StructuredError};
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

    #[test]
    fn empty_log_reports_a_session_without_errors() {
        assert!(
            DiagnosticLog::default()
                .report()
                .redacted_text(false)
                .contains("No errors have been recorded")
        );
    }
}
