//! Analytics and telemetry system for the Elysium game engine.
//!
//! Provides a complete analytics system including:
//! - Telemetry collection (events, sessions, performance metrics, player behavior)
//! - Event system with batching, flushing, and prioritization
//! - Data privacy (GDPR, CCPA, anonymization, opt-in/opt-out, data deletion)
//! - Reporting (daily/weekly/monthly reports, funnel analysis, retention, heatmaps, crash correlation)
//! - Integration (Google Analytics, Mixpanel, custom providers, webhooks, local storage)
//!
//! All collection is privacy-first and respects user consent.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

// ============================================================================
// Error types
// ============================================================================

/// Errors that can occur in the analytics system.
#[derive(Debug, Error)]
pub enum AnalyticsError {
    #[error("Analytics system not initialized. Call TelemetrySystem::init() first.")]
    NotInitialized,

    #[error("Analytics system already initialized")]
    AlreadyInitialized,

    #[error("Event queue full: max capacity {0} reached")]
    QueueFull(usize),

    #[error("Batch flush failed: {0}")]
    FlushFailed(String),

    #[error("Provider error: {0}")]
    ProviderError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Privacy consent required but not given for event type: {0}")]
    ConsentRequired(String),

    #[error("Data deletion request failed: {0}")]
    DeletionFailed(String),

    #[error("Invalid privacy region: {0}")]
    InvalidRegion(String),
}

pub type AnalyticsResult<T> = Result<T, AnalyticsError>;

// ============================================================================
// Privacy and compliance
// ============================================================================

/// Supported privacy/legal regions for data handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyRegion {
    /// European Union / EEA — GDPR applies.
    Gdpr,
    /// California, USA — CCPA/CPRA applies.
    Ccpa,
    /// Brazil — LGPD applies.
    Lgpd,
    /// Other regions without specific data-privacy legislation.
    Other,
}

impl fmt::Display for PrivacyRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrivacyRegion::Gdpr => write!(f, "gdpr"),
            PrivacyRegion::Ccpa => write!(f, "ccpa"),
            PrivacyRegion::Lgpd => write!(f, "lgpd"),
            PrivacyRegion::Other => write!(f, "other"),
        }
    }
}

use std::fmt;

/// User privacy consent settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConsent {
    pub user_id: Option<String>,
    pub region: PrivacyRegion,
    pub analytics_consent: bool,
    pub performance_consent: bool,
    pub personalization_consent: bool,
    pub advertising_consent: bool,
    pub timestamp: u64,
    pub consent_version: String,
}

impl PrivacyConsent {
    pub fn new(region: PrivacyRegion) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            user_id: None,
            region,
            analytics_consent: false,
            performance_consent: false,
            personalization_consent: false,
            advertising_consent: false,
            timestamp,
            consent_version: "1.0".to_string(),
        }
    }

    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn with_analytics(mut self, granted: bool) -> Self {
        self.analytics_consent = granted;
        self
    }

    pub fn with_performance(mut self, granted: bool) -> Self {
        self.performance_consent = granted;
        self
    }

    pub fn with_personalization(mut self, granted: bool) -> Self {
        self.personalization_consent = granted;
        self
    }

    pub fn with_advertising(mut self, granted: bool) -> Self {
        self.advertising_consent = granted;
        self
    }
}

impl Default for PrivacyConsent {
    fn default() -> Self {
        Self::new(PrivacyRegion::Other)
    }
}

/// Anonymization levels for PII handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnonymizationLevel {
    /// No anonymization — full data retained.
    None,
    /// Pseudonymization — replace identifiers with tokens.
    Pseudonymize,
    /// Full anonymization — remove all PII.
    Full,
    /// Aggregate only — no individual records.
    AggregateOnly,
}

/// Data anonymizer for PII fields.
pub struct DataAnonymizer {
    level: RwLock<AnonymizationLevel>,
    salt: Mutex<Option<String>>,
}

impl DataAnonymizer {
    pub fn new(level: AnonymizationLevel, salt: Option<String>) -> Self {
        Self {
            level: RwLock::new(level),
            salt: Mutex::new(salt),
        }
    }

    pub fn set_level(&self, level: AnonymizationLevel) {
        *self.level.write() = level;
    }

    pub fn level(&self) -> AnonymizationLevel {
        *self.level.read()
    }

    /// Anonymize an email address.
    pub fn anonymize_email(&self, email: &str) -> String {
        match *self.level.read() {
            AnonymizationLevel::None => email.to_string(),
            AnonymizationLevel::Pseudonymize => {
                let salt = self.salt.lock().unwrap();
                format!("{:x}", sha2::Sha256::digest(format!("{}{}", email, salt.as_deref().unwrap_or(""))))
            }
            AnonymizationLevel::Full | AnonymizationLevel::AggregateOnly => "anonymized@example.com".to_string(),
        }
    }

    /// Anonymize an IP address.
    pub fn anonymize_ip(&self, ip: &str) -> String {
        match *self.level.read() {
            AnonymizationLevel::None => ip.to_string(),
            AnonymizationLevel::Pseudonymize => {
                let salt = self.salt.lock().unwrap();
                format!("{:x}", sha2::Sha256::digest(format!("{}{}", ip, salt.as_deref().unwrap_or(""))))
            }
            AnonymizationLevel::Full | AnonymizationLevel::AggregateOnly => "0.0.0.0".to_string(),
        }
    }

    /// Anonymize a device identifier.
    pub fn anonymize_device_id(&self, device_id: &str) -> String {
        match *self.level.read() {
            AnonymizationLevel::None => device_id.to_string(),
            AnonymizationLevel::Pseudonymize => {
                let salt = self.salt.lock().unwrap();
                format!("{:x}", sha2::Sha256::digest(format!("{}{}", device_id, salt.as_deref().unwrap_or(""))))
            }
            AnonymizationLevel::Full | AnonymizationLevel::AggregateOnly => "anonymized-device-id".to_string(),
        }
    }
}

impl Default for DataAnonymizer {
    fn default() -> Self {
        Self::new(AnonymizationLevel::Pseudonymize, None)
    }
}

/// A data deletion request (GDPR right to erasure / CCPA deletion).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataDeletionRequest {
    pub request_id: String,
    pub user_id: String,
    pub region: PrivacyRegion,
    pub timestamp: u64,
    pub status: DeletionStatus,
    pub processed_at: Option<u64>,
    pub data_types: Vec<DataType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeletionStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Partial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataType {
    Analytics,
    Performance,
    CrashReports,
    PlayerBehavior,
    Personalization,
    All,
}

impl DataDeletionRequest {
    pub fn new(user_id: impl Into<String>, region: PrivacyRegion, data_types: Vec<DataType>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            request_id: Uuid::new_v4().to_string(),
            user_id: user_id.into(),
            region,
            timestamp,
            status: DeletionStatus::Pending,
            processed_at: None,
            data_types,
        }
    }
}

// ============================================================================
// Event system
// ============================================================================

