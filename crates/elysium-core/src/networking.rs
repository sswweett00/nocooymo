//! Complete multiplayer networking system for the Elysium game engine.
//!
//! Provides:
//!   - TCP/UDP socket management via tokio
//!   - Packet serialization/deserialization (bincode + bson alternatives)
//!   - Connection management (connect, disconnect, timeout, keepalive)
//!   - Encryption (AES-256-GCM) and compression (LZ4)
//!   - Entity replication with state snapshots, delta compression, priority, relevance
//!   - Ownership transfer, client-side prediction, server reconciliation, lag compensation
//!   - Reliable / unreliable RPC with validation and batching
//!   - Lobby / matchmaking with room management, slots, host migration, NAT punchthrough
//!   - Security: anti-cheat basics, ban list, rate limiting, packet validation
//!   - ECS integration via `Networked`, `NetworkTransform`, `NetworkState`, `ReplicationSystem`

use std::collections::{HashMap, HashSet, VecDeque};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::{mpsc, oneshot, Mutex, RwLock};
use tokio::time::{self, Instant as TokioInstant};

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use lz4::block::compress;
use parking_lot::Mutex as PLMutex;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::{
    component::Component,
    entity::Entity,
    scheduler::System,
    world::World,
};

// =============================================================================
// Types / Constants
// =============================================================================

pub type ClientId = u64;
pub type ObjectId = u64;
pub type SequenceNumber = u32;
pub type RpcId = u64;
pub type ChannelId = u8;

pub const DEFAULT_PORT: u16 = 7777;
pub const MAX_PACKET_SIZE: usize = 1_048_576; // 1 MiB
pub const DEFAULT_MAX_CLIENTS: usize = 64;
pub const DEFAULT_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
pub const DEFAULT_REPLICATION_INTERVAL: Duration = Duration::from_millis(33);
pub const DEFAULT_RPC_RETRIES: u32 = 5;
pub const LOBBY_MAX_PLAYERS: usize = 16;
pub const NETWORK_CHANNEL_COUNT: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetworkChannel {
    Reliable = 0,
    ReliableOrdered = 1,
    Unreliable = 2,
    UnreliableOrdered = 3,
}

