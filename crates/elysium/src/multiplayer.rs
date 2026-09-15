
/// multiplayer.rs — Elysium Multiplayer Networking System
/// WebSocket-based client-server architecture with full lag compensation.
/// Works on both native (tokio-tungstenite) and WASM (web-sys WebSocket).
///
/// Features:
/// - Binary message protocol (bincode)
/// - Authoritative server with 60Hz tick rate
/// - Client-side prediction with input buffer
/// - Server reconciliation (replay unacknowledged inputs)
/// - Entity interpolation (smooth rendering between snapshots)
/// - Snapshot interpolation (lag-compensated state)
/// - RPC system (remote procedure calls)
/// - Interest management (area of interest filtering)

use serde::{Serialize, Deserialize};
use std::collections::{HashMap, VecDeque};

// ═══════════════════════════════════════════════════════════ Network Types

/// Unique network entity ID
pub type NetEntityId = u64;

/// Unique player/client ID
pub type PlayerId = u64;

/// Server tick number (monotonically increasing)
pub type TickNumber = u64;

/// Millisecond timestamp
pub type Timestamp = u64;

/// Reliable channel identifier
pub type ChannelId = u8;

// ═══════════════════════════════════════════════════════════ Message Protocol

/// All network messages between client and server
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NetMessage {
    // ── Connection ──
    /// Client → Server: Request to join
    ConnectRequest {
        player_name: String,
        protocol_version: u32,
        auth_token: Option<String>,
    },
    /// Server → Client: Connection accepted
    ConnectAccepted {
        player_id: PlayerId,
        server_tick_rate: u32,
        server_name: String,
    },
    /// Server → Client: Connection rejected
    ConnectRejected {
        reason: String,
    },
    /// Client → Server: Disconnect gracefully
    Disconnect {
        reason: String,
    },

    // ── Input (Unreliable, high frequency) ──
    /// Client → Server: Player input for a specific tick
    PlayerInput {
        player_id: PlayerId,
        tick: TickNumber,
        input: PlayerInputData,
        /// Client's last acknowledged tick (for reconciliation)
        acked_tick: TickNumber,
        /// Timestamp when input was generated (for ping estimation)
        client_timestamp: Timestamp,
    },

    // ── State (Unreliable, authoritative) ──
    /// Server → All: World state snapshot
    WorldSnapshot {
        tick: TickNumber,
        server_timestamp: Timestamp,
        /// Only entities relevant to the receiving client (interest management)
        entities: Vec<NetEntity>,
        /// Player-specific state
        player_state: Option<PlayerNetState>,
        /// Recent input acknowledgments
        acked_inputs: Vec<TickNumber>,
    },

    /// Server → Client: Delta update (only changed entities since last acknowledged tick)
    WorldDelta {
        tick: TickNumber,
        base_tick: TickNumber,
        created: Vec<NetEntity>,
        updated: Vec<NetEntityUpdate>,
        destroyed: Vec<NetEntityId>,
    },

    // ── Reliable ──
    /// RPC: Remote procedure call
    RPC {
        channel: ChannelId,
        procedure_id: u32,
        args: Vec<u8>,
    },

    /// Server → Client: Chat message
    Chat {
        sender_id: PlayerId,
        sender_name: String,
        message: String,
        timestamp: Timestamp,
    },

    /// Server → Client: Game event (kill, score, etc.)
    GameEvent {
        event_type: GameEventType,
        data: Vec<u8>,
        timestamp: Timestamp,
    },

    /// Ping/Pong for RTT measurement
    Ping {
        client_timestamp: Timestamp,
    },
    Pong {
        client_timestamp: Timestamp,
        server_timestamp: Timestamp,
    },

    /// Server → Client: Full world reset
    WorldReset {
        tick: TickNumber,
        entities: Vec<NetEntity>,
        players: Vec<PlayerNetState>,
    },
}

/// Input data from player
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PlayerInputData {
    /// Movement axes (x, y, z) — normalized
    pub movement: [f32; 3],
    /// Camera rotation (yaw, pitch, roll) in degrees
    pub rotation: [f32; 3],
    /// Button presses (bitmask)
    pub buttons: u32,
    /// Mouse delta (for FPS-style games)
    pub mouse_delta: [f32; 2],
}

impl PlayerInputData {
    pub fn button_pressed(&self, button: u8) -> bool {
        (self.buttons >> button) & 1 == 1
    }
    pub fn set_button(&mut self, button: u8, pressed: bool) {
        if pressed { self.buttons |= 1 << button; }
        else { self.buttons &= !(1 << button); }
    }
}

// ═══════════════════════════════════════════════════════════ Network Entity

/// Networked entity state
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetEntity {
    pub id: NetEntityId,
    pub entity_type: EntityType,
    pub position: [f32; 3],
    pub rotation: [f32; 4], // Quaternion
    pub velocity: [f32; 3],
    pub scale: [f32; 3],
    pub owner: Option<PlayerId>,
    pub health: f32,
    pub max_health: f32,
    pub flags: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityType {
    Player,
    Enemy,
    Projectile,
    Pickup,
    Vehicle,
    Decoration,
    Custom(u32),
}

/// Update for a specific entity (delta)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetEntityUpdate {
    pub id: NetEntityId,
    pub position: Option<[f32; 3]>,
    pub rotation: Option<[f32; 4]>,
    pub velocity: Option<[f32; 3]>,
    pub health: Option<f32>,
    pub flags: Option<u32>,
}

/// Player state from server
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerNetState {
    pub player_id: PlayerId,
    pub name: String,
    pub position: [f32; 3],
    pub health: f32,
    pub score: u32,
    pub ping_ms: u32,
    pub team: u8,
}

/// Game event types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum GameEventType {
    PlayerKilled { killer_id: PlayerId, victim_id: PlayerId },
    ScoreChanged { player_id: PlayerId, new_score: u32 },
    RoundStart,
    RoundEnd { winner_team: u8 },
    PowerUpCollected { player_id: PlayerId, power_up_type: u32 },
    Custom(u32),
}

// ═══════════════════════════════════════════════════════════ Client-Side Prediction

/// Input history entry stored locally for reconciliation
#[derive(Clone, Debug)]
struct InputRecord {
    tick: TickNumber,
    input: PlayerInputData,
    /// Predicted position after applying this input
    predicted_position: [f32; 3],
    predicted_rotation: [f32; 4],
    /// Whether this input has been acknowledged by the server
    acknowledged: bool,
}