/// Priority levels for analytics events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Default for EventPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Analytics events that can be tracked.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AnalyticsEvent {
    // Session events
    SessionStarted {
        session_id: String,
        timestamp: u64,
        platform: String,
        build: String,
        region: PrivacyRegion,
    },
    SessionEnded {
        session_id: String,
        timestamp: u64,
        duration_secs: u64,
        exit_reason: ExitReason,
    },
    SessionPaused {
        session_id: String,
        timestamp: u64,
        pause_duration_secs: u64,
    },

    // Player behavior events
    LevelStarted {
        session_id: String,
        level_id: String,
        level_name: String,
        difficulty: String,
        timestamp: u64,
    },
    LevelCompleted {
        session_id: String,
        level_id: String,
        timestamp: u64,
        duration_secs: u64,
        score: Option<u64>,
        deaths: u32,
    },
    LevelFailed {
        session_id: String,
        level_id: String,
        timestamp: u64,
        duration_secs: u64,
        death_reason: Option<String>,
    },
    CheckpointReached {
        session_id: String,
        level_id: String,
        checkpoint_id: String,
        timestamp: u64,
        elapsed_secs: u64,
    },
    ItemCollected {
        session_id: String,
        item_id: String,
        item_type: String,
        quantity: u32,
        timestamp: u64,
    },
    ItemUsed {
        session_id: String,
        item_id: String,
        item_type: String,
        timestamp: u64,
    },
    AbilityUsed {
        session_id: String,
        ability_id: String,
        ability_type: String,
        timestamp: u64,
        cooldown_remaining_ms: u64,
    },
    DialogueChoice {
        session_id: String,
        dialogue_id: String,
        choice_index: u32,
        timestamp: u64,
    },
    CutsceneSkipped {
        session_id: String,
        cutscene_id: String,
        timestamp: u64,
        elapsed_secs: u64,
    },
    SettingsChanged {
        session_id: String,
        setting_key: String,
        old_value: String,
        new_value: String,
        timestamp: u64,
    },

    // UI events
    UiScreenViewed {
        session_id: String,
        screen_name: String,
        timestamp: u64,
        duration_secs: u64,
    },
    UiButtonClicked {
        session_id: String,
        button_id: String,
        screen_name: String,
        timestamp: u64,
    },
    TutorialCompleted {
        session_id: String,
        tutorial_id: String,
        timestamp: u64,
        duration_secs: u64,
    },
    TutorialSkipped {
        session_id: String,
        tutorial_id: String,
        timestamp: u64,
        elapsed_secs: u64,
    },

    // Purchase / monetization events
    PurchaseInitiated {
        session_id: String,
        item_id: String,
        item_type: String,
        price: f64,
        currency: String,
        timestamp: u64,
    },
    PurchaseCompleted {
        session_id: String,
        item_id: String,
        item_type: String,
        price: f64,
        currency: String,
        timestamp: u64,
        transaction_id: String,
    },
    PurchaseFailed {
        session_id: String,
        item_id: String,
        reason: String,
        timestamp: u64,
    },
    AdShown {
        session_id: String,
        ad_type: String,
        ad_placement: String,
        timestamp: u64,
        duration_secs: u64,
    },
    AdClicked {
        session_id: String,
        ad_type: String,
        ad_placement: String,
        timestamp: u64,
    },
    AdRewardClaimed {
        session_id: String,
        ad_type: String,
        reward_type: String,
        reward_quantity: u32,
        timestamp: u64,
    },

    // Performance events
    PerformanceSnapshot {
        session_id: String,
        timestamp: u64,
        fps: f32,
        frame_time_ms: f32,
        cpu_usage_percent: f32,
        memory_usage_mb: u64,
        gpu_usage_percent: Option<f32>,
        draw_calls: u32,
        triangles: u32,
    },
    PerformanceDegraded {
        session_id: String,
        timestamp: u64,
        reason: String,
        severity: PerformanceSeverity,
        fps_before: f32,
        fps_after: f32,
    },
    LoadingStarted {
        session_id: String,
        context: String,
        timestamp: u64,
    },
    LoadingCompleted {
        session_id: String,
        context: String,
        timestamp: u64,
        duration_secs: f64,
        assets_loaded: u32,
    },
    LoadingFailed {
        session_id: String,
        context: String,
        timestamp: u64,
        duration_secs: f64,
        error: String,
    },

    // Crash / error events
    CrashDetected {
        session_id: String,
        timestamp: u64,
        crash_type: CrashType,
        signal: Option<u32>,
        module: Option<String>,
        stack_trace_hash: Option<String>,
    },
    ErrorEncountered {
        session_id: String,
        timestamp: u64,
        error_category: String,
        message: String,
        module: String,
        recoverable: bool,
    },

    // Custom events
    Custom {
        session_id: String,
        event_name: String,
        timestamp: u64,
        parameters: HashMap<String, String>,
    },
}

impl AnalyticsEvent {
    pub fn session_id(&self) -> &str {
        match self {
            AnalyticsEvent::SessionStarted { session_id, .. } => session_id,
            AnalyticsEvent::SessionEnded { session_id, .. } => session_id,
            AnalyticsEvent::SessionPaused { session_id, .. } => session_id,
            AnalyticsEvent::LevelStarted { session_id, .. } => session_id,
            AnalyticsEvent::LevelCompleted { session_id, .. } => session_id,
            AnalyticsEvent::LevelFailed { session_id, .. } => session_id,
            AnalyticsEvent::CheckpointReached { session_id, .. } => session_id,
            AnalyticsEvent::ItemCollected { session_id, .. } => session_id,
            AnalyticsEvent::ItemUsed { session_id, .. } => session_id,
            AnalyticsEvent::AbilityUsed { session_id, .. } => session_id,
            AnalyticsEvent::DialogueChoice { session_id, .. } => session_id,
            AnalyticsEvent::CutsceneSkipped { session_id, .. } => session_id,
            AnalyticsEvent::SettingsChanged { session_id, .. } => session_id,
            AnalyticsEvent::UiScreenViewed { session_id, .. } => session_id,
            AnalyticsEvent::UiButtonClicked { session_id, .. } => session_id,
            AnalyticsEvent::TutorialCompleted { session_id, .. } => session_id,
            AnalyticsEvent::TutorialSkipped { session_id, .. } => session_id,
            AnalyticsEvent::PurchaseInitiated { session_id, .. } => session_id,
            AnalyticsEvent::PurchaseCompleted { session_id, .. } => session_id,
            AnalyticsEvent::PurchaseFailed { session_id, .. } => session_id,
            AnalyticsEvent::AdShown { session_id, .. } => session_id,
            AnalyticsEvent::AdClicked { session_id, .. } => session_id,
            AnalyticsEvent::AdRewardClaimed { session_id, .. } => session_id,
            AnalyticsEvent::PerformanceSnapshot { session_id, .. } => session_id,
            AnalyticsEvent::PerformanceDegraded { session_id, .. } => session_id,
            AnalyticsEvent::LoadingStarted { session_id, .. } => session_id,
            AnalyticsEvent::LoadingCompleted { session_id, .. } => session_id,
            AnalyticsEvent::LoadingFailed { session_id, .. } => session_id,
            AnalyticsEvent::CrashDetected { session_id, .. } => session_id,
            AnalyticsEvent::ErrorEncountered { session_id, .. } => session_id,
            AnalyticsEvent::Custom { session_id, .. } => session_id,
        }
    }

    pub fn timestamp(&self) -> u64 {
        match self {
            AnalyticsEvent::SessionStarted { timestamp, .. } => *timestamp,
            AnalyticsEvent::SessionEnded { timestamp, .. } => *timestamp,
            AnalyticsEvent::SessionPaused { timestamp, .. } => *timestamp,
            AnalyticsEvent::LevelStarted { timestamp, .. } => *timestamp,
            AnalyticsEvent::LevelCompleted { timestamp, .. } => *timestamp,
            AnalyticsEvent::LevelFailed { timestamp, .. } => *timestamp,
            AnalyticsEvent::CheckpointReached { timestamp, .. } => *timestamp,
            AnalyticsEvent::ItemCollected { timestamp, .. } => *timestamp,
            AnalyticsEvent::ItemUsed { timestamp, .. } => *timestamp,
            AnalyticsEvent::AbilityUsed { timestamp, .. } => *timestamp,
            AnalyticsEvent::DialogueChoice { timestamp, .. } => *timestamp,
            AnalyticsEvent::CutsceneSkipped { timestamp, .. } => *timestamp,
            AnalyticsEvent::SettingsChanged { timestamp, .. } => *timestamp,
            AnalyticsEvent::UiScreenViewed { timestamp, .. } => *timestamp,
            AnalyticsEvent::UiButtonClicked { timestamp, .. } => *timestamp,
            AnalyticsEvent::TutorialCompleted { timestamp, .. } => *timestamp,
            AnalyticsEvent::TutorialSkipped { timestamp, .. } => *timestamp,
            AnalyticsEvent::PurchaseInitiated { timestamp, .. } => *timestamp,
            AnalyticsEvent::PurchaseCompleted { timestamp, .. } => *timestamp,
            AnalyticsEvent::PurchaseFailed { timestamp, .. } => *timestamp,
            AnalyticsEvent::AdShown { timestamp, .. } => *timestamp,
            AnalyticsEvent::AdClicked { timestamp, .. } => *timestamp,
            AnalyticsEvent::AdRewardClaimed { timestamp, .. } => *timestamp,
            AnalyticsEvent::PerformanceSnapshot { timestamp, .. } => *timestamp,
            AnalyticsEvent::PerformanceDegraded { timestamp, .. } => *timestamp,
            AnalyticsEvent::LoadingStarted { timestamp, .. } => *timestamp,
            AnalyticsEvent::LoadingCompleted { timestamp, .. } => *timestamp,
            AnalyticsEvent::LoadingFailed { timestamp, .. } => *timestamp,
            AnalyticsEvent::CrashDetected { timestamp, .. } => *timestamp,
            AnalyticsEvent::ErrorEncountered { timestamp, .. } => *timestamp,
            AnalyticsEvent::Custom { timestamp, .. } => *timestamp,
        }
    }

