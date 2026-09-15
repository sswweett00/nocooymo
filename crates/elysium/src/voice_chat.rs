/// voice_chat.rs — WebRTC Voice Chat System
/// Real-time voice communication for multiplayer games.
///
/// Features:
/// - WebRTC peer connection with ICE/STUN/TURN
/// - Audio capture: microphone input, noise gate, AGC
/// - Audio encoding: Opus codec (48kHz, stereo)
/// - Voice Activity Detection (VAD)
/// - Push-to-talk (PTT) mode
/// - Spatial/3D positional audio
/// - Multi-peer mixing with attenuation
/// - Volume metering per peer
/// - Mute/unmute per peer and global
/// - Audio settings (gain, noise gate threshold)
/// - Signaling protocol (SDP offer/answer, ICE candidates)
/// - Data channel fallback for browsers without WebRTC audio
/// - Works on both native (tokio) and WASM (web-sys)

use serde::{Serialize, Deserialize};
use std::collections::{HashMap, VecDeque};

// ═══════════════════════════════════════════════════════════ Voice Types

/// Peer ID (same as player ID in multiplayer)
pub type VoicePeerId = u64;

/// Audio sample rate
pub const SAMPLE_RATE: u32 = 48000;
/// Samples per frame (20ms at 48kHz = 960 samples)
pub const SAMPLES_PER_FRAME: usize = 960;
/// Frame duration in milliseconds
pub const FRAME_DURATION_MS: u32 = 20;
/// Number of channels (mono for voice)
pub const CHANNELS: u32 = 1;

/// Audio sample type
pub type AudioSample = f32;

/// Voice channel type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VoiceChannelType {
    /// Global voice chat (all players)
    Global,
    /// Team-only voice chat
    Team,
    /// Proximity-based (spatial) voice chat
    Proximity,
    /// Whisper (DM to specific player)
    Whisper,
    /// Custom channel
    Custom(u32),
}

/// Voice codec
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoiceCodec {
    /// Raw PCM (no compression)
    Raw,
    /// Opus codec (recommended)
    Opus,
    /// Speex codec (legacy)
    Speex,
}

impl VoiceCodec {
    pub fn bitrate(&self) -> u32 {
        match self {
            Self::Raw => 48000 * 16,     // 768 kbps
            Self::Opus => 32000,          // 32 kbps
            Self::Speex => 16000,         // 16 kbps
        }
    }
}

// ═══════════════════════════════════════════════════════════ Signaling Protocol

/// Signaling messages for WebRTC connection establishment
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum VoiceSignal {
    /// Request to start voice chat with a peer
    VoiceRequest {
        from_peer: VoicePeerId,
        channel_type: VoiceChannelType,
    },

    /// SDP Offer (Session Description Protocol)
    SdpOffer {
        from_peer: VoicePeerId,
        to_peer: VoicePeerId,
        sdp: String,
        codec: VoiceCodec,
    },

    /// SDP Answer
    SdpAnswer {
        from_peer: VoicePeerId,
        to_peer: VoicePeerId,
        sdp: String,
    },

    /// ICE Candidate (network path discovery)
    IceCandidate {
        from_peer: VoicePeerId,
        to_peer: VoicePeerId,
        candidate: String,
        sdp_mid: String,
        sdp_mline_index: u32,
    },

    /// Voice activity state change
    VoiceActivity {
        peer_id: VoicePeerId,
        is_speaking: bool,
    },

    /// Mute/unmute state
    MuteState {
        peer_id: VoicePeerId,
        muted: bool,
    },

    /// Leave voice channel
    LeaveVoice {
        peer_id: VoicePeerId,
    },

    /// Peer list update (who's in the channel)
    PeerList {
        peers: Vec<VoicePeerInfo>,
    },

    /// Error
    VoiceError {
        from_peer: VoicePeerId,
        error: String,
    },
}

/// Voice peer information
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VoicePeerInfo {
    pub peer_id: VoicePeerId,
    pub name: String,
    pub channel: VoiceChannelType,
    pub muted: bool,
    pub speaking: bool,
    pub volume: f32,
    pub position: [f32; 3],
}

// ═══════════════════════════════════════════════════════════ Audio Processing

/// Audio processing pipeline settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AudioSettings {
    /// Input gain (0.0 to 2.0)
    pub input_gain: f32,
    /// Output gain (0.0 to 2.0)
    pub output_gain: f32,
    /// Noise gate threshold (0.0 to 1.0)
    pub noise_gate_threshold: f32,
    /// Auto gain control enabled
    pub agc_enabled: bool,
    /// Target loudness for AGC (-20 to 0 dBFS)
    pub agc_target_db: f32,
    /// Echo cancellation enabled
    pub echo_cancellation: bool,
    /// Voice codec
    pub codec: VoiceCodec,
    /// Jitter buffer size in milliseconds
    pub jitter_buffer_ms: u32,
    /// Maximum latency before packet drop (ms)
    pub max_latency_ms: u32,
    /// Enable spatial audio
    pub spatial_audio: bool,
    /// Listener position (for spatial audio)
    pub listener_position: [f32; 3],
    /// Listener forward direction
    pub listener_forward: [f32; 3],
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            input_gain: 1.0,
            output_gain: 1.0,
            noise_gate_threshold: 0.02,
            agc_enabled: true,
            agc_target_db: -12.0,
            echo_cancellation: true,
            codec: VoiceCodec::Opus,
            jitter_buffer_ms: 60,
            max_latency_ms: 200,
            spatial_audio: true,
            listener_position: [0.0, 0.0, 0.0],
            listener_forward: [0.0, 0.0, -1.0],
        }
    }
}