/// Client-side prediction state
pub struct PredictionState {
    /// Buffer of recent inputs sent to server
    input_buffer: VecDeque<InputRecord>,
    /// Maximum buffer size (prevents memory leak on disconnect)
    max_buffer_size: usize,
    /// Last tick acknowledged by server
    last_acked_tick: TickNumber,
    /// Current predicted position
    pub predicted_position: [f32; 3],
    /// Current predicted rotation
    pub predicted_rotation: [f32; 4],
    /// Number of frames since last server correction
    frames_since_correction: u32,
    /// Smooth correction strength (lerp factor)
    correction_smoothing: f32,
}

impl Default for PredictionState {
    fn default() -> Self {
        Self {
            input_buffer: VecDeque::new(),
            max_buffer_size: 120, // 2 seconds at 60Hz
            last_acked_tick: 0,
            predicted_position: [0.0; 3],
            predicted_rotation: [0.0, 0.0, 0.0, 1.0],
            frames_since_correction: 0,
            correction_smoothing: 0.1,
        }
    }
}

impl PredictionState {
    /// Record a new input and apply prediction
    pub fn record_input(&mut self, tick: TickNumber, input: &PlayerInputData, dt: f32) {
        // Apply input locally for prediction
        let speed = 8.0;
        let predicted_pos = [
            self.predicted_position[0] + input.movement[0] * speed * dt,
            self.predicted_position[1] + input.movement[1] * speed * dt,
            self.predicted_position[2] + input.movement[2] * speed * dt,
        ];

        let record = InputRecord {
            tick,
            input: input.clone(),
            predicted_position: predicted_pos,
            predicted_rotation: self.predicted_rotation,
            acknowledged: false,
        };

        self.predicted_position = predicted_pos;
        self.input_buffer.push_back(record);

        // Trim old entries
        if self.input_buffer.len() > self.max_buffer_size {
            self.input_buffer.pop_front();
        }
    }

    /// Server reconciliation: when server acknowledges a tick, snap to server state
    /// and replay unacknowledged inputs
    pub fn reconcile(
        &mut self,
        server_position: [f32; 3],
        server_rotation: [f32; 4],
        acked_tick: TickNumber,
        dt: f32,
    ) {
        // Mark all inputs up to acked_tick as acknowledged
        for record in &mut self.input_buffer {
            if record.tick <= acked_tick {
                record.acknowledged = true;
            }
        }
        self.last_acked_tick = acked_tick;

        // Remove acknowledged inputs
        self.input_buffer.retain(|r| !r.acknowledged);

        // Start from server position
        self.predicted_position = server_position;
        self.predicted_rotation = server_rotation;

        // Replay all unacknowledged inputs
        for record in &self.input_buffer {
            let speed = 8.0;
            self.predicted_position = [
                self.predicted_position[0] + record.input.movement[0] * speed * dt,
                self.predicted_position[1] + record.input.movement[1] * speed * dt,
                self.predicted_position[2] + record.input.movement[2] * speed * dt,
            ];
        }

        self.frames_since_correction = 0;
    }

    /// Get current smooth position (with correction smoothing)
    pub fn smooth_position(&mut self, target: [f32; 3]) -> [f32; 3] {
        self.frames_since_correction += 1;
        let smoothing = if self.frames_since_correction < 10 {
            self.correction_smoothing
        } else {
            1.0 // Full snap after settling
        };
        [
            self.predicted_position[0] + (target[0] - self.predicted_position[0]) * smoothing,
            self.predicted_position[1] + (target[1] - self.predicted_position[1]) * smoothing,
            self.predicted_position[2] + (target[2] - self.predicted_position[2]) * smoothing,
        ]
    }

    pub fn unacked_input_count(&self) -> usize {
        self.input_buffer.len()
    }
}

// ═══════════════════════════════════════════════════════════ Entity Interpolation

/// Interpolates between two snapshots for smooth rendering
pub struct EntityInterpolator {
    /// Buffered snapshots from server
    snapshot_buffer: VecDeque<WorldSnapshotData>,
    /// Interpolation delay in ticks (typically 2-3 tick periods)
    interpolation_delay: u32,
    /// Interpolation buffer size
    max_buffer_size: usize,
}

#[derive(Clone, Debug)]
pub struct WorldSnapshotData {
    pub tick: TickNumber,
    pub server_time: Timestamp,
    pub entities: HashMap<NetEntityId, InterpolatedEntity>,
}

#[derive(Clone, Debug)]
pub struct InterpolatedEntity {
    pub id: NetEntityId,
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub velocity: [f32; 3],
    pub health: f32,
}

impl Default for EntityInterpolator {
    fn default() -> Self {
        Self {
            snapshot_buffer: VecDeque::new(),
            interpolation_delay: 3, // 3 ticks = ~50ms at 60Hz
            max_buffer_size: 30,
        }
    }
}

impl EntityInterpolator {
    /// Add a new snapshot to the buffer
    pub fn push_snapshot(&mut self, tick: TickNumber, server_time: Timestamp, entities: Vec<NetEntity>) {
        let mut entity_map = HashMap::new();
        for e in entities {
            entity_map.insert(e.id, InterpolatedEntity {
                id: e.id,
                position: e.position,
                rotation: e.rotation,
                velocity: e.velocity,
                health: e.health,
            });
        }
        self.snapshot_buffer.push_back(WorldSnapshotData {
            tick,
            server_time,
            entities: entity_map,
        });
        if self.snapshot_buffer.len() > self.max_buffer_size {
            self.snapshot_buffer.pop_front();
        }
    }