    /// Default priority based on event type.
    pub fn default_priority(&self) -> EventPriority {
        match self {
            AnalyticsEvent::CrashDetected { .. } | AnalyticsEvent::ErrorEncountered { .. } => EventPriority::Critical,
            AnalyticsEvent::PurchaseCompleted { .. } | AnalyticsEvent::PurchaseInitiated { .. } => EventPriority::High,
            AnalyticsEvent::LevelStarted { .. }
            | AnalyticsEvent::LevelCompleted { .. }
            | AnalyticsEvent::LevelFailed { .. } => EventPriority::High,
            AnalyticsEvent::SessionStarted { .. } | AnalyticsEvent::SessionEnded { .. } => EventPriority::High,
            AnalyticsEvent::AdShown { .. } | AnalyticsEvent::AdClicked { .. } | AnalyticsEvent::AdRewardClaimed { .. } => EventPriority::High,
            AnalyticsEvent::PerformanceDegraded { .. } => EventPriority::High,
            AnalyticsEvent::TutorialCompleted { .. } | AnalyticsEvent::TutorialSkipped { .. } => EventPriority::Normal,
            _ => EventPriority::Normal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExitReason {
    Normal,
    QuitToMenu,
    QuitToDesktop,
    SystemShutdown,
    Crash,
    IdleTimeout,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PerformanceSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrashType {
    Segfault,
    IllegalInstruction,
    BusError,
    FloatingPoint,
    StackOverflow,
    Abort,
    Panic,
    AccessViolation,
    Unknown,
}

impl CrashType {
    pub fn from_signal(signal: Option<u32>) -> Self {
        match signal {
            Some(11) => CrashType::Segfault,
            Some(4) => CrashType::IllegalInstruction,
            Some(7) => CrashType::BusError,
            Some(8) => CrashType::FloatingPoint,
            Some(6) => CrashType::Abort,
            _ => CrashType::Unknown,
        }
    }
}

/// A queued analytics event with metadata.
#[derive(Debug, Clone)]
pub struct QueuedEvent {
    pub event: AnalyticsEvent,
    pub priority: EventPriority,
    pub queued_at: Instant,
    pub consent_checked: bool,
}

impl QueuedEvent {
    pub fn new(event: AnalyticsEvent, priority: EventPriority) -> Self {
        Self {
            event,
            priority,
            queued_at: Instant::now(),
            consent_checked: false,
        }
    }
}

/// Configuration for event batching and flushing.
#[derive(Debug, Clone)]
pub struct FlushConfig {
    /// Maximum number of events to batch before flushing.
    pub batch_size: usize,
    /// Maximum time to hold events before flushing.
    pub flush_interval: Duration,
    /// Maximum queue capacity (events older than this are dropped).
    pub max_queue_size: usize,
    /// Maximum age of a queued event before it is dropped.
    pub max_event_age: Duration,
}

impl Default for FlushConfig {
    fn default() -> Self {
        Self {
            batch_size: 50,
            flush_interval: Duration::from_secs(5),
            max_queue_size: 10000,
            max_event_age: Duration::from_secs(60),
        }
    }
}

// ============================================================================
// Session tracking
// ============================================================================

/// An active analytics session.
#[derive(Debug, Clone)]
pub struct AnalyticsSession {
    pub session_id: String,
    pub started_at: u64,
    pub last_activity: u64,
    pub region: PrivacyRegion,
    pub platform: String,
    pub build: String,
    pub consent: PrivacyConsent,
    pub event_count: u64,
    pub levels_started: u64,
    pub levels_completed: u64,
    pub levels_failed: u64,
    pub total_playtime_secs: u64,
    pub purchase_count: u64,
    pub ad_impressions: u64,
}

impl AnalyticsSession {
    pub fn new(region: PrivacyRegion, platform: impl Into<String>, build: impl Into<String>) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            session_id: Uuid::new_v4().to_string(),
            started_at: now,
            last_activity: now,
            region,
            platform: platform.into(),
            build: build.into(),
            consent: PrivacyConsent::new(region),
            event_count: 0,
            levels_started: 0,
            levels_completed: 0,
            levels_failed: 0,
            total_playtime_secs: 0,
            purchase_count: 0,
            ad_impressions: 0,
        }
    }

    pub fn with_consent(mut self, consent: PrivacyConsent) -> Self {
        self.consent = consent;
        self
    }

    pub fn touch(&mut self) {
        self.last_activity = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
    }

    pub fn record_event(&mut self) {
        self.event_count += 1;
        self.touch();
    }

    pub fn record_level_started(&mut self) {
        self.levels_started += 1;
        self.record_event();
    }

    pub fn record_level_completed(&mut self) {
        self.levels_completed += 1;
        self.record_event();
    }

    pub fn record_level_failed(&mut self) {
        self.levels_failed += 1;
        self.record_event();
    }

    pub fn record_playtime(&mut self, secs: u64) {
        self.total_playtime_secs += secs;
        self.touch();
    }

    pub fn record_purchase(&mut self) {
        self.purchase_count += 1;
        self.record_event();
    }

    pub fn record_ad_impression(&mut self) {
        self.ad_impressions += 1;
        self.record_event();
    }
}

// ============================================================================
// Performance metrics
// ============================================================================

/// Aggregated performance metrics for analytics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub session_id: String,
    pub timestamp: u64,
    pub samples: u64,
    pub avg_fps: f32,
    pub min_fps: f32,
    pub max_fps: f32,
    pub p1_fps: f32,
    pub p99_fps: f32,
    pub avg_frame_ms: f32,
    pub p99_frame_ms: f32,
    pub avg_cpu_ms: f32,
    pub avg_gpu_ms: f32,
    pub avg_update_ms: f32,
    pub avg_render_ms: f32,
    pub avg_physics_ms: f32,
    pub memory_peak_mb: u64,
    pub memory_avg_mb: u64,
    pub draw_calls_avg: u32,
    pub triangles_avg: u32,
    pub load_events: u64,
    pub failed_loads: u64,
}

impl PerformanceMetrics {
    pub fn new(session_id: impl Into<String>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            session_id: session_id.into(),
            timestamp,
            ..Default::default()
        }
    }
}

/// Rolling performance tracker for the current session.
pub struct RollingPerformanceTracker {
    samples: Mutex<VecDeque<PerformanceSample>>,
    max_samples: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSample {
    pub timestamp: u64,
    pub fps: f32,
    pub frame_time_ms: f32,
    pub cpu_ms: f32,
    pub gpu_ms: Option<f32>,
    pub update_ms: f32,
    pub render_ms: f32,
    pub physics_ms: f32,
    pub memory_mb: u64,
    pub draw_calls: u32,
    pub triangles: u32,
}

impl RollingPerformanceTracker {
    pub fn new(max_samples: usize) -> Self {
        Self {
            samples: Mutex::new(VecDeque::with_capacity(max_samples)),
            max_samples,
        }
    }

    pub fn record(&self, sample: PerformanceSample) {
        let mut samples = self.samples.lock().unwrap();
        if samples.len() >= self.max_samples {
            samples.pop_front();
        }
        samples.push_back(sample);
    }

    pub fn metrics(&self, session_id: impl Into<String>) -> PerformanceMetrics {
        let samples = self.samples.lock().unwrap();
        let session_id = session_id.into();

        if samples.is_empty() {
            return PerformanceMetrics::new(session_id);
        }

        let mut fps_values: Vec<f32> = samples.iter().map(|s| s.fps).collect();
        let mut frame_ms: Vec<f32> = samples.iter().map(|s| s.frame_time_ms).collect();
        let cpu_ms: Vec<f32> = samples.iter().map(|s| s.cpu_ms).collect();
        let gpu_ms: Vec<f32> = samples.iter().filter_map(|s| s.gpu_ms).collect();
        let update_ms: Vec<f32> = samples.iter().map(|s| s.update_ms).collect();
        let render_ms: Vec<f32> = samples.iter().map(|s| s.render_ms).collect();
        let physics_ms: Vec<f32> = samples.iter().map(|s| s.physics_ms).collect();
        let memory: Vec<u64> = samples.iter().map(|s| s.memory_mb).collect();
        let draw_calls: Vec<u32> = samples.iter().map(|s| s.draw_calls).collect();
        let triangles: Vec<u32> = samples.iter().map(|s| s.triangles).collect();

        fps_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        frame_ms.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let n = samples.len() as u64;
        let p1_idx = ((n as f32) * 0.01).max(0.0) as usize;
        let p99_idx = ((n as f32) * 0.99) as usize;

        PerformanceMetrics {
            session_id,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            samples: n,
            avg_fps: mean(&fps_values),
            min_fps: fps_values.first().copied().unwrap_or(0.0),
            max_fps: fps_values.last().copied().unwrap_or(0.0),
            p1_fps: fps_values.get(p1_idx).copied().unwrap_or(0.0),
            p99_fps: fps_values.get(p99_idx).copied().unwrap_or(0.0),
            avg_frame_ms: mean(&frame_ms),
            p99_frame_ms: frame_ms.get(p99_idx).copied().unwrap_or(0.0),
            avg_cpu_ms: mean(&cpu_ms),
            avg_gpu_ms: if gpu_ms.is_empty() { 0.0 } else { mean(&gpu_ms) },
            avg_update_ms: mean(&update_ms),
            avg_render_ms: mean(&render_ms),
            avg_physics_ms: mean(&physics_ms),
            memory_peak_mb: memory.iter().max().copied().unwrap_or(0),
            memory_avg_mb: if memory.is_empty() { 0 } else { memory.iter().sum::<u64>() / memory.len() as u64 },
            draw_calls_avg: if draw_calls.is_empty() { 0 } else { draw_calls.iter().sum::<u32>() / draw_calls.len() as u32 },
            triangles_avg: if triangles.is_empty() { 0 } else { triangles.iter().sum::<u32>() / triangles.len() as u32 },
            load_events: 0,
            failed_loads: 0,
        }
    }