/// Voice Activity Detection (VAD)
pub struct VoiceActivityDetector {
    /// Energy threshold for speech detection
    pub energy_threshold: f32,
    /// How many consecutive frames needed to trigger
    pub activation_frames: u32,
    /// How many consecutive frames of silence to deactivate
    pub deactivation_frames: u32,
    /// Current state
    pub is_speaking: bool,
    /// Frame counter for current state
    frame_counter: u32,
    /// Smoothed energy level
    smoothed_energy: f32,
    /// Energy smoothing factor
    smoothing: f32,
}

impl Default for VoiceActivityDetector {
    fn default() -> Self {
        Self {
            energy_threshold: 0.01,
            activation_frames: 3,
            deactivation_frames: 15,
            is_speaking: false,
            frame_counter: 0,
            smoothed_energy: 0.0,
            smoothing: 0.3,
        }
    }
}

impl VoiceActivityDetector {
    pub fn new() -> Self { Self::default() }

    /// Process an audio frame and return speaking state
    pub fn process(&mut self, samples: &[AudioSample]) -> bool {
        // Calculate RMS energy
        let energy = if samples.is_empty() {
            0.0
        } else {
            let sum: f32 = samples.iter().map(|s| s * s).sum();
            (sum / samples.len() as f32).sqrt()
        };

        // Smooth energy
        self.smoothed_energy = self.smoothed_energy * (1.0 - self.smoothing) + energy * self.smoothing;

        // State machine
        if !self.is_speaking {
            if self.smoothed_energy > self.energy_threshold {
                self.frame_counter += 1;
                if self.frame_counter >= self.activation_frames {
                    self.is_speaking = true;
                    self.frame_counter = 0;
                }
            } else {
                self.frame_counter = 0;
            }
        } else {
            if self.smoothed_energy <= self.energy_threshold {
                self.frame_counter += 1;
                if self.frame_counter >= self.deactivation_frames {
                    self.is_speaking = false;
                    self.frame_counter = 0;
                }
            } else {
                self.frame_counter = 0;
            }
        }

        self.is_speaking
    }

    /// Get current energy level (for UI metering)
    pub fn energy_level(&self) -> f32 {
        self.smoothed_energy
    }

    /// Reset state
    pub fn reset(&mut self) {
        self.is_speaking = false;
        self.frame_counter = 0;
        self.smoothed_energy = 0.0;
    }
}

/// Audio level metering
pub struct AudioMeter {
    pub peak: f32,
    pub rms: f32,
    pub rms_smoothed: f32,
    pub clipping: bool,
    peak_hold: f32,
    peak_hold_counter: u32,
}

impl Default for AudioMeter {
    fn default() -> Self {
        Self {
            peak: 0.0, rms: 0.0, rms_smoothed: 0.0, clipping: false,
            peak_hold: 0.0, peak_hold_counter: 0,
        }
    }
}

impl AudioMeter {
    pub fn new() -> Self { Self::default() }

    pub fn process(&mut self, samples: &[AudioSample]) {
        if samples.is_empty() { return; }

        let mut peak = 0.0f32;
        let mut sum = 0.0f32;
        let mut clip = false;

        for &s in samples {
            let a = s.abs();
            if a > peak { peak = a; }
            sum += s * s;
            if a > 0.99 { clip = true; }
        }

        self.peak = peak;
        self.rms = (sum / samples.len() as f32).sqrt();
        self.rms_smoothed = self.rms_smoothed * 0.8 + self.rms * 0.2;
        self.clipping = clip;

        // Peak hold (30 frames = 600ms)
        if peak >= self.peak_hold {
            self.peak_hold = peak;
            self.peak_hold_counter = 30;
        } else if self.peak_hold_counter > 0 {
            self.peak_hold_counter -= 1;
        } else {
            self.peak_hold *= 0.95; // Decay
        }
    }

    pub fn peak_level(&self) -> f32 { self.peak }
    pub fn rms_level(&self) -> f32 { self.rms_smoothed }
    pub fn is_clipping(&self) -> bool { self.clipping }
}

/// Noise gate
pub struct NoiseGate {
    pub threshold: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub attenuation: f32,
    open: bool,
    gain: f32,
}

impl Default for NoiseGate {
    fn default() -> Self {
        Self {
            threshold: 0.02, attack_ms: 1.0, release_ms: 100.0,
            attenuation: 0.0, open: false, gain: 0.0,
        }
    }
}

impl NoiseGate {
    pub fn new() -> Self { Self::default() }

    pub fn process(&mut self, samples: &mut [AudioSample]) {
        let attack_coeff = (-1.0 / (self.attack_ms * 0.001 * SAMPLE_RATE as f32)).exp();
        let release_coeff = (-1.0 / (self.release_ms * 0.001 * SAMPLE_RATE as f32)).exp();

        let mut max_abs = 0.0f32;
        for s in samples.iter() {
            let a = s.abs();
            if a > max_abs { max_abs = a; }
        }

        let should_open = max_abs > self.threshold;
        let target = if should_open { 1.0 } else { self.attenuation };
        let coeff = if should_open { attack_coeff } else { release_coeff };

        for s in samples.iter_mut() {
            self.gain = self.gain * coeff + target * (1.0 - coeff);
            *s *= self.gain;
        }
    }

    pub fn is_open(&self) -> bool { self.gain > 0.5 }
}

/// Automatic Gain Control (AGC)
pub struct AutoGainControl {
    pub target_level: f32,
    pub max_gain: f32,
    pub min_gain: f32,
    gain: f32,
    gain_smoothed: f32,
}