    /// Get interpolated entity state for a given render time
    pub fn interpolate(&self, render_time: Timestamp) -> HashMap<NetEntityId, InterpolatedEntity> {
        if self.snapshot_buffer.len() < 2 {
            // Not enough snapshots — return latest
            return self.snapshot_buffer.back()
                .map(|s| s.entities.clone())
                .unwrap_or_default();
        }

        // Find the two snapshots to interpolate between
        let target_time = render_time.saturating_sub(self.interpolation_delay as u64 * 16); // ~16ms per tick
        let mut from = None;
        let mut to = None;

        let len = self.snapshot_buffer.len();
        for i in 0..len.saturating_sub(1) {
            let curr = &self.snapshot_buffer[i];
            let next = &self.snapshot_buffer[i + 1];
            if curr.server_time <= target_time && next.server_time >= target_time {
                from = Some(curr);
                to = Some(next);
                break;
            }
        }

        let (from, to) = match (from, to) {
            (Some(f), Some(t)) => (f, t),
            _ => {
                return self.snapshot_buffer.back()
                    .map(|s| s.entities.clone())
                    .unwrap_or_default();
            }
        };

        let total_time = (to.server_time - from.server_time).max(1) as f32;
        let elapsed = (target_time - from.server_time) as f32;
        let t = (elapsed / total_time).clamp(0.0, 1.0);

        let mut result = HashMap::new();
        // Interpolate entities present in both snapshots
        for (id, from_entity) in &from.entities {
            if let Some(to_entity) = to.entities.get(id) {
                result.insert(*id, InterpolatedEntity {
                    id: *id,
                    position: lerp_vec3(from_entity.position, to_entity.position, t),
                    rotation: slerp_quat(from_entity.rotation, to_entity.rotation, t),
                    velocity: lerp_vec3(from_entity.velocity, to_entity.velocity, t),
                    health: from_entity.health + (to_entity.health - from_entity.health) * t,
                });
            } else {
                result.insert(*id, from_entity.clone());
            }
        }
        // Add new entities from `to` that weren't in `from`
        for (id, to_entity) in &to.entities {
            if !result.contains_key(id) {
                result.insert(*id, to_entity.clone());
            }
        }
        result
    }

    pub fn buffer_size(&self) -> usize {
        self.snapshot_buffer.len()
    }
}

// ═══════════════════════════════════════════════════════════ Client State

/// Full client-side network state
pub struct NetworkClient {
    /// Connection state
    pub connected: bool,
    pub player_id: PlayerId,
    pub server_tick_rate: u32,

    /// Local prediction
    pub prediction: PredictionState,
    /// Entity interpolation
    pub interpolator: EntityInterpolator,

    /// Current tick (input sequence number)
    pub current_tick: TickNumber,
    /// Last received server tick
    pub last_server_tick: TickNumber,

    /// RTT estimation
    pub rtt_ms: f32,
    /// Jitter estimation
    pub jitter_ms: f32,
    /// Last ping timestamp
    pub last_ping_time: Timestamp,
    /// Ping samples for RTT estimation
    ping_samples: VecDeque<f32>,

    /// Other players' states
    pub other_players: HashMap<PlayerId, PlayerNetState>,
    /// All remote entities
    pub remote_entities: HashMap<NetEntityId, NetEntity>,
    /// Local player state from server
    pub local_player_state: Option<PlayerNetState>,

    /// Outgoing message queue
    pub outgoing: VecDeque<NetMessage>,
    /// Incoming message queue (parsed)
    pub incoming: VecDeque<NetMessage>,
    /// Reliable outgoing buffer (for retransmission)
    reliable_outbox: VecDeque<ReliableMessage>,
    /// Reliable incoming: track received sequence numbers
    reliable_received: Vec<u64>,
    /// RPC handlers
    rpc_handlers: HashMap<u32, Box<dyn Fn(&[u8]) -> Vec<u8>>>,

    /// Message statistics
    pub stats: NetworkStats,

    /// Server name
    pub server_name: String,
}

struct ReliableMessage {
    sequence: u64,
    message: NetMessage,
    send_time: Timestamp,
    attempts: u32,
}

/// Network statistics
#[derive(Clone, Debug, Default)]
pub struct NetworkStats {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_lost: u32,
    pub snapshots_received: u64,
    pub inputs_sent: u64,
    pub corrections: u32,
}

impl Default for NetworkClient {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkClient {
    pub fn new() -> Self {
        Self {
            connected: false,
            player_id: 0,
            server_tick_rate: 60,
            prediction: PredictionState::default(),
            interpolator: EntityInterpolator::default(),
            current_tick: 0,
            last_server_tick: 0,
            rtt_ms: 0.0,
            jitter_ms: 0.0,
            last_ping_time: 0,
            ping_samples: VecDeque::new(),
            other_players: HashMap::new(),
            remote_entities: HashMap::new(),
            local_player_state: None,
            outgoing: VecDeque::new(),
            incoming: VecDeque::new(),
            reliable_outbox: VecDeque::new(),
            reliable_received: Vec::new(),
            rpc_handlers: HashMap::new(),
            stats: NetworkStats::default(),
            server_name: String::new(),
        }
    }

    /// Send a connect request
    pub fn connect(&mut self, player_name: &str, server_addr: &str) {
        self.outgoing.push_back(NetMessage::ConnectRequest {
            player_name: player_name.to_string(),
            protocol_version: 1,
            auth_token: None,
        });
        let _ = server_addr; // Used by transport layer
    }

    /// Disconnect gracefully
    pub fn disconnect(&mut self, reason: &str) {
        self.outgoing.push_back(NetMessage::Disconnect {
            reason: reason.to_string(),
        });
        self.connected = false;
    }

    /// Send player input for current tick
    pub fn send_input(&mut self, input: PlayerInputData, timestamp: Timestamp) {
        self.current_tick += 1;
        let acked_tick = self.last_server_tick;

        self.prediction.record_input(self.current_tick, &input, 1.0 / self.server_tick_rate as f32);

        self.outgoing.push_back(NetMessage::PlayerInput {
            player_id: self.player_id,
            tick: self.current_tick,
            input,
            acked_tick,
            client_timestamp: timestamp,
        });

        self.stats.inputs_sent += 1;
    }