    pub fn clear(&self) {
        self.samples.lock().unwrap().clear();
    }

    pub fn len(&self) -> usize {
        self.samples.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.lock().unwrap().is_empty()
    }
}

fn mean(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f32>() / values.len() as f32
}

// ============================================================================
// Player behavior tracking
// ============================================================================

/// Aggregated player behavior metrics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerBehaviorMetrics {
    pub session_id: String,
    pub timestamp: u64,
    pub session_duration_secs: u64,
    pub levels_started: u64,
    pub levels_completed: u64,
    pub levels_failed: u64,
    pub completion_rate: f32,
    pub avg_level_duration_secs: f32,
    pub total_deaths: u64,
    pub items_collected: u64,
    pub abilities_used: u64,
    pub dialogue_choices_made: u64,
    pub cutscenes_skipped: u64,
    pub settings_changes: u64,
    pub screens_viewed: HashMap<String, u64>,
    pub buttons_clicked: HashMap<String, u64>,
    pub tutorials_completed: u64,
    pub tutorials_skipped: u64,
    pub purchase_count: u64,
    pub total_spend: f64,
    pub ad_impressions: u64,
    pub ad_clicks: u64,
    pub ad_rewards_claimed: u64,
    pub sessions_played: u64,
}

impl PlayerBehaviorMetrics {
    pub fn new(session_id: impl Into<String>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            session_id: session_id.into(),
            timestamp,
            ..Default::default()
        }
    }
}

/// Rolling player behavior tracker.
pub struct RollingPlayerBehaviorTracker {
    session_metrics: Mutex<HashMap<String, PlayerBehaviorMetrics>>,
    max_sessions: usize,
}

impl RollingPlayerBehaviorTracker {
    pub fn new(max_sessions: usize) -> Self {
        Self {
            session_metrics: Mutex::new(HashMap::new()),
            max_sessions,
        }
    }

    pub fn get_or_create(&self, session_id: impl Into<String>) -> PlayerBehaviorMetrics {
        let session_id = session_id.into();
        let mut metrics = self.session_metrics.lock().unwrap();
        if let Some(existing) = metrics.get(&session_id) {
            existing.clone()
        } else {
            let new_metrics = PlayerBehaviorMetrics::new(&session_id);
            metrics.insert(session_id.clone(), new_metrics.clone());
            new_metrics
        }
    }

    pub fn record_level_started(&self, session_id: impl Into<String>, level_id: impl Into<String>) {
        let session_id = session_id.into();
        let mut metrics = self.session_metrics.lock().unwrap();
        let entry = metrics.entry(session_id.clone()).or_insert_with(|| PlayerBehaviorMetrics::new(session_id));
        entry.levels_started += 1;
        *entry.screens_viewed.entry(format!("level_{}", level_id.into())).or_insert(0) += 1;
    }

    pub fn record_level_completed(&self, session_id: impl Into<String>) {
        let mut metrics = self.session_metrics.lock().unwrap();
        if let Some(entry) = metrics.get_mut(&session_id.into()) {
            entry.levels_completed += 1;
        }
    }

    pub fn record_level_failed(&self, session_id: impl Into<String>) {
        let mut metrics = self.session_metrics.lock().unwrap();
        if let Some(entry) = metrics.get_mut(&session_id.into()) {
            entry.levels_failed += 1;
        }
    }

    pub fn record_purchase(&self, session_id: impl Into<String>, price: f64) {
        let session_id = session_id.into();
        let mut metrics = self.session_metrics.lock().unwrap();
        let entry = metrics.entry(session_id).or_insert_with(PlayerBehaviorMetrics::default);
        entry.purchase_count += 1;
        entry.total_spend += price;
    }

    pub fn record_ad_impression(&self, session_id: impl Into<String>) {
        let mut metrics = self.session_metrics.lock().unwrap();
        if let Some(entry) = metrics.get_mut(&session_id.into()) {
            entry.ad_impressions += 1;
        }
    }

    pub fn record_ad_click(&self, session_id: impl Into<String>) {
        let mut metrics = self.session_metrics.lock().unwrap();
        if let Some(entry) = metrics.get_mut(&session_id.into()) {
            entry.ad_clicks += 1;
        }
    }

    pub fn record_ad_reward_claimed(&self, session_id: impl Into<String>) {
        let mut metrics = self.session_metrics.lock().unwrap();
        if let Some(entry) = metrics.get_mut(&session_id.into()) {
            entry.ad_rewards_claimed += 1;
        }
    }

    pub fn snapshot(&self, session_id: impl Into<String>) -> Option<PlayerBehaviorMetrics> {
        self.session_metrics.lock().unwrap().get(&session_id.into()).cloned()
    }

    pub fn all_snapshots(&self) -> Vec<PlayerBehaviorMetrics> {
        self.session_metrics.lock().unwrap().values().cloned().collect()
    }

    pub fn remove_session(&self, session_id: impl Into<String>) {
        self.session_metrics.lock().unwrap().remove(&session_id.into());
    }

    pub fn clear(&self) {
        self.session_metrics.lock().unwrap().clear();
    }

    pub fn session_count(&self) -> usize {
        self.session_metrics.lock().unwrap().len()
    }
}

// ============================================================================
// Reporting
// ============================================================================

/// Time window for analytics reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportWindow {
    Daily,
    Weekly,
    Monthly,
    Custom { start: u64, end: u64 },
}

impl fmt::Display for ReportWindow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReportWindow::Daily => write!(f, "daily"),
            ReportWindow::Weekly => write!(f, "weekly"),
            ReportWindow::Monthly => write!(f, "monthly"),
            ReportWindow::Custom { start, end } => write!(f, "custom_{}_{}", start, end),
        }
    }
}

/// A funnel stage definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunnelStage {
    pub name: String,
    pub event: String,
    pub filters: HashMap<String, String>,
}

/// Funnel analysis result.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FunnelAnalysis {
    pub window: ReportWindow,
    pub generated_at: u64,
    pub stages: Vec<FunnelStageResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunnelStageResult {
    pub stage_name: String,
    pub count: u64,
    pub drop_off_rate: f32,
    pub conversion_rate: f32,
}

/// Retention analysis result.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RetentionAnalysis {
    pub window: ReportWindow,
    pub generated_at: u64,
    pub day_retention: Vec<DayRetention>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayRetention {
    pub day: u32,
    pub retained_users: u64,
    pub total_users: u64,
    pub retention_rate: f32,
}

/// Heatmap cell.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatmapCell {
    pub x: u32,
    pub y: u32,
    pub screen: String,
    pub intensity: f32,
    pub count: u64,
}

/// Heatmap data for UI/interaction analysis.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HeatmapData {
    pub window: ReportWindow,
    pub generated_at: u64,
    pub cells: Vec<HeatmapCell>,
    pub screen_width: u32,
    pub screen_height: u32,
}

/// Crash correlation report linking crashes to user behavior.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CrashCorrelationReport {
    pub window: ReportWindow,
    pub generated_at: u64,
    pub total_crashes: u64,
    pub crashes_by_module: HashMap<String, u64>,
    pub crashes_after_action: HashMap<String, u64>,
    pub avg_session_duration_before_crash_secs: f64,
    pub common_crash_sequences: Vec<CrashSequence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashSequence {
    pub sequence: Vec<String>,
    pub count: u64,
    pub avg_duration_secs: f64,
}

/// Complete analytics report.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalyticsReport {
    pub report_id: String,
    pub window: ReportWindow,
    pub generated_at: u64,
    pub session_count: u64,
    pub total_events: u64,
    pub event_breakdown: HashMap<String, u64>,
    pub performance: Option<PerformanceMetrics>,
    pub behavior: Option<PlayerBehaviorMetrics>,
    pub funnel: Option<FunnelAnalysis>,
    pub retention: Option<RetentionAnalysis>,
    pub heatmaps: Option<HeatmapData>,
    pub crash_correlation: Option<CrashCorrelationReport>,
}

impl AnalyticsReport {
    pub fn new(window: ReportWindow) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            report_id: Uuid::new_v4().to_string(),
            window,
            generated_at: timestamp,
            ..Default::default()
        }
    }
}

// ============================================================================
// Integration: providers, webhooks, local storage
// ============================================================================

/// Trait for analytics providers.
pub trait AnalyticsProvider: Send + Sync {
    fn name(&self) -> &str;
    fn is_enabled(&self) -> bool;
    fn send_event(&self, event: &AnalyticsEvent) -> AnalyticsResult<()>;
    fn flush(&self) -> AnalyticsResult<()>;
}

/// Google Analytics 4 provider (placeholder implementation).
pub struct GoogleAnalyticsProvider {
    measurement_id: String,
    api_secret: String,
    enabled: bool,
}

