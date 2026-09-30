//! Test reporting — JUnit XML, HTML, JSON, CSV output.

use super::{RunSummary, TestStatus};
use std::fmt::Write;
use std::path::{Path, PathBuf};

// ===========================================================================
// Report writer trait
// ===========================================================================

pub trait ReportWriter {
    fn write(&self, summary: &RunSummary, path: &Path) -> std::io::Result<()>;
}

// ===========================================================================
// JUnit XML reporter
// ===========================================================================

pub struct JunitReporter;

impl ReportWriter for JunitReporter {
    fn write(&self, summary: &RunSummary, path: &Path) -> std::io::Result<()> {
        let mut xml = String::new();
        xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        xml.push('\n');
        xml.push_str(r#"<testsuites>"#);
        xml.push('\n');

        let mut total_failures = 0;
        for (suite_name, results) in &summary.by_suite {
            let suite_failures = results.iter().filter(|r| r.is_failure()).count();
            total_failures += suite_failures;
            let _ = write!(xml, r#"<testsuite name="{}" tests="{}" failures="{}" time="{:.3}">"#,
                suite_name, results.len(), suite_failures, summary.duration.as_secs_f64());
            for result in results {
                let _ = write!(xml, r#"<testcase classname="{}" name="{}" time="{:.3}">"#,
                    suite_name, result.name, result.duration.as_secs_f64());
                match result.status {
                    TestStatus::Passed | TestStatus::Skipped => {}
                    _ => {
                        let msg = result.message.clone().unwrap_or_default();
                        let bt = result.backtrace.clone().unwrap_or_default();
                        let _ = write!(xml, r#"<failure message="{}" type="AssertionError">"#, msg);
                        xml.push_str(&bt);
                        xml.push_str("</failure>");
                    }
                }
                xml.push_str("</testcase>\n");
            }
            xml.push_str("</testsuite>\n");
        }

        xml.push_str("</testsuites>\n");
        std::fs::write(path, xml)
    }
}

// ===========================================================================
// HTML reporter
// ===========================================================================

pub struct HtmlReporter;

impl ReportWriter for HtmlReporter {
    fn write(&self, summary: &RunSummary, path: &Path) -> std::io::Result<()> {
        let mut html = String::new();
        html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
        html.push_str("<meta charset=\"UTF-8\">\n");
        html.push_str("<title>Elysium Test Report</title>\n");
        html.push_str("<style>\n");
        html.push_str("body { font-family: sans-serif; margin: 2rem; }\n");
        html.push_str("table { border-collapse: collapse; width: 100%; }\n");
        html.push_str("th, td { border: 1px solid #ccc; padding: 0.5rem; text-align: left; }\n");
        html.push_str(".passed { color: #1a7f37; }\n");
        html.push_str(".failed { color: #cf222e; }\n");
        html.push_str(".skipped { color: #9a6700; }\n");
        html.push_str("</style>\n</head>\n<body>\n");
        html.push_str("<h1>Elysium Test Report</h1>\n");

        let _ = write!(html, "<p><strong>Total:</strong> {} &nbsp; ", summary.total);
        let _ = write!(html, "<strong>Passed:</strong> <span class=\"passed\">{}</span> &nbsp; ", summary.passed);
        let _ = write!(html, "<strong>Failed:</strong> <span class=\"failed\">{}</span> &nbsp; ", summary.failed);
        let _ = write!(html, "<strong>Skipped:</strong> <span class=\"skipped\">{}</span></p>\n", summary.skipped);

        html.push_str("<table>\n<tr><th>Suite</th><th>Name</th><th>Status</th><th>Duration</th><th>Message</th></tr>\n");

        for result in &summary.results {
            let status_class = match result.status {
                TestStatus::Passed => "passed",
                TestStatus::Failed | TestStatus::Panicked => "failed",
                TestStatus::Skipped => "skipped",
                TestStatus::Timeout => "skipped",
            };
            let _ = write!(
                html,
                r#"<tr><td>{}</td><td>{}</td><td class="{}">{}</td><td>{:.2}ms</td><td>{}</td></tr>"#,
                result.suite,
                result.name,
                status_class,
                result.status,
                result.duration.as_secs_f64() * 1000.0,
                result.message.clone().unwrap_or_default()
            );
        }

        html.push_str("</table>\n</body>\n</html>\n");
        std::fs::write(path, html)
    }
}

// ===========================================================================
// JSON reporter
// ===========================================================================

pub struct JsonReporter;

impl ReportWriter for JsonReporter {
    fn write(&self, summary: &RunSummary, path: &Path) -> std::io::Result<()> {
        let json = serde_json::json!({
            "total": summary.total,
            "passed": summary.passed,
            "failed": summary.failed,
            "panicked": summary.panicked,
            "skipped": summary.skipped,
            "duration_ms": summary.duration.as_secs_f64() * 1000.0,
            "results": summary.results.iter().map(|r| {
                serde_json::json!({
                    "name": r.name,
                    "suite": r.suite,
                    "status": format!("{:?}", r.status),
                    "duration_ms": r.duration.as_secs_f64() * 1000.0,
                    "message": r.message,
                })
            }).collect::<Vec<_>>(),
        });
        std::fs::write(path, serde_json::to_string_pretty(&json).unwrap())
    }
}

// ===========================================================================
// CSV reporter
// ===========================================================================

pub struct CsvReporter;

impl ReportWriter for CsvReporter {
    fn write(&self, summary: &RunSummary, path: &Path) -> std::io::Result<()> {
        let mut csv = String::new();
        csv.push_str("suite,name,status,duration_ms,message\n");
        for result in &summary.results {
            let _ = write!(
                csv,
                r#""{}","{}","{}",{:.2},"{}""#,
                result.suite,
                result.name,
                result.status,
                result.duration.as_secs_f64() * 1000.0,
                result.message.clone().unwrap_or_default().replace('"', "\"\"")
            );
            csv.push('\n');
        }
        std::fs::write(path, csv)
    }
}

// ===========================================================================
// Text reporter (console)
// ===========================================================================

pub struct TextReporter;

impl TextReporter {
    pub fn write(&self, summary: &RunSummary) {
        eprintln!("\nTest Report\n===========");
        eprintln!("Total: {}  Passed: {}  Failed: {}  Skipped: {}", summary.total, summary.passed, summary.failed, summary.skipped);
        for result in &summary.results {
            let icon = match result.status {
                TestStatus::Passed => "PASS",
                TestStatus::Failed => "FAIL",
                TestStatus::Panicked => "PANIC",
                TestStatus::Skipped => "SKIP",
                TestStatus::Timeout => "TIMEOUT",
            };
            eprintln!("  [{}] {}.{} ({:.2}ms)", icon, result.suite, result.name, result.duration.as_secs_f64() * 1000.0);
            if let Some(ref msg) = result.message {
                eprintln!("       -> {}", msg);
            }
        }
    }
}

// ===========================================================================
// Report generator
// ===========================================================================

#[derive(Debug, Clone)]
pub struct ReportGenerator {
    pub output_dir: PathBuf,
    pub formats: Vec<super::ReportFormat>,
}

impl ReportGenerator {
    pub fn new(output_dir: impl Into<PathBuf>) -> Self {
        Self {
            output_dir: output_dir.into(),
            formats: vec![super::ReportFormat::Text, super::ReportFormat::JUnit],
        }
    }

    pub fn with_formats(mut self, formats: Vec<super::ReportFormat>) -> Self {
        self.formats = formats;
        self
    }

    /// Write all requested report formats.
    pub fn write(&self, summary: &RunSummary) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.output_dir)?;

        for format in &self.formats {
            match format {
                super::ReportFormat::JUnit => {
                    JunitReporter.write(summary, &self.output_dir.join("junit.xml"))?;
                }
                super::ReportFormat::Html => {
                    HtmlReporter.write(summary, &self.output_dir.join("report.html"))?;
                }
                super::ReportFormat::Json => {
                    JsonReporter.write(summary, &self.output_dir.join("report.json"))?;
                }
                super::ReportFormat::Csv => {
                    CsvReporter.write(summary, &self.output_dir.join("report.csv"))?;
                }
                super::ReportFormat::Text => {
                    TextReporter.write(summary);
                }
            }
        }

        Ok(())
    }
}
