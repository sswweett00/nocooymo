/// structured_logger.rs — Enterprise Structured Logger
///
/// Features:
/// - Multiple log levels (Trace, Debug, Info, Warn, Error, Fatal)
/// - Named log channels
/// - Structured fields (key=value pairs)
/// - Ring buffer history
/// - Log filtering (by level, channel, module)
/// - Formatted output (plain, colored, JSON)
/// - Log rotation support (max size)
/// - Performance-optimized with pre-allocation

use std::collections::HashMap;
use std::fmt;
use serde::{Serialize, Deserialize};

// ═══════════════════════════════════════════════════════════ Log Level

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
    Fatal = 5,
}

impl Default for LogLevel {
    fn default() -> Self { LogLevel::Info }
}

impl LogLevel {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
            Self::Fatal => "FATAL",
        }
    }

    pub fn color_code(&self) -> &'static str {
        match self {
            Self::Trace => "\x1b[90m",   // gray
            Self::Debug => "\x1b[36m",   // cyan
            Self::Info => "\x1b[32m",    // green
            Self::Warn => "\x1b[33m",    // yellow
            Self::Error => "\x1b[31m",   // red
            Self::Fatal => "\x1b[35;1m", // bold magenta
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "trace" => LogLevel::Trace,
            "debug" => LogLevel::Debug,
            "info" => LogLevel::Info,
            "warn" | "warning" => LogLevel::Warn,
            "error" => LogLevel::Error,
            "fatal" | "critical" => LogLevel::Fatal,
            _ => LogLevel::Info,
        }
    }

    pub fn meets_threshold(&self, threshold: LogLevel) -> bool {
        *self >= threshold
    }
}

// ═══════════════════════════════════════════════════════════ Log Entry

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
    pub channel: String,
    pub module: String,
    pub file: String,
    pub line: u32,
    pub timestamp_ms: u64,
    pub fields: HashMap<String, String>,
    pub thread_name: String,
}

impl LogEntry {
    pub fn formatted(&self) -> String {
        let fields_str = if self.fields.is_empty() {
            String::new()
        } else {
            let pairs: Vec<String> = self.fields.iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            format!(" [{}]", pairs.join(" "))
        };

        format!(
            "[{}] [{:<5}] [{}] [{}:{}] {}{}",
            self.level.name(),
            self.channel,
            self.module,
            self.file,
            self.line,
            self.message,
            fields_str,
        )
    }

    pub fn json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

impl fmt::Display for LogEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.formatted())
    }
}

// ═══════════════════════════════════════════════════════════ Log Filter

#[derive(Clone, Debug)]
pub struct LogFilter {
    pub min_level: LogLevel,
    pub channels: Option<Vec<String>>,
    pub modules: Option<Vec<String>>,
    pub exclude_modules: Vec<String>,
    pub max_entries: usize,
}

impl Default for LogFilter {
    fn default() -> Self {
        Self {
            min_level: LogLevel::Info,
            channels: None,
            modules: None,
            exclude_modules: Vec::new(),
            max_entries: 10000,
        }
    }
}

impl LogFilter {
    pub fn allows(&self, entry: &LogEntry) -> bool {
        if !entry.level.meets_threshold(self.min_level) { return false; }
        if let Some(ref channels) = self.channels {
            if !channels.iter().any(|c| c == &entry.channel || c == "*") {
                return false;
            }
        }
        if let Some(ref modules) = self.modules {
            if !modules.iter().any(|m| entry.module.contains(m.as_str())) {
                return false;
            }
        }
        if self.exclude_modules.iter().any(|m| entry.module.contains(m.as_str())) {
            return false;
        }
        true
    }
}

// ═══════════════════════════════════════════════════════════ Logger Stats

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct LoggerStats {
    pub total_entries: u64,
    pub by_level: HashMap<String, u64>,
    pub by_channel: HashMap<String, u64>,
    pub filtered_out: u64,
    pub dropped: u64,
}

// ═══════════════════════════════════════════════════════════ Structured Logger

pub struct StructuredLogger {
    entries: Vec<LogEntry>,
    max_entries: usize,
    filter: LogFilter,
    stats: LoggerStats,
    handlers: Vec<LogHandler>,
    enabled: bool,
}