impl GoogleAnalyticsProvider {
    pub fn new(measurement_id: impl Into<String>, api_secret: impl Into<String>) -> Self {
        Self {
            measurement_id: measurement_id.into(),
            api_secret: api_secret.into(),
            enabled: true,
        }
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl AnalyticsProvider for GoogleAnalyticsProvider {
    fn name(&self) -> &str {
        "google_analytics"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn send_event(&self, event: &AnalyticsEvent) -> AnalyticsResult<()> {
        if !self.enabled {
            return Ok(());
        }

        debug!(
            "GA4: would send event '{}' to measurement_id='{}'",
            event_name(event),
            self.measurement_id
        );

        // In a real implementation, this would POST to:
        // https://www.google-analytics.com/mp/collect?measurement_id=...&api_secret=...
        // with the Measurement Protocol (GA4).
        Ok(())
    }

    fn flush(&self) -> AnalyticsResult<()> {
        if !self.enabled {
            return Ok(());
        }

        debug!("GA4: flushing batched events");
        Ok(())
    }
}

/// Mixpanel provider (placeholder implementation).
pub struct MixpanelProvider {
    token: String,
    server_url: String,
    enabled: bool,
}

impl MixpanelProvider {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            server_url: "https://api.mixpanel.com/track".to_string(),
            enabled: true,
        }
    }

    pub fn with_server_url(mut self, url: impl Into<String>) -> Self {
        self.server_url = url.into();
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl AnalyticsProvider for MixpanelProvider {
    fn name(&self) -> &str {
        "mixpanel"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn send_event(&self, event: &AnalyticsEvent) -> AnalyticsResult<()> {
        if !self.enabled {
            return Ok(());
        }

        debug!(
            "Mixpanel: would send event '{}' with token='{}'",
            event_name(event),
            self.token
        );

        // In a real implementation, this would POST to self.server_url
        // with a JSON payload: { "event": "...", "properties": { ... } }
        Ok(())
    }

    fn flush(&self) -> AnalyticsResult<()> {
        if !self.enabled {
            return Ok(());
        }

        debug!("Mixpanel: flushing batched events");
        Ok(())
    }
}

/// Custom analytics provider defined by a callback.
pub struct CustomProvider<F>
where
    F: Fn(&AnalyticsEvent) -> AnalyticsResult<()> + Send + Sync,
{
    name: String,
    callback: F,
    enabled: bool,
}

impl<F> CustomProvider<F>
where
    F: Fn(&AnalyticsEvent) -> AnalyticsResult<()> + Send + Sync,
{
    pub fn new(name: impl Into<String>, callback: F) -> Self {
        Self {
            name: name.into(),
            callback,
            enabled: true,
        }
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl<F> AnalyticsProvider for CustomProvider<F>
where
    F: Fn(&AnalyticsEvent) -> AnalyticsResult<()> + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn send_event(&self, event: &AnalyticsEvent) -> AnalyticsResult<()> {
        if !self.enabled {
            return Ok(());
        }

        (self.callback)(event)
    }

    fn flush(&self) -> AnalyticsResult<()> {
        if !self.enabled {
            return Ok(());
        }

        debug!("Custom provider '{}': flush", self.name);
        Ok(())
    }
}

/// Webhook callback configuration.
#[derive(Debug, Clone)]
pub struct WebhookCallback {
    pub url: String,
    pub headers: HashMap<String, String>,
    pub timeout: Duration,
    pub retry_count: u32,
    pub retry_delay: Duration,
    pub enabled: bool,
}

impl WebhookCallback {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            headers: HashMap::new(),
            timeout: Duration::from_secs(10),
            retry_count: 3,
            retry_delay: Duration::from_secs(1),
            enabled: true,
        }
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_retry(mut self, count: u32, delay: Duration) -> Self {
        self.retry_count = count;
        self.retry_delay = delay;
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Local analytics storage for offline/debug modes.
pub struct LocalAnalyticsStorage {
    base_path: std::path::PathBuf,
    max_file_size: usize,
    compression_enabled: bool,
}

impl LocalAnalyticsStorage {
    pub fn new(base_path: impl AsRef<std::path::Path>) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
            max_file_size: 10 * 1024 * 1024, // 10 MB
            compression_enabled: true,
        }
    }

    pub fn with_max_file_size(mut self, size: usize) -> Self {
        self.max_file_size = size;
        self
    }

    pub fn with_compression(mut self, enabled: bool) -> Self {
        self.compression_enabled = enabled;
        self
    }

    /// Persist a batch of events to local storage.
    pub fn persist_batch(&self, events: &[QueuedEvent]) -> AnalyticsResult<std::path::PathBuf> {
        std::fs::create_dir_all(&self.base_path)?;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let filename = format!("analytics_{}.json", timestamp);
        let path = self.base_path.join(&filename);

        let data: Vec<&AnalyticsEvent> = events.iter().map(|e| &e.event).collect();

        let json = serde_json::to_string_pretty(&data)
            .map_err(|e| AnalyticsError::SerializationError(e.to_string()))?;

        if self.compression_enabled && json.len() > 4096 {
            let compressed = zstd::encode_all(json.as_bytes(), 0)
                .map_err(|e| AnalyticsError::SerializationError(e.to_string()))?;
            std::fs::write(path.with_extension("json.zst"), compressed)?;
        } else {
            std::fs::write(&path, json)?;
        }

        Ok(path)
    }

    /// Load all stored batches from local storage.
    pub fn load_all(&self) -> AnalyticsResult<Vec<AnalyticsEvent>> {
        let mut events = Vec::new();

        if !self.base_path.exists() {
            return Ok(events);
        }

        for entry in std::fs::read_dir(&self.base_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|e| e == "zst").unwrap_or(false) {
                let compressed = std::fs::read(&path)?;
                let json = zstd::decode_all(&compressed[..])
                    .map_err(|e| AnalyticsError::SerializationError(e.to_string()))?;
                let batch: Vec<AnalyticsEvent> = serde_json::from_slice(&json)
                    .map_err(|e| AnalyticsError::SerializationError(e.to_string()))?;
                events.extend(batch);
            } else if path.extension().map(|e| e == "json").unwrap_or(false) {
                let json = std::fs::read(&path)?;
                let batch: Vec<AnalyticsEvent> = serde_json::from_slice(&json)
                    .map_err(|e| AnalyticsError::SerializationError(e.to_string()))?;
                events.extend(batch);
            }
        }

        Ok(events)
    }

    /// Clear all locally stored analytics data.
    pub fn clear(&self) -> AnalyticsResult<()> {
        if self.base_path.exists() {
            std::fs::remove_dir_all(&self.base_path)?;
        }
        Ok(())
    }
}

// ============================================================================
// Main TelemetrySystem
// ============================================================================

/// The main telemetry and analytics system.
pub struct TelemetrySystem {
    initialized: AtomicBool,
    consent: RwLock<PrivacyConsent>,
    anonymizer: Arc<DataAnonymizer>,
    session: RwLock<Option<AnalyticsSession>>,
    event_queue: RwLock<VecDeque<QueuedEvent>>,
    flush_config: RwLock<FlushConfig>,
    providers: RwLock<Vec<Arc<dyn AnalyticsProvider>>>,
    webhooks: RwLock<Vec<WebhookCallback>>,
    local_storage: RwLock<Option<LocalAnalyticsStorage>>,
    performance_tracker: Arc<RollingPerformanceTracker>,
    behavior_tracker: Arc<RollingPlayerBehaviorTracker>,
    stats: RwLock<TelemetryStats>,
    flush_deadline: RwLock<Option<Instant>>,
    background_flush_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
    shutdown: AtomicBool,
}

/// Internal telemetry statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TelemetryStats {
    pub events_enqueued: u64,
    pub events_flushed: u64,
    pub events_dropped: u64,
    pub batches_sent: u64,
    pub provider_errors: u64,
    pub sessions_started: u64,
    pub sessions_ended: u64,
    pub consent_granted: u64,
    pub consent_denied: u64,
    pub data_deletions: u64,
    pub last_flush: Option<u64>,
}

impl TelemetrySystem {
    /// Create a new telemetry system with the given privacy settings.
    pub fn new(consent: PrivacyConsent) -> Self {
        Self {
            initialized: AtomicBool::new(false),
            consent: RwLock::new(consent),
            anonymizer: Arc::new(DataAnonymizer::default()),
            session: RwLock::new(None),
            event_queue: RwLock::new(VecDeque::new()),
            flush_config: RwLock::new(FlushConfig::default()),
            providers: RwLock::new(Vec::new()),
            webhooks: RwLock::new(Vec::new()),
            local_storage: RwLock::new(None),
            performance_tracker: Arc::new(RollingPerformanceTracker::new(300)),
            behavior_tracker: Arc::new(RollingPlayerBehaviorTracker::new(1000)),
            stats: RwLock::new(TelemetryStats::default()),
            flush_deadline: RwLock::new(None),
            background_flush_handle: Mutex::new(None),
            shutdown: AtomicBool::new(false),
        }
    }

    /// Initialize the telemetry system.
    pub fn init(&self) -> AnalyticsResult<()> {
        if self.initialized.load(Ordering::SeqCst) {
            return Err(AnalyticsError::AlreadyInitialized);
        }

        info!("Telemetry system initialized");
        self.initialized.store(true, Ordering::SeqCst);

        // Start background flush task
        let queue = Arc::new(self.event_queue.clone());
        let flush_config = self.flush_config.clone();
        let stats = Arc::new(self.stats.clone());
        let providers = self.providers.clone();
        let webhooks = self.webhooks.clone();
        let local_storage = self.local_storage.clone();
        let shutdown = self.shutdown.clone();

        let handle = tokio::spawn(async move {
            Self::background_flush_loop(queue, flush_config, stats, providers, webhooks, local_storage, shutdown).await;
        });

        *self.background_flush_handle.lock().unwrap() = Some(handle);

        Ok(())
    }

    /// Shutdown the telemetry system.
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);

        // Flush remaining events
        let _ = self.flush();

        if let Some(handle) = self.background_flush_handle.lock().unwrap().take() {
            handle.abort();
        }

        self.initialized.store(false, Ordering::SeqCst);
        info!("Telemetry system shut down");
    }