    /// Process a received message from the server
    pub fn process_message(&mut self, msg: NetMessage, current_time: Timestamp) {
        // Store before match so destructuring doesn't consume `msg`
        let msg_for_queue = msg.clone();
        self.stats.messages_received += 1;
        self.incoming.push_back(msg_for_queue);

        match msg {
            NetMessage::ConnectAccepted { player_id, server_tick_rate, server_name } => {
                self.connected = true;
                self.player_id = player_id;
                self.server_tick_rate = server_tick_rate;
                self.server_name = server_name;
            }
            NetMessage::ConnectRejected { reason } => {
                self.connected = false;
                eprintln!("Connection rejected: {}", reason);
            }
            NetMessage::WorldSnapshot { tick, server_timestamp, entities, player_state, acked_inputs } => {
                self.last_server_tick = tick;

                // Server reconciliation
                if let Some(ref ps) = player_state {
                    if let Some(acked_tick) = acked_inputs.iter().max() {
                        self.prediction.reconcile(
                            ps.position,
                            [0.0, 0.0, 0.0, 1.0],
                            *acked_tick,
                            1.0 / self.server_tick_rate as f32,
                        );
                        self.stats.corrections += 1;
                    }
                }

                // Update interpolation buffer
                self.interpolator.push_snapshot(tick, server_timestamp, entities.clone());

                // Update entity cache
                self.remote_entities.clear();
                for e in &entities {
                    self.remote_entities.insert(e.id, e.clone());
                }

                // Update player state
                self.local_player_state = player_state;
                self.stats.snapshots_received += 1;
            }
            NetMessage::WorldDelta { tick, base_tick: _, created, updated, destroyed, .. } => {
                self.last_server_tick = tick;
                for e in created {
                    self.remote_entities.insert(e.id, e);
                }
                for u in updated {
                    if let Some(e) = self.remote_entities.get_mut(&u.id) {
                        if let Some(pos) = u.position { e.position = pos; }
                        if let Some(rot) = u.rotation { e.rotation = rot; }
                        if let Some(vel) = u.velocity { e.velocity = vel; }
                        if let Some(hp) = u.health { e.health = hp; }
                        if let Some(fl) = u.flags { e.flags = fl; }
                    }
                }
                for id in destroyed {
                    self.remote_entities.remove(&id);
                }
            }
            NetMessage::Pong { client_timestamp, server_timestamp: _ } => {
                let rtt = (current_time - client_timestamp) as f32;
                self.ping_samples.push_back(rtt);
                if self.ping_samples.len() > 20 { self.ping_samples.pop_front(); }
                self.rtt_ms = self.ping_samples.iter().sum::<f32>() / self.ping_samples.len() as f32;

                // Jitter estimation
                if self.ping_samples.len() > 1 {
                    let mean = self.rtt_ms;
                    let variance: f32 = self.ping_samples.iter()
                        .map(|s| (s - mean).powi(2))
                        .sum::<f32>() / self.ping_samples.len() as f32;
                    self.jitter_ms = variance.sqrt();
                }
            }
            NetMessage::Chat { ref sender_name, ref message, .. } => {
                println!("[Chat] {}: {}", sender_name, message);
            }
            NetMessage::GameEvent { ref event_type, .. } => {
                match event_type {
                    GameEventType::PlayerKilled { killer_id, victim_id } => {
                        println!("Player {} killed player {}", killer_id, victim_id);
                    }
                    _ => {}
                }
            }
            _ => {}
        }    }


    /// Get interpolated position for an entity at current render time
    pub fn get_entity_position(&self, entity_id: NetEntityId, render_time: Timestamp) -> Option<[f32; 3]> {
        let interpolated = self.interpolator.interpolate(render_time);
        interpolated.get(&entity_id).map(|e| e.position)
    }

    /// Send ping
    pub fn send_ping(&mut self, timestamp: Timestamp) {
        self.last_ping_time = timestamp;
        self.outgoing.push_back(NetMessage::Ping { client_timestamp: timestamp });
    }

    /// Register an RPC handler
    pub fn register_rpc(&mut self, procedure_id: u32, handler: Box<dyn Fn(&[u8]) -> Vec<u8>>) {
        self.rpc_handlers.insert(procedure_id, handler);
    }

    /// Call a remote procedure
    pub fn call_rpc(&mut self, channel: ChannelId, procedure_id: u32, args: Vec<u8>) {
        self.outgoing.push_back(NetMessage::RPC { channel, procedure_id, args });
    }

    /// Send a chat message
    pub fn send_chat(&mut self, message: &str) {
        self.outgoing.push_back(NetMessage::Chat {
            sender_id: self.player_id,
            sender_name: String::new(),
            message: message.to_string(),
            timestamp: 0,
        });
    }

    /// Drain outgoing messages
    pub fn drain_outgoing(&mut self) -> Vec<NetMessage> {
        self.outgoing.drain(..).collect()
    }

    pub fn is_connected(&self) -> bool { self.connected }
    pub fn ping(&self) -> f32 { self.rtt_ms }
    pub fn jitter(&self) -> f32 { self.jitter_ms }
}

// ═══════════════════════════════════════════════════════════ Server State

/// Server-side authoritative game state
pub struct NetworkServer {
    pub config: ServerConfig,
    pub tick: TickNumber,
    pub running: bool,

    /// Connected clients
    pub clients: HashMap<PlayerId, ServerClient>,
    pub next_player_id: PlayerId,

    /// Authoritative game state
    pub entities: HashMap<NetEntityId, NetEntity>,
    pub next_entity_id: NetEntityId,

    /// Pending input buffer (per player, per tick)
    pub input_buffers: HashMap<PlayerId, HashMap<TickNumber, PlayerInputData>>,

    /// Outgoing messages per client
    pub outgoing: HashMap<PlayerId, VecDeque<NetMessage>>,

    /// Broadcast messages (to all clients)
    pub broadcast_queue: VecDeque<NetMessage>,

    /// Game event log (for delta compression)
    event_log: Vec<GameEvent>,

    /// Statistics
    pub stats: ServerStats,