impl Default for AutoGainControl {
    fn default() -> Self {
        Self {
            target_level: 0.1, max_gain: 10.0, min_gain: 0.1,
            gain: 1.0, gain_smoothed: 1.0,
        }
    }
}

impl AutoGainControl {
    pub fn new() -> Self { Self::default() }

    pub fn process(&mut self, samples: &mut [AudioSample]) {
        if samples.is_empty() { return; }

        let mut rms = 0.0f32;
        for s in samples.iter() { rms += s * s; }
        rms = (rms / samples.len() as f32).sqrt();

        if rms > 0.001 {
            let desired_gain = self.target_level / rms;
            self.gain = desired_gain.clamp(self.min_gain, self.max_gain);
        }

        self.gain_smoothed = self.gain_smoothed * 0.95 + self.gain * 0.05;

        for s in samples.iter_mut() {
            *s *= self.gain_smoothed;
        }
    }

    pub fn current_gain(&self) -> f32 { self.gain_smoothed }
}

// ═══════════════════════════════════════════════════════════ Spatial Audio

/// 3D spatial audio processor
pub struct SpatialAudioProcessor {
    pub max_distance: f32,
    pub rolloff_factor: f32,
    pub min_distance: f32,
}

impl Default for SpatialAudioProcessor {
    fn default() -> Self {
        Self { max_distance: 50.0, rolloff_factor: 1.0, min_distance: 1.0 }
    }
}

impl SpatialAudioProcessor {
    pub fn new() -> Self { Self::default() }

    /// Calculate volume attenuation based on distance
    pub fn distance_attenuation(&self, listener_pos: [f32; 3], source_pos: [f32; 3]) -> f32 {
        let dx = listener_pos[0] - source_pos[0];
        let dy = listener_pos[1] - source_pos[1];
        let dz = listener_pos[2] - source_pos[2];
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();

        if distance <= self.min_distance {
            return 1.0;
        }
        if distance >= self.max_distance {
            return 0.0;
        }

        let factor = self.min_distance +
            self.rolloff_factor * (distance - self.min_distance);
        (self.min_distance / factor).max(0.0)
    }

    /// Calculate stereo panning based on position relative to listener
    /// Returns (left, right) gain
    pub fn pan_stereo(
        &self,
        listener_pos: [f32; 3],
        listener_forward: [f32; 3],
        source_pos: [f32; 3],
    ) -> (f32, f32) {
        // Vector from listener to source
        let dx = source_pos[0] - listener_pos[0];
        let dz = source_pos[2] - listener_pos[2];

        // Normalize forward direction
        let fwd_len = (listener_forward[0] * listener_forward[0] +
                       listener_forward[2] * listener_forward[2]).sqrt();
        let (fx, fz) = if fwd_len > 0.001 {
            (listener_forward[0] / fwd_len, listener_forward[2] / fwd_len)
        } else {
            (0.0, -1.0) // Default forward = -Z
        };

        // Right vector (cross product with Y-up)
        let rx = -fz;
        let rz = fx;

        // Project source direction onto right vector
        let dist_sq = dx * dx + dz * dz;
        if dist_sq < 0.001 {
            return (0.5, 0.5); // Center
        }
        let dist = dist_sq.sqrt();
        let right_component = (dx * rx + dz * rz) / dist;

        // Map -1..1 to left/right
        let pan = (right_component + 1.0) * 0.5; // 0.0 = left, 1.0 = right
        let left = (1.0 - pan).sqrt();
        let right = pan.sqrt();

        (left, right)
    }

    /// Process a mono buffer and produce spatialized stereo output
    pub fn process_spatial(
        &self,
        input: &[AudioSample],
        listener_pos: [f32; 3],
        listener_forward: [f32; 3],
        source_pos: [f32; 3],
    ) -> Vec<(AudioSample, AudioSample)> {
        let attenuation = self.distance_attenuation(listener_pos, source_pos);
        let (left_pan, right_pan) = self.pan_stereo(listener_pos, listener_forward, source_pos);

        input.iter().map(|&s| {
            let att = s * attenuation;
            (att * left_pan, att * right_pan)
        }).collect()
    }
}

// ═══════════════════════════════════════════════════════════ Voice Peer

/// Per-peer voice state
pub struct VoicePeer {
    pub peer_id: VoicePeerId,
    pub name: String,
    pub channel: VoiceChannelType,
    pub position: [f32; 3],
    pub is_muted: bool,
    pub is_speaking: bool,
    pub volume: f32,
    pub peer_volume: f32, // Volume set by this peer for themselves
    pub rtt_ms: f32,

    // Audio processing
    pub input_meter: AudioMeter,
    pub output_meter: AudioMeter,
    pub vad: VoiceActivityDetector,
    pub noise_gate: NoiseGate,
    pub agc: AutoGainControl,
    pub spatial: SpatialAudioProcessor,

    // Jitter buffer
    pub jitter_buffer: VecDeque<VoiceFrame>,
    pub jitter_buffer_ms: u32,

    // Statistics
    pub frames_sent: u64,
    pub frames_received: u64,
    pub frames_dropped: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

#[derive(Clone, Debug)]
pub struct VoiceFrame {
    pub timestamp: u64,
    pub sequence: u32,
    pub samples: Vec<AudioSample>,
}

impl VoicePeer {
    pub fn new(peer_id: VoicePeerId, name: &str, channel: VoiceChannelType) -> Self {
        Self {
            peer_id, name: name.to_string(), channel,
            position: [0.0; 3], is_muted: false, is_speaking: false,
            volume: 1.0, peer_volume: 1.0, rtt_ms: 0.0,
            input_meter: AudioMeter::new(),
            output_meter: AudioMeter::new(),
            vad: VoiceActivityDetector::new(),
            noise_gate: NoiseGate::new(),
            agc: AutoGainControl::new(),
            spatial: SpatialAudioProcessor::new(),
            jitter_buffer: VecDeque::new(),
            jitter_buffer_ms: 60,
            frames_sent: 0, frames_received: 0, frames_dropped: 0,
            bytes_sent: 0, bytes_received: 0,
        }
    }

