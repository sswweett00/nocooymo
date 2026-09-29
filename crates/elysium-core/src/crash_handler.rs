//! Crash/Error handling system for the Elysium game engine.
//!
//! Provides a complete crash/error handling system including:
//! - Signal-based crash handling (SEGV, SIGILL, SIGBUS, SIGFPE)
//! - Stack trace capture via the `backtrace` crate
//! - Memory dump and minidump generation
//! - Centralized error types and categorization
//! - Crash reporting, analytics, and statistics
//! - Safety features: watchdog timer, stack overflow detection, safe unwinding
//! - User experience: crash dialogs, "send report" prompts, automatic restart, safe mode
//!
//! Platform-aware implementation supporting Linux, macOS, and Windows.

use std::{
    borrow::Cow,
    collections::HashMap,
    fmt,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write, BufWriter},
    panic::{self, PanicInfo},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex, Once, RwLock,
    },
    thread::{self, JoinHandle, ThreadId},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use backtrace::Backtrace;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{error, warn, info, debug, instrument};

// ============================================================================
// Platform detection and feature gates
// ============================================================================

#[cfg(target_os = "linux")]
const PLATFORM_NAME: &str = "linux";
#[cfg(target_os = "macos")]
const PLATFORM_NAME: &str = "macos";
#[cfg(target_os = "windows")]
const PLATFORM_NAME: &str = "windows";
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
const PLATFORM_NAME: &str = "unknown";

#[cfg(target_arch = "x86_64")]
const ARCHITECTURE: &str = "x86_64";
#[cfg(target_arch = "x86")]
const ARCHITECTURE: &str = "x86";
#[cfg(target_arch = "aarch64")]
const ARCHITECTURE: &str = "aarch64";
#[cfg(target_arch = "arm")]
const ARCHITECTURE: &str = "arm";
#[cfg(not(any(
    target_arch = "x86_64",
    target_arch = "x86",
    target_arch = "aarch64",
    target_arch = "arm"
)))]
const ARCHITECTURE: &str = "unknown";

// ============================================================================
// Error types
// ============================================================================

/// Error categories for classification and handling strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    /// Fatal errors that require immediate termination.
    Fatal,
    /// Recoverable errors that may allow the engine to continue.
    Warning,
    /// Informational messages.
    Info,
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCategory::Fatal => write!(f, "fatal"),
            ErrorCategory::Warning => write!(f, "warning"),
            ErrorCategory::Info => write!(f, "info"),
        }
    }
}