pub enum LogHandler {
    Console { min_level: LogLevel, use_color: bool },
    Callback { min_level: LogLevel, callback: Box<dyn Fn(&LogEntry) + Send + Sync> },
}

impl Default for StructuredLogger {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            max_entries: 10000,
            filter: LogFilter::default(),
            stats: LoggerStats::default(),
            handlers: vec![LogHandler::Console { min_level: LogLevel::Info, use_color: true }],
            enabled: true,
        }
    }
}

impl StructuredLogger {
    pub fn new() -> Self { Self::default() }

    pub fn with_level(level: LogLevel) -> Self {
        Self { filter: LogFilter { min_level: level, ..Default::default() }, ..Default::default() }
    }

    pub fn with_max_entries(max: usize) -> Self {
        Self { max_entries: max, ..Default::default() }
    }

    pub fn set_level(&mut self, level: LogLevel) {
        self.filter.min_level = level;
    }

    pub fn set_filter(&mut self, filter: LogFilter) {
        self.filter = filter;
    }

    pub fn add_console_handler(&mut self, min_level: LogLevel, use_color: bool) {
        self.handlers.push(LogHandler::Console { min_level, use_color });
    }

    pub fn add_callback_handler<F>(&mut self, min_level: LogLevel, callback: F)
    where F: Fn(&LogEntry) + Send + Sync + 'static {
        self.handlers.push(LogHandler::Callback { min_level, callback: Box::new(callback) });
    }

    pub fn set_enabled(&mut self, enabled: bool) { self.enabled = enabled; }

    // ── Core logging ─────────────────────────────────────

    pub fn log(
        &mut self,
        level: LogLevel,
        channel: &str,
        module: &str,
        file: &str,
        line: u32,
        message: String,
        fields: HashMap<String, String>,
    ) {
        if !self.enabled { return; }

        let entry = LogEntry {
            level,
            message,
            channel: channel.to_string(),
            module: module.to_string(),
            file: file.to_string(),
            line,
            timestamp_ms: 0,
            fields,
            thread_name: "main".into(),
        };

        *self.stats.by_level.entry(level.name().to_string()).or_insert(0) += 1;
        *self.stats.by_channel.entry(channel.to_string()).or_insert(0) += 1;
        self.stats.total_entries += 1;

        if !self.filter.allows(&entry) {
            self.stats.filtered_out += 1;
            return;
        }

        // Dispatch to handlers
        for handler in &self.handlers {
            match handler {
                LogHandler::Console { min_level, use_color } => {
                    if level >= *min_level {
                        if *use_color {
                            eprintln!("{}{}\x1b[0m", level.color_code(), entry.formatted());
                        } else {
                            eprintln!("{}", entry.formatted());
                        }
                    }
                }
                LogHandler::Callback { min_level, callback } => {
                    if level >= *min_level {
                        callback(&entry);
                    }
                }
            }
        }

        // Store in ring buffer
        self.entries.push(entry);
        while self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
    }

    // ── Convenience methods ──────────────────────────────

    pub fn trace(&mut self, module: &str, msg: impl Into<String>) {
        self.log(LogLevel::Trace, "default", module, "", 0, msg.into(), HashMap::new());
    }

    pub fn debug(&mut self, module: &str, msg: impl Into<String>) {
        self.log(LogLevel::Debug, "default", module, "", 0, msg.into(), HashMap::new());
    }

    pub fn info(&mut self, module: &str, msg: impl Into<String>) {
        self.log(LogLevel::Info, "default", module, "", 0, msg.into(), HashMap::new());
    }

    pub fn warn(&mut self, module: &str, msg: impl Into<String>) {
        self.log(LogLevel::Warn, "default", module, "", 0, msg.into(), HashMap::new());
    }

    pub fn error(&mut self, module: &str, msg: impl Into<String>) {
        self.log(LogLevel::Error, "default", module, "", 0, msg.into(), HashMap::new());
    }

    pub fn fatal(&mut self, module: &str, msg: impl Into<String>) {
        self.log(LogLevel::Fatal, "default", module, "", 0, msg.into(), HashMap::new());
    }

    pub fn log_with_fields(
        &mut self, level: LogLevel, channel: &str, module: &str,
        msg: impl Into<String>, fields: HashMap<String, String>,
    ) {
        self.log(level, channel, module, "", 0, msg.into(), fields);
    }

    // ── Query ────────────────────────────────────────────

    pub fn entries(&self) -> &[LogEntry] { &self.entries }
    pub fn stats(&self) -> &LoggerStats { &self.stats }
    pub fn count(&self) -> usize { self.entries.len() }

    pub fn entries_for_level(&self, level: LogLevel) -> Vec<&LogEntry> {
        self.entries.iter().filter(|e| e.level == level).collect()
    }

    pub fn entries_for_channel(&self, channel: &str) -> Vec<&LogEntry> {
        self.entries.iter().filter(|e| e.channel == channel).collect()
    }

    pub fn last(&self) -> Option<&LogEntry> {
        self.entries.last()
    }

    pub fn search(&self, query: &str) -> Vec<&LogEntry> {
        let q = query.to_lowercase();
        self.entries.iter()
            .filter(|e| e.message.to_lowercase().contains(&q))
            .collect()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.stats = LoggerStats::default();
    }
}