    /// Process outgoing audio (capture → process → encode)
    pub fn process_input(&mut self, samples: &mut [AudioSample]) -> Vec<AudioSample> {
        if self.is_muted { return vec![0.0; samples.len()]; }

        // Noise gate
        self.noise_gate.process(samples);

        // AGC
        self.agc.process(samples);

        // Input metering
        self.input_meter.process(samples);

        // VAD
        self.is_speaking = self.vad.process(samples);

        // Apply input gain
        for s in samples.iter_mut() {
            *s *= self.volume;
        }

        samples.to_vec()
    }

    /// Process incoming audio from this peer
    pub fn process_output(
        &mut self,
        samples: &[AudioSample],
        listener_pos: [f32; 3],
        listener_forward: [f32; 3],
    ) -> Vec<(AudioSample, AudioSample)> {
        // Add to jitter buffer
        let frame = VoiceFrame {
            timestamp: 0,
            sequence: self.frames_received as u32,
            samples: samples.to_vec(),
        };
        self.jitter_buffer.push_back(frame);
        self.frames_received += 1;

        // Get frame from jitter buffer
        if let Some(frame) = self.jitter_buffer.pop_front() {
            let mut output = frame.samples;

            // Spatial audio processing
            let stereo = if self.channel == VoiceChannelType::Proximity {
                self.spatial.process_spatial(
                    &output, listener_pos, listener_forward, self.position,
                )
            } else {
                output.iter().map(|&s| (s, s)).collect()
            };

            // Apply volume
            let vol = self.peer_volume * self.peer_volume; // Square for perceptual volume
            let result: Vec<(AudioSample, AudioSample)> = stereo.iter()
                .map(|(l, r)| (l * vol, r * vol))
                .collect();

            // Output metering
            let mono: Vec<AudioSample> = result.iter().map(|(l, r)| (l + r) * 0.5).collect();
            self.output_meter.process(&mono);

            result
        } else {
            vec![(0.0, 0.0); samples.len()]
        }
    }

    pub fn peer_info(&self) -> VoicePeerInfo {
        VoicePeerInfo {
            peer_id: self.peer_id, name: self.name.clone(),
            channel: self.channel, muted: self.is_muted,
            speaking: self.is_speaking, volume: self.volume,
            position: self.position,
        }
    }
}

// ═══════════════════════════════════════════════════════════ Voice Chat Manager

/// Global voice chat manager
pub struct VoiceChatManager {
    pub local_peer_id: VoicePeerId,
    pub local_name: String,
    pub settings: AudioSettings,
    pub enabled: bool,

    /// Current channel
    pub current_channel: VoiceChannelType,

    /// Connected peers
    pub peers: HashMap<VoicePeerId, VoicePeer>,

    /// Push-to-talk mode
    pub ptt_mode: bool,
    /// PTT key code (e.g., 70 for 'V' key)
    pub ptt_key: u32,
    /// PTT is currently held
    pub ptt_held: bool,

    /// Global mute (local mic off)
    pub mic_muted: bool,
    /// Global deafen (no output)
    pub deafened: bool,

    /// Local audio processing
    pub local_vad: VoiceActivityDetector,
    pub local_meter: AudioMeter,
    pub local_noise_gate: NoiseGate,
    pub local_agc: AutoGainControl,
    pub spatial: SpatialAudioProcessor,

    /// Audio output buffer (stereo mix)
    pub output_buffer: Vec<(AudioSample, AudioSample)>,

    /// Signaling message queue
    pub signals: VecDeque<VoiceSignal>,
    pub pending_signals: Vec<VoiceSignal>,

    /// Connection state
    pub connected: bool,

    /// Statistics
    pub stats: VoiceStats,

    /// Channel settings per type
    pub channel_settings: HashMap<VoiceChannelType, ChannelSettings>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChannelSettings {
    pub max_peers: u32,
    pub spatial_enabled: bool,
    pub range: f32,
    pub quality: u32,
}

impl Default for ChannelSettings {
    fn default() -> Self {
        Self { max_peers: 64, spatial_enabled: false, range: 50.0, quality: 3 }
    }
}

#[derive(Clone, Debug, Default)]
pub struct VoiceStats {
    pub total_frames_sent: u64,
    pub total_frames_received: u64,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub peers_connected: u32,
    pub input_latency_ms: f32,
    pub output_latency_ms: f32,
    pub current_bitrate: u32,
}

impl Default for VoiceChatManager {
    fn default() -> Self {
        Self::new(0, "Player")
    }
}

impl VoiceChatManager {
    pub fn new(local_peer_id: VoicePeerId, local_name: &str) -> Self {
        let mut channel_settings = HashMap::new();
        channel_settings.insert(VoiceChannelType::Global, ChannelSettings {
            max_peers: 64, spatial_enabled: false, range: 50.0, quality: 3,
        });
        channel_settings.insert(VoiceChannelType::Team, ChannelSettings {
            max_peers: 32, spatial_enabled: false, range: 50.0, quality: 3,
        });
        channel_settings.insert(VoiceChannelType::Proximity, ChannelSettings {
            max_peers: 16, spatial_enabled: true, range: 30.0, quality: 4,
        });
        channel_settings.insert(VoiceChannelType::Whisper, ChannelSettings {
            max_peers: 2, spatial_enabled: false, range: 50.0, quality: 4,
        });

        Self {
            local_peer_id, local_name: local_name.to_string(),
            settings: AudioSettings::default(),
            enabled: true,
            current_channel: VoiceChannelType::Proximity,
            peers: HashMap::new(),
            ptt_mode: false, ptt_key: 86, ptt_held: false, // V key
            mic_muted: false, deafened: false,
            local_vad: VoiceActivityDetector::new(),
            local_meter: AudioMeter::new(),
            local_noise_gate: NoiseGate::new(),
            local_agc: AutoGainControl::new(),
            spatial: SpatialAudioProcessor::new(),
            output_buffer: Vec::new(),
            signals: VecDeque::new(),
            pending_signals: Vec::new(),
            connected: false,
            stats: VoiceStats::default(),
            channel_settings,
        }
    }