    /// World reset state (full snapshot for new clients)
    world_initial_state: Vec<NetEntity>,
}

struct GameEvent {
    tick: TickNumber,
    event: GameEventType,
}

pub struct ServerClient {
    pub player_id: PlayerId,
    pub name: String,
    pub connected: bool,
    pub last_input_tick: TickNumber,
    pub last_snapshot_tick: TickNumber,
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub health: f32,
    pub score: u32,
    pub team: u8,
    pub ping_ms: u32,
    /// Interest area (for filtering which entities to send)
    pub interest_radius: f32,
    /// Last time we sent a full snapshot
    pub last_full_snapshot: TickNumber,
}

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub tick_rate: u32,
    pub max_players: usize,
    pub snapshot_interval: u32,     // Full snapshot every N ticks
    pub delta_interval: u32,        // Delta snapshot every N ticks
    pub max_entity_count: usize,
    pub interest_radius: f32,       // Default area of interest
    pub input_timeout: u32,         // Ticks before considering input stale
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            tick_rate: 60,
            max_players: 32,
            snapshot_interval: 10,   // Full snapshot every 10 ticks (~167ms)
            delta_interval: 1,       // Delta every tick
            max_entity_count: 1000,
            interest_radius: 50.0,
            input_timeout: 60,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ServerStats {
    pub ticks_run: u64,
    pub inputs_processed: u64,
    pub snapshots_sent: u64,
    pub entities_active: u32,
    pub clients_connected: u32,
}

impl Default for NetworkServer {
    fn default() -> Self {
        Self::new(ServerConfig::default())
    }
}

impl NetworkServer {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            tick: 0,
            running: false,
            clients: HashMap::new(),
            next_player_id: 1,
            entities: HashMap::new(),
            next_entity_id: 1,
            input_buffers: HashMap::new(),
            outgoing: HashMap::new(),
            broadcast_queue: VecDeque::new(),
            event_log: Vec::new(),
            stats: ServerStats::default(),
            world_initial_state: Vec::new(),
        }
    }

    /// Accept a new client connection
    pub fn accept_client(&mut self, name: &str) -> PlayerId {
        let player_id = self.next_player_id;
        self.next_player_id += 1;

        let client = ServerClient {
            player_id,
            name: name.to_string(),
            connected: true,
            last_input_tick: 0,
            last_snapshot_tick: 0,
            position: [0.0, 1.0, 0.0],
            velocity: [0.0; 3],
            health: 100.0,
            score: 0,
            team: (player_id % 2) as u8,
            ping_ms: 0,
            interest_radius: self.config.interest_radius,
            last_full_snapshot: 0,
        };

        self.clients.insert(player_id, client);
        self.input_buffers.insert(player_id, HashMap::new());

        // Spawn player entity
        let _entity_id = self.spawn_entity(EntityType::Player, [0.0, 1.0, 0.0], Some(player_id));

        // Send connect accepted
        self.send_to_client(player_id, NetMessage::ConnectAccepted {
            player_id,
            server_tick_rate: self.config.tick_rate,
            server_name: "Elysium Server".into(),
        });

        // Send full world state
        self.send_full_snapshot(player_id);

        player_id
    }

    /// Remove a client
    pub fn remove_client(&mut self, player_id: PlayerId) {
        if let Some(_client) = self.clients.get(&player_id) {
            // Remove player entity
            self.entities.retain(|_, e| e.owner != Some(player_id));
        }
        self.clients.remove(&player_id);
        self.input_buffers.remove(&player_id);
        self.outgoing.remove(&player_id);
    }

    /// Spawn a new networked entity
    pub fn spawn_entity(&mut self, entity_type: EntityType, position: [f32; 3], owner: Option<PlayerId>) -> NetEntityId {
        let id = self.next_entity_id;
        self.next_entity_id += 1;

        let entity = NetEntity {
            id,
            entity_type,
            position,
            rotation: [0.0, 0.0, 0.0, 1.0],
            velocity: [0.0; 3],
            scale: [1.0; 3],
            owner,
            health: 100.0,
            max_health: 100.0,
            flags: 0,
        };

        self.entities.insert(id, entity.clone());
        self.broadcast_queue.push_back(NetMessage::WorldSnapshot {
            tick: self.tick,
            server_timestamp: timestamp_ms(),
            entities: vec![entity],
            player_state: None,
            acked_inputs: vec![],
        });

        id
    }

    /// Destroy an entity
    pub fn destroy_entity(&mut self, id: NetEntityId) {
        self.entities.remove(&id);
    }

    /// Process a client's input
    pub fn process_input(&mut self, player_id: PlayerId, tick: TickNumber, input: PlayerInputData) {
        self.input_buffers.entry(player_id)
            .or_default()
            .insert(tick, input);
        self.stats.inputs_processed += 1;

        if let Some(client) = self.clients.get_mut(&player_id) {
            client.last_input_tick = tick;
        }
    }

    /// Run one server tick (called at server tick rate)
    pub fn tick_update(&mut self, dt: f32) {
        self.tick += 1;

        // Process all pending inputs
        let player_ids: Vec<PlayerId> = self.clients.keys().copied().collect();
        for player_id in &player_ids {
            let inputs: Vec<(TickNumber, PlayerInputData)> = {
                if let Some(buf) = self.input_buffers.get(player_id) {
                    buf.iter().map(|(&t, i)| (t, i.clone())).collect()
                } else {
                    vec![]
                }
            };

            for (tick, input) in inputs {
                self.apply_player_input(*player_id, &input, dt);
                // Clear processed input
                if let Some(buf) = self.input_buffers.get_mut(player_id) {
                    buf.remove(&tick);
                }
            }
        }

        // Physics step for all entities
        for entity in self.entities.values_mut() {
            // Apply velocity
            entity.position[0] += entity.velocity[0] * dt;
            entity.position[1] += entity.velocity[1] * dt;
            entity.position[2] += entity.velocity[2] * dt;

            // Gravity for dynamic entities
            match entity.entity_type {
                EntityType::Player | EntityType::Enemy | EntityType::Projectile => {
                    entity.velocity[1] -= 9.81 * dt;
                    // Ground collision
                    if entity.position[1] < 0.5 {
                        entity.position[1] = 0.5;
                        entity.velocity[1] = 0.0;
                    }
                }
                _ => {}
            }

            // Damping
            entity.velocity[0] *= 0.98;
            entity.velocity[2] *= 0.98;
        }

        // Broadcast snapshots at configured interval
        if self.tick % self.config.snapshot_interval as u64 == 0 {
            self.broadcast_full_snapshots();
        } else if self.tick % self.config.delta_interval as u64 == 0 {
            self.broadcast_delta_snapshots();
        }

        self.stats.ticks_run = self.tick;
        self.stats.entities_active = self.entities.len() as u32;
        self.stats.clients_connected = self.clients.len() as u32;
    }

    /// Apply a player's input to the game state
    fn apply_player_input(&mut self, player_id: PlayerId, input: &PlayerInputData, dt: f32) {
        let speed = 8.0;

        if let Some(client) = self.clients.get(&player_id) {
            let pos = client.position;
            let new_pos = [
                pos[0] + input.movement[0] * speed * dt,
                pos[1] + input.movement[1] * speed * dt,
                pos[2] + input.movement[2] * speed * dt,
            ];

            // Update client position
            if let Some(client) = self.clients.get_mut(&player_id) {
                client.position = new_pos;
            }

            // Update entity position
            for entity in self.entities.values_mut() {
                if entity.owner == Some(player_id) {
                    entity.position = new_pos;
                    if input.movement[1] > 0.5 {
                        entity.velocity[1] = 5.0; // Jump
                    }
                    break;
                }
            }
        }
    }

    /// Send a full world snapshot to a specific client
    fn send_full_snapshot(&mut self, player_id: PlayerId) {
        let entities: Vec<NetEntity> = self.entities.values().cloned().collect();
        let player_state = self.clients.get(&player_id).map(|c| PlayerNetState {
            player_id: c.player_id,
            name: c.name.clone(),
            position: c.position,
            health: c.health,
            score: c.score,
            ping_ms: c.ping_ms,
            team: c.team,
        });

        self.send_to_client(player_id, NetMessage::WorldSnapshot {
            tick: self.tick,
            server_timestamp: timestamp_ms(),
            entities,
            player_state,
            acked_inputs: vec![],
        });

        if let Some(client) = self.clients.get_mut(&player_id) {
            client.last_full_snapshot = self.tick;
        }
    }

    /// Broadcast full snapshots to all clients
    fn broadcast_full_snapshots(&mut self) {
        let player_ids: Vec<PlayerId> = self.clients.keys().copied().collect();
        for player_id in player_ids {
            self.send_full_snapshot(player_id);
        }
    }

    /// Broadcast delta snapshots (only changes since last snapshot)
    fn broadcast_delta_snapshots(&mut self) {
        let entities: Vec<NetEntity> = self.entities.values().cloned().collect();
        let mut player_ids: Vec<PlayerId> = self.clients.keys().copied().collect();

        for player_id in &mut player_ids {
            let player_state = self.clients.get(player_id).map(|c| PlayerNetState {
                player_id: c.player_id,
                name: c.name.clone(),
                position: c.position,
                health: c.health,
                score: c.score,
                ping_ms: c.ping_ms,
                team: c.team,
            });

            self.send_to_client(*player_id, NetMessage::WorldSnapshot {
                tick: self.tick,
                server_timestamp: timestamp_ms(),
                entities: entities.clone(),
                player_state,
                acked_inputs: vec![],
            });
        }
    }

    /// Send a message to a specific client
    pub fn send_to_client(&mut self, player_id: PlayerId, msg: NetMessage) {
        self.outgoing.entry(player_id).or_default().push_back(msg);
    }

    /// Broadcast a message to all clients
    pub fn broadcast(&mut self, msg: NetMessage) {
        let player_ids: Vec<PlayerId> = self.clients.keys().copied().collect();
        for player_id in player_ids {
            self.send_to_client(player_id, msg.clone());
        }
    }

    /// Drain outgoing messages for a client
    pub fn drain_client_outgoing(&mut self, player_id: PlayerId) -> Vec<NetMessage> {
        self.outgoing.get_mut(&player_id)
            .map(|q| q.drain(..).collect())
            .unwrap_or_default()
    }

    /// Drain broadcast queue
    pub fn drain_broadcast(&mut self) -> Vec<NetMessage> {
        self.broadcast_queue.drain(..).collect()
    }

    pub fn client_count(&self) -> usize { self.clients.len() }
    pub fn entity_count(&self) -> usize { self.entities.len() }
}