    async fn background_flush_loop(
        queue: Arc<RwLock<VecDeque<QueuedEvent>>>,
        flush_config: Arc<RwLock<FlushConfig>>,
        stats: Arc<RwLock<TelemetryStats>>,
        providers: Arc<RwLock<Vec<Arc<dyn AnalyticsProvider>>>>,
        webhooks: Arc<RwLock<Vec<WebhookCallback>>>,
        local_storage: Arc<RwLock<Option<LocalAnalyticsStorage>>>,
        shutdown: AtomicBool,
    ) {
        let mut interval = tokio::time::interval(Duration::from_secs(1));

        loop {
            interval.tick().await;

            if shutdown.load(Ordering::SeqCst) {
                break;
            }

            let config = flush_config.read().unwrap().clone();
            let should_flush = {
                let q = queue.read().unwrap();
                q.len() >= config.batch_size
            };

            if should_flush {
                let _ = Self::drain_and_flush(
                    queue.clone(),
                    stats.clone(),
                    providers.clone(),
                    webhooks.clone(),
                    local_storage.clone(),
                ).await;
            }
        }
    }

    async fn drain_and_flush(
        queue: Arc<RwLock<VecDeque<QueuedEvent>>>,
        stats: Arc<RwLock<TelemetryStats>>,
        providers: Arc<RwLock<Vec<Arc<dyn AnalyticsProvider>>>>,
        webhooks: Arc<RwLock<Vec<WebhookCallback>>>,
        local_storage: Arc<RwLock<Option<LocalAnalyticsStorage>>>,
    ) {
        let mut batch: Vec<QueuedEvent> = Vec::new();

        {
            let mut q = queue.write().unwrap();
            let config = q.len().min(100);
            for _ in 0..config {
                if let Some(event) = q.pop_front() {
                    batch.push(event);
                }
            }
        }

        if batch.is_empty() {
            return;
        }

        let events: Vec<&AnalyticsEvent> = batch.iter().map(|e| &e.event).collect();

        // Send to providers
        for provider in providers.read().unwrap().iter() {
            if provider.is_enabled() {
                for event in &events {
                    if let Err(e) = provider.send_event(event) {
                        warn!("Provider '{}' send_event error: {}", provider.name(), e);
                        stats.write().unwrap().provider_errors += 1;
                    }
                }

                if let Err(e) = provider.flush() {
                    warn!("Provider '{}' flush error: {}", provider.name(), e);
                    stats.write().unwrap().provider_errors += 1;
                }

                stats.write().unwrap().batches_sent += 1;
            }
        }

        // Send to webhooks
        for webhook in webhooks.read().unwrap().iter() {
            if webhook.enabled {
                let payload = serde_json::to_vec(&events)
                    .map_err(|e| AnalyticsError::SerializationError(e.to_string()));

                if let Ok(body) = payload {
                    debug!("Webhook: would POST {} events to {}", events.len(), webhook.url);
                }
            }
        }

        // Persist locally
        if let Some(storage) = local_storage.read().unwrap().as_ref() {
            if let Err(e) = storage.persist_batch(&batch) {
                warn!("Local storage persist error: {}", e);
            }
        }

        stats.write().unwrap().events_flushed += batch.len() as u64;
        stats.write().unwrap().last_flush = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        );
    }

    /// Enqueue an analytics event.
    pub fn enqueue(&self, event: AnalyticsEvent) -> AnalyticsResult<()> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err(AnalyticsError::NotInitialized);
        }

        let consent = self.consent.read().unwrap();
        let consent_ok = match &event {
            AnalyticsEvent::PerformanceSnapshot { .. }
            | AnalyticsEvent::PerformanceDegraded { .. }
            | AnalyticsEvent::LoadingStarted { .. }
            | AnalyticsEvent::LoadingCompleted { .. }
            | AnalyticsEvent::LoadingFailed { .. } => consent.performance_consent,
            AnalyticsEvent::SettingsChanged { .. }
            | AnalyticsEvent::TutorialCompleted { .. }
            | AnalyticsEvent::TutorialSkipped { .. }
            | AnalyticsEvent::DialogueChoice { .. } => consent.personalization_consent,
            AnalyticsEvent::AdShown { .. }
            | AnalyticsEvent::AdClicked { .. }
            | AnalyticsEvent::AdRewardClaimed { .. }
            | AnalyticsEvent::PurchaseInitiated { .. }
            | AnalyticsEvent::PurchaseCompleted { .. }
            | AnalyticsEvent::PurchaseFailed { .. } => consent.analytics_consent && consent.personalization_consent,
            _ => consent.analytics_consent,
        };

        if !consent_ok {
            return Err(AnalyticsError::ConsentRequired(event_name(&event)));
        }
        drop(consent);

        let priority = event.default_priority();
        let mut queued = QueuedEvent::new(event, priority);
        queued.consent_checked = true;

        let mut queue = self.event_queue.write().unwrap();
        let config = self.flush_config.read().unwrap();

        if queue.len() >= config.max_queue_size {
            // Drop oldest low-priority events
            let mut dropped = 0;
            queue.retain(|e| {
                if dropped < 100 && e.priority == EventPriority::Low {
                    dropped += 1;
                    false
                } else {
                    true
                }
            });
            self.stats.write().unwrap().events_dropped += dropped;
        }

        // Drop stale events
        queue.retain(|e| e.queued_at.elapsed() < config.max_event_age);

        queue.push_back(queued);
        self.stats.write().unwrap().events_enqueued += 1;

        // Update flush deadline
        let mut deadline = self.flush_deadline.write().unwrap();
        *deadline = Some(Instant::now() + config.flush_interval);

        // Update session if active
        if let Some(session) = self.session.write().unwrap().as_mut() {
            session.record_event();
        }

        Ok(())
    }

    /// Force flush all queued events.
    pub fn flush(&self) -> AnalyticsResult<()> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err(AnalyticsError::NotInitialized);
        }

        let mut batch: Vec<QueuedEvent> = Vec::new();
        {
            let mut queue = self.event_queue.write().unwrap();
            batch.extend(queue.drain(..));
        }

        if batch.is_empty() {
            return Ok(());
        }

        let events: Vec<&AnalyticsEvent> = batch.iter().map(|e| &e.event).collect();

        for provider in self.providers.read().unwrap().iter() {
            if provider.is_enabled() {
                for event in &events {
                    if let Err(e) = provider.send_event(event) {
                        warn!("Provider '{}' send_event error: {}", provider.name(), e);
                        self.stats.write().unwrap().provider_errors += 1;
                    }
                }
                let _ = provider.flush();
                self.stats.write().unwrap().batches_sent += 1;
            }
        }

        for webhook in self.webhooks.read().unwrap().iter() {
            if webhook.enabled {
                let _ = serde_json::to_vec(&events);
                debug!("Webhook flush: {} events to {}", events.len(), webhook.url);
            }
        }

        if let Some(storage) = self.local_storage.read().unwrap().as_ref() {
            let _ = storage.persist_batch(&batch);
        }

        self.stats.write().unwrap().events_flushed += batch.len() as u64;
        self.stats.write().unwrap().last_flush = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        );

        Ok(())
    }

    /// Start a new analytics session.
    pub fn start_session(&self, platform: impl Into<String>, build: impl Into<String>) -> AnalyticsResult<String> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err(AnalyticsError::NotInitialized);
        }

        let consent = self.consent.read().unwrap();
        let region = consent.region;
        let session = AnalyticsSession::new(region, platform, build).with_consent(consent.clone());

        let session_id = session.session_id.clone();

        *self.session.write().unwrap() = Some(session);
        self.behavior_tracker.get_or_create(&session_id);
        self.stats.write().unwrap().sessions_started += 1;

        let _ = self.enqueue(AnalyticsEvent::SessionStarted {
            session_id: session_id.clone(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            platform: platform.into(),
            build: build.into(),
            region,
        });

        info!("Analytics session started: {}", session_id);
        Ok(session_id)
    }

    /// End the current session.
    pub fn end_session(&self, exit_reason: ExitReason) -> AnalyticsResult<()> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err(AnalyticsError::NotInitialized);
        }

        let session = self.session.write().unwrap().take();
        if let Some(session) = session {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);

            let duration = now.saturating_sub(session.started_at);

            let _ = self.enqueue(AnalyticsEvent::SessionEnded {
                session_id: session.session_id.clone(),
                timestamp: now,
                duration_secs: duration,
                exit_reason,
            });

            self.stats.write().unwrap().sessions_ended += 1;
            info!("Analytics session ended: {} (duration: {}s)", session.session_id, duration);
        }

        Ok(())
    }

    /// Pause the current session.
    pub fn pause_session(&self) -> AnalyticsResult<()> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err(AnalyticsError::NotInitialized);
        }

        if let Some(session) = self.session.write().unwrap().as_mut() {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);

            let last = session.last_activity;
            let pause_duration = now.saturating_sub(last);

            let _ = self.enqueue(AnalyticsEvent::SessionPaused {
                session_id: session.session_id.clone(),
                timestamp: now,
                pause_duration_secs: pause_duration,
            });
        }

        Ok(())
    }

    /// Get the current session ID.
    pub fn current_session_id(&self) -> Option<String> {
        self.session.read().unwrap().as_ref().map(|s| s.session_id.clone())
    }

    /// Get current privacy consent.
    pub fn consent(&self) -> PrivacyConsent {
        self.consent.read().unwrap().clone()
    }

    /// Update privacy consent.
    pub fn set_consent(&self, consent: PrivacyConsent) {
        let old = self.consent.write().unwrap().clone();
        *self.consent.write().unwrap() = consent;

        if consent.analytics_consent && !old.analytics_consent {
            self.stats.write().unwrap().consent_granted += 1;
        } else if !consent.analytics_consent && old.analytics_consent {
            self.stats.write().unwrap().consent_denied += 1;
        }

        info!("Privacy consent updated: analytics={}, performance={}, personalization={}, advertising={}",
            consent.analytics_consent,
            consent.performance_consent,
            consent.personalization_consent,
            consent.advertising_consent,
        );
    }

    /// Process a data deletion request.
    pub fn delete_user_data(&self, request: DataDeletionRequest) -> AnalyticsResult<()> {
        info!(
            "Processing data deletion request {} for user {}",
            request.request_id, request.user_id
        );

        // Remove local behavior data
        self.behavior_tracker.remove_session(&request.user_id);

        // Remove session data if matching
        {
            let mut session = self.session.write().unwrap();
            if let Some(s) = session.as_ref() {
                if s.session_id == request.user_id {
                    *session = None;
                }
            }
        }

        // Clear local storage for this user
        if let Some(storage) = self.local_storage.read().unwrap().as_ref() {
            let _ = storage.clear();
        }

        self.stats.write().unwrap().data_deletions += 1;
        Ok(())
    }

    /// Record a performance sample.
    pub fn record_performance(&self, sample: PerformanceSample) {
        self.performance_tracker.record(sample);
    }

    /// Get performance metrics.
    pub fn performance_metrics(&self) -> PerformanceMetrics {
        let session_id = self.current_session_id().unwrap_or_else(|| "unknown".to_string());
        self.performance_tracker.metrics(session_id)
    }

    /// Get player behavior metrics for the current session.
    pub fn behavior_metrics(&self) -> PlayerBehaviorMetrics {
        let session_id = self.current_session_id().unwrap_or_else(|| "unknown".to_string());
        self.behavior_tracker
            .snapshot(session_id)
            .unwrap_or_else(|| PlayerBehaviorMetrics::new(session_id))
    }

    /// Generate an analytics report.
    pub fn generate_report(&self, window: ReportWindow) -> AnalyticsReport {
        let mut report = AnalyticsReport::new(window);

        report.session_count = self.stats.read().unwrap().sessions_started;
        report.total_events = self.stats.read().unwrap().events_enqueued;
        report.performance = Some(self.performance_metrics());
        report.behavior = Some(self.behavior_metrics());

        // Build event breakdown from current session
        if let Some(session) = self.session.read().unwrap().as_ref() {
            report.event_breakdown.insert("session_start".to_string(), 1);
            report.event_breakdown.insert("levels_started".to_string(), session.levels_started);
            report.event_breakdown.insert("levels_completed".to_string(), session.levels_completed);
            report.event_breakdown.insert("levels_failed".to_string(), session.levels_failed);
            report.event_breakdown.insert("purchases".to_string(), session.purchase_count);
            report.event_breakdown.insert("ad_impressions".to_string(), session.ad_impressions);
        }

        info!(
            "Generated {} analytics report: sessions={}, events={}",
            window, report.session_count, report.total_events
        );

        report
    }

    /// Add an analytics provider.
    pub fn add_provider(&self, provider: Arc<dyn AnalyticsProvider>) {
        self.providers.write().unwrap().push(provider);
    }

    /// Add a webhook callback.
    pub fn add_webhook(&self, webhook: WebhookCallback) {
        self.webhooks.write().unwrap().push(webhook);
    }

    /// Set local analytics storage.
    pub fn set_local_storage(&self, storage: LocalAnalyticsStorage) {
        *self.local_storage.write().unwrap() = Some(storage);
    }

    /// Get telemetry statistics.
    pub fn stats(&self) -> TelemetryStats {
        self.stats.read().unwrap().clone()
    }

    /// Check if the system is initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }

    /// Get the number of queued events.
    pub fn queue_size(&self) -> usize {
        self.event_queue.read().unwrap().len()
    }

    /// Update flush configuration.
    pub fn set_flush_config(&self, config: FlushConfig) {
        *self.flush_config.write().unwrap() = config;
    }

    /// Get a reference to the performance tracker.
    pub fn performance_tracker(&self) -> Arc<RollingPerformanceTracker> {
        self.performance_tracker.clone()
    }

    /// Get a reference to the behavior tracker.
    pub fn behavior_tracker(&self) -> Arc<RollingPlayerBehaviorTracker> {
        self.behavior_tracker.clone()
    }

    /// Get a reference to the data anonymizer.
    pub fn anonymizer(&self) -> Arc<DataAnonymizer> {
        self.anonymizer.clone()
    }
}