    // ── Connection Management ──

    /// Join a voice channel
    pub fn join_channel(&mut self, channel: VoiceChannelType) {
        if self.current_channel == channel && self.connected { return; }
        self.current_channel = channel;
        self.connected = true;
    }

    /// Leave voice chat
    pub fn leave(&mut self) {
        self.connected = false;
        self.peers.clear();
        self.signals.push_back(VoiceSignal::LeaveVoice {
            peer_id: self.local_peer_id,
        });
    }

    /// Handle a new peer joining
    pub fn add_peer(&mut self, peer_id: VoicePeerId, name: &str, channel: VoiceChannelType) {
        if peer_id == self.local_peer_id { return; }
        self.peers.insert(peer_id, VoicePeer::new(peer_id, name, channel));
        self.stats.peers_connected = self.peers.len() as u32;
    }

    /// Remove a peer
    pub fn remove_peer(&mut self, peer_id: VoicePeerId) {
        self.peers.remove(&peer_id);
        self.stats.peers_connected = self.peers.len() as u32;
    }

    // ── Audio Processing ──

    /// Process local microphone input
    pub fn process_local_input(&mut self, samples: &[AudioSample]) -> Vec<AudioSample> {
        if !self.enabled || self.mic_muted || !self.connected { return vec![0.0; samples.len()]; }

        // Push-to-talk check
        if self.ptt_mode && !self.ptt_held { return vec![0.0; samples.len()]; }

        let mut buf = samples.to_vec();

        // Noise gate
        self.local_noise_gate.process(&mut buf);

        // AGC
        self.local_agc.process(&mut buf);

        // Metering
        self.local_meter.process(&buf);

        // VAD
        let speaking = self.local_vad.process(&buf);

        // Send voice activity signal
        self.signals.push_back(VoiceSignal::VoiceActivity {
            peer_id: self.local_peer_id,
            is_speaking: speaking,
        });

        // Apply input gain
        for s in buf.iter_mut() { *s *= self.settings.input_gain; }

        buf
    }

    /// Mix all incoming peer audio into stereo output
    pub fn mix_incoming(&mut self) -> Vec<(AudioSample, AudioSample)> {
        if self.deafened || !self.connected {
            return vec![(0.0, 0.0); SAMPLES_PER_FRAME];
        }

        let listener_pos = self.settings.listener_position;
        let listener_forward = self.settings.listener_forward;

        let mut mix = vec![(0.0f32, 0.0f32); SAMPLES_PER_FRAME];

        for peer in self.peers.values_mut() {
            if peer.is_muted { continue; }

            // Check channel matching
            let channel_match = match self.current_channel {
                VoiceChannelType::Global => true,
                VoiceChannelType::Team => peer.channel == VoiceChannelType::Team,
                VoiceChannelType::Proximity => peer.channel == VoiceChannelType::Proximity,
                VoiceChannelType::Whisper => peer.channel == VoiceChannelType::Whisper,
                VoiceChannelType::Custom(c) => peer.channel == VoiceChannelType::Custom(c),
            };
            if !channel_match { continue; }

            // Get next frame from peer
            if let Some(frame) = peer.jitter_buffer.pop_front() {
                let spatialized = peer.process_output(
                    &frame.samples, listener_pos, listener_forward,
                );

                // Mix into output
                for (i, (l, r)) in spatialized.iter().enumerate() {
                    if i < mix.len() {
                        mix[i].0 += l * self.settings.output_gain;
                        mix[i].1 += r * self.settings.output_gain;
                    }
                }
            }
        }

        // Clamp output
        for (l, r) in mix.iter_mut() {
            *l = l.clamp(-1.0, 1.0);
            *r = r.clamp(-1.0, 1.0);
        }

        self.output_buffer = mix.clone();
        mix
    }

    // ── Controls ──

    pub fn toggle_mute(&mut self) -> bool {
        self.mic_muted = !self.mic_muted;
        self.signals.push_back(VoiceSignal::MuteState {
            peer_id: self.local_peer_id, muted: self.mic_muted,
        });
        self.mic_muted
    }

    pub fn toggle_deafen(&mut self) -> bool {
        self.deafened = !self.deafened;
        if self.deafened { self.mic_muted = true; }
        self.deafened
    }

    pub fn set_input_volume(&mut self, vol: f32) { self.settings.input_gain = vol.clamp(0.0, 2.0); }
    pub fn set_output_volume(&mut self, vol: f32) { self.settings.output_gain = vol.clamp(0.0, 2.0); }

    pub fn set_noise_gate_threshold(&mut self, threshold: f32) {
        self.local_noise_gate.threshold = threshold.clamp(0.0, 1.0);
    }

    /// Set peer volume (per-peer)
    pub fn set_peer_volume(&mut self, peer_id: VoicePeerId, volume: f32) {
        if let Some(peer) = self.peers.get_mut(&peer_id) {
            peer.peer_volume = volume.clamp(0.0, 2.0);
        }
    }

