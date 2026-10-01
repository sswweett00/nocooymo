//! # Cloud Services and UGC Tools for Elysium Engine
//!
//! Provides a complete cloud services and UGC system including:
//! - Cloud save with Steam Cloud, Epic Online Services, and custom REST provider
//! - Save game sync and conflict resolution
//! - Matchmaking and lobby management
//! - Party system and friend invites
//! - Leaderboard service
//! - User generated content (UGC) platform
//! - UGC upload, download, rating, and review
//! - Workshop integration hooks
//! - In-app purchases (IAP) and receipt validation
//! - Ad integration hooks
//! - Identity service with cross-platform authentication

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

// Re-export save game platform type used by cloud save slots.
pub use crate::save_game::Platform as SavePlatform;

// ============================================================================
// Error types
// ============================================================================

/// Errors that can occur in the cloud services system.
#[derive(Debug, Error)]
pub enum CloudError {
    #[error("service not initialized")]
    NotInitialized,

    #[error("service already initialized")]
    AlreadyInitialized,

    #[error("authentication required")]
    AuthRequired,

    #[error("authentication failed: {0}")]
    AuthFailed(String),

    #[error("network error: {0}")]
    NetworkError(String),

    #[error("cloud save conflict: {0}")]
    SaveConflict(String),

    #[error("save slot {0} not found")]
    SlotNotFound(u32),

    #[error("matchmaking request timed out")]
    MatchmakingTimeout,

    #[error("lobby {0} not found")]
    LobbyNotFound(String),

    #[error("user {0} not found")]
    UserNotFound(String),

    #[error("UGM item {0} not found")]
    UgcItemNotFound(String),

    #[error("purchase failed: {0}")]
    PurchaseFailed(String),

    #[error("receipt validation failed: {0}")]
    ReceiptValidationFailed(String),

    #[error("rate limit exceeded")]
    RateLimited,

    #[error("serialization error: {0}")]
    SerializationError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type CloudResult<T> = Result<T, CloudError>;

// ============================================================================
// Cloud Save
// ============================================================================

/// Cloud save slot metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSaveSlot {
    pub slot_id: u32,
    pub name: String,
    pub platform: SavePlatform,
    pub last_modified: u64,
    pub size_bytes: u64,
    pub is_auto_save: bool,
    pub tags: Vec<String>,
}

impl CloudSaveSlot {
    pub fn new(slot_id: u32, name: impl Into<String>) -> Self {
        let timestamp = current_unix_ts();
        Self {
            slot_id,
            name: name.into(),
            platform: SavePlatform::default(),
            last_modified: timestamp,
            size_bytes: 0,
            is_auto_save: false,
            tags: Vec::new(),
        }
    }
}

/// Conflict resolution strategy when cloud and local saves differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolution {
    LocalWins,
    CloudWins,
    NewestWins,
    AskUser,
}

impl Default for ConflictResolution {
    fn default() -> Self {
        Self::NewestWins
    }
}

impl fmt::Display for ConflictResolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LocalWins => write!(f, "local_wins"),
            Self::CloudWins => write!(f, "cloud_wins"),
            Self::NewestWins => write!(f, "newest_wins"),
            Self::AskUser => write!(f, "ask_user"),
        }
    }
}

/// Save conflict information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveConflict {
    pub slot_id: u32,
    pub local_timestamp: u64,
    pub cloud_timestamp: u64,
    pub local_size_bytes: u64,
    pub cloud_size_bytes: u64,
    pub resolution: ConflictResolution,
}

impl SaveConflict {
    pub fn new(slot_id: u32) -> Self {
        Self {
            slot_id,
            local_timestamp: 0,
            cloud_timestamp: 0,
            local_size_bytes: 0,
            cloud_size_bytes: 0,
            resolution: ConflictResolution::default(),
        }
    }
}

/// Abstraction over cloud save backends.
pub trait CloudSaveService: Send + Sync {
    fn name(&self) -> &str;
    fn is_available(&self) -> bool;

    fn upload(&self, slot_id: u32, data: &[u8]) -> CloudResult<()>;
    fn download(&self, slot_id: u32) -> CloudResult<Option<Vec<u8>>>;
    fn delete(&self, slot_id: u32) -> CloudResult<()>;

    fn list_slots(&self) -> CloudResult<Vec<CloudSaveSlot>>;
    fn slot_metadata(&self, slot_id: u32) -> CloudResult<Option<CloudSaveSlot>>;

    fn resolve_conflict(&self, conflict: SaveConflict) -> CloudResult<Vec<u8>>;
}

/// No-op cloud save service used when cloud saves are disabled.
#[derive(Clone, Debug, Default)]
pub struct NoopCloudSaveService;

impl CloudSaveService for NoopCloudSaveService {
    fn name(&self) -> &str {
        "noop"
    }

    fn is_available(&self) -> bool {
        false
    }

    fn upload(&self, _slot_id: u32, _data: &[u8]) -> CloudResult<()> {
        Ok(())
    }

    fn download(&self, _slot_id: u32) -> CloudResult<Option<Vec<u8>>> {
        Ok(None)
    }

    fn delete(&self, _slot_id: u32) -> CloudResult<()> {
        Ok(())
    }

    fn list_slots(&self) -> CloudResult<Vec<CloudSaveSlot>> {
        Ok(Vec::new())
    }

    fn slot_metadata(&self, _slot_id: u32) -> CloudResult<Option<CloudSaveSlot>> {
        Ok(None)
    }

    fn resolve_conflict(&self, conflict: SaveConflict) -> CloudResult<Vec<u8>> {
        warn!(slot_id = conflict.slot_id, "noop cloud save: cannot resolve conflict");
        Ok(Vec::new())
    }
}

// ============================================================================
// Cloud Save Providers
// ============================================================================

/// Steam Cloud save provider (placeholder implementation).
#[derive(Clone, Debug, Default)]
pub struct SteamCloudProvider {
    app_id: u32,
    user_id: Option<u32>,
    enabled: bool,
}

impl SteamCloudProvider {
    pub fn new(app_id: u32) -> Self {
        Self {
            app_id,
            user_id: None,
            enabled: true,
        }
    }