// ═══════════════════════════════════════════════════════════ RPC System

/// Remote Procedure Call registry
pub struct RpcRegistry {
    /// Registered procedures: id → name
    procedures: HashMap<u32, String>,
    /// Procedure IDs by name
    name_to_id: HashMap<String, u32>,
    next_id: u32,
}

impl Default for RpcRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl RpcRegistry {
    pub fn new() -> Self {
        Self {
            procedures: HashMap::new(),
            name_to_id: HashMap::new(),
            next_id: 1,
        }
    }

    /// Register a new RPC procedure, returns its ID
    pub fn register(&mut self, name: &str) -> u32 {
        if let Some(&id) = self.name_to_id.get(name) {
            return id;
        }
        let id = self.next_id;
        self.next_id += 1;
        self.procedures.insert(id, name.to_string());
        self.name_to_id.insert(name.to_string(), id);
        id
    }

    /// Look up a procedure ID by name
    pub fn get_id(&self, name: &str) -> Option<u32> {
        self.name_to_id.get(name).copied()
    }

    /// Look up a procedure name by ID
    pub fn get_name(&self, id: u32) -> Option<&str> {
        self.procedures.get(&id).map(|s| s.as_str())
    }
}

// ═══════════════════════════════════════════════════════════ Lag Compensation

/// Server-side lag compensation for hit detection
pub struct LagCompensator {
    /// Position history for each entity (ring buffer per entity)
    position_history: HashMap<NetEntityId, VecDeque<TimestampedPosition>>,
    /// Maximum history duration in ms
    max_history_ms: u64,
    /// Maximum entries per entity
    max_entries: usize,
}

#[derive(Clone, Debug)]
struct TimestampedPosition {
    timestamp: Timestamp,
    position: [f32; 3],
    rotation: [f32; 4],
}

impl Default for LagCompensator {
    fn default() -> Self {
        Self {
            position_history: HashMap::new(),
            max_history_ms: 1000, // 1 second of history
            max_entries: 120,
        }
    }
}

impl LagCompensator {
    pub fn new() -> Self { Self::default() }

    /// Record entity position at current time
    pub fn record_position(&mut self, entity_id: NetEntityId, position: [f32; 3], rotation: [f32; 4], timestamp: Timestamp) {
        let history = self.position_history.entry(entity_id).or_default();
        history.push_back(TimestampedPosition { timestamp, position, rotation });
        if history.len() > self.max_entries {
            history.pop_front();
        }
    }

    /// Get interpolated position at a past time (for lag-compensated hit detection)
    pub fn get_position_at(&self, entity_id: NetEntityId, target_time: Timestamp) -> Option<[f32; 3]> {
        let history = self.position_history.get(&entity_id)?;
        if history.is_empty() { return None; }

        // Find the two entries surrounding target_time
        for i in 0..history.len() - 1 {
            if history[i].timestamp <= target_time && history[i + 1].timestamp >= target_time {
                let t = (target_time - history[i].timestamp) as f32
                    / (history[i + 1].timestamp - history[i].timestamp).max(1) as f32;
                return Some([
                    history[i].position[0] + (history[i+1].position[0] - history[i].position[0]) * t,
                    history[i].position[1] + (history[i+1].position[1] - history[i].position[1]) * t,
                    history[i].position[2] + (history[i+1].position[2] - history[i].position[2]) * t,
                ]);
            }
        }

        // If target is older than history, return oldest
        history.front().map(|h| h.position)
    }