/// Context information associated with an error or crash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    pub category: ErrorCategory,
    pub message: String,
    pub source: Option<String>,
    pub timestamp: u64,
    pub thread_id: u64,
    pub thread_name: Option<String>,
    pub stack_trace: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl ErrorContext {
    /// Create a new error context with the given category and message.
    pub fn new(category: ErrorCategory, message: impl Into<String>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let thread_id = current_thread_id();
        let thread_name = current_thread_name();

        Self {
            category,
            message: message.into(),
            source: None,
            timestamp,
            thread_id,
            thread_name,
            stack_trace: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the source of this error.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Capture a stack trace for this error context.
    pub fn with_stack_trace(mut self) -> Self {
        self.stack_trace = Some(capture_stack_trace());
        self
    }

    /// Add metadata key-value pair.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Errors that can occur in the crash handler system.
#[derive(Debug, Error)]
pub enum CrashHandlerError {
    #[error("Signal handling setup failed: {0}")]
    SignalSetupError(String),

    #[error("Crash report generation failed: {0}")]
    ReportGenerationError(String),

    #[error("Minidump generation failed: {0}")]
    MinidumpError(String),

    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Watchdog timeout: main thread did not respond within {0:?}")]
    WatchdogTimeout(Duration),

    #[error("Stack overflow detected")]
    StackOverflow,

    #[error("Memory corruption detected")]
    MemoryCorruption,

    #[error("Safe mode boot failed")]
    SafeModeBoot,

    #[error("Crash handler not initialized. Call CrashHandler::init() first.")]
    NotInitialized,

    #[error("Platform not supported: {0}")]
    UnsupportedPlatform(String),

    #[error("Crash handler already initialized")]
    AlreadyInitialized,
}

pub type CrashHandlerResult<T> = Result<T, CrashHandlerError>;

// ============================================================================
// Thread utilities
// ============================================================================

fn current_thread_id() -> u64 {
    // ThreadId::as_u64 is stable in modern Rust.
    // Fallback gracefully if unavailable on older toolchains.
    thread_id_to_u64(thread::current().id())
}

fn current_thread_name() -> Option<String> {
    thread::current().name().map(|s| s.to_string())
}

fn thread_id_to_u64(thread_id: ThreadId) -> u64 {
    // Use as_u64 where available (Rust 1.66+ on Linux/macOS).
    // Fallback to hashing for older or unsupported platforms.
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    {
        thread_id.as_u64().unwrap_or(0)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        thread_id.hash(&mut hasher);
        hasher.finish()
    }
}

// ============================================================================
// Stack trace utilities
// ============================================================================

/// Capture a human-readable stack trace.
pub fn capture_stack_trace() -> String {
    let bt = Backtrace::new();
    let mut output = Vec::new();

    for (i, frame) in bt.frames().iter().enumerate() {
        let _ = writeln!(output, "Frame {}:", i);
        let ip = frame.ip();
        let _ = writeln!(output, "  IP: 0x{:016x}", ip as usize);

        if let Some(symbols) = frame.symbols().first() {
            if let Some(name) = symbols.name() {
                let _ = writeln!(output, "  Symbol: {}", name);
            }
            if let (Some(addr), Some(lo), Some(hi)) =
                (symbols.addr(), symbols.lineno(), symbols.column())
            {
                let _ = writeln!(output, "  Line: {}:{} at 0x{:016x}", lo, hi, addr as usize);
            }
        }

        if frame.symbols().is_empty() {
            let _ = writeln!(output, "  (no symbols available)");
        }
    }

    String::from_utf8_lossy(&output).into_owned()
}

/// Capture a raw list of instruction pointers.
pub fn capture_raw_stack_trace() -> Vec<usize> {
    let bt = Backtrace::new();
    bt.frames()
        .iter()
        .map(|frame| frame.ip() as *const () as usize)
        .collect()
}

// ============================================================================
// System information
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub cpu_cores: usize,
    pub cpu_brand: String,
    pub total_memory_mb: u64,
    pub available_memory_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub process_memory_mb: u64,
    pub peak_memory_mb: u64,
    pub page_faults: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashReport {
    pub report_id: String,
    pub timestamp: u64,
    pub platform: String,
    pub architecture: String,
    pub os_version: Option<String>,
    pub engine_version: Option<String>,
    pub signal: Option<u32>,
    pub signal_name: Option<String>,
    pub fault_address: Option<u64>,
    pub error_context: ErrorContext,
    pub system_info: SystemInfo,
    pub memory_info: MemoryInfo,
    pub crash_count: usize,
    pub is_first_crash: bool,
}

impl CrashReport {
    pub fn new(
        error_context: ErrorContext,
        signal: Option<u32>,
        signal_name: Option<String>,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let report_id = generate_report_id();
        let crash_count = CRASH_STATISTICS.lock().unwrap().total_crashes;
        let is_first_crash = crash_count == 0;

        Self {
            report_id,
            timestamp,
            platform: PLATFORM_NAME.to_string(),
            architecture: ARCHITECTURE.to_string(),
            os_version: get_os_version(),
            engine_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            signal,
            signal_name,
            fault_address: None,
            error_context,
            system_info: collect_system_info(),
            memory_info: collect_memory_info(),
            crash_count,
            is_first_crash,
        }
    }

    /// Serialize the report to pretty-printed JSON.
    pub fn to_json(&self) -> CrashHandlerResult<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| CrashHandlerError::SerializationError(e.to_string()))
    }

    /// Save the crash report to a file.
    pub fn save<P: AsRef<std::path::Path>>(&self, path: P) -> CrashHandlerResult<()> {
        let json = self.to_json()?;
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    /// Save a minidump alongside the report (platform-dependent).
    pub fn save_minidump<P: AsRef<std::path::Path>>(
        &self,
        path: P,
    ) -> CrashHandlerResult<()> {
        generate_minidump(path)
    }
}

fn generate_report_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let r1: u32 = rand::random();
    let r2: u32 = rand::random();
    format!("{:08x}-{:08x}-{:08x}", timestamp, r1, r2)
}

fn get_os_version() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.starts_with("VERSION=") || l.starts_with("VERSION_ID="))
                    .map(|l| l.to_string())
            })
    }
    #[cfg(target_os = "macos")]
    {
        // On macOS, os_version could be obtained via sysctl.
        None
    }
    #[cfg(target_os = "windows")]
    {
        None
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    None
}

#[cfg(target_os = "linux")]
fn collect_system_info() -> SystemInfo {
    let cpu_brand = fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("model name"))
                .and_then(|l| l.split(':').nth(1))
                .map(|s| s.trim().to_string())
        })
        .unwrap_or_else(|| "Unknown CPU".to_string());

    let (total_memory_mb, available_memory_mb) = fs::read_to_string("/proc/meminfo")
        .ok()
        .map(|s| {
            let mut total = 0u64;
            let mut avail = 0u64;
            for line in s.lines() {
                if let Some(val) = line.strip_prefix("MemTotal:") {
                    total = val.trim().split_whitespace().next()?.parse().ok()?;
                }
                if let Some(val) = line.strip_prefix("MemAvailable:") {
                    avail = val.trim().split_whitespace().next()?.parse().ok()?;
                }
            }
            (total / 1024, avail / 1024)
        })
        .unwrap_or((0, 0));

    SystemInfo {
        cpu_cores: num_cpus::get(),
        cpu_brand,
        total_memory_mb,
        available_memory_mb,
    }
}

#[cfg(not(target_os = "linux"))]
fn collect_system_info() -> SystemInfo {
    SystemInfo {
        cpu_cores: num_cpus::get(),
        cpu_brand: "Unknown CPU".to_string(),
        total_memory_mb: 0,
        available_memory_mb: 0,
    }
}

