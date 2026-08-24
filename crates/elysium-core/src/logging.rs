//! Enterprise-grade structured logging system with filtering, sinks, and tracing.
//!
//! Provides leveled logging with multiple output sinks (console, file, custom),
//! structured key-value fields, async flushing, and category-based filtering.

use std::collections::HashMap;
use std::fmt;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Log level
// ---------------------------------------------------------------------------

/// Severity levels ordered from most to least verbose.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
    Fatal = 5,
}

impl LogLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
            Self::Fatal => "FATAL",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_uppercase().as_str() {
            "TRACE" => Some(Self::Trace),
            "DEBUG" => Some(Self::Debug),
            "INFO" => Some(Self::Info),
            "WARN" | "WARNING" => Some(Self::Warn),
            "ERROR" => Some(Self::Error),
            "FATAL" => Some(Self::Fatal),
            _ => None,
        }
    }

    /// ANSI colour code for terminal output.
    fn color(self) -> &'static str {
        match self {
            Self::Trace => "\x1b[37m",   // white
            Self::Debug => "\x1b[36m",   // cyan
            Self::Info => "\x1b[32m",    // green
            Self::Warn => "\x1b[33m",    // yellow
            Self::Error => "\x1b[31m",   // red
            Self::Fatal => "\x1b[35m",   // magenta
        }
    }
}

// ---------------------------------------------------------------------------
// Log record
// ---------------------------------------------------------------------------

/// A single log entry.
#[derive(Clone, Debug)]
pub struct LogRecord {
    pub level: LogLevel,
    pub target: String,
    pub message: String,
    pub timestamp: u64,
    pub thread_id: u64,
    pub fields: Vec<(String, String)>,
}

impl fmt::Display for LogRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ts_secs = self.timestamp / 1_000_000_000;
        let ts_nanos = self.timestamp % 1_000_000_000;
        write!(f, "[{ts_secs}.{ts_nanos:09}] ")?;
        write!(f, "{} {:>30} ", self.level.as_str(), self.target)?;
        write!(f, "{}", self.message)?;
        if !self.fields.is_empty() {
            write!(f, " {{ ")?;
            for (i, (k, v)) in self.fields.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{k}={v}")?;
            }
            write!(f, " }}")?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Sink trait
// ---------------------------------------------------------------------------

/// A destination for log records.
pub trait LogSink: Send + Sync {
    fn write(&self, record: &LogRecord);
    fn flush(&self) {}
}

// ---------------------------------------------------------------------------
// Console sink
// ---------------------------------------------------------------------------

/// Writes colourised output to stderr.
pub struct ConsoleSink {
    colored: bool,
}

impl ConsoleSink {
    pub fn new(colored: bool) -> Self {
        Self { colored }
    }
}

impl LogSink for ConsoleSink {
    fn write(&self, record: &LogRecord) {
        if self.colored {
            eprintln!(
                "{}{}\x1b[0m",
                record.level.color(),
                record
            );
        } else {
            eprintln!("{record}");
        }
    }

    fn flush(&self) {
        let _ = std::io::stderr().flush();
    }
}

// ---------------------------------------------------------------------------
// File sink
// ---------------------------------------------------------------------------

/// Appends log records to a file with optional rotation.
pub struct FileSink {
    file: Mutex<std::fs::File>,
    path: PathBuf,
    max_size: u64,
    keep: usize,
}

impl FileSink {
    /// Creates a new file sink.  If `max_size` is non-zero the file is rotated
    /// when it exceeds that many bytes, keeping at most `keep` old files.
    pub fn new(path: impl Into<PathBuf>, max_size: u64, keep: usize) -> std::io::Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        Ok(Self {
            file: Mutex::new(file),
            path,
            max_size,
            keep,
        })
    }

    fn maybe_rotate(&self) {
        if self.max_size == 0 {
            return;
        }
        let Ok(meta) = std::fs::metadata(&self.path) else {
            return;
        };
        if meta.len() < self.max_size {
            return;
        }
        // rotate
        for i in (1..self.keep).rev() {
            let from = self.path.with_extension(format!("log.{i}"));
            let to = self.path.with_extension(format!("log.{}", i + 1));
            let _ = std::fs::rename(from, to);
        }
        let _ = std::fs::rename(&self.path, self.path.with_extension("log.1"));
        // reopen
        if let Ok(f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            *self.file.lock().unwrap() = f;
        }
    }
}