    pub fn with_user(mut self, user_id: u32) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl CloudSaveService for SteamCloudProvider {
    fn name(&self) -> &str {
        "steam_cloud"
    }

    fn is_available(&self) -> bool {
        self.enabled && self.user_id.is_some()
    }

    fn upload(&self, slot_id: u32, data: &[u8]) -> CloudResult<()> {
        if !self.is_available() {
            return Err(CloudError::AuthRequired);
        }
        debug!(app_id = self.app_id, slot_id, bytes = data.len(), "Steam Cloud: uploading save");
        Ok(())
    }

    fn download(&self, slot_id: u32) -> CloudResult<Option<Vec<u8>>> {
        if !self.is_available() {
            return Err(CloudError::AuthRequired);
        }
        debug!(app_id = self.app_id, slot_id, "Steam Cloud: downloading save");
        Ok(None)
    }

    fn delete(&self, slot_id: u32) -> CloudResult<()> {
        if !self.is_available() {
            return Err(CloudError::AuthRequired);
        }
        debug!(app_id = self.app_id, slot_id, "Steam Cloud: deleting save");
        Ok(())
    }

    fn list_slots(&self) -> CloudResult<Vec<CloudSaveSlot>> {
        if !self.is_available() {
            return Err(CloudError::AuthRequired);
        }
        Ok(Vec::new())
    }

    fn slot_metadata(&self, _slot_id: u32) -> CloudResult<Option<CloudSaveSlot>> {
        if !self.is_available() {
            return Err(CloudError::AuthRequired);
        }
        Ok(None)
    }

    fn resolve_conflict(&self, conflict: SaveConflict) -> CloudResult<Vec<u8>> {
        warn!(slot_id = conflict.slot_id, "Steam Cloud: conflict resolution not implemented");
        Ok(Vec::new())
    }
}

/// Epic Online Services cloud save provider (placeholder implementation).
#[derive(Clone, Debug, Default)]
pub struct EosCloudProvider {
    deployment_id: Option<String>,
    user_id: Option<String>,
    enabled: bool,
}

impl EosCloudProvider {
    pub fn new(deployment_id: impl Into<String>) -> Self {
        Self {
            deployment_id: Some(deployment_id.into()),
            user_id: None,
            enabled: true,
        }
    }

    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl CloudSaveService for EosCloudProvider {
    fn name(&self) -> &str {
        "eos_cloud"
    }

    fn is_available(&self) -> bool {
        self.enabled && self.user_id.is_some()
    }

    fn upload(&self, slot_id: u32, data: &[u8]) -> CloudResult<()> {
        if !self.is_available() {
            return Err(CloudError::AuthRequired);
        }
        debug!(slot_id, bytes = data.len(), "EOS Cloud: uploading save");
        Ok(())
    }

    fn download(&self, slot_id: u32) -> CloudResult<Option<Vec<u8>>> {
        if !self.is_available() {
            return Err(CloudError::AuthRequired);
        }
        debug!(slot_id, "EOS Cloud: downloading save");
        Ok(None)
    }

    fn delete(&self, slot_id: u32) -> CloudResult<()> {
        if !self.is_available() {
            return Err(CloudError::AuthRequired);
        }
        debug!(slot_id, "EOS Cloud: deleting save");
        Ok(())
    }

    fn list_slots(&self) -> CloudResult<Vec<CloudSaveSlot>> {
        if !self.is_available() {
            return Err(CloudError::AuthRequired);
        }
        Ok(Vec::new())
    }

    fn slot_metadata(&self, _slot_id: u32) -> CloudResult<Option<CloudSaveSlot>> {
        if !self.is_available() {
            return Err(CloudError::AuthRequired);
        }
        Ok(None)
    }

    fn resolve_conflict(&self, conflict: SaveConflict) -> CloudResult<Vec<u8>> {
        warn!(slot_id = conflict.slot_id, "EOS Cloud: conflict resolution not implemented");
        Ok(Vec::new())
    }
}

/// Custom REST cloud save provider (placeholder implementation).
#[derive(Clone, Debug, Default)]
pub struct RestCloudProvider {
    base_url: String,
    api_key: Option<String>,
    user_token: Option<String>,
    enabled: bool,
}

impl RestCloudProvider {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: None,
            user_token: None,
            enabled: true,
        }
    }

    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    pub fn with_user_token(mut self, token: impl Into<String>) -> Self {
        self.user_token = Some(token.into());
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl CloudSaveService for RestCloudProvider {
    fn name(&self) -> &str {
        "rest_cloud"
    }

    fn is_available(&self) -> bool {
        self.enabled && self.user_token.is_some()
    }

    fn upload(&self, slot_id: u32, data: &[u8]) -> CloudResult<()> {
        if !self.is_available() {
            return Err(CloudError::AuthRequired);
        }
        debug!(url = %self.base_url, slot_id, bytes = data.len(), "REST Cloud: uploading save");
        Ok(())
    }

    fn download(&self, slot_id: u32) -> CloudResult<Option<Vec<u8>>> {
        if !self.is_available() {
            return Err(CloudError::AuthRequired);
        }
        debug!(url = %self.base_url, slot_id, "REST Cloud: downloading save");
        Ok(None)
    }

    fn delete(&self, slot_id: u32) -> CloudResult<()> {
        if !self.is_available() {
            return Err(CloudError::AuthRequired);
        }
        debug!(url = %self.base_url, slot_id, "REST Cloud: deleting save");
        Ok(())
    }

    fn list_slots(&self) -> CloudResult<Vec<CloudSaveSlot>> {
        if !self.is_available() {
            return Err(CloudError::AuthRequired);
        }
        Ok(Vec::new())
    }

    fn slot_metadata(&self, _slot_id: u32) -> CloudResult<Option<CloudSaveSlot>> {
        if !self.is_available() {
            return Err(CloudError::AuthRequired);
        }
        Ok(None)
    }

    fn resolve_conflict(&self, conflict: SaveConflict) -> CloudResult<Vec<u8>> {
        warn!(slot_id = conflict.slot_id, "REST Cloud: conflict resolution not implemented");
        Ok(Vec::new())
    }
}

// ============================================================================
// Matchmaking / Lobby
// ============================================================================

/// Matchmaking game mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameMode {
    Solo,
    Coop,
    Pvp,
    Custom,
}

impl Default for GameMode {
    fn default() -> Self {
        Self::Solo
    }
}

/// Skill rating / MMR tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillTier {
    Bronze,
    Silver,
    Gold,
    Platinum,
    Diamond,
    Master,
}

impl Default for SkillTier {
    fn default() -> Self {
        Self::Bronze
    }
}

/// Matchmaking request parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchmakingRequest {
    pub request_id: String,
    pub user_id: String,
    pub game_mode: GameMode,
    pub region: String,
    pub skill_tier: SkillTier,
    pub party_size: u32,
    pub max_party_size: u32,
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub filters: HashMap<String, String>,
}

impl MatchmakingRequest {
    pub fn new(user_id: impl Into<String>, game_mode: GameMode) -> Self {
        let now = current_unix_ts();
        Self {
            request_id: Uuid::new_v4().to_string(),
            user_id: user_id.into(),
            game_mode,
            region: "default".to_string(),
            skill_tier: SkillTier::default(),
            party_size: 1,
            max_party_size: 4,
            created_at: now,
            expires_at: Some(now + 300),
            filters: HashMap::new(),
        }
    }

    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = region.into();
        self
    }

    pub fn with_skill_tier(mut self, tier: SkillTier) -> Self {
        self.skill_tier = tier;
        self
    }

    pub fn with_party_size(mut self, size: u32, max: u32) -> Self {
        self.party_size = size;
        self.max_party_size = max;
        self
    }
}

/// Matchmaking result status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchmakingStatus {
    Searching,
    Matched,
    Cancelled,
    Expired,
    Failed,
}

impl Default for MatchmakingStatus {
    fn default() -> Self {
        Self::Searching
    }
}