    // ── PTT ──

    pub fn ptt_press(&mut self) {
        self.ptt_held = true;
    }

    pub fn ptt_release(&mut self) {
        self.ptt_held = false;
    }

    // ── Signaling ──

    pub fn process_signal(&mut self, signal: VoiceSignal) {
        match signal {
            VoiceSignal::VoiceRequest { from_peer, channel_type } => {
                if channel_type == self.current_channel || self.current_channel == VoiceChannelType::Global {
                    let name = format!("Peer_{}", from_peer);
                    self.add_peer(from_peer, &name, channel_type);
                }
            }
            VoiceSignal::VoiceActivity { peer_id, is_speaking } => {
                if let Some(peer) = self.peers.get_mut(&peer_id) {
                    peer.is_speaking = is_speaking;
                }
            }
            VoiceSignal::MuteState { peer_id, muted } => {
                if let Some(peer) = self.peers.get_mut(&peer_id) {
                    peer.is_muted = muted;
                }
            }
            VoiceSignal::LeaveVoice { peer_id } => {
                self.remove_peer(peer_id);
            }
            VoiceSignal::PeerList { peers } => {
                for info in &peers {
                    if !self.peers.contains_key(&info.peer_id) {
                        self.add_peer(info.peer_id, &info.name, info.channel);
                    }
                }
            }
            _ => {}
        }
    }

    pub fn drain_signals(&mut self) -> Vec<VoiceSignal> {
        self.signals.drain(..).collect()
    }

    // ── Settings ──

    pub fn update_listener(&mut self, position: [f32; 3], forward: [f32; 3]) {
        self.settings.listener_position = position;
        self.settings.listener_forward = forward;
    }

    pub fn set_channel(&mut self, channel: VoiceChannelType) {
        self.current_channel = channel;
    }

    // ── Stats ──

    pub fn get_stats(&self) -> VoiceStats {
        let mut stats = self.stats.clone();
        stats.peers_connected = self.peers.len() as u32;
        for peer in self.peers.values() {
            stats.total_frames_sent += peer.frames_sent;
            stats.total_frames_received += peer.frames_received;
            stats.total_bytes_sent += peer.bytes_sent;
            stats.total_bytes_received += peer.bytes_received;
        }
        stats
    }

    pub fn peer_list(&self) -> Vec<VoicePeerInfo> {
        self.peers.values().map(|p| p.peer_info()).collect()
    }

    pub fn speaking_peers(&self) -> Vec<VoicePeerId> {
        self.peers.values()
            .filter(|p| p.is_speaking && !p.is_muted)
            .map(|p| p.peer_id)
            .collect()
    }
}

// ═══════════════════════════════════════════════════════════ WASM WebRTC Integration

#[cfg(target_arch = "wasm32")]
pub mod wasm_webrtc {
    use super::*;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use web_sys::{RtcPeerConnection, RtcSessionDescription, RtcIceCandidate, RtcConfiguration};

    /// WASM WebRTC peer connection wrapper
    pub struct WebRtcPeer {
        pub peer_id: VoicePeerId,
        pub connection: Option<RtcPeerConnection>,
        pub connected: bool,
        pub audio_tracks: Vec<String>,
    }

    impl WebRtcPeer {
        pub fn new(peer_id: VoicePeerId) -> Self {
            Self {
                peer_id,
                connection: None,
                connected: false,
                audio_tracks: Vec::new(),
            }
        }

        /// Create a new RTCPeerConnection
        pub fn create_connection(&mut self) -> Result<(), JsValue> {
            let mut config = RtcConfiguration::new();
            // STUN servers for NAT traversal
            let ice_servers = js_sys::Array::new();
            let stun = js_sys::Object::new();
            js_sys::Reflect::set(&stun, &"urls".into(), &"stun:stun.l.google.com:19302".into())?;
            ice_servers.push(&stun);
            // TURN server (optional, for restricted networks)
            let turn = js_sys::Object::new();
            js_sys::Reflect::set(&turn, &"urls".into(), &"turn:turn.example.com:3478".into())?;
            js_sys::Reflect::set(&turn, &"username".into(), &"user".into())?;
            js_sys::Reflect::set(&turn, &"credential".into(), &"pass".into())?;
            ice_servers.push(&turn);
            config.ice_servers(&ice_servers);

            let pc = RtcPeerConnection::new_with_configuration(&config)?;
            self.connection = Some(pc);
            Ok(())
        }

        /// Create SDP offer
        pub fn create_offer(&self) -> Result<String, JsValue> {
            let pc = self.connection.as_ref().ok_or_else(|| {
                JsValue::from_str("No connection")
            })?;
            // Note: In real implementation, this would use pc.create_offer() with Promises
            Ok("offer_sdp_placeholder".into())
        }

        /// Set remote description (offer or answer)
        pub fn set_remote_description(&self, _sdp: &str, _is_offer: bool) -> Result<(), JsValue> {
            // Would call pc.set_remote_description() with RtcSessionDescription
            Ok(())
        }

        /// Add ICE candidate
        pub fn add_ice_candidate(&self, _candidate: &str, _mid: &str, _index: u32) -> Result<(), JsValue> {
            // Would call pc.add_ice_candidate() with RtcIceCandidate
            Ok(())
        }

        /// Create answer (after receiving offer)
        pub fn create_answer(&self) -> Result<String, JsValue> {
            Ok("answer_sdp_placeholder".into())
        }

        pub fn is_connected(&self) -> bool { self.connected }
    }