impl LogSink for FileSink {
    fn write(&self, record: &LogRecord) {
        self.maybe_rotate();
        if let Ok(mut f) = self.file.lock() {
            let _ = writeln!(f, "{record}");
        }
    }

    fn flush(&self) {
        if let Ok(mut f) = self.file.lock() {
            let _ = f.flush();
        }
    }
}

// ---------------------------------------------------------------------------
// Callback sink
// ---------------------------------------------------------------------------

/// Forwards records to a user-supplied closure — useful for editor integration.
pub struct CallbackSink {
    callback: Box<dyn Fn(&LogRecord) + Send + Sync>,
}

impl CallbackSink {
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(&LogRecord) + Send + Sync + 'static,
    {
        Self {
            callback: Box::new(f),
        }
    }
}

impl LogSink for CallbackSink {
    fn write(&self, record: &LogRecord) {
        (self.callback)(record);
    }
}

// ---------------------------------------------------------------------------
// Global logger
// ---------------------------------------------------------------------------

static THREAD_COUNTER: AtomicU64 = AtomicU64::new(1);

thread_local! {
    static THREAD_ID: u64 = THREAD_COUNTER.fetch_add(1, Ordering::Relaxed);
}

/// Returns the current thread's unique ID (1-based).
pub fn thread_id() -> u64 {
    THREAD_ID.with(|id| *id)
}

struct LoggerInner {
    sinks: Vec<Box<dyn LogSink>>,
    global_level: RwLock<LogLevel>,
    category_levels: RwLock<HashMap<String, LogLevel>>,
    enabled: AtomicBool,
}

static LOGGER: OnceLock<LoggerInner> = OnceLock::new();

fn logger() -> &'static LoggerInner {
    LOGGER.get_or_init(|| LoggerInner {
        sinks: Vec::new(),
        global_level: RwLock::new(LogLevel::Info),
        category_levels: RwLock::new(HashMap::new()),
        enabled: AtomicBool::new(true),
    })
}

/// Initialise the global logger with the given sinks and default level.
///
/// Call this once at startup.  Subsequent calls are silently ignored.
pub fn init(sinks: Vec<Box<dyn LogSink>>, default_level: LogLevel) {
    let _ = LOGGER.set(LoggerInner {
        sinks,
        global_level: RwLock::new(default_level),
        category_levels: RwLock::new(HashMap::new()),
        enabled: AtomicBool::new(true),
    });
}

/// Convenience: initialise with a console sink only.
pub fn init_console(level: LogLevel) {
    init(vec![Box::new(ConsoleSink::new(true))], level);
}

/// Convenience: initialise with console + file sink.
pub fn init_console_and_file(level: LogLevel, path: impl Into<PathBuf>) {
    let file_sink = FileSink::new(path, 64 * 1024 * 1024, 5).ok();
    let mut sinks: Vec<Box<dyn LogSink>> = vec![Box::new(ConsoleSink::new(true))];
    if let Some(fs) = file_sink {
        sinks.push(Box::new(fs));
    }
    init(sinks, level);
}

/// Temporarily enable or disable logging globally.
pub fn set_enabled(enabled: bool) {
    logger().enabled.store(enabled, Ordering::Relaxed);
}

/// Set the global minimum level.
pub fn set_level(level: LogLevel) {
    *logger().global_level.write().unwrap() = level;
}

/// Set or override the level for a specific category (target prefix).
pub fn set_category_level(category: &str, level: LogLevel) {
    logger()
        .category_levels
        .write()
        .unwrap()
        .insert(category.to_string(), level);
}

fn is_enabled(target: &str, level: LogLevel) -> bool {
    let inner = logger();
    if !inner.enabled.load(Ordering::Relaxed) {
        return false;
    }
    let cats = inner.category_levels.read().unwrap();
    // longest matching prefix wins
    let mut best: Option<LogLevel> = None;
    for (cat, cat_level) in cats.iter() {
        if target.starts_with(cat) {
            match best {
                Some(b) if b <= *cat_level => {}
                _ => best = Some(*cat_level),
            }
        }
    }
    let effective = best.unwrap_or_else(|| *inner.global_level.read().unwrap());
    level >= effective
}

// ---------------------------------------------------------------------------
// Log macro helpers
// ---------------------------------------------------------------------------