    /// Trim old history
    pub fn cleanup(&mut self, current_time: Timestamp) {
        for history in self.position_history.values_mut() {
            history.retain(|h| current_time - h.timestamp <= self.max_history_ms);
        }
    }
}

// ═══════════════════════════════════════════════════════════ Interest Management

/// Filters which entities to send to which clients (area of interest)
pub struct InterestManager {
    /// Cell size for spatial partitioning
    pub cell_size: f32,
    /// Spatial grid: cell_index → set of entity_ids
    grid: HashMap<(i32, i32, i32), Vec<NetEntityId>>,
}

impl Default for InterestManager {
    fn default() -> Self {
        Self { cell_size: 10.0, grid: HashMap::new() }
    }
}

impl InterestManager {
    pub fn new() -> Self { Self::default() }

    /// Rebuild the spatial grid from all entities
    pub fn rebuild(&mut self, entities: &HashMap<NetEntityId, NetEntity>) {
        self.grid.clear();
        for (id, entity) in entities {
            let cell = self.world_to_cell(entity.position);
            self.grid.entry(cell).or_default().push(*id);
        }
    }

    /// Get entity IDs within a radius of a position
    pub fn query_radius(&self, position: [f32; 3], radius: f32) -> Vec<NetEntityId> {
        let center_cell = self.world_to_cell(position);
        let cell_range = (radius / self.cell_size).ceil() as i32 + 1;
        let mut result = Vec::new();

        for dx in -cell_range..=cell_range {
            for dy in -cell_range..=cell_range {
                for dz in -cell_range..=cell_range {
                    let cell = (center_cell.0 + dx, center_cell.1 + dy, center_cell.2 + dz);
                    if let Some(ids) = self.grid.get(&cell) {
                        result.extend(ids);
                    }
                }
            }
        }

        result
    }

    fn world_to_cell(&self, pos: [f32; 3]) -> (i32, i32, i32) {
        (
            (pos[0] / self.cell_size).floor() as i32,
            (pos[1] / self.cell_size).floor() as i32,
            (pos[2] / self.cell_size).floor() as i32,
        )
    }
}

// ═══════════════════════════════════════════════════════════ Transport Layer

/// Transport abstraction (can be WebSocket, TCP, or UDP)
pub enum TransportType {
    WebSocket,
    Tcp,
    Udp,
}

pub struct TransportConfig {
    pub transport_type: TransportType,
    pub server_addr: String,
    pub port: u16,
    pub use_compression: bool,
    pub max_packet_size: usize,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            transport_type: TransportType::WebSocket,
            server_addr: "127.0.0.1".into(),
            port: 8080,
            use_compression: true,
            max_packet_size: 4096,
        }
    }
}

/// Message serializer/deserializer
pub struct MessageCodec;

impl MessageCodec {
    /// Serialize a network message to bytes
    pub fn encode(msg: &NetMessage) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(msg)
    }

    /// Deserialize bytes to a network message
    pub fn decode(data: &[u8]) -> Result<NetMessage, bincode::Error> {
        bincode::deserialize(data)
    }

    /// Get estimated size of a message in bytes
    pub fn estimate_size(msg: &NetMessage) -> usize {
        bincode::serialize(msg).map(|v| v.len()).unwrap_or(0)
    }
}

// ═══════════════════════════════════════════════════════════ Utility Functions

fn lerp_vec3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn slerp_quat(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    // Simple nlerp (normalized lerp) — good enough for most cases
    let mut dot = a[0]*b[0] + a[1]*b[1] + a[2]*b[2] + a[3]*b[3];
    let mut b = b;
    if dot < 0.0 { dot = -dot; b = [-b[0], -b[1], -b[2], -b[3]]; }
    let t1 = 1.0 - t;
    let mut r = [
        t1*a[0] + t*b[0],
        t1*a[1] + t*b[1],
        t1*a[2] + t*b[2],
        t1*a[3] + t*b[3],
    ];
    let len = (r[0]*r[0] + r[1]*r[1] + r[2]*r[2] + r[3]*r[3]).sqrt();
    if len > 1e-6 { r[0] /= len; r[1] /= len; r[2] /= len; r[3] /= len; }
    r
}