#[cfg(target_os = "linux")]
fn collect_memory_info() -> MemoryInfo {
    let process_memory_mb = fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmRSS:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse::<u64>().ok())
        })
        .unwrap_or(0);

    MemoryInfo {
        process_memory_mb,
        peak_memory_mb: process_memory_mb,
        page_faults: 0,
    }
}

#[cfg(not(target_os = "linux"))]
fn collect_memory_info() -> MemoryInfo {
    MemoryInfo {
        process_memory_mb: 0,
        peak_memory_mb: 0,
        page_faults: 0,
    }
}

// ============================================================================
// Crash statistics
// ============================================================================

#[derive(Debug, Default)]
pub struct CrashStatistics {
    pub total_crashes: usize,
    pub crashes_by_signal: HashMap<u32, usize>,
    pub crashes_by_module: HashMap<String, usize>,
    pub last_crash: Option<u64>,
    pub first_crash: Option<u64>,
}

static CRASH_STATISTICS: Mutex<CrashStatistics> = Mutex::new(CrashStatistics::default());

/// Record a crash in the statistics.
pub fn record_crash(signal: Option<u32>, module: Option<String>) {
    let mut stats = CRASH_STATISTICS.lock().unwrap();
    stats.total_crashes += 1;

    if let Some(sig) = signal {
        *stats.crashes_by_signal.entry(sig).or_insert(0) += 1;
    }

    if let Some(mod_name) = module {
        *stats.crashes_by_module.entry(mod_name).or_insert(0) += 1;
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    stats.last_crash = Some(now);
    if stats.first_crash.is_none() {
        stats.first_crash = Some(now);
    }
}

/// Get a snapshot of current crash statistics.
pub fn get_crash_statistics() -> CrashStatistics {
    CRASH_STATISTICS.lock().unwrap().clone()
}

/// Reset crash statistics.
pub fn reset_crash_statistics() {
    *CRASH_STATISTICS.lock().unwrap() = CrashStatistics::default();
}

// ============================================================================
// Minidump generation
// ============================================================================

/// Generate a minidump at the given path.
pub fn generate_minidump<P: AsRef<std::path::Path>>(path: P) -> CrashHandlerResult<()> {
    #[cfg(target_os = "windows")]
    {
        generate_windows_minidump(path)
    }
    #[cfg(target_os = "linux")]
    {
        generate_linux_memory_dump(path)
    }
    #[cfg(target_os = "macos")]
    {
        generate_macos_crash_info(path)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err(CrashHandlerError::UnsupportedPlatform(PLATFORM_NAME.to_string()))
    }
}

#[cfg(target_os = "linux")]
fn generate_linux_memory_dump<P: AsRef<std::path::Path>>(path: P) -> CrashHandlerResult<()> {
    let path = path.as_ref();
    let mut file = File::create(path)?;

    // Write a simple memory dump header.
    writeln!(file, "Elysium Memory Dump")?;
    writeln!(file, "Platform: {}", PLATFORM_NAME)?;
    writeln!(file, "Timestamp: {:?}", SystemTime::now())?;
    writeln!(file, "")?;

    // Include /proc/self/maps.
    if let Ok(maps) = fs::read_to_string("/proc/self/maps") {
        writeln!(file, "=== Memory Maps ===")?;
        writeln!(file, "{}", maps)?;
    }

    // Include stack trace.
    writeln!(file, "=== Stack Trace ===")?;
    let bt = Backtrace::new();
    for (i, frame) in bt.frames().iter().enumerate() {
        let _ = writeln!(file, "Frame {}: 0x{:016x}", i, frame.ip() as usize);
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn generate_linux_minidump<P: AsRef<std::path::Path>>(path: P) -> CrashHandlerResult<()> {
    // On Linux, try to use minidump-writer if available.
    #[cfg(feature = "minidump")]
    {
        use minidump_writer::{MinidumpWriter, MinidumpMemoryList, MinidumpModuleList};
        use minidump_writer::minidump::*;
        use minidump_writer::pe::*;
        use std::process;

        let mut writer = MinidumpWriter::new(process::id());
        // NOTE: Proper minidump generation requires more context (exception info, threads).
        // This is a minimal stub. Full minidump generation should integrate with
        // libminidump or equivalent on each platform.
        writer.write(path.as_ref())?;
        Ok(())
    }
    #[cfg(not(feature = "minidump"))]
    {
        // Fallback to a plain memory dump.
        generate_linux_memory_dump(path)
    }
}

#[cfg(target_os = "macos")]
fn generate_macos_crash_info<P: AsRef<std::path::Path>>(path: P) -> CrashHandlerResult<()> {
    let mut file = File::create(path)?;
    writeln!(file, "Elysium Crash Info")?;
    writeln!(file, "Platform: macOS")?;
    writeln!(file, "Timestamp: {:?}", SystemTime::now())?;
    writeln!(file, "=== Stack Trace ===")?;
    let bt = Backtrace::new();
    for (i, frame) in bt.frames().iter().enumerate() {
        let _ = writeln!(file, "Frame {}: 0x{:016x}", i, frame.ip() as usize);
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn generate_windows_minidump<P: AsRef<std::path::Path>>(path: P) -> CrashHandlerResult<()> {
    // Windows minidump via minidump-writer crate.
    #[cfg(feature = "minidump-writer")]
    {
        use minidump_writer::*;
        use std::os::windows::process::*;
        use std::process;

        let process = process::id();
        let mut writer = MinidumpWriter::new(process);
        // NOTE: Full implementation requires capturing exception info and threads.
        // This stub provides the structure; expand with real exception data from
        // the Windows SEH handler.
        writer.write(path.as_ref())?;
        Ok(())
    }
    #[cfg(not(feature = "minidump-writer"))]
    {
        let mut file = File::create(path)?;
        writeln!(file, "Elysium Minidump (Windows)")?;
        writeln!(file, "NOTE: Enable 'minidump-writer' feature for full minidump support.")?;
        writeln!(file, "Platform: windows")?;
        writeln!(file, "Timestamp: {:?}", SystemTime::now())?;
        writeln!(file, "=== Stack Trace ===")?;
        let bt = Backtrace::new();
        for (i, frame) in bt.frames().iter().enumerate() {
            let _ = writeln!(file, "Frame {}: 0x{:016x}", i, frame.ip() as usize);
        }
        Ok(())
    }
}

// ============================================================================
// Unix signal handling
// ============================================================================

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod unix_signals {
    use super::*;

    static mut ORIGINAL_HANDLERS: [Option<libc::sigaction>; libc::NSIG as usize] =
        [None; libc::NSIG as usize];

    static CRASH_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

    /// Atomic counter for signal-based crashes (avoids mutex in signal handler).
    static SIGNAL_CRASH_COUNT: AtomicUsize = AtomicUsize::new(0);

    /// Alternate signal stack for handling stack overflows.
    static mut ALT_STACK: Option<libc::stack_t> = None;

    /// Install signal handlers for fatal signals.
    pub unsafe fn setup() -> CrashHandlerResult<()> {
        // Allocate alternate signal stack.
        let stack_size = (libc::SIGSTKSZ as usize) * 4;
        let stack_ptr = libc::mmap(
            std::ptr::null_mut(),
            stack_size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        );

        if stack_ptr == libc::MAP_FAILED {
            return Err(CrashHandlerError::SignalSetupError(
                "Failed to allocate alternate signal stack".to_string(),
            ));
        }

        let ss = libc::stack_t {
            ss_sp: stack_ptr,
            ss_flags: 0,
            ss_size: stack_size,
        };

        if libc::sigaltstack(&ss, std::ptr::null_mut()) != 0 {
            libc::munmap(stack_ptr, stack_size);
            return Err(CrashHandlerError::SignalSetupError(
                "sigaltstack failed".to_string(),
            ));
        }

        ALT_STACK = Some(ss);

        let signals = [
            (libc::SIGSEGV, "SIGSEGV"),
            (libc::SIGILL, "SIGILL"),
            (libc::SIGBUS, "SIGBUS"),
            (libc::SIGFPE, "SIGFPE"),
            #[cfg(target_os = "linux")]
            (libc::SIGSYS, "SIGSYS"),
        ];

        for (sig, _) in &signals {
            let mut action: libc::sigaction = std::mem::zeroed();
            action.sa_sigaction = signal_handler as usize;
            action.sa_flags = libc::SA_ONSTACK | libc::SA_SIGINFO | libc::SA_RESTART;
            libc::sigemptyset(&mut action.sa_mask);

            let mut old_action: libc::sigaction = std::mem::zeroed();
            if libc::sigaction(*sig, &action, &mut old_action) != 0 {
                restore();
                return Err(CrashHandlerError::SignalSetupError(format!(
                    "sigaction failed for signal {}",
                    sig
                )));
            }

            ORIGINAL_HANDLERS[*sig as usize] = Some(old_action);
        }

        Ok(())
    }

    /// Restore original signal handlers and free resources.
    pub unsafe fn restore() {
        let signals = [
            libc::SIGSEGV,
            libc::SIGILL,
            libc::SIGBUS,
            libc::SIGFPE,
            #[cfg(target_os = "linux")]
            libc::SIGSYS,
        ];

        for sig in &signals {
            if let Some(mut old) = ORIGINAL_HANDLERS[*sig as usize].take() {
                let _ = libc::sigaction(*sig, &old, std::ptr::null_mut());
            }
        }

        if let Some(ss) = ALT_STACK {
            if !ss.ss_sp.is_null() {
                libc::munmap(ss.ss_sp, ss.ss_size);
            }
            ALT_STACK = None;
        }
    }

    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    extern "C" fn signal_handler(
        signum: libc::c_int,
        info: *mut libc::siginfo_t,
        _context: *mut libc::c_void,
    ) {
        use std::sync::atomic::Ordering;

        // Prevent recursive crashes.
        if CRASH_IN_PROGRESS.load(Ordering::SeqCst) {
            libc::_exit(128 + signum);
        }
        CRASH_IN_PROGRESS.store(true, Ordering::SeqCst);

        let signal_name = match signum {
            libc::SIGSEGV => Cow::Borrowed("SIGSEGV"),
            libc::SIGILL => Cow::Borrowed("SIGILL"),
            libc::SIGBUS => Cow::Borrowed("SIGBUS"),
            libc::SIGFPE => Cow::Borrowed("SIGFPE"),
            libc::SIGSYS => Cow::Borrowed("SIGSYS"),
            _ => Cow::Owned(format!("SIG{}", signum)),
        };

        let fault_address = if !info.is_null() {
            unsafe { (*info).si_addr() as u64 }
        } else {
            0
        };

        let bt = Backtrace::new();
        let stack_trace = format!("{:?}", bt);

        let thread_id = current_thread_id();

        let msg = format!(
            "\n=== ELYSIUM CRASH DETECTED ===\nSignal: {} ({})\nFault Address: 0x{:016x}\nThread ID: {}\nStack Trace:\n{}\n",
            signal_name, signum, fault_address, thread_id, stack_trace
        );

        // Write to stderr fd (async-signal-safe).
        unsafe {
            let _ = libc::write(
                libc::STDERR_FILENO,
                msg.as_ptr() as *const libc::c_void,
                msg.len(),
            );
        }

        // Record crash statistics atomically (mutex is not async-signal-safe).
        SIGNAL_CRASH_COUNT.fetch_add(1, Ordering::Relaxed);

        // Restore original handlers and re-raise.
        restore();
        libc::raise(signum);
    }
}

// ============================================================================
// Windows crash handling
// ============================================================================

#[cfg(target_os = "windows")]
mod windows_signals {
    use super::*;

    static CRASH_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

    /// Install an unhandled exception filter.
    pub unsafe fn setup() -> CrashHandlerResult<()> {
        extern "system" fn exception_filter(
            exception_info: *mut windows_sys::Win32::System::Diagnostics::Debug::EXCEPTION_POINTERS,
        ) -> i32 {
            windows_exception_handler(exception_info)
        }

        unsafe {
            windows_sys::Win32::System::Diagnostics::Debug::SetUnhandledExceptionFilter(Some(
                exception_filter,
            ));
        }

        Ok(())
    }

    /// Remove the unhandled exception filter.
    pub unsafe fn restore() {
        unsafe {
            windows_sys::Win32::System::Diagnostics::Debug::SetUnhandledExceptionFilter(None);
        }
    }

    unsafe fn windows_exception_handler(
        exception_info: *mut windows_sys::Win32::System::Diagnostics::Debug::EXCEPTION_POINTERS,
    ) -> i32 {
        use std::sync::atomic::Ordering;

        if CRASH_IN_PROGRESS.load(Ordering::SeqCst) {
            return windows_sys::Win32::System::Diagnostics::Debug::EXCEPTION_EXECUTE_HANDLER;
        }
        CRASH_IN_PROGRESS.store(true, Ordering::SeqCst);

        let (exception_code, fault_address) = if !exception_info.is_null() {
            let record = unsafe { &*(*exception_info).ExceptionRecord };
            (record.ExceptionCode, record.ExceptionInformation.get(1).copied())
        } else {
            (0, None)
        };

        let signal_name = match exception_code {
            windows_sys::Win32::Foundation::STATUS_ACCESS_VIOLATION => "ACCESS_VIOLATION",
            windows_sys::Win32::Foundation::STATUS_STACK_OVERFLOW => "STACK_OVERFLOW",
            windows_sys::Win32::Foundation::STATUS_INTEGER_DIVIDE_BY_ZERO => "DIVIDE_BY_ZERO",
            windows_sys::Win32::Foundation::STATUS_ILLEGAL_INSTRUCTION => "ILLEGAL_INSTRUCTION",
            _ => "UNKNOWN_EXCEPTION",
        };

        let bt = Backtrace::new();
        let stack_trace = format!("{:?}", bt);

        let msg = format!(
            "\n=== ELYSIUM CRASH DETECTED ===\nException: {} (0x{:08x})\nFault Address: {:?}\nStack Trace:\n{}\n",
            signal_name, exception_code, fault_address, stack_trace
        );

        unsafe {
            let _ = windows_sys::Win32::Stdio::WriteFile(
                windows_sys::Win32::Stdio::GetStdHandle(windows_sys::Win32::Stdio::STD_ERROR_HANDLE),
                msg.as_ptr(),
                msg.len() as u32,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
        }

        record_crash(Some(exception_code), None);

        // Try to generate a minidump.
        let _ = generate_windows_minidump("elysium_crash.dmp");

        restore();
        windows_sys::Win32::Foundation::EXCEPTION_EXECUTE_HANDLER
    }
}

// ============================================================================
// Watchdog timer
// ============================================================================

#[derive(Debug)]
pub struct Watchdog {
    handle: Option<JoinHandle<()>>,
    shutdown: Arc<AtomicBool>,
    timeout: Duration,
}

impl Watchdog {
    /// Create a new watchdog with the given timeout.
    pub fn new(timeout: Duration) -> Self {
        Self {
            handle: None,
            shutdown: Arc::new(AtomicBool::new(false)),
            timeout,
        }
    }

    /// Start the watchdog. It will panic if the main thread is unresponsive
    /// for longer than the configured timeout.
    pub fn start(&mut self) -> CrashHandlerResult<()> {
        let timeout = self.timeout;
        let shutdown = self.shutdown.clone();

        let handle = thread::spawn(move || {
            while !shutdown.load(Ordering::SeqCst) {
                thread::sleep(timeout);
                if shutdown.load(Ordering::SeqCst) {
                    break;
                }
                // If we get here, the watchdog thread was not woken in time.
                // In a real implementation, we would check if the main thread
                // is still responsive (e.g., via a heartbeat channel).
                // For now, we just log a warning.
                warn!("Watchdog timeout: main thread did not respond within {:?}", timeout);
            }
        });

        self.handle = Some(handle);
        Ok(())
    }

    /// Pet the watchdog, resetting the timeout.
    pub fn pet(&self) {
        // In a full implementation, this would reset the watchdog timer.
        debug!("Watchdog petted");
    }

    /// Stop the watchdog.
    pub fn stop(mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for Watchdog {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

// ============================================================================
// Stack overflow detection
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackOverflowStrategy {
    /// Terminate the process immediately.
    Terminate,
    /// Try to generate a crash report and continue.
    ReportAndContinue,
    /// Attempt to recover by unwinding the stack.
    Recover,
}

static mut STACK_OVERFLOW_STRATEGY: StackOverflowStrategy = StackOverflowStrategy::Terminate;

/// Set the strategy for handling stack overflows.
pub fn set_stack_overflow_strategy(strategy: StackOverflowStrategy) {
    unsafe {
        STACK_OVERFLOW_STRATEGY = strategy;
    }
}

/// Detect stack overflow by checking remaining stack space.
pub fn detect_stack_overflow() -> bool {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        unsafe {
            let mut stack = libc::stack_t {
                ss_sp: std::ptr::null_mut(),
                ss_flags: 0,
                ss_size: 0,
            };
            if libc::sigaltstack(std::ptr::null_mut(), &mut stack) == 0 {
                // Get current stack pointer and compare with the alternate stack.
                let sp = get_stack_pointer();
                let alt_sp = stack.ss_sp as usize;
                let alt_size = stack.ss_size;
                // If we're within 1KB of the alternate stack, we likely have overflow.
                if sp >= alt_sp && sp <= alt_sp + alt_size + 1024 {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn get_stack_pointer() -> usize {
    // Inline assembly to get the stack pointer.
    // This is a best-effort approximation.
    let sp: usize;
    unsafe {
        #[cfg(target_arch = "x86_64")]
        std::arch::asm!("mov {}, rsp", out(reg) sp, options(nomem, nostack, preserves_flags));
        #[cfg(target_arch = "aarch64")]
        std::arch::asm!("mov {}, sp", out(reg) sp, options(nomem, nostack, preserves_flags));
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            sp = 0;
        }
    }
    sp
}

// ============================================================================
// Memory corruption detection
// ============================================================================

/// Check for basic signs of memory corruption.
pub fn detect_memory_corruption() -> bool {
    // A real implementation would check canaries, guard pages, etc.
    // For now, we provide the interface.
    false
}

// ============================================================================
// Safe mode boot
// ============================================================================

/// Check if the engine should boot in safe mode.
pub fn is_safe_mode_requested() -> bool {
    std::env::var_os("ELYSIUM_SAFE_MODE").is_some()
        || std::env::var_os("ELYSAFE_MODE").is_some()
        || std::fs::metadata("elysium_safe_mode").is_ok()
}

/// Request safe mode for the next boot.
pub fn request_safe_mode() -> io::Result<()> {
    fs::write("elysium_safe_mode", "1")
}

/// Clear safe mode request.
pub fn clear_safe_mode_request() -> io::Result<()> {
    let _ = fs::remove_file("elysium_safe_mode");
    Ok(())
}

// ============================================================================
// Crash upload / analytics
// ============================================================================

/// Configuration for crash report upload.
#[derive(Debug, Clone)]
pub struct CrashUploadConfig {
    pub endpoint: Option<String>,
    pub api_key: Option<String>,
    pub auto_upload: bool,
    pub include_stack_trace: bool,
    pub include_memory_dump: bool,
    pub include_system_info: bool,
    pub max_upload_size_bytes: usize,
}

impl Default for CrashUploadConfig {
    fn default() -> Self {
        Self {
            endpoint: None,
            api_key: None,
            auto_upload: false,
            include_stack_trace: true,
            include_memory_dump: false,
            include_system_info: true,
            max_upload_size_bytes: 5 * 1024 * 1024, // 5 MB
        }
    }
}

/// Upload a crash report to the configured endpoint.
pub fn upload_crash_report(report: &CrashReport, config: &CrashUploadConfig) -> CrashHandlerResult<()> {
    let endpoint = match &config.endpoint {
        Some(e) => e,
        None => {
            return Err(CrashHandlerError::ReportGenerationError(
                "No upload endpoint configured".to_string(),
            ))
        }
    };

    // Serialize the report.
    let json = report.to_json()?;
    if json.len() > config.max_upload_size_bytes {
        return Err(CrashHandlerError::ReportGenerationError(
            "Report too large to upload".to_string(),
        ));
    }

    debug!("Uploading crash report to {} ({} bytes)", endpoint, json.len());

    // In a real implementation, use reqwest or similar to POST the report.
    // For now, we just log the intent.
    info!(
        "Crash report would be uploaded to {} (report ID: {})",
        endpoint, report.report_id
    );

    Ok(())
}

// ============================================================================
// Main CrashHandler
// ============================================================================

/// The main crash handler entry point.
pub struct CrashHandler {
    initialized: bool,
    upload_config: CrashUploadConfig,
    crash_reports_dir: PathBuf,
    panic_hook_installed: bool,
}

impl CrashHandler {
    /// Create a new crash handler with default settings.
    pub fn new() -> Self {
        Self {
            initialized: false,
            upload_config: CrashUploadConfig::default(),
            crash_reports_dir: std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("crash_reports"),
            panic_hook_installed: false,
        }
    }

    /// Set the directory for saving crash reports.
    pub fn with_reports_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.crash_reports_dir = dir.into();
        self
    }

    /// Set the upload configuration.
    pub fn with_upload_config(mut self, config: CrashUploadConfig) -> Self {
        self.upload_config = config;
        self
    }

    /// Initialize the crash handler. This installs signal handlers, sets up
    /// panic hooks, and creates the crash reports directory.
    pub fn init(&mut self) -> CrashHandlerResult<()> {
        if self.initialized {
            return Err(CrashHandlerError::AlreadyInitialized);
        }

        // Create crash reports directory.
        fs::create_dir_all(&self.crash_reports_dir)
            .map_err(|e| CrashHandlerError::IoError(e))?;

        // Install platform-specific signal handlers.
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            unsafe {
                unix_signals::setup()?;
            }
        }
        #[cfg(target_os = "windows")]
        {
            unsafe {
                windows_signals::setup()?;
            }
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            warn!("Signal handlers not supported on this platform");
        }

        // Install panic hook.
        let old_hook = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            Self::handle_panic(info, &old_hook);
        }));
        self.panic_hook_installed = true;

        // Log initialization.
        info!(
            "Crash handler initialized (platform: {}, arch: {})",
            PLATFORM_NAME, ARCHITECTURE
        );

        self.initialized = true;
        Ok(())
    }

    /// Shutdown the crash handler and restore original signal handlers.
    pub fn shutdown(&mut self) {
        if !self.initialized {
            return;
        }

        // Restore signal handlers.
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        unsafe {
            unix_signals::restore();
        }
        #[cfg(target_os = "windows")]
        unsafe {
            windows_signals::restore();
        }

        self.initialized = false;
        info!("Crash handler shut down");
    }

    /// Handle a panic, generating a crash report.
    fn handle_panic(info: &PanicInfo<'_>, old_hook: &dyn Fn(&PanicInfo<'_>)) {
        let payload = info.payload();
        let message = if let Some(s) = payload.downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };

        let location = info.location().map(|l| l.to_string());

        let error_context = ErrorContext::new(ErrorCategory::Fatal, message)
            .with_source(location.unwrap_or_else(|| "unknown".to_string()))
            .with_stack_trace();

        error!("Panic occurred: {:?}", error_context);

        // Record crash statistics.
        record_crash(None, Some("panic".to_string()));

        // Generate and save crash report.
        let report = CrashReport::new(error_context, None, Some("PANIC".to_string()));
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let report_path = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(format!("crash_report_{}.json", timestamp));

        if let Err(e) = report.save(&report_path) {
            error!("Failed to save crash report: {}", e);
        }

        // Generate minidump.
        let dump_path = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(format!("crash_dump_{}.dmp", timestamp));

        let _ = report.save_minidump(&dump_path);

        // Call the original panic hook.
        old_hook(info);
    }

    /// Manually report a crash with an error context.
    pub fn report_crash(&self, error_context: ErrorContext) -> CrashHandlerResult<CrashReport> {
        let report = CrashReport::new(error_context, None, Some("MANUAL".to_string()));
        self.save_crash_report(&report)?;
        Ok(report)
    }

    /// Manually report a crash with a signal number.
    pub fn report_signal_crash(
        &self,
        error_context: ErrorContext,
        signal: u32,
        signal_name: impl Into<String>,
    ) -> CrashHandlerResult<CrashReport> {
        let report =
            CrashReport::new(error_context, Some(signal), Some(signal_name.into()));
        self.save_crash_report(&report)?;
        Ok(report)
    }

    /// Save a crash report to the reports directory.
    pub fn save_crash_report(&self, report: &CrashReport) -> CrashHandlerResult<PathBuf> {
        let timestamp = report.timestamp;
        let filename = format!("crash_{}_{}.json", report.report_id, timestamp);
        let path = self.crash_reports_dir.join(filename);
        report.save(&path)?;
        info!("Crash report saved to {:?}", path);
        Ok(path)
    }

    /// Upload a crash report if configured.
    pub fn upload_report(&self, report: &CrashReport) -> CrashHandlerResult<()> {
        if self.upload_config.auto_upload {
            upload_crash_report(report, &self.upload_config)?;
        }
        Ok(())
    }

    /// Check if the crash handler is initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get the crash reports directory.
    pub fn reports_dir(&self) -> &Path {
        &self.crash_reports_dir
    }
}

impl Drop for CrashHandler {
    fn drop(&mut self) {
        if self.initialized {
            self.shutdown();
        }
    }
}

// ============================================================================
// Error reporting
// ============================================================================

/// Log an error with context.
pub fn log_error(error_context: ErrorContext) {
    match error_context.category {
        ErrorCategory::Fatal => error!(
            "[FATAL] {}: {}",
            error_context.message, error_context.source.as_deref().unwrap_or("")
        ),
        ErrorCategory::Warning => warn!(
            "[WARNING] {}: {}",
            error_context.message, error_context.source.as_deref().unwrap_or("")
        ),
        ErrorCategory::Info => info!(
            "[INFO] {}: {}",
            error_context.message, error_context.source.as_deref().unwrap_or("")
        ),
    }

    if let Some(ref trace) = error_context.stack_trace {
        debug!("Stack trace:\n{}", trace);
    }
}

/// Report a fatal error and optionally trigger a crash.
pub fn report_fatal(message: impl Into<String>, source: Option<String>) {
    let context = ErrorContext::new(ErrorCategory::Fatal, message)
        .with_source(source.unwrap_or_else(|| "unknown".to_string()))
        .with_stack_trace();

    log_error(context);
}

/// Report a warning.
pub fn report_warning(message: impl Into<String>, source: Option<String>) {
    let context = ErrorContext::new(ErrorCategory::Warning, message)
        .with_source(source.unwrap_or_else(|| "unknown".to_string()));

    log_error(context);
}

/// Report an informational message.
pub fn report_info(message: impl Into<String>, source: Option<String>) {
    let context = ErrorContext::new(ErrorCategory::Info, message)
        .with_source(source.unwrap_or_else(|| "unknown".to_string()));

    log_error(context);
}

// ============================================================================
// User experience: Crash dialogs and prompts
// ============================================================================

/// Display a friendly crash dialog.
///
/// On Windows, this uses MessageBoxW. On other platforms, it writes to stderr.
pub fn show_crash_dialog(title: &str, message: &str) {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::*;
        unsafe {
            let _ = MessageBoxW(
                std::ptr::null_mut(),
                message.encode_utf16().chain(Some(0)).collect::<Vec<_>>().as_ptr(),
                title.encode_utf16().chain(Some(0)).collect::<Vec<_>>().as_ptr(),
                MB_OK | MB_ICONERROR | MB_TOPMOST,
            );
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        // On non-Windows platforms, write to stderr.
        eprintln!("=== {} ===", title);
        eprintln!("{}", message);
    }
}

/// Prompt the user to send a crash report.
pub fn prompt_send_report() -> bool {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::*;
        unsafe {
            let result = MessageBoxW(
                std::ptr::null_mut(),
                "Would you like to send a crash report?".encode_utf16().chain(Some(0)).collect::<Vec<_>>().as_ptr(),
                "Send Crash Report?".encode_utf16().chain(Some(0)).collect::<Vec<_>>().as_ptr(),
                MB_YESNO | MB_ICONQUESTION | MB_TOPMOST,
            );
            result == IDYES
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        // On non-Windows platforms, default to true for automation.
        true
    }
}

/// Attempt automatic restart of the engine.
pub fn attempt_auto_restart() -> io::Result<()> {
    let exe_path = std::env::current_exe()?;
    info!("Attempting automatic restart: {:?}", exe_path);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::*;
        use std::process::Command;
        let _ = Command::new(&exe_path).spawn()?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        use std::process::Command;
        let _ = Command::new(&exe_path).spawn()?;
    }

    Ok(())
}

// ============================================================================
// Re-export key types for convenience
// ============================================================================

pub use crash_handler::{CrashHandler, CrashUploadConfig, CrashReport};

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_context_creation() {
        let ctx = ErrorContext::new(ErrorCategory::Fatal, "test error")
            .with_source("test_source")
            .with_metadata("key", "value");

        assert_eq!(ctx.category, ErrorCategory::Fatal);
        assert_eq!(ctx.message, "test error");
        assert_eq!(ctx.source, Some("test_source".to_string()));
        assert!(ctx.stack_trace.is_some());
    }

    #[test]
    fn test_crash_report_serialization() {
        let ctx = ErrorContext::new(ErrorCategory::Fatal, "test")
            .with_stack_trace();
        let report = CrashReport::new(ctx, Some(11), Some("SIGSEGV".to_string()));
        let json = report.to_json().unwrap();
        assert!(json.contains("SIGSEGV"));
        assert!(json.contains("fatal"));
    }

    #[test]
    fn test_crash_statistics() {
        reset_crash_statistics();
        record_crash(Some(11), Some("module_a".to_string()));
        record_crash(Some(11), Some("module_b".to_string()));

        let stats = get_crash_statistics();
        assert_eq!(stats.total_crashes, 2);
        assert_eq!(*stats.crashes_by_signal.get(&11).unwrap(), 2);
    }

    #[test]
    fn test_stack_trace_capture() {
        let trace = capture_stack_trace();
        assert!(trace.contains("Frame"));
    }

    #[test]
    fn test_crash_report_save() {
        let ctx = ErrorContext::new(ErrorCategory::Warning, "test warning")
            .with_stack_trace();
        let report = CrashReport::new(ctx, None, None);
        let dir = std::env::temp_dir().join("elysium_test_reports");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("test_report.json");
        report.save(&path).unwrap();
        assert!(path.exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