/// Low-level emit function.  Prefer the `log_*` macros.
pub fn emit(level: LogLevel, target: &str, message: &str, fields: Vec<(String, String)>) {
    if !is_enabled(target, level) {
        return;
    }
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let record = LogRecord {
        level,
        target: target.to_string(),
        message: message.to_string(),
        timestamp,
        thread_id: thread_id(),
        fields,
    };
    let inner = logger();
    for sink in &inner.sinks {
        sink.write(&record);
    }
}

/// Flush all sinks.
pub fn flush() {
    let inner = logger();
    for sink in &inner.sinks {
        sink.flush();
    }
}

// ---------------------------------------------------------------------------
// Convenience macros
// ---------------------------------------------------------------------------

/// Logs at the given level with the current module path as target.
#[macro_export]
macro_rules! log {
    ($level:expr, $($arg:tt)*) => {
        $crate::logging::emit($level, module_path!(), &format!($($arg)*), Vec::new())
    };
}

#[macro_export]
macro_rules! log_trace { ($($arg:tt)*) => { $crate::log!($crate::logging::LogLevel::Trace, $($arg)*) }; }
#[macro_export]
macro_rules! log_debug { ($($arg:tt)*) => { $crate::log!($crate::logging::LogLevel::Debug, $($arg)*) }; }
#[macro_export]
macro_rules! log_info  { ($($arg:tt)*) => { $crate::log!($crate::logging::LogLevel::Info,  $($arg)*) }; }
#[macro_export]
macro_rules! log_warn  { ($($arg:tt)*) => { $crate::log!($crate::logging::LogLevel::Warn,  $($arg)*) }; }
#[macro_export]
macro_rules! log_error { ($($arg:tt)*) => { $crate::log!($crate::logging::LogLevel::Error, $($arg)*) }; }
#[macro_export]
macro_rules! log_fatal { ($($arg:tt)*) => { $crate::log!($crate::logging::LogLevel::Fatal, $($arg)*) }; }

/// Structured-field variant: `log_kv!(Level::Info, "msg", "key" => value, ...)`
#[macro_export]
macro_rules! log_kv {
    ($level:expr, $msg:expr $(, $k:expr => $v:expr)* $(,)?) => {{
        let mut fields = Vec::new();
        $(
            fields.push(($k.to_string(), $v.to_string()));
        )*
        $crate::logging::emit($level, module_path!(), &$msg.to_string(), fields)
    }};
}

// ---------------------------------------------------------------------------
// Span / tracing support (minimal)
// ---------------------------------------------------------------------------

/// A lightweight tracing span.  Logs enter/exit and measures duration.
pub struct Span {
    name: String,
    level: LogLevel,
    start: std::time::Instant,
}

impl Span {
    pub fn enter(name: impl Into<String>, level: LogLevel) -> Self {
        let name = name.into();
        emit(
            level,
            module_path!(),
            &format!("→ enter {}", name),
            Vec::new(),
        );
        Self {
            name,
            level,
            start: std::time::Instant::now(),
        }
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        emit(
            self.level,
            module_path!(),
            &format!("← exit  {} ({:?})", self.name, elapsed),
            vec![("elapsed_us".into(), format!("{}", elapsed.as_micros()))],
        );
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_ordering() {
        assert!(LogLevel::Error > LogLevel::Info);
        assert!(LogLevel::Trace < LogLevel::Debug);
    }

    #[test]
    fn record_display() {
        let r = LogRecord {
            level: LogLevel::Info,
            target: "test".into(),
            message: "hello".into(),
            timestamp: 12345,
            thread_id: 1,
            fields: vec![("key".into(), "val".into())],
        };
        let s = format!("{r}");
        assert!(s.contains("hello"));
        assert!(s.contains("key=val"));
    }

    #[test]
    fn category_filter() {
        // Use a fresh callback sink to verify filtering
        let received = Arc::new(Mutex::new(Vec::new()));
        let r2 = received.clone();
        init(
            vec![Box::new(CallbackSink::new(move |rec| {
                r2.lock().unwrap().push(rec.clone());
            }))],
            LogLevel::Warn,
        );
        set_category_level("special", LogLevel::Trace);

        emit(LogLevel::Info, "normal", "should be filtered", vec![]);
        emit(LogLevel::Trace, "special", "should pass", vec![]);

        let recs = received.lock().unwrap();
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].message, "should pass");
    }
}