impl fmt::Display for MatchmakingStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Searching => write!(f, "searching"),
            Self::Matched => write!(f, "matched"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Expired => write!(f, "expired"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

/// Matchmaking result containing lobby information when matched.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchmakingResult {
    pub request_id: String,
    pub status: MatchmakingStatus,
    pub lobby: Option<Lobby>,
    pub search_duration_ms: u64,
}

impl MatchmakingResult {
    pub fn matched(request_id: impl Into<String>, lobby: Lobby, duration_ms: u64) -> Self {
        Self {
            request_id: request_id.into(),
            status: MatchmakingStatus::Matched,
            lobby: Some(lobby),
            search_duration_ms: duration_ms,
        }
    }

    pub fn searching(request_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            status: MatchmakingStatus::Searching,
            lobby: None,
            search_duration_ms: 0,
        }
    }
}

/// Lobby state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LobbyState {
    Open,
    InGame,
    Closed,
}

impl Default for LobbyState {
    fn default() -> Self {
        Self::Open
    }
}

impl fmt::Display for LobbyState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open => write!(f, "open"),
            Self::InGame => write!(f, "in_game"),
            Self::Closed => write!(f, "closed"),
        }
    }
}

/// Lobby slot state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotState {
    Open,
    Taken,
    Reserved,
    Closed,
}

impl Default for SlotState {
    fn default() -> Self {
        Self::Open
    }
}

/// Lobby slot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbySlot {
    pub slot_index: u32,
    pub user_id: Option<String>,
    pub state: SlotState,
    pub is_host: bool,
}

impl LobbySlot {
    pub fn open(slot_index: u32) -> Self {
        Self {
            slot_index,
            user_id: None,
            state: SlotState::Open,
            is_host: false,
        }
    }

    pub fn taken(slot_index: u32, user_id: impl Into<String>, is_host: bool) -> Self {
        Self {
            slot_index,
            user_id: Some(user_id.into()),
            state: SlotState::Taken,
            is_host,
        }
    }
}

/// Lobby configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbyConfig {
    pub max_players: u32,
    pub is_private: bool,
    pub password: Option<String>,
    pub region: String,
    pub game_mode: GameMode,
    pub settings: HashMap<String, String>,
}

impl Default for LobbyConfig {
    fn default() -> Self {
        Self {
            max_players: 4,
            is_private: false,
            password: None,
            region: "default".to_string(),
            game_mode: GameMode::default(),
            settings: HashMap::new(),
        }
    }
}

/// Lobby summary for public listings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbySummary {
    pub lobby_id: String,
    pub name: String,
    pub host_id: String,
    pub state: LobbyState,
    pub current_players: u32,
    pub max_players: u32,
    pub region: String,
    pub game_mode: GameMode,
    pub ping_ms: u32,
    pub is_full: bool,
    pub created_at: u64,
}

impl LobbySummary {
    pub fn new(lobby: &Lobby) -> Self {
        Self {
            lobby_id: lobby.lobby_id.clone(),
            name: lobby.name.clone(),
            host_id: lobby.host_id.clone(),
            state: lobby.state,
            current_players: lobby.slots.iter().filter(|s| s.state == SlotState::Taken).count() as u32,
            max_players: lobby.config.max_players,
            region: lobby.config.region.clone(),
            game_mode: lobby.config.game_mode,
            ping_ms: lobby.ping_ms,
            is_full: lobby.is_full(),
            created_at: lobby.created_at,
        }
    }
}

/// Lobby room.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lobby {
    pub lobby_id: String,
    pub name: String,
    pub host_id: String,
    pub state: LobbyState,
    pub slots: Vec<LobbySlot>,
    pub config: LobbyConfig,
    pub ping_ms: u32,
    pub created_at: u64,
    pub metadata: HashMap<String, String>,
}

impl Lobby {
    pub fn new(name: impl Into<String>, host_id: impl Into<String>, config: LobbyConfig) -> Self {
        let now = current_unix_ts();
        Self {
            lobby_id: Uuid::new_v4().to_string(),
            name: name.into(),
            host_id: host_id.into(),
            state: LobbyState::Open,
            slots: vec![LobbySlot::taken(0, &host_id.into(), true)],
            config,
            ping_ms: 0,
            created_at: now,
            metadata: HashMap::new(),
        }
    }

    pub fn is_full(&self) -> bool {
        self.slots.iter().filter(|s| s.state == SlotState::Taken).count() as u32 >= self.config.max_players
    }

    pub fn player_count(&self) -> u32 {
        self.slots.iter().filter(|s| s.state == SlotState::Taken).count() as u32
    }
}

/// Party member.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyMember {
    pub user_id: String,
    pub display_name: String,
    pub is_leader: bool,
    pub is_ready: bool,
    pub skill_tier: SkillTier,
}

impl PartyMember {
    pub fn leader(user_id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            display_name: display_name.into(),
            is_leader: true,
            is_ready: false,
            skill_tier: SkillTier::default(),
        }
    }

    pub fn member(user_id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            display_name: display_name.into(),
            is_leader: false,
            is_ready: false,
            skill_tier: SkillTier::default(),
        }
    }
}

/// Party group for matchmaking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    pub party_id: String,
    pub members: Vec<PartyMember>,
    pub max_size: u32,
    pub is_public: bool,
}

impl Party {
    pub fn new(leader: PartyMember, max_size: u32) -> Self {
        Self {
            party_id: Uuid::new_v4().to_string(),
            members: vec![leader],
            max_size,
            is_public: true,
        }
    }

    pub fn is_full(&self) -> bool {
        self.members.len() as u32 >= self.max_size
    }
}

/// Friend invite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendInvite {
    pub invite_id: String,
    pub from_user_id: String,
    pub to_user_id: String,
    pub lobby_id: Option<String>,
    pub message: Option<String>,
    pub created_at: u64,
    pub expires_at: u64,
    pub status: InviteStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InviteStatus {
    Pending,
    Accepted,
    Declined,
    Expired,
    Cancelled,
}

impl Default for InviteStatus {
    fn default() -> Self {
        Self::Pending
    }
}