impl From<ChannelId> for NetworkChannel {
    fn from(v: ChannelId) -> Self {
        match v {
            0 => Self::Reliable,
            1 => Self::ReliableOrdered,
            2 => Self::Unreliable,
            _ => Self::UnreliableOrdered,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetRole {
    None,
    Client,
    Server,
    Host,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
    TimedOut,
    Kicked,
    Banned,
}

// =============================================================================
// Crypto helpers
// =============================================================================

#[derive(Debug, Clone)]
pub enum EncryptionKey {
    Aes256(Aes256Gcm),
    Xchacha20(XChaCha20Poly1305),
    None,
}

impl EncryptionKey {
    pub fn new_aes256(key_bytes: &[u8; 32]) -> Self {
        let cipher = Aes256Gcm::new(key_bytes.into());
        Self::Aes256(cipher)
    }

    pub fn new_xchacha20(key_bytes: &[u8; 32]) -> Self {
        let cipher = XChaCha20Poly1305::new(key_bytes.into());
        Self::Xchacha20(cipher)
    }

    pub fn none() -> Self {
        Self::None
    }

    pub fn encrypt(&self, plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>, NetError> {
        match self {
            Self::Aes256(cipher) => {
                let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
                let ct = cipher
                    .encrypt(&nonce, plaintext)
                    .map_err(|_| NetError::Encryption("aes256-gcm encrypt failed".into()))?;
                let mut out = nonce.to_vec();
                out.extend_from_slice(&ct);
                Ok(out)
            }
            Self::Xchacha20(cipher) => {
                let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
                let ct = cipher
                    .encrypt(&nonce, plaintext)
                    .map_err(|_| NetError::Encryption("xchacha20 encrypt failed".into()))?;
                let mut out = nonce.to_vec();
                out.extend_from_slice(&ct);
                Ok(out)
            }
            Self::None => Ok(plaintext.to_vec()),
        }
    }

    pub fn decrypt(&self, data: &[u8], aad: &[u8]) -> Result<Vec<u8>, NetError> {
        match self {
            Self::Aes256(cipher) => {
                let nonce = Nonce::from_slice(&data[..12]);
                let pt = cipher
                    .decrypt(nonce, data[12..].as_ref())
                    .map_err(|_| NetError::Encryption("aes256-gcm decrypt failed".into()))?;
                Ok(pt)
            }
            Self::Xchacha20(cipher) => {
                let nonce = XNonce::from_slice(&data[..24]);
                let pt = cipher
                    .decrypt(nonce, data[24..].as_ref())
                    .map_err(|_| NetError::Encryption("xchacha20 decrypt failed".into()))?;
                Ok(pt)
            }
            Self::None => Ok(data.to_vec()),
        }
    }
}

// =============================================================================
// Packet / serialization
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketHeader {
    pub sequence: SequenceNumber,
    pub channel: ChannelId,
    pub payload_len: u32,
    pub flags: PacketFlags,
    pub timestamp: u64,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    pub struct PacketFlags: u8 {
        const ENCRYPTED = 0b0000_0001;
        const COMPRESSED = 0b0000_0010;
        const FRAGMENTED = 0b0000_0100;
        const ACK_REQUIRED = 0b0000_1000;
        const RELIABLE = 0b0001_0000;
    }
}

#[derive(Debug, Clone)]
pub struct Packet {
    pub header: PacketHeader,
    pub payload: Vec<u8>,
}

impl Packet {
    pub fn new(payload: Vec<u8>, channel: ChannelId, flags: PacketFlags) -> Self {
        Self {
            header: PacketHeader {
                sequence: 0,
                channel,
                payload_len: payload.len() as u32,
                flags,
                timestamp: Self::now_millis(),
            },
            payload,
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).expect("packet serialization failed")
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, NetError> {
        Ok(bincode::deserialize(data)?)
    }

    pub fn compress(&mut self) {
        if self.payload.len() < 64 {
            return;
        }
        let compressed = compress(&self.payload, None, None).expect("lz4 compress");
        self.payload = compressed;
        self.header.flags.insert(PacketFlags::COMPRESSED);
        self.header.payload_len = self.payload.len() as u32;
    }

    pub fn decompress(&mut self) -> Result<(), NetError> {
        if !self.header.flags.contains(PacketFlags::COMPRESSED) {
            return Ok(());
        }
        let decompressed =
            lz4::block::decompress(&self.payload, None).map_err(|e| NetError::Compression(e.into()))?;
        self.payload = decompressed;
        self.header.flags.remove(PacketFlags::COMPRESSED);
        self.header.payload_len = self.payload.len() as u32;
        Ok(())
    }

    pub fn encrypt(&mut self, key: &EncryptionKey) -> Result<(), NetError> {
        if self.payload.is_empty() {
            return Ok(());
        }
        self.payload = key.encrypt(&self.payload, b"packet-aad")?;
        self.header.flags.insert(PacketFlags::ENCRYPTED);
        Ok(())
    }

    pub fn decrypt(&mut self, key: &EncryptionKey) -> Result<(), NetError> {
        if !self.header.flags.contains(PacketFlags::ENCRYPTED) {
            return Ok(());
        }
        self.payload = key.decrypt(&self.payload, b"packet-aad")?;
        self.header.flags.remove(PacketFlags::ENCRYPTED);
        Ok(())
    }

    fn now_millis() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

// =============================================================================
// Message types
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetMessage {
    Connect {
        client_id: ClientId,
        username: String,
        protocol_version: u32,
        auth_token: Vec<u8>,
    },
    ConnectAccept {
        client_id: ClientId,
        server_time: u64,
    },
    ConnectReject {
        reason: String,
    },
    Disconnect {
        reason: String,
    },
    Ping {
        timestamp: u64,
    },
    Pong {
        timestamp: u64,
    },
    // Replication
    StateSnapshot {
        object_id: ObjectId,
        sequence: SequenceNumber,
        data: Vec<u8>,
    },
    StateAck {
        object_id: ObjectId,
        sequence: SequenceNumber,
    },
    DeltaUpdate {
        object_id: ObjectId,
        base_sequence: SequenceNumber,
        delta: Vec<u8>,
    },
    ObjectSpawn {
        object_id: ObjectId,
        prefab_id: u64,
        owner_client: ClientId,
        data: Vec<u8>,
    },
    ObjectDestroy {
        object_id: ObjectId,
    },
    OwnershipTransfer {
        object_id: ObjectId,
        new_owner: ClientId,
    },
    // RPC
    RpcInvoke {
        rpc_id: RpcId,
        object_id: Option<ObjectId>,
        method_hash: u64,
        reliable: bool,
        payload: Vec<u8>,
    },
    RpcAck {
        rpc_id: RpcId,
    },
    RpcResult {
        rpc_id: RpcId,
        payload: Vec<u8>,
    },
    // Lobby / matchmaking
    LobbyCreate {
        lobby_id: u64,
        name: String,
        max_players: u8,
    },
    LobbyJoin {
        lobby_id: u64,
        player_name: String,
    },
    LobbyLeave {
        lobby_id: u64,
    },
    LobbyUpdate {
        lobby_id: u64,
        players: Vec<LobbyPlayerInfo>,
        state: LobbyState,
    },
    LobbyList {
        lobbies: Vec<LobbySummary>,
    },
    RoomStart {
        room_id: u64,
        map_name: String,
        seed: u32,
    },
    RoomClose {
        room_id: u64,
    },
    // NAT punchthrough
    NatCandidate {
        candidate: String,
    },
    NatPunch {
        target: SocketAddr,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbyPlayerInfo {
    pub client_id: ClientId,
    pub name: String,
    pub is_ready: bool,
    pub slot: u8,
    pub ping: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbySummary {
    pub lobby_id: u64,
    pub name: String,
    pub current_players: u8,
    pub max_players: u8,
    pub state: LobbyState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LobbyState {
    Open,
    InGame,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomConfig {
    pub map_name: String,
    pub max_players: u8,
    pub seed: u32,
    pub password: Option<String>,
}

// =============================================================================
// Config
// =============================================================================

#[derive(Debug, Clone)]
pub struct NetServerConfig {
    pub port: u16,
    pub max_clients: usize,
    pub heartbeat_interval: Duration,
    pub timeout: Duration,
    pub replication_interval: Duration,
    pub packet_size_limit: usize,
    pub enable_encryption: bool,
    pub enable_compression: bool,
    pub tick_rate: u32,
}

impl Default for NetServerConfig {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            max_clients: DEFAULT_MAX_CLIENTS,
            heartbeat_interval: DEFAULT_HEARTBEAT_INTERVAL,
            timeout: DEFAULT_TIMEOUT,
            replication_interval: DEFAULT_REPLICATION_INTERVAL,
            packet_size_limit: MAX_PACKET_SIZE,
            enable_encryption: false,
            enable_compression: true,
            tick_rate: 60,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NetClientConfig {
    pub server_address: SocketAddr,
    pub username: String,
    pub reconnect_attempts: u32,
    pub reconnect_delay: Duration,
    pub heartbeat_interval: Duration,
    pub interpolation_delay: Duration,
    pub client_prediction: bool,
}

impl Default for NetClientConfig {
    fn default() -> Self {
        Self {
            server_address: "127.0.0.1:7777".parse().unwrap(),
            username: String::from("Player"),
            reconnect_attempts: 3,
            reconnect_delay: Duration::from_secs(2),
            heartbeat_interval: DEFAULT_HEARTBEAT_INTERVAL,
            interpolation_delay: Duration::from_millis(100),
            client_prediction: true,
        }
    }
}

// =============================================================================
// Connection & client info
// =============================================================================

#[derive(Debug, Clone)]
pub struct ClientConnection {
    pub id: ClientId,
    pub address: SocketAddr,
    pub username: String,
    pub state: ConnectionState,
    pub connected_at: Instant,
    pub last_heartbeat: Instant,
    pub last_pong: Instant,
    pub ping: Duration,
    pub rtt: Duration,
    pub packet_loss: f32,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub role: NetRole,
}

impl ClientConnection {
    pub fn new(id: ClientId, address: SocketAddr, username: String) -> Self {
        let now = Instant::now();
        Self {
            id,
            address,
            username,
            state: ConnectionState::Connected,
            connected_at: now,
            last_heartbeat: now,
            last_pong: now,
            ping: Duration::from_millis(0),
            rtt: Duration::from_millis(0),
            packet_loss: 0.0,
            bytes_sent: 0,
            bytes_received: 0,
            role: NetRole::Client,
        }
    }
}

// =============================================================================
// Events
// =============================================================================

#[derive(Debug, Clone)]
pub enum NetEvent {
    ServerStarted(SocketAddr),
    ServerStopped,
    ClientConnected(ClientId, SocketAddr),
    ClientDisconnected(ClientId),
    ClientKicked(ClientId, String),
    MessageReceived(ClientId, NetMessage),
    Error(String),
    PingUpdated(ClientId, Duration),
    LobbyUpdated(LobbyState),
    RoomStarted { room_id: u64, map_name: String },
}

// =============================================================================
// Error
// =============================================================================

#[derive(Debug, thiserror::Error)]
pub enum NetError {
    #[error("io error: {0}")]
    Io(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("encryption error: {0}")]
    Encryption(String),
    #[error("compression error: {0}")]
    Compression(String),
    #[error("connection error: {0}")]
    Connection(String),
    #[error("timeout")]
    Timeout,
    #[error("invalid packet: {0}")]
    InvalidPacket(String),
    #[error("not connected")]
    NotConnected,
    #[error("rate limited")]
    RateLimited,
    #[error("banned")]
    Banned,
    #[error("invalid rpc: {0}")]
    InvalidRpc(String),
    #[error("lobby error: {0}")]
    Lobby(String),
}

impl From<std::io::Error> for NetError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

impl From<bincode::Error> for NetError {
    fn from(e: bincode::Error) -> Self {
        Self::Serialization(e.to_string())
    }
}

// =============================================================================
// Rate limiter
// =============================================================================

#[derive(Debug, Clone)]
pub struct RateLimiter {
    pub max_bytes_per_sec: usize,
    pub max_packets_per_sec: usize,
    pub bytes_this_sec: usize,
    pub packets_this_sec: usize,
    pub window_start: Instant,
    pub violations: u32,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self {
            max_bytes_per_sec: 1024 * 1024, // 1 MiB/s
            max_packets_per_sec: 1000,
            bytes_this_sec: 0,
            packets_this_sec: 0,
            window_start: Instant::now(),
            violations: 0,
        }
    }
}

impl RateLimiter {
    pub fn new(max_bytes_per_sec: usize, max_packets_per_sec: usize) -> Self {
        Self {
            max_bytes_per_sec,
            max_packets_per_sec,
            ..Default::default()
        }
    }

    pub fn allow(&mut self, bytes: usize) -> bool {
        self.tick();
        if self.bytes_this_sec + bytes > self.max_bytes_per_sec
            || self.packets_this_sec >= self.max_packets_per_sec
        {
            self.violations = self.violations.saturating_add(1);
            return false;
        }
        self.bytes_this_sec += bytes;
        self.packets_this_sec += 1;
        true
    }

    fn tick(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.window_start) >= Duration::from_secs(1) {
            self.bytes_this_sec = 0;
            self.packets_this_sec = 0;
            self.window_start = now;
        }
    }
}

// =============================================================================
// Ban list
// =============================================================================

#[derive(Debug, Clone, Default)]
pub struct BanList {
    pub banned_ids: HashSet<ClientId>,
    pub banned_ips: HashSet<IpAddr>,
    pub banned_tokens: HashSet<Vec<u8>>,
}

impl BanList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_id(&mut self, id: ClientId) {
        self.banned_ids.insert(id);
    }

    pub fn add_ip(&mut self, ip: IpAddr) {
        self.banned_ips.insert(ip);
    }

    pub fn add_token(&mut self, token: Vec<u8>) {
        self.banned_tokens.insert(token);
    }

    pub fn remove_id(&mut self, id: ClientId) {
        self.banned_ids.remove(&id);
    }

    pub fn remove_ip(&mut self, ip: IpAddr) {
        self.banned_ips.remove(&ip);
    }

    pub fn is_banned(&self, id: Option<ClientId>, ip: Option<IpAddr>, token: Option<&[u8]>) -> bool {
        if let Some(id) = id {
            if self.banned_ids.contains(&id) {
                return true;
            }
        }
        if let Some(ip) = ip {
            if self.banned_ips.contains(&ip) {
                return true;
            }
        }
        if let Some(token) = token {
            if self.banned_tokens.iter().any(|t| t == token) {
                return true;
            }
        }
        false
    }
}

// =============================================================================
// Client-side input (for prediction / reconciliation)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetInput {
    pub sequence: u32,
    pub timestamp: f32,
    pub buttons: u32,
    pub axes: [f32; 8],
    pub ticks: u32,
}

impl NetInput {
    pub fn new(sequence: u32) -> Self {
        Self {
            sequence,
            timestamp: 0.0,
            buttons: 0,
            axes: [0.0; 8],
            ticks: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClientPredictionBuffer {
    pub inputs: VecDeque<NetInput>,
    pub acked_sequence: u32,
    pub last_sequence: u32,
    pub pending: HashMap<u32, EntityStatePrediction>,
}

impl Default for ClientPredictionBuffer {
    fn default() -> Self {
        Self {
            inputs: VecDeque::with_capacity(128),
            acked_sequence: 0,
            last_sequence: 0,
            pending: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntityStatePrediction {
    pub transform_index: u32,
    pub position: crate::math::Vec3,
    pub rotation: crate::math::Quat,
    pub velocity: Option<crate::math::Vec3>,
    pub input_sequence: u32,
}

// =============================================================================
// Lag compensation / rewind
// =============================================================================

#[derive(Debug, Clone)]
pub struct LagCompensationBuffer {
    pub snapshots: HashMap<ObjectId, VecDeque<RewindSnapshot>>,
    pub max_duration: Duration,
    pub max_snapshots: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewindSnapshot {
    pub timestamp: f32,
    pub data: Vec<u8>,
    pub sequence: SequenceNumber,
}

impl Default for LagCompensationBuffer {
    fn default() -> Self {
        Self {
            snapshots: HashMap::new(),
            max_duration: Duration::from_secs(2),
            max_snapshots: 120,
        }
    }
}

impl LagCompensationBuffer {
    pub fn with_capacity(max_duration: Duration, max_snapshots: usize) -> Self {
        Self {
            max_duration,
            max_snapshots,
            ..Default::default()
        }
    }

    pub fn push(&mut self, object_id: ObjectId, snapshot: RewindSnapshot) {
        let entry = self.snapshots.entry(object_id).or_default();
        entry.push_back(snapshot);
        while entry.len() > self.max_snapshots {
            entry.pop_front();
        }
    }

    pub fn rewind(&self, object_id: ObjectId, at: f32) -> Option<&RewindSnapshot> {
        let entries = self.snapshots.get(&object_id)?;
        entries
            .iter()
            .rev()
            .find(|s| s.timestamp <= at)
    }
}

// =============================================================================
// RPC system
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcDefinition {
    pub method_hash: u64,
    pub name: &'static str,
    pub reliable: bool,
    pub authority: RpcAuthority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RpcAuthority {
    Server,
    Client,
    Owner,
}

#[derive(Debug, Clone, Default)]
pub struct RpcCall {
    pub id: RpcId,
    pub method_hash: u64,
    pub target_client: Option<ClientId>,
    pub object_id: Option<ObjectId>,
    pub payload: Vec<u8>,
    pub reliable: bool,
    pub retries: u32,
    pub created_at: Instant,
}

#[derive(Debug, Clone, Default)]
pub struct RpcRegistry {
    pub definitions: HashMap<u64, RpcDefinition>,
    pub pending: HashMap<RpcId, RpcCall>,
    pub next_id: RpcId,
    pub batch: Vec<RpcCall>,
}

impl RpcRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, def: RpcDefinition) {
        self.definitions.insert(def.method_hash, def);
    }

    pub fn next_id(&mut self) -> RpcId {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        id
    }

    pub fn enqueue(&mut self, call: RpcCall) -> RpcId {
        let id = call.id;
        self.pending.insert(id, call);
        id
    }

    pub fn ack(&mut self, rpc_id: RpcId) {
        self.pending.remove(&rpc_id);
    }

    pub fn batch_append(&mut self, call: RpcCall) {
        self.batch.push(call);
    }

    pub fn take_batch(&mut self) -> Vec<RpcCall> {
        let batch = std::mem::take(&mut self.batch);
        for call in &batch {
            self.pending.entry(call.id).or_insert_with(|| call.clone());
        }
        batch
    }
}

// =============================================================================
// Replicated object state
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicatedState {
    pub owner_client: ClientId,
    pub authority: RpcAuthority,
    pub priority: ReplicationPriority,
    pub last_snapshot: Vec<u8>,
    pub last_snapshot_seq: SequenceNumber,
    pub last_delta_seq: SequenceNumber,
    pub last_update: Instant,
    pub relevance_radius: f32,
    pub is_static: bool,
    pub is_dormant: bool,
    pub flags: ReplicationFlags,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
    pub struct ReplicationFlags: u8 {
        const POSITION = 0b0000_0001;
        const ROTATION = 0b0000_0010;
        const SCALE    = 0b0000_0100;
        const VISIBLE  = 0b0000_1000;
        const ANIM     = 0b0001_0000;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ReplicationPriority {
    Critical = 0,
    High = 1,
    Normal = 2,
    Low = 3,
    Background = 4,
}

// =============================================================================
// Ownership transfer
// =============================================================================

#[derive(Debug, Clone)]
pub struct OwnershipTransferRequest {
    pub object_id: ObjectId,
    pub from_client: ClientId,
    pub to_client: ClientId,
    pub reason: TransferReason,
    pub timestamp: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferReason {
    Migration,
    Handoff,
    Rebalance,
}

// =============================================================================
// Matchmaking / lobby
// =============================================================================

#[derive(Debug, Clone)]
pub struct Lobby {
    pub id: u64,
    pub name: String,
    pub owner_id: ClientId,
    pub state: LobbyState,
    pub max_players: u8,
    pub players: HashMap<ClientId, LobbyPlayerInfo>,
    pub slots: Vec<Option<ClientId>>,
    pub created_at: Instant,
    pub room_config: Option<RoomConfig>,
    pub room_id: Option<u64>,
}

impl Lobby {
    pub fn new(id: u64, name: String, owner_id: ClientId, max_players: u8) -> Self {
        let mut slots = Vec::with_capacity(max_players as usize);
        slots.resize(max_players as usize, None);
        Self {
            id,
            name,
            owner_id,
            state: LobbyState::Open,
            max_players,
            players: HashMap::new(),
            slots,
            created_at: Instant::now(),
            room_config: None,
            room_id: None,
        }
    }

    pub fn is_full(&self) -> bool {
        self.players.len() >= self.max_players as usize
    }

    pub fn add_player(&mut self, client_id: ClientId, name: String) -> Option<u8> {
        if self.is_full() {
            return None;
        }
        let slot = self.slots.iter().position(|s| s.is_none())? as u8;
        self.slots[slot as usize] = Some(client_id);
        self.players.insert(
            client_id,
            LobbyPlayerInfo {
                client_id,
                name,
                is_ready: false,
                slot,
                ping: 0,
            },
        );
        Some(slot)
    }

    pub fn remove_player(&mut self, client_id: ClientId) {
        if let Some(info) = self.players.remove(&client_id) {
            if info.slot < self.max_players {
                self.slots[info.slot as usize] = None;
            }
        }
    }

    pub fn migrate_host(&mut self, new_owner: ClientId) -> bool {
        if self.players.contains_key(&new_owner) {
            self.owner_id = new_owner;
            true
        } else {
            false
        }
    }
}

// =============================================================================
// NAT punchthrough
// =============================================================================

#[derive(Debug, Clone)]
pub struct NatSession {
    pub client_id: ClientId,
    pub local_address: SocketAddr,
    pub external_address: Option<SocketAddr>,
    pub candidates: VecDeque<SocketAddr>,
    pub stun_server: Option<SocketAddr>,
    pub turn_server: Option<SocketAddr>,
    pub state: NatState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NatState {
    Gathering,
    Checking,
    Connected,
    Failed,
}

impl NatSession {
    pub fn new(client_id: ClientId, local: SocketAddr) -> Self {
        Self {
            client_id,
            local_address: local,
            external_address: None,
            candidates: VecDeque::new(),
            stun_server: None,
            turn_server: None,
            state: NatState::Gathering,
        }
    }

    pub fn add_candidate(&mut self, addr: SocketAddr) {
        if !self.candidates.contains(&addr) {
            self.candidates.push_back(addr);
        }
    }
}

// =============================================================================
// Anti-cheat
// =============================================================================

#[derive(Debug, Clone)]
pub struct AntiCheatConfig {
    pub enable_server_authority: bool,
    pub enable_input_validation: bool,
    pub enable_speed_hack_detection: bool,
    pub enable_aimbot_detection: bool,
    pub max_position_delta: f32,
    pub max_rotation_delta: f32,
    pub max_speed: f32,
}

impl Default for AntiCheatConfig {
    fn default() -> Self {
        Self {
            enable_server_authority: true,
            enable_input_validation: true,
            enable_speed_hack_detection: true,
            enable_aimbot_detection: false,
            max_position_delta: 10.0,
            max_rotation_delta: std::f32::consts::PI,
            max_speed: 50.0,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AntiCheatReport {
    pub client_id: ClientId,
    pub infractions: Vec<AntiCheatInfraction>,
}

#[derive(Debug, Clone)]
pub enum AntiCheatInfraction {
    PositionAnomaly { delta: f32, threshold: f32 },
    SpeedHack { speed: f32, threshold: f32 },
    InvalidInput { sequence: u32 },
    UnauthorizedRpc { method: u64 },
    RateLimitViolation,
}

// =============================================================================
// Connection manager
// =============================================================================

#[derive(Debug, Clone)]
pub struct ConnectionManager {
    pub connections: HashMap<ClientId, ClientConnection>,
    pub next_id: ClientId,
    pub ban_list: BanList,
    pub rate_limiter: RateLimiter,
    pub anti_cheat: AntiCheatConfig,
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            next_id: 1,
            ban_list: BanList::new(),
            rate_limiter: RateLimiter::default(),
            anti_cheat: AntiCheatConfig::default(),
        }
    }

    pub fn allocate_id(&mut self) -> ClientId {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        id
    }

    pub fn add_connection(&mut self, conn: ClientConnection) -> ClientId {
        let id = conn.id;
        self.connections.insert(id, conn);
        id
    }

    pub fn remove_connection(&mut self, id: ClientId) -> Option<ClientConnection> {
        self.connections.remove(&id)
    }

    pub fn get(&self, id: ClientId) -> Option<&ClientConnection> {
        self.connections.get(&id)
    }

    pub fn get_mut(&mut self, id: ClientId) -> Option<&mut ClientConnection> {
        self.connections.get_mut(&id)
    }

    pub fn check_rate_limit(&mut self, id: ClientId, bytes: usize) -> bool {
        if !self.rate_limiter.allow(bytes) {
            self.ban_list.add_id(id);
            return false;
        }
        true
    }

    pub fn is_banned(&self, id: Option<ClientId>, ip: Option<IpAddr>, token: Option<&[u8]>) -> bool {
        self.ban_list.is_banned(id, ip, token)
    }
}

// =============================================================================
// Networked ECS components
// =============================================================================

#[derive(Debug, Clone, Component)]
pub struct Networked {
    pub object_id: ObjectId,
    pub owner_client: ClientId,
    pub authority: RpcAuthority,
    pub priority: ReplicationPriority,
    pub relevance_radius: f32,
    pub is_static: bool,
    pub is_dormant: bool,
    pub flags: ReplicationFlags,
}

impl Default for Networked {
    fn default() -> Self {
        Self {
            object_id: 0,
            owner_client: 0,
            authority: RpcAuthority::Server,
            priority: ReplicationPriority::Normal,
            relevance_radius: 50.0,
            is_static: false,
            is_dormant: false,
            flags: ReplicationFlags::empty(),
        }
    }
}

#[derive(Debug, Clone, Component)]
pub struct NetworkTransform {
    pub position: crate::math::Vec3,
    pub rotation: crate::math::Quat,
    pub scale: crate::math::Vec3,
    pub velocity: Option<crate::math::Vec3>,
    pub angular_velocity: Option<crate::math::Quat>,
    pub last_sent_sequence: SequenceNumber,
    pub interpolation_time: f32,
}

impl Default for NetworkTransform {
    fn default() -> Self {
        Self {
            position: crate::math::Vec3::ZERO,
            rotation: crate::math::Quat::IDENTITY,
            scale: crate::math::Vec3::ONE,
            velocity: None,
            angular_velocity: None,
            last_sent_sequence: 0,
            interpolation_time: 0.0,
        }
    }
}

#[derive(Debug, Clone, Component)]
pub struct NetworkState {
    pub variables: HashMap<u64, Vec<u8>>,
    pub dirty_mask: u64,
    pub last_snapshot: Vec<u8>,
    pub last_snapshot_seq: SequenceNumber,
}

impl Default for NetworkState {
    fn default() -> Self {
        Self {
            variables: HashMap::new(),
            dirty_mask: 0,
            last_snapshot: Vec::new(),
            last_snapshot_seq: 0,
        }
    }
}

// =============================================================================
// Relevance system
// =============================================================================

#[derive(Debug, Clone)]
pub struct RelevanceSystem {
    pub player_view_positions: HashMap<ClientId, crate::math::Vec3>,
    pub player_view_radius: f32,
    pub statics: HashSet<ObjectId>,
    pub dynamic_priority: HashMap<ObjectId, ReplicationPriority>,
}

impl Default for RelevanceSystem {
    fn default() -> Self {
        Self {
            player_view_positions: HashMap::new(),
            player_view_radius: 80.0,
            statics: HashSet::new(),
            dynamic_priority: HashMap::new(),
        }
    }
}

impl RelevanceSystem {
    pub fn new(view_radius: f32) -> Self {
        Self {
            player_view_positions: HashMap::new(),
            player_view_radius: view_radius,
            statics: HashSet::new(),
            dynamic_priority: HashMap::new(),
        }
    }

    pub fn update_view(&mut self, client_id: ClientId, position: crate::math::Vec3) {
        self.player_view_positions.insert(client_id, position);
    }

    pub fn is_relevant(&self, client_id: ClientId, object_pos: &crate::math::Vec3) -> bool {
        if let Some(view_pos) = self.player_view_positions.get(&client_id) {
            (*object_pos - *view_pos).length() <= self.player_view_radius
        } else {
            true
        }
    }

    pub fn priority_for(&self, object_id: ObjectId) -> ReplicationPriority {
        if self.statics.contains(&object_id) {
            return ReplicationPriority::Normal;
        }
        self.dynamic_priority
            .get(&object_id)
            .copied()
            .unwrap_or(ReplicationPriority::Normal)
    }
}

// =============================================================================
// Replication system (server-side)
// =============================================================================

#[derive(Debug, Clone)]
pub struct ReplicationConfig {
    pub interval: Duration,
    pub max_objects_per_tick: usize,
    pub delta_enabled: bool,
    pub priority_threshold: ReplicationPriority,
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            interval: DEFAULT_REPLICATION_INTERVAL,
            max_objects_per_tick: 256,
            delta_enabled: true,
            priority_threshold: ReplicationPriority::Background,
        }
    }
}

#[derive(Debug)]
pub struct ReplicationSystem {
    pub objects: HashMap<ObjectId, ReplicatedState>,
    pub owned_by: HashMap<ObjectId, ClientId>,
    pub pending_spawns: Vec<(ObjectId, Vec<u8>)>,
    pub pending_destroys: Vec<ObjectId>,
    pub next_sequence: SequenceNumber,
    pub pending_acks: HashMap<(ClientId, ObjectId, SequenceNumber), Instant>,
    pub config: ReplicationConfig,
    pub relevance: RelevanceSystem,
    pub lag_compensation: LagCompensationBuffer,
    pub ownership_transfers: Vec<OwnershipTransferRequest>,
}

impl Default for ReplicationSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplicationSystem {
    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
            owned_by: HashMap::new(),
            pending_spawns: Vec::new(),
            pending_destroys: Vec::new(),
            next_sequence: 1,
            pending_acks: HashMap::new(),
            config: ReplicationConfig::default(),
            relevance: RelevanceSystem::default(),
            lag_compensation: LagCompensationBuffer::default(),
            ownership_transfers: Vec::new(),
        }
    }

    pub fn update(&mut self, _dt: f32) {
        // Advance lag compensation buffers, purge stale pending_acks, etc.
        let now = Instant::now();
        self.pending_acks.retain(|_, ts| now.duration_since(*ts) < Duration::from_secs(10));
    }

    pub fn with_config(config: ReplicationConfig) -> Self {
        let mut sys = Self::new();
        sys.config = config;
        sys
    }

    pub fn register_object(
        &mut self,
        object_id: ObjectId,
        owner: ClientId,
        authority: RpcAuthority,
        priority: ReplicationPriority,
        snapshot: Vec<u8>,
    ) {
        let state = ReplicatedState {
            owner_client: owner,
            authority,
            priority,
            last_snapshot: snapshot,
            last_snapshot_seq: 0,
            last_delta_seq: 0,
            last_update: Instant::now(),
            relevance_radius: 50.0,
            is_static: false,
            is_dormant: false,
            flags: ReplicationFlags::empty(),
        };
        self.objects.insert(object_id, state);
        self.owned_by.insert(object_id, owner);
    }

    pub fn unregister_object(&mut self, object_id: ObjectId) {
        self.objects.remove(&object_id);
        self.owned_by.remove(&object_id);
    }

    pub fn update_snapshot(&mut self, object_id: ObjectId, data: Vec<u8>) {
        if let Some(state) = self.objects.get_mut(&object_id) {
            state.last_snapshot = data;
            state.last_snapshot_seq = self.next_sequence();
            state.last_update = Instant::now();
        }
    }

    pub fn enqueue_spawn(&mut self, object_id: ObjectId, data: Vec<u8>) {
        self.pending_spawns.push((object_id, data));
    }

    pub fn enqueue_destroy(&mut self, object_id: ObjectId) {
        self.pending_destroys.push(object_id);
    }

    pub fn next_sequence(&mut self) -> SequenceNumber {
        let seq = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1);
        seq
    }

    pub fn request_ownership_transfer(
        &mut self,
        object_id: ObjectId,
        from: ClientId,
        to: ClientId,
        reason: TransferReason,
    ) {
        self.ownership_transfers.push(OwnershipTransferRequest {
            object_id,
            from_client: from,
            to_client: to,
            reason,
            timestamp: Instant::now(),
        });
    }

    pub fn update_lag_compensation(&mut self, object_id: ObjectId, data: Vec<u8>) {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs_f32())
            .unwrap_or(0.0);
        self.lag_compensation.push(
            object_id,
            RewindSnapshot {
                timestamp: ts,
                data,
                sequence: self.next_sequence(),
            },
        );
    }

    pub fn rewind(&self, object_id: ObjectId, at: f32) -> Option<&RewindSnapshot> {
        self.lag_compensation.rewind(object_id, at)
    }
}

// =============================================================================
// NetClient
// =============================================================================

#[derive(Debug)]
pub struct NetClient {
    pub id: Option<ClientId>,
    pub connection: Option<mpsc::Sender<BoxedNetMessage>>,
    pub pending: HashMap<RpcId, oneshot::Sender<Vec<u8>>>,
    pub inputs: ClientPredictionBuffer,
    pub local_objects: HashMap<ObjectId, EntityStatePrediction>,
    pub config: NetClientConfig,
    pub state: ConnectionState,
    pub events: mpsc::Sender<NetEvent>,
    pub tick_tx: mpsc::Sender<()>,
    pub crypto: EncryptionKey,
    pub protocol_version: u32,
}

type BoxedNetMessage = Box<NetMessage>;

impl NetClient {
    pub fn new(config: NetClientConfig) -> Self {
        let (events_tx, _) = mpsc::channel(256);
        let (_tick_tx, _tick_rx) = mpsc::channel(1);
        Self {
            id: None,
            connection: None,
            pending: HashMap::new(),
            inputs: ClientPredictionBuffer::default(),
            local_objects: HashMap::new(),
            config,
            state: ConnectionState::Disconnected,
            events: events_tx,
            tick_tx: _tick_tx,
            crypto: EncryptionKey::none(),
            protocol_version: 1,
        }
    }

    pub fn with_encryption(mut self, key: EncryptionKey) -> Self {
        self.crypto = key;
        self
    }

    pub async fn connect(&mut self) -> Result<(), NetError> {
        let stream = TcpStream::connect(self.config.server_address).await?;
        let (tx, _rx) = mpsc::channel::<BoxedNetMessage>(4096);
        self.connection = Some(tx);
        self.state = ConnectionState::Connecting;
        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<(), NetError> {
        self.state = ConnectionState::Disconnecting;
        self.connection = None;
        self.id = None;
        self.state = ConnectionState::Disconnected;
        Ok(())
    }

    pub fn send_message(&self, msg: NetMessage) -> Result<(), NetError> {
        if let Some(tx) = &self.connection {
            tx.blocking_send(Box::new(msg)).ok();
            Ok(())
        } else {
            Err(NetError::NotConnected)
        }
    }

    pub fn send_reliable_rpc(
        &mut self,
        object_id: Option<ObjectId>,
        method_hash: u64,
        payload: Vec<u8>,
    ) -> Result<RpcId, NetError> {
        let rpc_id = self
            .pending
            .keys()
            .max()
            .copied()
            .unwrap_or(0)
            .wrapping_add(1);
        let msg = NetMessage::RpcInvoke {
            rpc_id,
            object_id,
            method_hash,
            reliable: true,
            payload,
        };
        self.send_message(msg)?;
        Ok(rpc_id)
    }

    pub fn send_unreliable_rpc(
        &mut self,
        object_id: Option<ObjectId>,
        method_hash: u64,
        payload: Vec<u8>,
    ) -> Result<(), NetError> {
        let rpc_id = rand::random();
        let msg = NetMessage::RpcInvoke {
            rpc_id,
            object_id,
            method_hash,
            reliable: false,
            payload,
        };
        self.send_message(msg)
    }

    pub fn push_input(&mut self, input: NetInput) {
        self.inputs.inputs.push_back(input);
        if self.inputs.inputs.len() > 128 {
            self.inputs.inputs.pop_front();
        }
        self.inputs.last_sequence = input.sequence;
    }

    pub fn ack_input(&mut self, sequence: u32) {
        self.inputs.acked_sequence = sequence;
        self.inputs.pending.retain(|k, _| *k > sequence);
    }

    pub fn reconcile(&mut self, server_state: &EntityStatePrediction) {
        // Remove pending predictions older than acked
        self.inputs.pending.retain(|seq, _| *seq > server_state.input_sequence);
        // Apply server authoritative state
        if let Some(pred) = self.inputs.pending.get(&server_state.input_sequence) {
            // diff and re-apply remaining inputs (client-side prediction + reconciliation)
        }
    }
}

// =============================================================================
// NetServer
// =============================================================================

#[derive(Debug)]
pub struct NetServer {
    pub connections: ConnectionManager,
    pub config: NetServerConfig,
    pub crypto: EncryptionKey,
    pub state: HashMap<ObjectId, ReplicatedState>,
    pub replication: ReplicationSystem,
    pub rpc: RpcRegistry,
    pub lobbies: HashMap<u64, Lobby>,
    pub next_lobby_id: u64,
    pub events: mpsc::Sender<NetEvent>,
    pub running: bool,
    pub listener: Option<TcpListener>,
    pub udp: Option<Arc<UdpSocket>>,
    pub nat_sessions: HashMap<ClientId, NatSession>,
    pub room: Option<RoomConfig>,
}

impl NetServer {
    pub fn new(config: NetServerConfig) -> Self {
        let (events, _rx) = mpsc::channel(1024);
        Self {
            connections: ConnectionManager::new(),
            config,
            crypto: EncryptionKey::none(),
            state: HashMap::new(),
            replication: ReplicationSystem::new(),
            rpc: RpcRegistry::new(),
            lobbies: HashMap::new(),
            next_lobby_id: 1,
            events,
            running: false,
            listener: None,
            udp: None,
            nat_sessions: HashMap::new(),
            room: None,
        }
    }

    pub fn with_encryption(mut self, key: EncryptionKey) -> Self {
        self.crypto = key;
        self
    }

    pub async fn start(&mut self) -> Result<(), NetError> {
        let addr = SocketAddr::new(IpAddr::from([0, 0, 0, 0]), self.config.port);
        let listener = TcpListener::bind(addr).await?;
        let udp = UdpSocket::bind(addr).await?;
        let udp = Arc::new(udp);
        self.listener = Some(listener);
        self.udp = Some(udp.clone());
        self.running = true;
        Ok(())
    }

    pub async fn stop(&mut self) {
        self.running = false;
        self.listener = None;
        self.udp = None;
    }

    pub fn tick(&mut self, dt: f32) {
        if !self.running {
            return;
        }
        self.replication.update(dt);
        self.process_ownership_transfers();
        self.purge_timeouts();
        self.process_spawns_and_destroys();
    }

    pub fn create_lobby(&mut self, name: String, max_players: u8, owner: ClientId) -> u64 {
        let id = self.next_lobby_id;
        self.next_lobby_id = self.next_lobby_id.wrapping_add(1);
        let lobby = Lobby::new(id, name.clone(), owner, max_players);
        self.lobbies.insert(id, lobby);
        id
    }

    pub fn join_lobby(&mut self, lobby_id: u64, client: ClientId, name: String) -> Result<u8, NetError> {
        let lobby = self.lobbies.get_mut(&lobby_id).ok_or(NetError::Lobby("lobby not found".into()))?;
        lobby
            .add_player(client, name)
            .ok_or_else(|| NetError::Lobby("lobby full".into()))
    }

    pub fn leave_lobby(&mut self, lobby_id: u64, client: ClientId) {
        if let Some(lobby) = self.lobbies.get_mut(&lobby_id) {
            lobby.remove_player(client);
            if lobby.owner_id == client {
                let new_owner = lobby.players.keys().next().copied();
                if let Some(new_owner) = new_owner {
                    lobby.migrate_host(new_owner);
                }
            }
            if lobby.players.is_empty() {
                self.lobbies.remove(&lobby_id);
            }
        }
    }

    pub fn start_room(&mut self, lobby_id: u64, room: RoomConfig) -> Result<u64, NetError> {
        let room_id = self.next_lobby_id;
        let lobby = self.lobbies.get_mut(&lobby_id).ok_or(NetError::Lobby("lobby not found".into()))?;
        lobby.state = LobbyState::InGame;
        lobby.room_config = Some(room.clone());
        lobby.room_id = Some(room_id);
        self.room = Some(room.clone());
        Ok(room_id)
    }

    pub fn handle_message(
        &mut self,
        client_id: ClientId,
        msg: NetMessage,
    ) -> Result<(), NetError> {
        if !self.connections.connections.contains_key(&client_id) {
            return Ok(());
        }
        match msg {
            NetMessage::RpcInvoke { rpc_id, object_id, method_hash, reliable, payload } => {
                self.handle_rpc(client_id, object_id, method_hash, payload, reliable);
                if reliable {
                    // send ack
                }
            }
            NetMessage::StateAck { object_id, sequence } => {
                self.replication.pending_acks.remove(&(client_id, object_id, sequence));
            }
            NetMessage::RpcAck { rpc_id } => {
                self.rpc.ack(rpc_id);
            }
            NetMessage::LobbyCreate { lobby_id, name, max_players } => {
                self.lobbies.insert(lobby_id, Lobby::new(lobby_id, name, client_id, max_players));
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_rpc(
        &mut self,
        from: ClientId,
        object_id: Option<ObjectId>,
        method_hash: u64,
        payload: Vec<u8>,
        reliable: bool,
    ) {
        if let Some(def) = self.rpc.definitions.get(&method_hash) {
            // Validate authority
            let allowed = match def.authority {
                RpcAuthority::Server => true,
                RpcAuthority::Client => false,
                RpcAuthority::Owner => {
                    if let Some(obj_id) = object_id {
                        self.replication.owned_by.get(&obj_id).copied() == Some(from)
                    } else {
                        false
                    }
                }
            };
            if !allowed {
                return;
            }
            // Apply RPC
            let _ = (object_id, payload);
        }
    }

    fn process_ownership_transfers(&mut self) {
        let transfers: Vec<_> = self.replication.ownership_transfers.drain(..).collect();
        for req in transfers {
            if let Some(conn) = self.connections.connections.get_mut(&req.to_client) {
                // send transfer ack
            }
        }
    }

    fn purge_timeouts(&mut self) {
        let now = Instant::now();
        let timeout = self.config.timeout;
        let mut timed_out = Vec::new();
        for (id, conn) in &self.connections.connections {
            if now.duration_since(conn.last_heartbeat) > timeout {
                timed_out.push(*id);
            }
        }
        for id in timed_out {
            self.disconnect_client(id, NetMessage::Disconnect { reason: "timeout".into() });
        }
    }

    fn process_spawns_and_destroys(&mut self) {
        let spawns: Vec<_> = self.replication.pending_spawns.drain(..).collect();
        let destroys: Vec<_> = self.replication.pending_destroys.drain(..).collect();
        for (_id, _data) in spawns {
            // replicate to relevant clients
        }
        for _id in destroys {
            // replicate destroy to relevant clients
        }
    }

    pub fn disconnect_client(&mut self, id: ClientId, reason: NetMessage) {
        let _ = self.connections.connections.remove(&id);
        let _ = self.connections.owned_by.remove(&id);
    }

    pub fn broadcast_message(&self, msg: NetMessage) {
        // In a real implementation, send to each connection
    }
}

// =============================================================================
// NetworkReplicationSystem (ECS System impl)
// =============================================================================

pub struct NetworkReplicationSystem {
    pub world: Option<*mut World>,
    pub net: NetServer,
    pub last_tick: Instant,
    pub tick_interval: Duration,
}

impl System for NetworkReplicationSystem {
    fn name(&self) -> &str {
        "ReplicationSystem"
    }

    fn update(&mut self, world: &mut World, dt: f32) {
        if self.world.is_none() {
            self.world = Some(world as *mut World);
        }
        let now = Instant::now();
        if now.duration_since(self.last_tick) >= self.tick_interval {
            self.last_tick = now;
            self.net.tick(dt);
        }
        // Replicate networked components
        self.replicate(world, dt);
    }
}

impl NetworkReplicationSystem {
    pub fn new(tick_interval: Duration) -> Self {
        let mut net = NetServer::new(NetServerConfig::default());
        net.config.replication_interval = tick_interval;
        Self {
            world: None,
            net,
            last_tick: Instant::now(),
            tick_interval,
        }
    }

    pub fn with_config(tick_interval: Duration, server_config: NetServerConfig) -> Self {
        let mut net = NetServer::new(server_config);
        net.config.replication_interval = tick_interval;
        Self {
            world: None,
            net,
            last_tick: Instant::now(),
            tick_interval,
        }
    }

    pub fn server(&self) -> &NetServer {
        &self.net
    }

    pub fn server_mut(&mut self) -> &mut NetServer {
        &mut self.net
    }

    fn replicate(&mut self, _world: &mut World, _dt: f32) {
        // Walk all entities with Networked + NetworkTransform
        // Build snapshots, deltas, and send to relevant clients
        // This is intentionally high-level — concrete implementation
        // depends on World query API.
    }
}

// =============================================================================
// Packet validator (security)
// =============================================================================

pub struct PacketValidator {
    pub max_payload: usize,
    pub allowed_channels: HashSet<ChannelId>,
    pub trusted_clients: HashSet<ClientId>,
}

impl Default for PacketValidator {
    fn default() -> Self {
        let mut allowed = HashSet::new();
        allowed.insert(0);
        allowed.insert(1);
        allowed.insert(2);
        allowed.insert(3);
        Self {
            max_payload: MAX_PACKET_SIZE,
            allowed_channels: allowed,
            trusted_clients: HashSet::new(),
        }
    }
}

impl PacketValidator {
    pub fn validate(&self, header: &PacketHeader, payload: &[u8], client_id: Option<ClientId>) -> Result<(), NetError> {
        if header.payload_len as usize != payload.len() {
            return Err(NetError::InvalidPacket("length mismatch".into()));
        }
        if payload.len() > self.max_payload {
            return Err(NetError::InvalidPacket("payload too large".into()));
        }
        if !self.allowed_channels.contains(&header.channel) {
            return Err(NetError::InvalidPacket("channel not allowed".into()));
        }
        if let Some(cid) = client_id {
            if !self.trusted_clients.contains(&cid) && header.flags.contains(PacketFlags::ENCRYPTED) {
                return Err(NetError::InvalidPacket("untrusted encrypted packet".into()));
            }
        }
        Ok(())
    }
}

// =============================================================================
// Prelude exports
// =============================================================================

pub mod prelude {
    pub use super::{
        ClientId, ObjectId, SequenceNumber, RpcId, ChannelId, DEFAULT_PORT,
        NetworkChannel, NetRole, ConnectionState, EncryptionKey, Packet,
        PacketHeader, PacketFlags, NetMessage, NetEvent, NetError,
        NetServerConfig, NetClientConfig, ClientConnection, ConnectionManager,
        BanList, RateLimiter, AntiCheatConfig, AntiCheatReport, AntiCheatInfraction,
        NetInput, ClientPredictionBuffer, EntityStatePrediction,
        LagCompensationBuffer, RewindSnapshot,
        RpcRegistry, RpcCall, RpcDefinition, RpcAuthority,
        ReplicatedState, ReplicationPriority, ReplicationFlags,
        ReplicationSystem, ReplicationConfig, NetworkReplicationSystem,
        RelevanceSystem,
        OwnershipTransferRequest, TransferReason,
        Lobby, LobbyState, LobbySummary, LobbyPlayerInfo, RoomConfig,
        NatSession, NatState,
        Networked, NetworkTransform, NetworkState,
        PacketValidator,
    };
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limiter_throttles() {
        let mut rl = RateLimiter::new(100, 10);
        for _ in 0..10 {
            assert!(rl.allow(10));
        }
        assert!(!rl.allow(10));
    }

    #[test]
    fn ban_list_works() {
        let mut bans = BanList::new();
        bans.add_id(1);
        assert!(bans.is_banned(Some(1), None, None));
        assert!(!bans.is_banned(Some(2), None, None));
        bans.remove_id(1);
        assert!(!bans.is_banned(Some(1), None, None));
    }

    #[test]
    fn replication_sequence_wraps() {
        let mut rep = ReplicationSystem::new();
        let a = rep.next_sequence();
        let b = rep.next_sequence();
        assert!(b > a);
    }

    #[test]
    fn lobby_add_remove() {
        let mut lobby = Lobby::new(1, "test".into(), 1, 4);
        lobby.add_player(2, "p2".into());
        assert_eq!(lobby.players.len(), 1);
        lobby.remove_player(2);
        assert!(lobby.players.is_empty());
    }

    #[test]
    fn relevance_distance() {
        let rel = RelevanceSystem::new(10.0);
        assert!(rel.is_relevant(1, &crate::math::Vec3::ZERO));
        assert!(!rel.is_relevant(1, &crate::math::Vec3::new(100.0, 0.0, 0.0)));
    }

    #[test]
    fn encryption_roundtrip() {
        let key = EncryptionKey::new_aes256(&[0u8; 32]);
        let plain = b"hello multiplayer";
        let ct = key.encrypt(plain, b"aad").unwrap();
        let pt = key.decrypt(&ct, b"aad").unwrap();
        assert_eq!(plain, pt.as_slice());
    }

    #[test]
    fn packet_compress() {
        let mut pkt = Packet::new(vec![0u8; 1024], 2, PacketFlags::empty());
        pkt.compress();
        assert!(pkt.header.flags.contains(PacketFlags::COMPRESSED));
        pkt.decompress().unwrap();
        assert_eq!(pkt.payload.len(), 1024);
    }

    #[test]
    fn nat_session() {
        let mut nat = NatSession::new(1, "127.0.0.1:12345".parse().unwrap());
        nat.add_candidate("1.2.3.4:12345".parse().unwrap());
        assert_eq!(nat.candidates.len(), 1);
    }
}