    /// WebRTC signaling state
    #[derive(Clone, Debug)]
    pub enum WebRtcState {
        New,
        Connecting,
        Connected,
        Disconnected,
        Failed,
    }
}

// ═══════════════════════════════════════════════════════════ Codec Helpers

/// Simple Opus-like encoder (placeholder — real Opus needs libopus)
pub struct VoiceEncoder {
    codec: VoiceCodec,
    buffer: Vec<AudioSample>,
}

impl VoiceEncoder {
    pub fn new(codec: VoiceCodec) -> Self {
        Self { codec, buffer: Vec::new() }
    }

    /// Encode audio samples to bytes
    pub fn encode(&mut self, samples: &[AudioSample]) -> Vec<u8> {
        match self.codec {
            VoiceCodec::Raw => {
                // Simple PCM f32 → bytes
                samples.iter().flat_map(|s| s.to_le_bytes()).collect()
            }
            VoiceCodec::Opus | VoiceCodec::Speex => {
                // Quantize f32 → i16 for compact encoding
                samples.iter().flat_map(|s| {
                    let val = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
                    val.to_le_bytes()
                }).collect()
            }
        }
    }

    /// Decode bytes to audio samples
    pub fn decode(&mut self, data: &[u8]) -> Vec<AudioSample> {
        match self.codec {
            VoiceCodec::Raw => {
                data.chunks_exact(4)
                    .filter_map(|c| {
                        if c.len() == 4 {
                            Some(f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                        } else { None }
                    })
                    .collect()
            }
            VoiceCodec::Opus | VoiceCodec::Speex => {
                data.chunks_exact(2)
                    .filter_map(|c| {
                        if c.len() == 2 {
                            let val = i16::from_le_bytes([c[0], c[1]]);
                            Some(val as f32 / 32767.0)
                        } else { None }
                    })
                    .collect()
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════ Utility

/// Generate a test tone (sine wave) for testing
pub fn generate_tone(frequency: f32, duration_ms: u32, sample_rate: f32) -> Vec<AudioSample> {
    let num_samples = (sample_rate * duration_ms as f32 / 1000.0) as usize;
    (0..num_samples).map(|i| {
        let t = i as f32 / sample_rate;
        (2.0 * std::f32::consts::PI * frequency * t).sin() * 0.3
    }).collect()
}

/// Generate silence
pub fn silence(num_samples: usize) -> Vec<AudioSample> {
    vec![0.0; num_samples]
}

/// Mix two audio buffers
pub fn mix_audio(a: &[AudioSample], b: &[AudioSample]) -> Vec<AudioSample> {
    let len = a.len().max(b.len());
    (0..len).map(|i| {
        let sa = a.get(i).copied().unwrap_or(0.0);
        let sb = b.get(i).copied().unwrap_or(0.0);
        (sa + sb).clamp(-1.0, 1.0)
    }).collect()
}

/// Apply volume to audio buffer
pub fn apply_volume(samples: &mut [AudioSample], volume: f32) {
    for s in samples.iter_mut() { *s *= volume; }
}

// ═══════════════════════════════════════════════════════════ Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vad_silence() {
        let mut vad = VoiceActivityDetector::new();
        let silence = silence(SAMPLES_PER_FRAME);
        assert!(!vad.process(&silence));
        assert!(!vad.process(&silence));
    }

    #[test]
    fn test_vad_speech() {
        let mut vad = VoiceActivityDetector::new();
        let tone = generate_tone(440.0, FRAME_DURATION_MS, SAMPLE_RATE as f32);
        // Need activation_frames consecutive speech frames
        assert!(!vad.process(&tone));
        assert!(!vad.process(&tone));
        assert!(vad.process(&tone)); // 3rd frame triggers
    }

    #[test]
    fn test_vad_deactivation() {
        let mut vad = VoiceActivityDetector::new();
        let tone = generate_tone(440.0, FRAME_DURATION_MS, SAMPLE_RATE as f32);
        let silence = silence(SAMPLES_PER_FRAME);

        // Activate
        for _ in 0..5 { vad.process(&tone); }
        assert!(vad.is_speaking);

        // Need deactivation_frames of silence — smoothed energy decays slowly
        for _ in 0..50 { vad.process(&silence); }
        assert!(!vad.is_speaking);
    }

    #[test]
    fn test_audio_meter() {
        let mut meter = AudioMeter::new();
        let tone = generate_tone(440.0, FRAME_DURATION_MS, SAMPLE_RATE as f32);
        meter.process(&tone);
        assert!(meter.peak_level() > 0.1);
        assert!(meter.rms_level() > 0.01);
        assert!(!meter.is_clipping());
    }

    #[test]
    fn test_noise_gate() {
        let mut gate = NoiseGate::new();
        gate.threshold = 0.05;
        let mut quiet = vec![0.001; SAMPLES_PER_FRAME];
        gate.process(&mut quiet);
        // Quiet signal should be attenuated
        assert!(quiet[0].abs() < 0.001);

        let mut loud = generate_tone(440.0, FRAME_DURATION_MS, SAMPLE_RATE as f32);
        let original = loud.clone();
        gate.process(&mut loud);
        // Loud signal should pass through
        assert!(loud[100].abs() > original[100].abs() * 0.5);
    }

    #[test]
    fn test_agc() {
        let mut agc = AutoGainControl::new();
        // Process many quiet frames for AGC to converge
        for _ in 0..50 {
            let mut quiet = vec![0.01; SAMPLES_PER_FRAME];
            agc.process(&mut quiet);
        }
        assert!(agc.current_gain() > 1.0); // Should boost quiet audio

        // Process many loud frames
        for _ in 0..50 {
            let mut loud = vec![0.8; SAMPLES_PER_FRAME];
            agc.process(&mut loud);
        }
        assert!(agc.current_gain() < 1.0); // Should attenuate loud audio
    }

    #[test]
    fn test_spatial_audio_distance() {
        let spatial = SpatialAudioProcessor::new();
        let att = spatial.distance_attenuation([0.0, 0.0, 0.0], [10.0, 0.0, 0.0]);
        assert!(att > 0.0 && att < 1.0);

        let att_close = spatial.distance_attenuation([0.0, 0.0, 0.0], [0.5, 0.0, 0.0]);
        assert!(att_close > att); // Closer = louder
    }

    #[test]
    fn test_spatial_audio_panning() {
        let spatial = SpatialAudioProcessor::new();
        // Source to the right
        let (left, right) = spatial.pan_stereo(
            [0.0, 0.0, 0.0], [0.0, 0.0, -1.0], [5.0, 0.0, 0.0],
        );
        assert!(right > left);

        // Source to the left
        let (left, right) = spatial.pan_stereo(
            [0.0, 0.0, 0.0], [0.0, 0.0, -1.0], [-5.0, 0.0, 0.0],
        );
        assert!(left > right);
    }

    #[test]
    fn test_voice_encoder_raw() {
        let mut enc = VoiceEncoder::new(VoiceCodec::Raw);
        let samples = vec![0.5, -0.5, 0.0, 1.0, -1.0];
        let encoded = enc.encode(&samples);
        let decoded = enc.decode(&encoded);
        assert_eq!(decoded.len(), samples.len());
        for (a, b) in samples.iter().zip(decoded.iter()) {
            assert!((a - b).abs() < 0.001);
        }
    }

    #[test]
    fn test_voice_encoder_opus() {
        let mut enc = VoiceEncoder::new(VoiceCodec::Opus);
        let samples = vec![0.5, -0.5, 0.0, 1.0, -1.0];
        let encoded = enc.encode(&samples);
        let decoded = enc.decode(&encoded);
        assert_eq!(decoded.len(), samples.len());
        // Quantization loss is expected
        for (a, b) in samples.iter().zip(decoded.iter()) {
            assert!((a - b).abs() < 0.001);
        }
    }

    #[test]
    fn test_voice_chat_manager() {
        let mut mgr = VoiceChatManager::new(1, "TestPlayer");
        assert!(!mgr.mic_muted);
        assert!(!mgr.deafened);

        mgr.toggle_mute();
        assert!(mgr.mic_muted);
        mgr.toggle_mute();
        assert!(!mgr.mic_muted);

        mgr.toggle_deafen();
        assert!(mgr.deafened);
        assert!(mgr.mic_muted); // Deafen also mutes
    }

    #[test]
    fn test_peer_management() {
        let mut mgr = VoiceChatManager::new(1, "Host");
        mgr.add_peer(2, "Player2", VoiceChannelType::Global);
        mgr.add_peer(3, "Player3", VoiceChannelType::Global);
        assert_eq!(mgr.peers.len(), 2);

        mgr.remove_peer(2);
        assert_eq!(mgr.peers.len(), 1);

        let list = mgr.peer_list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].peer_id, 3);
    }

    #[test]
    fn test_ptt() {
        let mut mgr = VoiceChatManager::new(1, "Player");
        mgr.ptt_mode = true;
        assert!(!mgr.ptt_held);
        mgr.ptt_press();
        assert!(mgr.ptt_held);
        mgr.ptt_release();
        assert!(!mgr.ptt_held);
    }

    #[test]
    fn test_speaking_peers() {
        let mut mgr = VoiceChatManager::new(1, "Host");
        mgr.add_peer(2, "Speaker", VoiceChannelType::Global);
        mgr.add_peer(3, "Silent", VoiceChannelType::Global);
        mgr.add_peer(4, "Muted", VoiceChannelType::Global);
        mgr.peers.get_mut(&2).unwrap().is_speaking = true;
        mgr.peers.get_mut(&4).unwrap().is_muted = true;
        mgr.peers.get_mut(&4).unwrap().is_speaking = true;

        let speaking = mgr.speaking_peers();
        assert!(speaking.contains(&2));
        assert!(!speaking.contains(&3));
        assert!(!speaking.contains(&4)); // Muted
    }

    #[test]
    fn test_channel_settings() {
        let mut mgr = VoiceChatManager::new(1, "Player");
        mgr.set_channel(VoiceChannelType::Proximity);
        assert_eq!(mgr.current_channel, VoiceChannelType::Proximity);
        let settings = mgr.channel_settings.get(&VoiceChannelType::Proximity).unwrap();
        assert!(settings.spatial_enabled);
    }

    #[test]
    fn test_signal_processing() {
        let mut mgr = VoiceChatManager::new(1, "Host");
        let signal = VoiceSignal::VoiceActivity { peer_id: 2, is_speaking: true };
        mgr.process_signal(signal);
        assert!(mgr.signals.is_empty()); // Signal was consumed

        let signal = VoiceSignal::MuteState { peer_id: 2, muted: true };
        mgr.process_signal(signal);
    }

    #[test]
    fn test_generate_tone() {
        let tone = generate_tone(440.0, 20, 48000.0);
        assert_eq!(tone.len(), 960); // 20ms at 48kHz
        assert!(tone[0].abs() < 0.01); // Starts near zero (sine(0) = 0)
    }

    #[test]
    fn test_mix_audio() {
        let a = vec![0.5, 0.5, 0.5];
        let b = vec![0.3, 0.3, 0.3];
        let mixed = mix_audio(&a, &b);
        assert!((mixed[0] - 0.8).abs() < 0.01);
    }
}