impl FriendInvite {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        let now = current_unix_ts();
        Self {
            invite_id: Uuid::new_v4().to_string(),
            from_user_id: from.into(),
            to_user_id: to.into(),
            lobby_id: None,
            message: None,
            created_at: now,
            expires_at: now + 86400,
            status: InviteStatus::default(),
        }
    }

    pub fn with_lobby(mut self, lobby_id: impl Into<String>) -> Self {
        self.lobby_id = Some(lobby_id.into());
        self
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

/// Trait for matchmaking and lobby services.
pub trait MatchmakingService: Send + Sync {
    fn name(&self) -> &str;
    fn is_available(&self) -> bool;

    fn start_matchmaking(&self, request: MatchmakingRequest) -> CloudResult<MatchmakingResult>;
    fn cancel_matchmaking(&self, request_id: &str) -> CloudResult<()>;
    fn get_matchmaking_status(&self, request_id: &str) -> CloudResult<Option<MatchmakingStatus>>;

    fn create_lobby(&self, config: LobbyConfig) -> CloudResult<Lobby>;
    fn join_lobby(&self, lobby_id: &str, user_id: impl Into<String>) -> CloudResult<()>;
    fn leave_lobby(&self, lobby_id: &str, user_id: impl Into<String>) -> CloudResult<()>;
    fn get_lobby(&self, lobby_id: &str) -> CloudResult<Option<Lobby>>;
    fn list_lobbies(&self, game_mode: GameMode) -> CloudResult<Vec<LobbySummary>>;
    fn update_lobby_state(&self, lobby_id: &str, state: LobbyState) -> CloudResult<()>;

    fn send_invite(&self, invite: FriendInvite) -> CloudResult<()>;
    fn accept_invite(&self, invite_id: &str) -> CloudResult<()>;
    fn decline_invite(&self, invite_id: &str) -> CloudResult<()>;
    fn get_pending_invites(&self, user_id: &str) -> CloudResult<Vec<FriendInvite>>;

    fn create_party(&self, leader: PartyMember) -> CloudResult<Party>;
    fn join_party(&self, party_id: &str, user_id: impl Into<String>) -> CloudResult<()>;
    fn leave_party(&self, party_id: &str, user_id: impl Into<String>) -> CloudResult<()>;
    fn get_party(&self, party_id: &str) -> CloudResult<Option<Party>>;
}

/// No-op matchmaking service.
#[derive(Clone, Debug, Default)]
pub struct NoopMatchmakingService;

impl MatchmakingService for NoopMatchmakingService {
    fn name(&self) -> &str {
        "noop"
    }

    fn is_available(&self) -> bool {
        false
    }

    fn start_matchmaking(&self, request: MatchmakingRequest) -> CloudResult<MatchmakingResult> {
        Ok(MatchmakingResult::searching(request.request_id))
    }

    fn cancel_matchmaking(&self, _request_id: &str) -> CloudResult<()> {
        Ok(())
    }

    fn get_matchmaking_status(&self, _request_id: &str) -> CloudResult<Option<MatchmakingStatus>> {
        Ok(None)
    }

    fn create_lobby(&self, config: LobbyConfig) -> CloudResult<Lobby> {
        let lobby = Lobby::new("Lobby", "system", config);
        Ok(lobby)
    }

    fn join_lobby(&self, _lobby_id: &str, _user_id: impl Into<String>) -> CloudResult<()> {
        Ok(())
    }

    fn leave_lobby(&self, _lobby_id: &str, _user_id: impl Into<String>) -> CloudResult<()> {
        Ok(())
    }

    fn get_lobby(&self, _lobby_id: &str) -> CloudResult<Option<Lobby>> {
        Ok(None)
    }

    fn list_lobbies(&self, _game_mode: GameMode) -> CloudResult<Vec<LobbySummary>> {
        Ok(Vec::new())
    }

    fn update_lobby_state(&self, _lobby_id: &str, _state: LobbyState) -> CloudResult<()> {
        Ok(())
    }

    fn send_invite(&self, invite: FriendInvite) -> CloudResult<()> {
        debug!(invite_id = %invite.invite_id, "noop matchmaking: invite queued");
        Ok(())
    }

    fn accept_invite(&self, _invite_id: &str) -> CloudResult<()> {
        Ok(())
    }

    fn decline_invite(&self, _invite_id: &str) -> CloudResult<()> {
        Ok(())
    }

    fn get_pending_invites(&self, _user_id: &str) -> CloudResult<Vec<FriendInvite>> {
        Ok(Vec::new())
    }

    fn create_party(&self, leader: PartyMember) -> CloudResult<Party> {
        let party = Party::new(leader, 4);
        Ok(party)
    }

    fn join_party(&self, _party_id: &str, _user_id: impl Into<String>) -> CloudResult<()> {
        Ok(())
    }

    fn leave_party(&self, _party_id: &str, _user_id: impl Into<String>) -> CloudResult<()> {
        Ok(())
    }

    fn get_party(&self, _party_id: &str) -> CloudResult<Option<Party>> {
        Ok(None)
    }
}

// ============================================================================
// Leaderboards
// ============================================================================

/// Leaderboard type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaderboardType {
    Global,
    Friends,
    Country,
    Weekly,
    Daily,
}

impl Default for LeaderboardType {
    fn default() -> Self {
        Self::Global
    }
}

/// Leaderboard entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub rank: u32,
    pub user_id: String,
    pub display_name: String,
    pub score: i64,
    pub metadata: HashMap<String, String>,
    pub updated_at: u64,
}

impl LeaderboardEntry {
    pub fn new(rank: u32, user_id: impl Into<String>, score: i64) -> Self {
        Self {
            rank,
            user_id: user_id.into(),
            display_name: String::new(),
            score,
            metadata: HashMap::new(),
            updated_at: current_unix_ts(),
        }
    }

    pub fn with_display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = name.into();
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Leaderboard definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub lb_type: LeaderboardType,
    pub sort_order: SortOrder,
    pub max_entries: u32,
    pub is_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    HighToLow,
    LowToHigh,
}

impl Default for SortOrder {
    fn default() -> Self {
        Self::HighToLow
    }
}

impl LeaderboardDefinition {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            lb_type: LeaderboardType::default(),
            sort_order: SortOrder::default(),
            max_entries: 100,
            is_enabled: true,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_type(mut self, lb_type: LeaderboardType) -> Self {
        self.lb_type = lb_type;
        self
    }

    pub fn with_max_entries(mut self, max: u32) -> Self {
        self.max_entries = max;
        self
    }
}

/// Trait for leaderboard services.
pub trait LeaderboardService: Send + Sync {
    fn name(&self) -> &str;
    fn is_available(&self) -> bool;

    fn create_leaderboard(&self, def: LeaderboardDefinition) -> CloudResult<()>;
    fn submit_score(&self, leaderboard_id: &str, user_id: impl Into<String>, score: i64, metadata: HashMap<String, String>) -> CloudResult<()>;
    fn get_rank(&self, leaderboard_id: &str, user_id: &str) -> CloudResult<Option<u32>>;
    fn get_entries(&self, leaderboard_id: &str, lb_type: LeaderboardType, offset: u32, limit: u32) -> CloudResult<Vec<LeaderboardEntry>>;
    fn get_user_entry(&self, leaderboard_id: &str, user_id: &str) -> CloudResult<Option<LeaderboardEntry>>;
    fn reset_leaderboard(&self, leaderboard_id: &str) -> CloudResult<()>;
}

/// No-op leaderboard service.
#[derive(Clone, Debug, Default)]
pub struct NoopLeaderboardService;

impl LeaderboardService for NoopLeaderboardService {
    fn name(&self) -> &str {
        "noop"
    }

    fn is_available(&self) -> bool {
        false
    }

    fn create_leaderboard(&self, _def: LeaderboardDefinition) -> CloudResult<()> {
        Ok(())
    }

    fn submit_score(&self, _leaderboard_id: &str, _user_id: impl Into<String>, _score: i64, _metadata: HashMap<String, String>) -> CloudResult<()> {
        Ok(())
    }

    fn get_rank(&self, _leaderboard_id: &str, _user_id: &str) -> CloudResult<Option<u32>> {
        Ok(None)
    }