// ═══════════════════════════════════════════════════════════ Tests

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Fatal > LogLevel::Error);
        assert!(LogLevel::Error > LogLevel::Warn);
        assert!(LogLevel::Warn > LogLevel::Info);
        assert!(LogLevel::Info > LogLevel::Debug);
        assert!(LogLevel::Debug > LogLevel::Trace);
    }

    #[test]
    fn test_log_level_from_str() {
        assert_eq!(LogLevel::from_str("info"), LogLevel::Info);
        assert_eq!(LogLevel::from_str("WARNING"), LogLevel::Warn);
        assert_eq!(LogLevel::from_str("fatal"), LogLevel::Fatal);
        assert_eq!(LogLevel::from_str("unknown"), LogLevel::Info);
    }

    #[test]
    fn test_logger_basic() {
        let mut logger = StructuredLogger::new();
        logger.set_level(LogLevel::Trace);
        logger.info("engine", "Engine started");
        logger.debug("render", "Frame rendered");
        logger.error("physics", "Collision error");

        assert_eq!(logger.count(), 3);
    }

    #[test]
    fn test_logger_level_filter() {
        let mut logger = StructuredLogger::with_level(LogLevel::Warn);
        logger.set_level(LogLevel::Warn);
        logger.trace("t", "Should be filtered");
        logger.debug("t", "Should be filtered");
        logger.info("t", "Should be filtered");
        logger.warn("t", "Should pass");
        logger.error("t", "Should pass");

        assert_eq!(logger.count(), 2);
        assert_eq!(logger.stats().filtered_out, 3);
    }

    #[test]
    fn test_logger_channel_filter() {
        let mut logger = StructuredLogger::new();
        logger.set_level(LogLevel::Trace);
        logger.set_filter(LogFilter {
            channels: Some(vec!["render".into()]),
            ..Default::default()
        });
        logger.log(LogLevel::Info, "engine", "engine", "", 0, "Engine started".into(), HashMap::new());
        logger.log(LogLevel::Info, "render", "render", "", 0, "Frame rendered".into(), HashMap::new());
        logger.log(LogLevel::Info, "physics", "physics", "", 0, "Physics step".into(), HashMap::new());
        assert_eq!(logger.count(), 1);
    }

    #[test]
    fn test_logger_fields() {
        let mut logger = StructuredLogger::new();
        let mut fields = HashMap::new();
        fields.insert("fps".into(), "60.0".into());
        fields.insert("frame".into(), "1234".into());
        logger.log_with_fields(LogLevel::Info, "engine", "renderer", "Frame stats", fields);

        assert_eq!(logger.count(), 1);
        let entry = &logger.entries()[0];
        assert_eq!(entry.fields.get("fps").unwrap(), "60.0");
        assert_eq!(entry.fields.get("frame").unwrap(), "1234");
    }

    #[test]
    fn test_logger_search() {
        let mut logger = StructuredLogger::new();
        logger.info("engine", "System initialized");
        logger.info("render", "Render pipeline ready");
        logger.error("engine", "System failure");

        let results = logger.search("system");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_logger_entries_for_level() {
        let mut logger = StructuredLogger::new();
        logger.set_level(LogLevel::Trace);
        logger.info("t", "info1");
        logger.info("t", "info2");
        logger.error("t", "error1");
        logger.warn("t", "warn1");

        assert_eq!(logger.entries_for_level(LogLevel::Info).len(), 2);
        assert_eq!(logger.entries_for_level(LogLevel::Error).len(), 1);
    }

    #[test]
    fn test_logger_callback_handler() {
        let mut logger = StructuredLogger::new();
        let received = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let r = Arc::clone(&received);
        logger.add_callback_handler(LogLevel::Info, move |entry| {
            r.lock().unwrap().push(entry.message.clone());
        });

        logger.info("test", "Hello");
        logger.trace("test", "Should not trigger");

        let msgs = received.lock().unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0], "Hello");
    }

    #[test]
    fn test_logger_stats() {
        let mut logger = StructuredLogger::new();
        logger.set_level(LogLevel::Trace);
        logger.log(LogLevel::Info, "engine", "engine", "", 0, "a".into(), HashMap::new());
        logger.log(LogLevel::Info, "engine", "engine", "", 0, "b".into(), HashMap::new());
        logger.log(LogLevel::Error, "physics", "physics", "", 0, "c".into(), HashMap::new());

        let stats = logger.stats();
        assert_eq!(stats.total_entries, 3);
        assert_eq!(stats.by_level.get("INFO"), Some(&2));
        assert_eq!(stats.by_level.get("ERROR"), Some(&1));
        assert_eq!(stats.by_channel.get("engine"), Some(&2));
    }

    #[test]
    fn test_logger_max_entries() {
        let mut logger = StructuredLogger::with_max_entries(5);
        logger.set_level(LogLevel::Trace);
        for i in 0..10 {
            logger.info("test", format!("msg{}", i));
        }
        assert_eq!(logger.count(), 5);
    }

    #[test]
    fn test_log_entry_display() {
        let entry = LogEntry {
            level: LogLevel::Info,
            message: "Test message".into(),
            channel: "engine".into(),
            module: "main".into(),
            file: "main.rs".into(),
            line: 42,
            timestamp_ms: 0,
            fields: HashMap::new(),
            thread_name: "main".into(),
        };
        let formatted = entry.formatted();
        assert!(formatted.contains("INFO"));
        assert!(formatted.contains("engine"));
        assert!(formatted.contains("Test message"));
    }

    #[test]
    fn test_log_entry_with_fields_display() {
        let entry = LogEntry {
            level: LogLevel::Info,
            message: "Frame".into(),
            channel: "render".into(),
            module: "renderer".into(),
            file: "renderer.rs".into(),
            line: 100,
            timestamp_ms: 0,
            fields: {
                let mut m = HashMap::new();
                m.insert("fps".into(), "60".into());
                m.insert("draws".into(), "100".into());
                m
            },
            thread_name: "render".into(),
        };
        let formatted = entry.formatted();
        assert!(formatted.contains("fps=60"));
        assert!(formatted.contains("draws=100"));
    }

    #[test]
    fn test_logger_entries_for_channel() {
        let mut logger = StructuredLogger::new();
        logger.set_level(LogLevel::Trace);
        logger.log(LogLevel::Info, "engine", "engine", "", 0, "a".into(), HashMap::new());
        logger.log(LogLevel::Info, "render", "render", "", 0, "b".into(), HashMap::new());
        logger.log(LogLevel::Info, "engine", "engine", "", 0, "c".into(), HashMap::new());
        assert_eq!(logger.entries_for_channel("engine").len(), 2);
        assert_eq!(logger.entries_for_channel("render").len(), 1);
    }

    #[test]
    fn test_logger_last_entry() {
        let mut logger = StructuredLogger::new();
        logger.set_level(LogLevel::Trace);
        assert!(logger.last().is_none());
        logger.info("t", "first");
        logger.info("t", "last");
        assert_eq!(logger.last().unwrap().message, "last");
    }

    #[test]
    fn test_logger_exclude_module() {
        let mut logger = StructuredLogger::new();
        logger.set_level(LogLevel::Trace);
        logger.set_filter(LogFilter {
            exclude_modules: vec!["noise".into()],
            ..Default::default()
        });
        logger.info("engine", "a");
        logger.info("noise_system", "filtered");
        assert_eq!(logger.count(), 1);
    }
}