impl Drop for TelemetrySystem {
    fn drop(&mut self) {
        if self.initialized.load(Ordering::SeqCst) {
            self.shutdown();
        }
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Get a human-readable event name for logging/debugging.
fn event_name(event: &AnalyticsEvent) -> &'static str {
    match event {
        AnalyticsEvent::SessionStarted { .. } => "session_started",
        AnalyticsEvent::SessionEnded { .. } => "session_ended",
        AnalyticsEvent::SessionPaused { .. } => "session_paused",
        AnalyticsEvent::LevelStarted { .. } => "level_started",
        AnalyticsEvent::LevelCompleted { .. } => "level_completed",
        AnalyticsEvent::LevelFailed { .. } => "level_failed",
        AnalyticsEvent::CheckpointReached { .. } => "checkpoint_reached",
        AnalyticsEvent::ItemCollected { .. } => "item_collected",
        AnalyticsEvent::ItemUsed { .. } => "item_used",
        AnalyticsEvent::AbilityUsed { .. } => "ability_used",
        AnalyticsEvent::DialogueChoice { .. } => "dialogue_choice",
        AnalyticsEvent::CutsceneSkipped { .. } => "cutscene_skipped",
        AnalyticsEvent::SettingsChanged { .. } => "settings_changed",
        AnalyticsEvent::UiScreenViewed { .. } => "ui_screen_viewed",
        AnalyticsEvent::UiButtonClicked { .. } => "ui_button_clicked",
        AnalyticsEvent::TutorialCompleted { .. } => "tutorial_completed",
        AnalyticsEvent::TutorialSkipped { .. } => "tutorial_skipped",
        AnalyticsEvent::PurchaseInitiated { .. } => "purchase_initiated",
        AnalyticsEvent::PurchaseCompleted { .. } => "purchase_completed",
        AnalyticsEvent::PurchaseFailed { .. } => "purchase_failed",
        AnalyticsEvent::AdShown { .. } => "ad_shown",
        AnalyticsEvent::AdClicked { .. } => "ad_clicked",
        AnalyticsEvent::AdRewardClaimed { .. } => "ad_reward_claimed",
        AnalyticsEvent::PerformanceSnapshot { .. } => "performance_snapshot",
        AnalyticsEvent::PerformanceDegraded { .. } => "performance_degraded",
        AnalyticsEvent::LoadingStarted { .. } => "loading_started",
        AnalyticsEvent::LoadingCompleted { .. } => "loading_completed",
        AnalyticsEvent::LoadingFailed { .. } => "loading_failed",
        AnalyticsEvent::CrashDetected { .. } => "crash_detected",
        AnalyticsEvent::ErrorEncountered { .. } => "error_encountered",
        AnalyticsEvent::Custom { event_name, .. } => event_name,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privacy_consent_creation() {
        let consent = PrivacyConsent::new(PrivacyRegion::Gdpr)
            .with_analytics(true)
            .with_performance(true)
            .with_personalization(false)
            .with_advertising(false);

        assert!(consent.analytics_consent);
        assert!(consent.performance_consent);
        assert!(!consent.personalization_consent);
        assert!(!consent.advertising_consent);
        assert_eq!(consent.region, PrivacyRegion::Gdpr);
    }

    #[test]
    fn test_data_anonymizer() {
        let anonymizer = DataAnonymizer::new(AnonymizationLevel::Pseudonymize, Some("salt123".to_string()));
        
        let email = anonymizer.anonymize_email("test@example.com");
        assert_ne!(email, "test@example.com");
        assert!(email.len() == 64); // SHA256 hex length

        let ip = anonymizer.anonymize_ip("192.168.1.1");
        assert_ne!(ip, "192.168.1.1");

        let device = anonymizer.anonymize_device_id("device-123");
        assert_ne!(device, "device-123");
    }

    #[test]
    fn test_data_anonymizer_full() {
        let anonymizer = DataAnonymizer::new(AnonymizationLevel::Full, None);
        assert_eq!(anonymizer.anonymize_email("test@example.com"), "anonymized@example.com");
        assert_eq!(anonymizer.anonymize_ip("192.168.1.1"), "0.0.0.0");
        assert_eq!(anonymizer.anonymize_device_id("device-123"), "anonymized-device-id");
    }

    #[test]
    fn test_data_deletion_request() {
        let request = DataDeletionRequest::new("user-123", PrivacyRegion::Gdpr, vec![DataType::All]);
        assert_eq!(request.user_id, "user-123");
        assert_eq!(request.region, PrivacyRegion::Gdpr);
        assert_eq!(request.status, DeletionStatus::Pending);
        assert!(matches!(request.data_types[0], DataType::All));
    }

    #[test]
    fn test_event_priority() {
        let session_event = AnalyticsEvent::SessionStarted {
            session_id: "test".to_string(),
            timestamp: 0,
            platform: "pc".to_string(),
            build: "1.0".to_string(),
            region: PrivacyRegion::Other,
        };
        assert_eq!(session_event.default_priority(), EventPriority::High);

        let crash_event = AnalyticsEvent::CrashDetected {
            session_id: "test".to_string(),
            timestamp: 0,
            crash_type: CrashType::Segfault,
            signal: Some(11),
            module: Some("test".to_string()),
            stack_trace_hash: None,
        };
        assert_eq!(crash_event.default_priority(), EventPriority::Critical);
    }

    #[test]
    fn test_analytics_session() {
        let mut session = AnalyticsSession::new(PrivacyRegion::Gdpr, "pc", "1.0");
        session.record_level_started();
        session.record_level_completed();
        session.record_purchase();
        session.record_ad_impression();

        assert_eq!(session.levels_started, 1);
        assert_eq!(session.levels_completed, 1);
        assert_eq!(session.purchase_count, 1);
        assert_eq!(session.ad_impressions, 1);
        assert_eq!(session.event_count, 4);
    }

    #[test]
    fn test_performance_tracker() {
        let tracker = RollingPerformanceTracker::new(10);
        let sample = PerformanceSample {
            timestamp: 0,
            fps: 60.0,
            frame_time_ms: 16.67,
            cpu_ms: 8.0,
            gpu_ms: Some(10.0),
            update_ms: 2.0,
            render_ms: 12.0,
            physics_ms: 1.0,
            memory_mb: 256,
            draw_calls: 100,
            triangles: 1000,
        };

        tracker.record(sample);
        assert_eq!(tracker.len(), 1);
        assert!(!tracker.is_empty());

        let metrics = tracker.metrics("session-1");
        assert_eq!(metrics.avg_fps, 60.0);
        assert_eq!(metrics.avg_cpu_ms, 8.0);
        assert_eq!(metrics.avg_gpu_ms, 10.0);
        assert_eq!(metrics.memory_peak_mb, 256);
    }

    #[test]
    fn test_player_behavior_tracker() {
        let tracker = RollingPlayerBehaviorTracker::new(100);
        tracker.record_level_started("session-1", "level-1");
        tracker.record_level_completed("session-1");
        tracker.record_purchase("session-1", 9.99);
        tracker.record_ad_impression("session-1");
        tracker.record_ad_click("session-1");
        tracker.record_ad_reward_claimed("session-1");

        let metrics = tracker.snapshot("session-1").unwrap();
        assert_eq!(metrics.levels_started, 1);
        assert_eq!(metrics.levels_completed, 1);
        assert_eq!(metrics.purchase_count, 1);
        assert_eq!(metrics.ad_impressions, 1);
        assert_eq!(metrics.ad_clicks, 1);
        assert_eq!(metrics.ad_rewards_claimed, 1);
        assert_eq!(metrics.total_spend, 9.99);
    }

    #[test]
    fn test_telemetry_system_creation() {
        let consent = PrivacyConsent::new(PrivacyRegion::Gdpr)
            .with_analytics(true)
            .with_performance(true);
        let telemetry = TelemetrySystem::new(consent);
        assert!(!telemetry.is_initialized());
        assert_eq!(telemetry.queue_size(), 0);
    }

    #[test]
    fn test_telemetry_stats() {
        let consent = PrivacyConsent::new(PrivacyRegion::Other);
        let telemetry = TelemetrySystem::new(consent);
        let stats = telemetry.stats();
        assert_eq!(stats.events_enqueued, 0);
        assert_eq!(stats.sessions_started, 0);
        assert_eq!(stats.sessions_ended, 0);
    }

    #[test]
    fn test_crash_type_from_signal() {
        assert_eq!(CrashType::from_signal(Some(11)), CrashType::Segfault);
        assert_eq!(CrashType::from_signal(Some(4)), CrashType::IllegalInstruction);
        assert_eq!(CrashType::from_signal(Some(7)), CrashType::BusError);
        assert_eq!(CrashType::from_signal(Some(8)), CrashType::FloatingPoint);
        assert_eq!(CrashType::from_signal(Some(6)), CrashType::Abort);
        assert_eq!(CrashType::from_signal(None), CrashType::Unknown);
    }

    #[test]
    fn test_report_window_display() {
        assert_eq!(format!("{}", ReportWindow::Daily), "daily");
        assert_eq!(format!("{}", ReportWindow::Weekly), "weekly");
        assert_eq!(format!("{}", ReportWindow::Monthly), "monthly");
    }

    #[test]
    fn test_webhook_callback() {
        let webhook = WebhookCallback::new("https://example.com/webhook")
            .with_header("Authorization", "Bearer token")
            .with_timeout(Duration::from_secs(5))
            .with_retry(3, Duration::from_secs(1));

        assert_eq!(webhook.url, "https://example.com/webhook");
        assert_eq!(webhook.headers.get("Authorization").unwrap(), "Bearer token");
        assert_eq!(webhook.timeout, Duration::from_secs(5));
        assert_eq!(webhook.retry_count, 3);
    }

    #[test]
    fn test_local_analytics_storage() {
        let temp_dir = std::env::temp_dir().join("elysium_test_analytics");
        let storage = LocalAnalyticsStorage::new(&temp_dir)
            .with_max_file_size(1024 * 1024)
            .with_compression(false);

        let events = vec![AnalyticsEvent::SessionStarted {
            session_id: "test".to_string(),
            timestamp: 0,
            platform: "pc".to_string(),
            build: "1.0".to_string(),
            region: PrivacyRegion::Other,
        }];

        let path = storage.persist_batch(&events).unwrap();
        assert!(path.exists());

        let loaded = storage.load_all().unwrap();
        assert_eq!(loaded.len(), 1);

        let _ = storage.clear();
        assert!(!temp_dir.exists());
    }

    #[test]
    fn test_analytics_report_creation() {
        let report = AnalyticsReport::new(ReportWindow::Daily);
        assert!(report.report_id.len() > 0);
        assert!(matches!(report.window, ReportWindow::Daily));
        assert_eq!(report.session_count, 0);
        assert_eq!(report.total_events, 0);
    }

    #[test]
    fn test_funnel_analysis_default() {
        let funnel = FunnelAnalysis::default();
        assert_eq!(funnel.stages.len(), 0);
        assert_eq!(funnel.window, ReportWindow::Daily); // Default derives this
    }

    #[test]
    fn test_retention_analysis_default() {
        let retention = RetentionAnalysis::default();
        assert_eq!(retention.day_retention.len(), 0);
    }

    #[test]
    fn test_heatmap_data_default() {
        let heatmap = HeatmapData::default();
        assert_eq!(heatmap.cells.len(), 0);
        assert_eq!(heatmap.screen_width, 0);
        assert_eq!(heatmap.screen_height, 0);
    }

    #[test]
    fn test_crash_correlation_default() {
        let correlation = CrashCorrelationReport::default();
        assert_eq!(correlation.total_crashes, 0);
        assert_eq!(correlation.crashes_by_module.len(), 0);
    }
}