    fn get_entries(&self, _leaderboard_id: &str, _lb_type: LeaderboardType, _offset: u32, _limit: u32) -> CloudResult<Vec<LeaderboardEntry>> {
        Ok(Vec::new())
    }

    fn get_user_entry(&self, _leaderboard_id: &str, _user_id: &str) -> CloudResult<Option<LeaderboardEntry>> {
        Ok(None)
    }

    fn reset_leaderboard(&self, _leaderboard_id: &str) -> CloudResult<()> {
        Ok(())
    }
}

// ============================================================================
// User Generated Content
// ============================================================================

/// UGC item visibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UgcVisibility {
    Private,
    Public,
    FriendsOnly,
    Unlisted,
}

impl Default for UgcVisibility {
    fn default() -> Self {
        Self::Private
    }
}

impl fmt::Display for UgcVisibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Private => write!(f, "private"),
            Self::Public => write!(f, "public"),
            Self::FriendsOnly => write!(f, "friends_only"),
            Self::Unlisted => write!(f, "unlisted"),
        }
    }
}

/// UGC file metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UgcFile {
    pub filename: String,
    pub content_type: String,
    pub size_bytes: u64,
    pub checksum: String,
    pub url: Option<String>,
}

impl UgcFile {
    pub fn new(filename: impl Into<String>, content_type: impl Into<String>, size_bytes: u64) -> Self {
        Self {
            filename: filename.into(),
            content_type: content_type.into(),
            size_bytes,
            checksum: String::new(),
            url: None,
        }
    }
}

/// UGC item metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UgcItem {
    pub item_id: String,
    pub title: String,
    pub description: String,
    pub author_id: String,
    pub author_name: String,
    pub files: Vec<UgcFile>,
    pub tags: Vec<String>,
    pub visibility: UgcVisibility,
    pub created_at: u64,
    pub updated_at: u64,
    pub download_count: u64,
    pub rating_sum: u64,
    pub rating_count: u64,
    pub metadata: HashMap<String, String>,
}

impl UgcItem {
    pub fn new(title: impl Into<String>, author_id: impl Into<String>, author_name: impl Into<String>) -> Self {
        let now = current_unix_ts();
        Self {
            item_id: Uuid::new_v4().to_string(),
            title: title.into(),
            description: String::new(),
            author_id: author_id.into(),
            author_name: author_name.into(),
            files: Vec::new(),
            tags: Vec::new(),
            visibility: UgcVisibility::default(),
            created_at: now,
            updated_at: now,
            download_count: 0,
            rating_sum: 0,
            rating_count: 0,
            metadata: HashMap::new(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_file(mut self, file: UgcFile) -> Self {
        self.files.push(file);
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn with_visibility(mut self, visibility: UgcVisibility) -> Self {
        self.visibility = visibility;
        self
    }

    pub fn average_rating(&self) -> f32 {
        if self.rating_count == 0 {
            0.0
        } else {
            self.rating_sum as f32 / self.rating_count as f32
        }
    }
}

/// UGC search query.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UgcSearchQuery {
    pub query: String,
    pub author_id: Option<String>,
    pub tags: Vec<String>,
    pub visibility: Option<UgcVisibility>,
    pub sort_by: UgcSortBy,
    pub offset: u32,
    pub limit: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UgcSortBy {
    Newest,
    Oldest,
    Popular,
    Rating,
    Downloaded,
}

impl Default for UgcSortBy {
    fn default() -> Self {
        Self::Newest
    }
}

impl UgcSearchQuery {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            ..Default::default()
        }
    }

    pub fn with_author(mut self, author_id: impl Into<String>) -> Self {
        self.author_id = Some(author_id.into());
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn with_sort(mut self, sort_by: UgcSortBy) -> Self {
        self.sort_by = sort_by;
        self
    }

    pub fn with_limit(mut self, offset: u32, limit: u32) -> Self {
        self.offset = offset;
        self.limit = limit;
        self
    }
}

/// UGC search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UgcSearchResult {
    pub items: Vec<UgcItem>,
    pub total_count: u32,
    pub offset: u32,
    pub has_more: bool,
}

impl UgcSearchResult {
    pub fn empty() -> Self {
        Self {
            items: Vec::new(),
            total_count: 0,
            offset: 0,
            has_more: false,
        }
    }
}

/// UGC review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UgcReview {
    pub review_id: String,
    pub item_id: String,
    pub user_id: String,
    pub user_name: String,
    pub rating: u32,
    pub comment: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub helpful_count: u64,
}

impl UgcReview {
    pub fn new(item_id: impl Into<String>, user_id: impl Into<String>, user_name: impl Into<String>, rating: u32) -> Self {
        let now = current_unix_ts();
        Self {
            review_id: Uuid::new_v4().to_string(),
            item_id: item_id.into(),
            user_id: user_id.into(),
            user_name: user_name.into(),
            rating: rating.clamp(1, 5),
            comment: String::new(),
            created_at: now,
            updated_at: now,
            helpful_count: 0,
        }
    }

    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = comment.into();
        self
    }
}

/// Workshop integration hooks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkshopSubscription {
    pub user_id: String,
    pub item_id: String,
    pub subscribed_at: u64,
    pub notify_on_update: bool,
}

impl WorkshopSubscription {
    pub fn new(user_id: impl Into<String>, item_id: impl Into<String>) -> Self {
        let now = current_unix_ts();
        Self {
            user_id: user_id.into(),
            item_id: item_id.into(),
            subscribed_at: now,
            notify_on_update: true,
        }
    }
}

/// Trait for user generated content platforms.
pub trait UgcPlatform: Send + Sync {
    fn name(&self) -> &str;
    fn is_available(&self) -> bool;

    fn upload_item(&self, item: UgcItem, files: Vec<UgcFile>) -> CloudResult<String>;
    fn download_item(&self, item_id: &str) -> CloudResult<Option<UgcItem>>;
    fn download_file(&self, item_id: &str, filename: &str) -> CloudResult<Option<Vec<u8>>>;
    fn delete_item(&self, item_id: &str, author_id: &str) -> CloudResult<()>;
    fn update_item(&self, item_id: &str, author_id: &str, updates: UgcItem) -> CloudResult<()>;

    fn search(&self, query: UgcSearchQuery) -> CloudResult<UgcSearchResult>;
    fn get_item(&self, item_id: &str) -> CloudResult<Option<UgcItem>>;

    fn rate_item(&self, item_id: &str, user_id: &str, rating: u32) -> CloudResult<()>;
    fn submit_review(&self, review: UgcReview) -> CloudResult<()>;
    fn get_reviews(&self, item_id: &str, offset: u32, limit: u32) -> CloudResult<Vec<UgcReview>>;

    fn subscribe(&self, subscription: WorkshopSubscription) -> CloudResult<()>;
    fn unsubscribe(&self, user_id: &str, item_id: &str) -> CloudResult<()>;
    fn get_subscriptions(&self, user_id: &str) -> CloudResult<Vec<WorkshopSubscription>>;
    fn get_subscriber_count(&self, item_id: &str) -> CloudResult<u64>;