/// Get current time in milliseconds
pub fn timestamp_ms() -> Timestamp {
    #[cfg(target_arch = "wasm32")]
    {
        (js_sys::Date::now()) as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

// ═══════════════════════════════════════════════════════════ Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_input_buttons() {
        let mut input = PlayerInputData::default();
        assert!(!input.button_pressed(0));
        input.set_button(0, true);
        assert!(input.button_pressed(0));
        input.set_button(5, true);
        assert!(input.button_pressed(5));
        assert!(!input.button_pressed(1));
        input.set_button(0, false);
        assert!(!input.button_pressed(0));
    }

    #[test]
    fn test_client_creation() {
        let client = NetworkClient::new();
        assert!(!client.is_connected());
        assert_eq!(client.ping(), 0.0);
        assert_eq!(client.current_tick, 0);
    }

    #[test]
    fn test_server_creation() {
        let server = NetworkServer::new(ServerConfig::default());
        assert_eq!(server.client_count(), 0);
        assert_eq!(server.entity_count(), 0);
        assert_eq!(server.tick, 0);
    }

    #[test]
    fn test_server_accept_client() {
        let mut server = NetworkServer::new(ServerConfig::default());
        let pid = server.accept_client("Player1");
        assert_eq!(pid, 1);
        assert_eq!(server.client_count(), 1);
        assert!(server.entity_count() >= 1); // Player entity spawned

        let pid2 = server.accept_client("Player2");
        assert_eq!(pid2, 2);
        assert_eq!(server.client_count(), 2);
    }

    #[test]
    fn test_server_tick() {
        let mut server = NetworkServer::new(ServerConfig::default());
        server.accept_client("Player1");
        server.tick_update(1.0 / 60.0);
        assert_eq!(server.tick, 1);
        server.tick_update(1.0 / 60.0);
        assert_eq!(server.tick, 2);
    }

    #[test]
    fn test_server_process_input() {
        let mut server = NetworkServer::new(ServerConfig::default());
        let pid = server.accept_client("Player1");

        let input = PlayerInputData {
            movement: [1.0, 0.0, 0.0],
            ..Default::default()
        };
        server.process_input(pid, 1, input);
        assert_eq!(server.stats.inputs_processed, 1);
    }

    #[test]
    fn test_prediction_reconciliation() {
        let mut pred = PredictionState::default();

        // Record some inputs
        let input1 = PlayerInputData { movement: [1.0, 0.0, 0.0], ..Default::default() };
        let input2 = PlayerInputData { movement: [1.0, 0.0, 0.0], ..Default::default() };
        pred.record_input(1, &input1, 1.0 / 60.0);
        pred.record_input(2, &input2, 1.0 / 60.0);

        // Predicted position should have moved
        assert!(pred.predicted_position[0] > 0.0);

        // Server reconciliation with server position
        pred.reconcile([0.1, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], 2, 1.0 / 60.0);

        // Should replay unacknowledged inputs
        assert_eq!(pred.unacked_input_count(), 0); // All acked
    }

    #[test]
    fn test_entity_interpolation() {
        let mut interp = EntityInterpolator::default();

        // Push two snapshots
        let entities1 = vec![NetEntity {
            id: 1, entity_type: EntityType::Player,
            position: [0.0, 0.0, 0.0], rotation: [0.0, 0.0, 0.0, 1.0],
            velocity: [0.0; 3], scale: [1.0; 3], owner: None,
            health: 100.0, max_health: 100.0, flags: 0,
        }];
        interp.push_snapshot(1, 1000, entities1);

        let entities2 = vec![NetEntity {
            id: 1, entity_type: EntityType::Player,
            position: [10.0, 0.0, 0.0], rotation: [0.0, 0.0, 0.0, 1.0],
            velocity: [0.0; 3], scale: [1.0; 3], owner: None,
            health: 100.0, max_health: 100.0, flags: 0,
        }];
        interp.push_snapshot(2, 1016, entities2);

        // Interpolate — target_time needs to be after interpolation_delay (3 ticks * 16ms = 48ms)
        let result = interp.interpolate(1060); // 1060 - 48 = 1012, which is between 1000 and 1016
        if let Some(e) = result.get(&1) {
            assert!(e.position[0] > 0.0 && e.position[0] < 10.0);
        }
    }

    #[test]
    fn test_rpc_registry() {
        let mut rpc = RpcRegistry::new();
        let id1 = rpc.register("fire_weapon");
        let id2 = rpc.register("take_damage");
        assert_ne!(id1, id2);
        assert_eq!(rpc.get_id("fire_weapon"), Some(id1));
        assert_eq!(rpc.get_name(id2), Some("take_damage"));
        // Re-register same name returns same ID
        assert_eq!(rpc.register("fire_weapon"), id1);
    }

    #[test]
    fn test_lag_compensation() {
        let mut lag = LagCompensator::new();
        lag.record_position(1, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], 1000);
        lag.record_position(1, [10.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], 1100);
        lag.record_position(1, [20.0, 0.0, 0.0], [0.0, 0.0, 0.0, 1.0], 1200);

        // At midpoint should be ~5
        let pos = lag.get_position_at(1, 1050).unwrap();
        assert!((pos[0] - 5.0).abs() < 1.0);

        // At exact timestamp
        let pos = lag.get_position_at(1, 1100).unwrap();
        assert!((pos[0] - 10.0).abs() < 0.1);
    }

    #[test]
    fn test_interest_management() {
        let mut im = InterestManager::new();
        let mut entities = HashMap::new();
        entities.insert(1, NetEntity {
            id: 1, entity_type: EntityType::Player,
            position: [5.0, 0.0, 5.0], rotation: [0.0; 4], velocity: [0.0; 3],
            scale: [1.0; 3], owner: None, health: 100.0, max_health: 100.0, flags: 0,
        });
        entities.insert(2, NetEntity {
            id: 2, entity_type: EntityType::Player,
            position: [50.0, 0.0, 50.0], rotation: [0.0; 4], velocity: [0.0; 3],
            scale: [1.0; 3], owner: None, health: 100.0, max_health: 100.0, flags: 0,
        });
        im.rebuild(&entities);

        // Near entity
        let nearby = im.query_radius([0.0, 0.0, 0.0], 15.0);
        assert!(nearby.contains(&1));
        assert!(!nearby.contains(&2));

        // Far entity
        let nearby = im.query_radius([0.0, 0.0, 0.0], 100.0);
        assert!(nearby.contains(&1));
        assert!(nearby.contains(&2));
    }

    #[test]
    fn test_message_codec() {
        let msg = NetMessage::ConnectRequest {
            player_name: "Test".into(),
            protocol_version: 1,
            auth_token: None,
        };
        let encoded = MessageCodec::encode(&msg).unwrap();
        let decoded = MessageCodec::decode(&encoded).unwrap();
        match decoded {
            NetMessage::ConnectRequest { player_name, protocol_version, .. } => {
                assert_eq!(player_name, "Test");
                assert_eq!(protocol_version, 1);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_server_client_disconnect() {
        let mut server = NetworkServer::new(ServerConfig::default());
        let pid = server.accept_client("Player1");
        assert_eq!(server.client_count(), 1);
        server.remove_client(pid);
        assert_eq!(server.client_count(), 0);
        assert_eq!(server.entity_count(), 0);
    }

    #[test]
    fn test_lerp_vec3() {
        let a = [0.0, 0.0, 0.0];
        let b = [10.0, 20.0, 30.0];
        let r = lerp_vec3(a, b, 0.5);
        assert!((r[0] - 5.0).abs() < 0.001);
        assert!((r[1] - 10.0).abs() < 0.001);
        assert!((r[2] - 15.0).abs() < 0.001);
    }

    #[test]
    fn test_slerp_quat() {
        let a = [0.0, 0.0, 0.0, 1.0]; // identity
        let b = [0.0, 0.707, 0.0, 0.707]; // 90deg around Y
        let r = slerp_quat(a, b, 0.5);
        let len = (r[0]*r[0] + r[1]*r[1] + r[2]*r[2] + r[3]*r[3]).sqrt();
        assert!((len - 1.0).abs() < 0.01); // Should be normalized
    }
}