    fn increment_download(&self, item_id: &str) -> CloudResult<()>;
}

/// No-op UGC platform.
#[derive(Clone, Debug, Default)]
pub struct NoopUgcPlatform;

impl UgcPlatform for NoopUgcPlatform {
    fn name(&self) -> &str {
        "noop"
    }

    fn is_available(&self) -> bool {
        false
    }

    fn upload_item(&self, item: UgcItem, _files: Vec<UgcFile>) -> CloudResult<String> {
        debug!(title = %item.title, "noop UGC: upload queued");
        Ok(item.item_id)
    }

    fn download_item(&self, _item_id: &str) -> CloudResult<Option<UgcItem>> {
        Ok(None)
    }

    fn download_file(&self, _item_id: &str, _filename: &str) -> CloudResult<Option<Vec<u8>>> {
        Ok(None)
    }

    fn delete_item(&self, _item_id: &str, _author_id: &str) -> CloudResult<()> {
        Ok(())
    }

    fn update_item(&self, _item_id: &str, _author_id: &str, _updates: UgcItem) -> CloudResult<()> {
        Ok(())
    }

    fn search(&self, query: UgcSearchQuery) -> CloudResult<UgcSearchResult> {
        debug!(query = %query.query, "noop UGC: search queued");
        Ok(UgcSearchResult::empty())
    }

    fn get_item(&self, _item_id: &str) -> CloudResult<Option<UgcItem>> {
        Ok(None)
    }

    fn rate_item(&self, item_id: &str, user_id: &str, rating: u32) -> CloudResult<()> {
        debug!(item_id, user_id, rating, "noop UGC: rating queued");
        Ok(())
    }

    fn submit_review(&self, review: UgcReview) -> CloudResult<()> {
        debug!(review_id = %review.review_id, "noop UGC: review queued");
        Ok(())
    }

    fn get_reviews(&self, _item_id: &str, _offset: u32, _limit: u32) -> CloudResult<Vec<UgcReview>> {
        Ok(Vec::new())
    }

    fn subscribe(&self, subscription: WorkshopSubscription) -> CloudResult<()> {
        debug!(user_id = %subscription.user_id, item_id = %subscription.item_id, "noop UGC: subscription queued");
        Ok(())
    }

    fn unsubscribe(&self, _user_id: &str, _item_id: &str) -> CloudResult<()> {
        Ok(())
    }

    fn get_subscriptions(&self, _user_id: &str) -> CloudResult<Vec<WorkshopSubscription>> {
        Ok(Vec::new())
    }

    fn get_subscriber_count(&self, _item_id: &str) -> CloudResult<u64> {
        Ok(0)
    }

    fn increment_download(&self, _item_id: &str) -> CloudResult<()> {
        Ok(())
    }
}

// ============================================================================
// Monetization
// ============================================================================

/// Purchase item type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PurchaseType {
    Consumable,
    NonConsumable,
    Subscription,
    Durable,
}

impl Default for PurchaseType {
    fn default() -> Self {
        Self::Consumable
    }
}

/// Purchase item definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseItem {
    pub item_id: String,
    pub title: String,
    pub description: String,
    pub price: f64,
    pub currency: String,
    pub purchase_type: PurchaseType,
    pub product_id: String,
    pub tags: Vec<String>,
}

impl PurchaseItem {
    pub fn new(title: impl Into<String>, product_id: impl Into<String>, price: f64, currency: impl Into<String>) -> Self {
        Self {
            item_id: Uuid::new_v4().to_string(),
            title: title.into(),
            description: String::new(),
            price,
            currency: currency.into(),
            purchase_type: PurchaseType::default(),
            product_id: product_id.into(),
            tags: Vec::new(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn with_type(mut self, purchase_type: PurchaseType) -> Self {
        self.purchase_type = purchase_type;
        self
    }
}

/// Purchase result status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PurchaseStatus {
    Pending,
    Purchased,
    Failed,
    Restored,
    Cancelled,
    Deferred,
}

impl Default for PurchaseStatus {
    fn default() -> Self {
        Self::Pending
    }
}

impl fmt::Display for PurchaseStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Purchased => write!(f, "purchased"),
            Self::Failed => write!(f, "failed"),
            Self::Restored => write!(f, "restored"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Deferred => write!(f, "deferred"),
        }
    }
}

/// Purchase result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseResult {
    pub transaction_id: String,
    pub item_id: String,
    pub status: PurchaseStatus,
    pub receipt: Option<String>,
    pub purchase_time: u64,
    pub error: Option<String>,
}

impl PurchaseResult {
    pub fn success(item_id: impl Into<String>, transaction_id: impl Into<String>, receipt: impl Into<String>) -> Self {
        let now = current_unix_ts();
        Self {
            transaction_id: transaction_id.into(),
            item_id: item_id.into(),
            status: PurchaseStatus::Purchased,
            receipt: Some(receipt.into()),
            purchase_time: now,
            error: None,
        }
    }

    pub fn failed(item_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            transaction_id: Uuid::new_v4().to_string(),
            item_id: item_id.into(),
            status: PurchaseStatus::Failed,
            receipt: None,
            purchase_time: current_unix_ts(),
            error: Some(error.into()),
        }
    }
}

/// Receipt validation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptValidationResult {
    pub valid: bool,
    pub product_id: Option<String>,
    pub transaction_id: Option<String>,
    pub purchase_time: Option<u64>,
    pub error: Option<String>,
}

impl ReceiptValidationResult {
    pub fn valid(product_id: impl Into<String>, transaction_id: impl Into<String>, purchase_time: u64) -> Self {
        Self {
            valid: true,
            product_id: Some(product_id.into()),
            transaction_id: Some(transaction_id.into()),
            purchase_time: Some(purchase_time),
            error: None,
        }
    }

    pub fn invalid(error: impl Into<String>) -> Self {
        Self {
            valid: false,
            product_id: None,
            transaction_id: None,
            purchase_time: None,
            error: Some(error.into()),
        }
    }
}

/// Ad configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdConfig {
    pub ad_unit_id: String,
    pub ad_type: AdType,
    pub placement: AdPlacement,
    pub is_enabled: bool,
    pub cooldown_secs: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdType {
    Banner,
    Interstitial,
    Rewarded,
}

impl Default for AdType {
    fn default() -> Self {
        Self::Interstitial
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdPlacement {
    Menu,
    LevelStart,
    LevelEnd,
    Pause,
    Continue,
}

impl Default for AdPlacement {
    fn default() -> Self {
        Self::Menu
    }
}

impl AdConfig {
    pub fn new(ad_unit_id: impl Into<String>) -> Self {
        Self {
            ad_unit_id: ad_unit_id.into(),
            ad_type: AdType::default(),
            placement: AdPlacement::default(),
            is_enabled: true,
            cooldown_secs: 60,
        }
    }
}

/// Ad event result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdEvent {
    pub ad_unit_id: String,
    pub ad_type: AdType,
    pub placement: AdPlacement,
    pub event_type: AdEventType,
    pub reward_granted: bool,
    pub reward_type: Option<String>,
    pub reward_quantity: Option<u32>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdEventType {
    Loaded,
    FailedToLoad,
    Opened,
    Closed,
    Clicked,
    RewardGranted,
}

impl AdEvent {
    pub fn loaded(ad_unit_id: impl Into<String>) -> Self {
        Self {
            ad_unit_id: ad_unit_id.into(),
            ad_type: AdType::default(),
            placement: AdPlacement::default(),
            event_type: AdEventType::Loaded,
            reward_granted: false,
            reward_type: None,
            reward_quantity: None,
            timestamp: current_unix_ts(),
        }
    }
}

/// Trait for in-app purchase and ad services.
pub trait IapService: Send + Sync {
    fn name(&self) -> &str;
    fn is_available(&self) -> bool;

    fn get_available_items(&self) -> CloudResult<Vec<PurchaseItem>>;
    fn purchase(&self, user_id: &str, item_id: &str) -> CloudResult<PurchaseResult>;
    fn restore_purchases(&self, user_id: &str) -> CloudResult<Vec<PurchaseResult>>;
    fn validate_receipt(&self, receipt: &str) -> CloudResult<ReceiptValidationResult>;
    fn finish_transaction(&self, transaction_id: &str) -> CloudResult<()>;

    fn load_ad(&self, config: AdConfig) -> CloudResult<()>;
    fn show_ad(&self, ad_unit_id: &str) -> CloudResult<AdEvent>;
    fn hide_ad(&self, ad_unit_id: &str) -> CloudResult<()>;
    fn is_ad_ready(&self, ad_unit_id: &str) -> CloudResult<bool>;
}

/// No-op IAP service.
#[derive(Clone, Debug, Default)]
pub struct NoopIapService;

impl IapService for NoopIapService {
    fn name(&self) -> &str {
        "noop"
    }

    fn is_available(&self) -> bool {
        false
    }

    fn get_available_items(&self) -> CloudResult<Vec<PurchaseItem>> {
        Ok(Vec::new())
    }

    fn purchase(&self, _user_id: &str, item_id: &str) -> CloudResult<PurchaseResult> {
        warn!(item_id, "noop IAP: purchase not implemented");
        Ok(PurchaseResult::failed(item_id, "noop IAP service"))
    }

    fn restore_purchases(&self, _user_id: &str) -> CloudResult<Vec<PurchaseResult>> {
        Ok(Vec::new())
    }

    fn validate_receipt(&self, _receipt: &str) -> CloudResult<ReceiptValidationResult> {
        Ok(ReceiptValidationResult::invalid("noop IAP service"))
    }

    fn finish_transaction(&self, _transaction_id: &str) -> CloudResult<()> {
        Ok(())
    }

    fn load_ad(&self, config: AdConfig) -> CloudResult<()> {
        debug!(ad_unit_id = %config.ad_unit_id, "noop IAP: ad load queued");
        Ok(())
    }

    fn show_ad(&self, ad_unit_id: &str) -> CloudResult<AdEvent> {
        debug!(ad_unit_id, "noop IAP: ad show queued");
        Ok(AdEvent::loaded(ad_unit_id))
    }

    fn hide_ad(&self, _ad_unit_id: &str) -> CloudResult<()> {
        Ok(())
    }

    fn is_ad_ready(&self, _ad_unit_id: &str) -> CloudResult<bool> {
        Ok(false)
    }
}

// ============================================================================
// Identity / Auth
// ============================================================================

/// Authentication provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthProvider {
    Steam,
    Epic,
    Google,
    Apple,
    Custom,
}

impl Default for AuthProvider {
    fn default() -> Self {
        Self::Custom
    }
}

/// Authentication token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    pub token: String,
    pub refresh_token: Option<String>,
    pub expires_at: u64,
    pub provider: AuthProvider,
    pub user_id: String,
}

impl AuthToken {
    pub fn new(user_id: impl Into<String>, provider: AuthProvider) -> Self {
        let now = current_unix_ts();
        Self {
            token: Uuid::new_v4().to_string(),
            refresh_token: None,
            expires_at: now + 3600,
            provider,
            user_id: user_id.into(),
        }
    }

    pub fn with_refresh_token(mut self, refresh_token: impl Into<String>) -> Self {
        self.refresh_token = Some(refresh_token.into());
        self
    }

    pub fn with_ttl(mut self, ttl_secs: u64) -> Self {
        let now = current_unix_ts();
        self.expires_at = now + ttl_secs;
        self
    }

    pub fn is_expired(&self) -> bool {
        current_unix_ts() >= self.expires_at
    }
}

/// User profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub user_id: String,
    pub display_name: String,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub country: Option<String>,
    pub language: String,
    pub created_at: u64,
    pub last_login: u64,
    pub metadata: HashMap<String, String>,
    pub linked_accounts: HashMap<AuthProvider, String>,
}

impl UserProfile {
    pub fn new(user_id: impl Into<String>, display_name: impl Into<String>) -> Self {
        let now = current_unix_ts();
        Self {
            user_id: user_id.into(),
            display_name: display_name.into(),
            email: None,
            avatar_url: None,
            country: None,
            language: "en".to_string(),
            created_at: now,
            last_login: now,
            metadata: HashMap::new(),
            linked_accounts: HashMap::new(),
        }
    }

    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = language.into();
        self
    }

    pub fn with_linked_account(mut self, provider: AuthProvider, account_id: impl Into<String>) -> Self {
        self.linked_accounts.insert(provider, account_id.into());
        self
    }
}

/// Identity service trait.
pub trait IdentityService: Send + Sync {
    fn name(&self) -> &str;
    fn is_available(&self) -> bool;

    fn login(&self, provider: AuthProvider, token: &str) -> CloudResult<AuthToken>;
    fn refresh_token(&self, refresh_token: &str) -> CloudResult<AuthToken>;
    fn logout(&self, user_id: &str) -> CloudResult<()>;
    fn validate_token(&self, token: &str) -> CloudResult<Option<UserProfile>>;

    fn get_profile(&self, user_id: &str) -> CloudResult<Option<UserProfile>>;
    fn update_profile(&self, user_id: &str, profile: UserProfile) -> CloudResult<()>;

    fn link_account(&self, user_id: &str, provider: AuthProvider, token: &str) -> CloudResult<()>;
    fn unlink_account(&self, user_id: &str, provider: AuthProvider) -> CloudResult<()>;

    fn search_users(&self, query: &str, limit: u32) -> CloudResult<Vec<UserProfile>>;
}

/// No-op identity service.
#[derive(Clone, Debug, Default)]
pub struct NoopIdentityService;

impl IdentityService for NoopIdentityService {
    fn name(&self) -> &str {
        "noop"
    }

    fn is_available(&self) -> bool {
        false
    }

    fn login(&self, provider: AuthProvider, token: &str) -> CloudResult<AuthToken> {
        debug!(?provider, "noop identity: login queued");
        let auth_token = AuthToken::new("user", provider);
        Ok(auth_token)
    }

    fn refresh_token(&self, _refresh_token: &str) -> CloudResult<AuthToken> {
        warn!("noop identity: refresh not implemented");
        Err(CloudError::AuthFailed("noop identity service".into()))
    }

    fn logout(&self, _user_id: &str) -> CloudResult<()> {
        Ok(())
    }

    fn validate_token(&self, _token: &str) -> CloudResult<Option<UserProfile>> {
        Ok(None)
    }

    fn get_profile(&self, user_id: &str) -> CloudResult<Option<UserProfile>> {
        debug!(user_id, "noop identity: get profile queued");
        Ok(Some(UserProfile::new(user_id, "Player")))
    }

    fn update_profile(&self, user_id: &str, profile: UserProfile) -> CloudResult<()> {
        debug!(user_id, display_name = %profile.display_name, "noop identity: update profile queued");
        Ok(())
    }

    fn link_account(&self, user_id: &str, provider: AuthProvider, _token: &str) -> CloudResult<()> {
        debug!(user_id, ?provider, "noop identity: link account queued");
        Ok(())
    }

    fn unlink_account(&self, user_id: &str, provider: AuthProvider) -> CloudResult<()> {
        debug!(user_id, ?provider, "noop identity: unlink account queued");
        Ok(())
    }

    fn search_users(&self, query: &str, limit: u32) -> CloudResult<Vec<UserProfile>> {
        debug!(query, limit, "noop identity: search queued");
        Ok(Vec::new())
    }
}

// ============================================================================
// Cloud services manager
// ============================================================================

/// Central cloud services manager.
pub struct CloudServicesManager {
    initialized: AtomicBool,
    cloud_save: Arc<dyn CloudSaveService>,
    matchmaking: Arc<dyn MatchmakingService>,
    leaderboard: Arc<dyn LeaderboardService>,
    ugc: Arc<dyn UgcPlatform>,
    iap: Arc<dyn IapService>,
    identity: Arc<dyn IdentityService>,
    request_id_counter: AtomicU64,
}

impl CloudServicesManager {
    pub fn new(
        cloud_save: Arc<dyn CloudSaveService>,
        matchmaking: Arc<dyn MatchmakingService>,
        leaderboard: Arc<dyn LeaderboardService>,
        ugc: Arc<dyn UgcPlatform>,
        iap: Arc<dyn IapService>,
        identity: Arc<dyn IdentityService>,
    ) -> Self {
        Self {
            initialized: AtomicBool::new(true),
            cloud_save,
            matchmaking,
            leaderboard,
            ugc,
            iap,
            identity,
            request_id_counter: AtomicU64::new(0),
        }
    }

    pub fn cloud_save(&self) -> &dyn CloudSaveService {
        self.cloud_save.as_ref()
    }

    pub fn matchmaking(&self) -> &dyn MatchmakingService {
        self.matchmaking.as_ref()
    }

    pub fn leaderboard(&self) -> &dyn LeaderboardService {
        self.leaderboard.as_ref()
    }

    pub fn ugc(&self) -> &dyn UgcPlatform {
        self.ugc.as_ref()
    }

    pub fn iap(&self) -> &dyn IapService {
        self.iap.as_ref()
    }

    pub fn identity(&self) -> &dyn IdentityService {
        self.identity.as_ref()
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }
}

impl Default for CloudServicesManager {
    fn default() -> Self {
        Self {
            initialized: AtomicBool::new(true),
            cloud_save: Arc::new(NoopCloudSaveService::default()),
            matchmaking: Arc::new(NoopMatchmakingService::default()),
            leaderboard: Arc::new(NoopLeaderboardService::default()),
            ugc: Arc::new(NoopUgcPlatform::default()),
            iap: Arc::new(NoopIapService::default()),
            identity: Arc::new(NoopIdentityService::default()),
            request_id_counter: AtomicU64::new(0),
        }
    }
}

// ============================================================================
// Helpers
// ============================================================================

fn current_unix_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloud_save_slot_defaults() {
        let slot = CloudSaveSlot::new(0, "Test Save");
        assert_eq!(slot.slot_id, 0);
        assert!(!slot.is_auto_save);
        assert!(slot.tags.is_empty());
    }

    #[test]
    fn test_noop_cloud_service() {
        let service = NoopCloudSaveService::default();
        assert_eq!(service.name(), "noop");
        assert!(!service.is_available());
        assert!(service.download(0).unwrap().is_none());
    }

    #[test]
    fn test_matchmaking_request_builder() {
        let request = MatchmakingRequest::new("user123", GameMode::Coop)
            .with_region("eu-west")
            .with_skill_tier(SkillTier::Gold)
            .with_party_size(2, 4);

        assert_eq!(request.user_id, "user123");
        assert_eq!(request.game_mode, GameMode::Coop);
        assert_eq!(request.region, "eu-west");
        assert_eq!(request.skill_tier, SkillTier::Gold);
        assert_eq!(request.party_size, 2);
        assert_eq!(request.max_party_size, 4);
    }

    #[test]
    fn test_lobby_lifecycle() {
        let config = LobbyConfig {
            max_players: 4,
            ..Default::default()
        };
        let lobby = Lobby::new("Test Lobby", "host1", config);
        assert_eq!(lobby.state, LobbyState::Open);
        assert!(!lobby.is_full());
        assert_eq!(lobby.player_count(), 1);
    }

    #[test]
    fn test_ugc_item_rating() {
        let item = UgcItem::new("Test Map", "author1", "Author")
            .with_description("A test map")
            .with_tag("adventure")
            .with_tag("easy");

        assert_eq!(item.average_rating(), 0.0);
        assert_eq!(item.tags.len(), 2);
        assert_eq!(item.visibility, UgcVisibility::Private);
    }

    #[test]
    fn test_purchase_result_success() {
        let result = PurchaseResult::success("item123", "tx456", "receipt789");
        assert_eq!(result.status, PurchaseStatus::Purchased);
        assert_eq!(result.item_id, "item123");
        assert!(result.receipt.is_some());
    }

    #[test]
    fn test_auth_token_expiry() {
        let token = AuthToken::new("user1", AuthProvider::Steam).with_ttl(3600);
        assert!(!token.is_expired());
    }

    #[test]
    fn test_user_profile() {
        let profile = UserProfile::new("user1", "PlayerOne")
            .with_email("player@example.com")
            .with_language("tr")
            .with_linked_account(AuthProvider::Steam, "steam765");

        assert_eq!(profile.user_id, "user1");
        assert_eq!(profile.display_name, "PlayerOne");
        assert_eq!(profile.language, "tr");
        assert!(profile.linked_accounts.contains_key(&AuthProvider::Steam));
    }

    #[test]
    fn test_cloud_services_manager_default() {
        let manager = CloudServicesManager::default();
        assert!(manager.is_initialized());
        assert_eq!(manager.cloud_save().name(), "noop");
        assert_eq!(manager.matchmaking().name(), "noop");
        assert_eq!(manager.leaderboard().name(), "noop");
        assert_eq!(manager.ugc().name(), "noop");
        assert_eq!(manager.iap().name(), "noop");
        assert_eq!(manager.identity().name(), "noop");
    }
}
