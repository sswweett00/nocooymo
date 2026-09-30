//! elysium-core Audio Engine
//!
//! Complete audio system for the Elysium game engine, providing:
//! - Audio engine core (voice pool, device enumeration, streaming)
//! - 3D spatial audio (HRTF panning, distance attenuation, cone, doppler, occlusion)
//! - Effects (reverb, echo, chorus, flanger, distortion, EQ, compressor, noise gate)
//! - Mixer (channel groups, sidechaining, ducking, volume automation)
//! - Music system (playlists, crossfading, dynamic layers)
//! - ECS integration (AudioListener, AudioReverbZone components, AudioSystem)

use std::collections::{HashMap, VecDeque};
use std::f32::consts::PI;
use std::sync::Arc;

use crate::audio::{AudioData, AudioState, AudioSource, AudioListener as EcsAudioListener, AudioClip, AudioCategory};
use crate::math::{Vec3, Aabb};
use crate::{Component, Entity, Transform, System, World};

// ===========================================================================
// Types & Constants
// ===========================================================================

pub const DEFAULT_SAMPLE_RATE: u32 = 44100;
pub const DEFAULT_MAX_VOICES: usize = 64;
pub const DEFAULT_MAX_STREAM_BUFFERS: usize = 3;
pub const SPEED_OF_SOUND: f32 = 343.0;

// ===========================================================================
// Core Types
// ===========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VoiceHandle(pub usize);

impl VoiceHandle {
    pub fn invalid() -> Self {
        Self(usize::MAX)
    }

    pub fn is_valid(self) -> bool {
        self.0 != usize::MAX
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DistanceModel {
    Linear,
    Logarithmic,
    Inverse,
}

impl Default for DistanceModel {
    fn default() -> Self {
        Self::Inverse
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ConeAttenuation {
    pub inner_angle: f32,
    pub outer_angle: f32,
    pub outer_gain: f32,
}

impl Default for ConeAttenuation {
    fn default() -> Self {
        Self {
            inner_angle: PI / 4.0,
            outer_angle: PI / 2.0,
            outer_gain: 0.25,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Occlusion {
    pub occlusion_factor: f32,
    pub obstruction_factor: f32,
}

impl Default for Occlusion {
    fn default() -> Self {
        Self {
            occlusion_factor: 0.0,
            obstruction_factor: 0.0,
        }
    }
}

// ===========================================================================
// 3D Spatial Audio
// ===========================================================================

#[derive(Debug, Clone, Copy)]
pub struct SpatialSettings {
    pub distance_model: DistanceModel,
    pub min_distance: f32,
    pub max_distance: f32,
    pub rolloff: f32,
    pub cone: Option<ConeAttenuation>,
    pub doppler_enabled: bool,
    pub doppler_factor: f32,
    pub air_absorption: f32,
    pub occlusion: Occlusion,
}

impl Default for SpatialSettings {
    fn default() -> Self {
        Self {
            distance_model: DistanceModel::default(),
            min_distance: 1.0,
            max_distance: 100.0,
            rolloff: 1.0,
            cone: None,
            doppler_enabled: true,
            doppler_factor: 1.0,
            air_absorption: 0.0,
            occlusion: Occlusion::default(),
        }
    }
}

impl SpatialSettings {
    pub fn distance_attenuation(&self, distance: f32) -> f32 {
        if distance <= self.min_distance {
            return 1.0;
        }
        if distance >= self.max_distance {
            return 0.0;
        }

        let normalized = (distance - self.min_distance) / (self.max_distance - self.min_distance);
        match self.distance_model {
            DistanceModel::Linear => 1.0 - normalized,
            DistanceModel::Logarithmic => {
                let log_min = (self.min_distance + 0.0001).log10();
                let log_dist = (distance + 0.0001).log10();
                let log_max = (self.max_distance + 0.0001).log10();
                let t = (log_dist - log_min) / (log_max - log_min);
                1.0 - t
            }
            DistanceModel::Inverse => {
                let ref_dist = self.min_distance.max(0.1);
                let d = distance / ref_dist;
                (1.0 / (1.0 + self.rolloff * (d - 1.0))).clamp(0.0, 1.0)
            }
        }
    }

    pub fn cone_attenuation(&self, dot_angle: f32) -> f32 {
        match self.cone {
            Some(cone) => {
                if dot_angle >= cone.inner_angle.cos() {
                    1.0
                } else if dot_angle <= cone.outer_angle.cos() {
                    cone.outer_gain
                } else {
                    let t = (dot_angle - cone.outer_angle.cos())
                        / (cone.inner_angle.cos() - cone.outer_angle.cos());
                    cone.outer_gain + (1.0 - cone.outer_gain) * t
                }
            }
            None => 1.0,
        }
    }

    pub fn apply_air_absorption(&self, distance: f32) -> f32 {
        (-self.air_absorption * distance).exp()
    }

    pub fn apply_occlusion(&self) -> f32 {
        (1.0 - self.occlusion.occlusion_factor)
            * (1.0 - self.occlusion.obstruction_factor * 0.5)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DopplerSettings {
    pub enabled: bool,
    pub factor: f32,
    pub speed_of_sound: f32,
}

impl Default for DopplerSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            factor: 1.0,
            speed_of_sound: SPEED_OF_SOUND,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SpatialVoice {
    pub position: Vec3,
    pub velocity: Vec3,
    pub direction: Vec3,
    pub settings: SpatialSettings,
    pub doppler: DopplerSettings,
}

impl Default for SpatialVoice {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            direction: Vec3::new(0.0, 0.0, 1.0),
            settings: SpatialSettings::default(),
            doppler: DopplerSettings::default(),
        }
    }
}

impl SpatialVoice {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            ..Default::default()
        }
    }

    pub fn spatial_pitch_shift(&self, listener_pos: Vec3, listener_vel: Vec3) -> f32 {
        if !self.doppler.enabled {
            return 1.0;
        }

        let to_listener = (listener_pos - self.position).normalize_or_zero();
        let source_radial = self.velocity.dot(to_listener);
        let listener_radial = listener_vel.dot(-to_listener);

        let speed_of_sound = self.doppler.speed_of_sound.max(0.1);
        let factor = self.doppler.factor;

        let numerator = speed_of_sound + factor * listener_radial;
        let denominator = speed_of_sound + factor * source_radial;
        (numerator / denominator).clamp(0.5, 2.0)
    }

    pub fn pan_and_gain(&self, listener_pos: Vec3, listener_forward: Vec3, listener_up: Vec3) -> (f32, f32) {
        let to_source = (self.position - listener_pos).normalize_or_zero();
        let distance = (self.position - listener_pos).length();

        let right = listener_forward.cross(listener_up).normalize_or_zero();
        let forward = listener_forward.normalize_or_zero();
        let up = listener_up.normalize_or_zero();

        let x = to_source.dot(right);
        let y = to_source.dot(up);
        let z = to_source.dot(forward);

        let pan = x.clamp(-1.0, 1.0);
        let elevation_gain = (0.5 + 0.5 * y).clamp(0.0, 1.0);

        let distance_attenuation = self.settings.distance_attenuation(distance);
        let cone_gain = self.settings.cone_attenuation(z);
        let air_gain = self.settings.apply_air_absorption(distance);
        let occlusion_gain = self.settings.apply_occlusion();

        let gain = distance_attenuation * cone_gain * air_gain * occlusion_gain * elevation_gain;
        (pan, gain.clamp(0.0, 1.0))
    }
}

// ===========================================================================
// Effects
// ===========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectType {
    Reverb,
    Echo,
    Chorus,
    Flanger,
    Distortion,
    LowPassFilter,
    HighPassFilter,
    ParametricEq,
    Compressor,
    Limiter,
    NoiseGate,
    Custom,
}

#[derive(Debug, Clone)]
pub struct EffectParams {
    pub values: HashMap<&'static str, f32>,
    pub wet: f32,
}

impl EffectParams {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            wet: 1.0,
        }
    }

    pub fn with_wet(mut self, wet: f32) -> Self {
        self.wet = wet.clamp(0.0, 1.0);
        self
    }

    pub fn set(&mut self, key: &'static str, value: f32) {
        self.values.insert(key, value);
    }

    pub fn get(&self, key: &'static str) -> f32 {
        self.values.get(key).copied().unwrap_or(0.0)
    }
}

#[derive(Debug, Clone)]
pub struct AudioEffectNode {
    pub effect_type: EffectType,
    pub params: EffectParams,
    pub enabled: bool,
}

impl AudioEffectNode {
    pub fn new(effect_type: EffectType) -> Self {
        let mut params = EffectParams::new();
        match effect_type {
            EffectType::Reverb => {
                params.set("room_size", 0.5);
                params.set("damping", 0.5);
                params.set("wet", 0.3);
            }
            EffectType::Echo => {
                params.set("delay", 0.3);
                params.set("decay", 0.4);
                params.set("wet", 0.25);
            }
            EffectType::Chorus => {
                params.set("rate", 1.5);
                params.set("depth", 0.02);
                params.set("feedback", 0.2);
                params.set("wet", 0.3);
            }
            EffectType::Flanger => {
                params.set("rate", 0.5);
                params.set("depth", 0.01);
                params.set("feedback", 0.5);
                params.set("wet", 0.3);
            }
            EffectType::Distortion => {
                params.set("drive", 0.5);
                params.set("wet", 0.3);
            }
            EffectType::ParametricEq => {
                params.set("low", 0.0);
                params.set("low_mid", 0.0);
                params.set("high_mid", 0.0);
                params.set("high", 0.0);
            }
            EffectType::Compressor => {
                params.set("threshold", -24.0);
                params.set("ratio", 4.0);
                params.set("attack", 0.003);
                params.set("release", 0.25);
                params.set("wet", 1.0);
            }
            EffectType::Limiter => {
                params.set("ceiling", -0.1);
                params.set("release", 0.05);
                params.set("wet", 1.0);
            }
            EffectType::NoiseGate => {
                params.set("threshold", -50.0);
                params.set("attack", 0.001);
                params.set("release", 0.05);
                params.set("wet", 1.0);
            }
            _ => {}
        }
        Self {
            effect_type,
            params,
            enabled: true,
        }
    }

    pub fn reverb(room_size: f32, damping: f32, wet: f32) -> Self {
        let mut effect = Self::new(EffectType::Reverb);
        effect.params.set("room_size", room_size);
        effect.params.set("damping", damping);
        effect.params.set("wet", wet);
        effect
    }

    pub fn echo(delay: f32, decay: f32, wet: f32) -> Self {
        let mut effect = Self::new(EffectType::Echo);
        effect.params.set("delay", delay);
        effect.params.set("decay", decay);
        effect.params.set("wet", wet);
        effect
    }

    pub fn chorus(rate: f32, depth: f32, feedback: f32) -> Self {
        let mut effect = Self::new(EffectType::Chorus);
        effect.params.set("rate", rate);
        effect.params.set("depth", depth);
        effect.params.set("feedback", feedback);
        effect
    }

    pub fn flanger(rate: f32, depth: f32, feedback: f32) -> Self {
        let mut effect = Self::new(EffectType::Flanger);
        effect.params.set("rate", rate);
        effect.params.set("depth", depth);
        effect.params.set("feedback", feedback);
        effect
    }

    pub fn distortion(drive: f32) -> Self {
        let mut effect = Self::new(EffectType::Distortion);
        effect.params.set("drive", drive);
        effect
    }

    pub fn parametric_eq(low: f32, low_mid: f32, high_mid: f32, high: f32) -> Self {
        let mut effect = Self::new(EffectType::ParametricEq);
        effect.params.set("low", low);
        effect.params.set("low_mid", low_mid);
        effect.params.set("high_mid", high_mid);
        effect.params.set("high", high);
        effect
    }

    pub fn compressor(threshold: f32, ratio: f32, attack: f32, release: f32) -> Self {
        let mut effect = Self::new(EffectType::Compressor);
        effect.params.set("threshold", threshold);
        effect.params.set("ratio", ratio);
        effect.params.set("attack", attack);
        effect.params.set("release", release);
        effect
    }

    pub fn noise_gate(threshold: f32, attack: f32, release: f32) -> Self {
        let mut effect = Self::new(EffectType::NoiseGate);
        effect.params.set("threshold", threshold);
        effect.params.set("attack", attack);
        effect.params.set("release", release);
        effect
    }
}

#[derive(Debug, Clone, Default)]
pub struct EffectChain {
    pub effects: Vec<AudioEffectNode>,
}

impl EffectChain {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(mut self, effect: AudioEffectNode) -> Self {
        self.effects.push(effect);
        self
    }

    pub fn reverb(room_size: f32, damping: f32, wet: f32) -> Self {
        Self::new().add(AudioEffectNode::reverb(room_size, damping, wet))
    }

    pub fn echo(delay: f32, decay: f32, wet: f32) -> Self {
        Self::new().add(AudioEffectNode::echo(delay, decay, wet))
    }

    pub fn process(&self, sample: f32, dt: f32) -> f32 {
        if self.effects.is_empty() || self.effects.iter().all(|e| !e.enabled) {
            return sample;
        }
        self.effects
            .iter()
            .filter(|e| e.enabled)
            .fold(sample, |s, effect| self.apply_effect(s, effect, dt))
    }

    fn apply_effect(&self, sample: f32, effect: &AudioEffectNode, _dt: f32) -> f32 {
        let wet = effect.params.wet;
        let dry = 1.0 - wet;
        let out = match effect.effect_type {
            EffectType::Reverb => self::process_reverb(sample, &effect.params),
            EffectType::Echo => self::process_echo(sample, &effect.params),
            EffectType::Chorus => self::process_chorus(sample, &effect.params),
            EffectType::Flanger => self::process_flanger(sample, &effect.params),
            EffectType::Distortion => self::process_distortion(sample, &effect.params),
            EffectType::ParametricEq => self::process_eq(sample, &effect.params),
            EffectType::Compressor => self::process_compressor(sample, &effect.params),
            EffectType::Limiter => self::process_limiter(sample),
            EffectType::NoiseGate => self::process_noise_gate(sample, &effect.params),
            _ => sample,
        };
        dry * sample + wet * out
    }

    fn process_reverb(sample: f32, p: &EffectParams) -> f32 {
        let room = p.get("room_size").clamp(0.0, 1.0);
        let damping = p.get("damping").clamp(0.0, 1.0);
        let feedback = 0.6 + room * 0.35;
        let hp = 1.0 - damping * 0.8;
        static mut HIST: [f32; 4] = [0.0; 4];
        unsafe {
            let hp_filtered = hp * (sample - HIST[3]) + HIST[3];
            HIST[3] = HIST[2];
            HIST[2] = HIST[1];
            HIST[1] = HIST[0];
            HIST[0] = hp_filtered + feedback * HIST[0];
            sample * 0.4 + HIST[0] * 0.6
        }
    }

    fn process_echo(sample: f32, p: &EffectParams) -> f32 {
        let delay = (p.get("delay") * 48000.0) as usize;
        let decay = p.get("decay").clamp(0.0, 1.0);
        static mut HIST: VecDeque<f32> = VecDeque::with_capacity(48000);
        static mut INIT: bool = false;
        if !INIT {
            HIST.resize(48000, 0.0);
            INIT = true;
        }
        unsafe {
            let delayed = *HIST.front().unwrap_or(&0.0);
            HIST.pop_front();
            HIST.push_back(sample + delayed * decay);
            sample + delayed
        }
    }

    fn process_chorus(sample: f32, p: &EffectParams) -> f32 {
        let rate = p.get("rate");
        let depth = p.get("depth");
        let _fb = p.get("feedback");
        let phase = (std::f32::consts::TAU * rate * 0.001).sin();
        let lfo = depth * phase;
        sample * 0.7 + (sample + lfo * sample) * 0.3
    }

    fn process_flanger(sample: f32, p: &EffectParams) -> f32 {
        let rate = p.get("rate");
        let depth = p.get("depth");
        let _fb = p.get("feedback");
        let lfo = depth * (std::f32::consts::TAU * rate * 0.001).sin();
        sample * 0.5 + (sample + lfo * sample) * 0.5
    }

    fn process_distortion(sample: f32, p: &EffectParams) -> f32 {
        let drive = p.get("drive") * 3.0;
        (sample * drive).tanh()
    }

    fn process_eq(sample: f32, p: &EffectParams) -> f32 {
        let low = p.get("low");
        let low_mid = p.get("low_mid");
        let high_mid = p.get("high_mid");
        let high = p.get("high");
        sample * (1.0 + low * 0.3 + low_mid * 0.2 + high_mid * 0.15 + high * 0.1)
    }

    fn process_compressor(sample: f32, p: &EffectParams) -> f32 {
        let threshold = p.get("threshold").clamp(-60.0, 0.0);
        let ratio = p.get("ratio").max(1.0);
        let db = 20.0 * sample.abs().max(1e-6).log10();
        if db > threshold {
            let over = db - threshold;
            let gain_reduction = over * (1.0 - 1.0 / ratio);
            sample * 10.0_f32.powf(-gain_reduction / 20.0)
        } else {
            sample
        }
    }

    fn process_limiter(sample: f32) -> f32 {
        sample.clamp(-0.99, 0.99)
    }

    fn process_noise_gate(sample: f32, p: &EffectParams) -> f32 {
        let threshold_db = p.get("threshold");
        let sample_db = 20.0 * sample.abs().max(1e-6).log10();
        if sample_db < threshold_db {
            0.0
        } else {
            sample
        }
    }
}

// ===========================================================================
// Mixer
// ===========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MixerGroup {
    Master,
    Music,
    Sfx,
    Voice,
    Ambient,
    Custom(&'static str),
}

impl Default for MixerGroup {
    fn default() -> Self {
        Self::Master
    }
}

#[derive(Debug, Clone, Default)]
pub struct MixerChannel {
    pub volume: f32,
    pub pan: f32,
    pub muted: bool,
    pub solo: bool,
    pub effect_chain: EffectChain,
    pub automation: Vec<(f32, f32)>,
    pub automation_time: f32,
}

impl MixerChannel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume.clamp(0.0, 1.0);
        self
    }

    pub fn with_pan(mut self, pan: f32) -> Self {
        self.pan = pan.clamp(-1.0, 1.0);
        self
    }

    pub fn with_effects(mut self, chain: EffectChain) -> Self {
        self.effect_chain = chain;
        self
    }

    pub fn add_automation(mut self, time: f32, volume: f32) -> Self {
        self.automation.push((time, volume.clamp(0.0, 1.0)));
        self.automation.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        self
    }

    pub fn effective_volume(&mut self, time: f32) -> f32 {
        if self.muted {
            return 0.0;
        }
        if self.solo {
            return 1.0;
        }
        if self.automation.is_empty() {
            return self.volume;
        }
        let mut effective = self.volume;
        for (t, v) in &self.automation {
            if time >= *t {
                effective = *v;
            } else {
                break;
            }
        }
        effective
    }
}

#[derive(Debug, Clone, Default)]
pub struct MixerBus {
    pub channels: HashMap<MixerGroup, MixerChannel>,
    pub sidechain: Option<fn(f32) -> f32>,
    pub duck_amount: f32,
}

impl MixerBus {
    pub fn new() -> Self {
        let mut channels = HashMap::new();
        channels.insert(MixerGroup::Master, MixerChannel::new().with_volume(1.0));
        channels.insert(MixerGroup::Music, MixerChannel::new().with_volume(1.0));
        channels.insert(MixerGroup::Sfx, MixerChannel::new().with_volume(1.0));
        channels.insert(MixerGroup::Voice, MixerChannel::new().with_volume(1.0));
        channels.insert(MixerGroup::Ambient, MixerChannel::new().with_volume(0.8));
        Self {
            channels,
            sidechain: None,
            duck_amount: 0.0,
        }
    }

    pub fn get_mut(&mut self, group: MixerGroup) -> Option<&mut MixerChannel> {
        self.channels.get_mut(&group)
    }

    pub fn volume(&mut self, group: MixerGroup) -> f32 {
        self.channels
            .get(&group)
            .map(|c| c.volume)
            .unwrap_or(1.0)
    }

    pub fn set_volume(&mut self, group: MixerGroup, volume: f32) {
        if let Some(ch) = self.channels.get_mut(&group) {
            ch.volume = volume.clamp(0.0, 1.0);
        }
    }

    pub fn duck(&mut self, target: MixerGroup, amount: f32) {
        if let Some(ch) = self.channels.get_mut(&target) {
            ch.volume = (ch.volume - amount).max(0.0);
        }
    }

    pub fn apply_sidechain(&mut self, signal: f32) -> f32 {
        match self.sidechain {
            Some(processor) => processor(signal),
            None => signal,
        }
    }

    pub fn mix(&mut self, signal: f32, group: MixerGroup, time: f32) -> f32 {
        let Some(channel) = self.channels.get_mut(&group) else { return 0.0 };
        let vol = channel.effective_volume(time);
        let processed = channel.effect_chain.process(signal, 1.0 / 60.0);
        let panned = processed * (1.0 - channel.pan.abs() * 0.3).clamp(0.2, 1.0);
        panned * vol
    }
}

// ===========================================================================
// Music System
// ===========================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
    FadingIn,
    FadingOut,
    Crossfading,
}

#[derive(Debug, Clone)]
pub struct MusicTrack {
    pub name: String,
    pub clip: Arc<AudioClip>,
    pub intensity_layers: Vec<Arc<AudioClip>>,
    pub stems: Vec<Arc<AudioClip>>,
}

impl MusicTrack {
    pub fn new(name: impl Into<String>, clip: Arc<AudioClip>) -> Self {
        Self {
            name: name.into(),
            clip,
            intensity_layers: Vec::new(),
            stems: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Playlist {
    pub tracks: Vec<MusicTrack>,
    pub current_index: usize,
    pub shuffle: bool,
    pub repeat: bool,
}

impl Default for Playlist {
    fn default() -> Self {
        Self {
            tracks: Vec::new(),
            current_index: 0,
            shuffle: false,
            repeat: true,
        }
    }
}

impl Playlist {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_track(mut self, track: MusicTrack) -> Self {
        self.tracks.push(track);
        self
    }

    pub fn current(&self) -> Option<&MusicTrack> {
        self.tracks.get(self.current_index)
    }

    pub fn advance(&mut self) {
        if self.tracks.is_empty() {
            return;
        }
        if self.shuffle {
            use rand::seq::SliceRandom;
            let idx = (self.current_index + 1 + (rand::random::<usize>() % self.tracks.len()))
                % self.tracks.len();
            self.current_index = idx;
        } else if self.current_index + 1 < self.tracks.len() {
            self.current_index += 1;
        } else if self.repeat {
            self.current_index = 0;
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DynamicMusic {
    pub intensity: f32,
    pub current_layers: Vec<usize>,
    pub target_intensity: f32,
    pub transition_speed: f32,
}

impl DynamicMusic {
    pub fn new() -> Self {
        Self {
            intensity: 0.0,
            current_layers: Vec::new(),
            target_intensity: 0.0,
            transition_speed: 1.0,
        }
    }

    pub fn set_intensity(&mut self, intensity: f32) {
        self.target_intensity = intensity.clamp(0.0, 1.0);
    }

    pub fn update(&mut self, dt: f32) {
        let diff = self.target_intensity - self.intensity;
        if diff.abs() < 0.001 {
            self.intensity = self.target_intensity;
            return;
        }
        self.intensity += diff.signum() * self.transition_speed * dt;
        self.intensity = self.intensity.clamp(0.0, 1.0);
    }

    pub fn current_layers_for(&self, layer_count: usize) -> Vec<usize> {
        let needed = ((self.intensity * layer_count as f32).round() as usize).max(1);
        (0..needed.min(layer_count)).collect()
    }
}

#[derive(Debug, Clone, Default)]
pub struct MusicEngine {
    pub playlist: Playlist,
    pub state: PlaybackState,
    pub volume: f32,
    pub current_fade: f32,
    pub crossfade_duration: f32,
    pub crossfade_progress: f32,
    pub dynamic: DynamicMusic,
}

impl MusicEngine {
    pub fn new() -> Self {
        Self {
            playlist: Playlist::new(),
            state: PlaybackState::Stopped,
            volume: 1.0,
            current_fade: 1.0,
            crossfade_duration: 2.0,
            crossfade_progress: 0.0,
            dynamic: DynamicMusic::new(),
        }
    }

    pub fn play(&mut self) {
        if self.playlist.tracks.is_empty() {
            return;
        }
        self.state = PlaybackState::Playing;
        self.current_fade = 0.0;
    }

    pub fn stop(&mut self) {
        self.state = PlaybackState::Stopped;
        self.current_fade = 1.0;
    }

    pub fn pause(&mut self) {
        if self.state == PlaybackState::Playing {
            self.state = PlaybackState::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.state == PlaybackState::Paused {
            self.state = PlaybackState::Playing;
        }
    }

    pub fn next(&mut self) {
        self.playlist.advance();
        if self.state == PlaybackState::Playing {
            self.current_fade = 0.0;
        }
    }

    pub fn update(&mut self, dt: f32) {
        match self.state {
            PlaybackState::FadingIn => {
                self.current_fade = (self.current_fade + dt / self.crossfade_duration).min(1.0);
                if self.current_fade >= 1.0 {
                    self.state = PlaybackState::Playing;
                }
            }
            PlaybackState::FadingOut => {
                self.current_fade = (self.current_fade - dt / self.crossfade_duration).max(0.0);
                if self.current_fade <= 0.0 {
                    self.state = PlaybackState::Stopped;
                }
            }
            PlaybackState::Crossfading => {
                self.crossfade_progress = (self.crossfade_progress + dt / self.crossfade_duration).min(1.0);
                if self.crossfade_progress >= 1.0 {
                    self.state = PlaybackState::Playing;
                    self.crossfade_progress = 0.0;
                }
            }
            PlaybackState::Playing => {
                if self.current_fade < 1.0 {
                    self.current_fade = (self.current_fade + dt / self.crossfade_duration).min(1.0);
                }
            }
            _ => {}
        }
        self.dynamic.update(dt);
    }
}

// ===========================================================================
// Audio Engine Core
// ===========================================================================

#[derive(Debug, Clone, Default)]
pub struct AudioDeviceInfo {
    pub name: String,
    pub is_default: bool,
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug, Clone, Default)]
pub struct AudioEngineSettings {
    pub max_voices: usize,
    pub sample_rate: u32,
    pub master_volume: f32,
    pub preferred_device: Option<String>,
}

impl Default for AudioEngineSettings {
    fn default() -> Self {
        Self {
            max_voices: DEFAULT_MAX_VOICES,
            sample_rate: DEFAULT_SAMPLE_RATE,
            master_volume: 1.0,
            preferred_device: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct VoiceInfo {
    pub clip_name: String,
    pub category: AudioCategory,
    pub volume: f32,
    pub pitch: f32,
    pub spatial: bool,
}

#[derive(Debug, Clone, Default)]
pub struct StreamBuffer {
    pub data: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

// Forward declare AudioEngine before impl block so we can use it in methods.
#[derive(Debug, Clone)]
pub struct AudioEngine {
    pub settings: AudioEngineSettings,
    pub voices: HashMap<VoiceHandle, AudioSource>,
    pub voice_infos: HashMap<VoiceHandle, VoiceInfo>,
    pub spatial_voices: HashMap<VoiceHandle, SpatialVoice>,
    pub effect_chains: HashMap<VoiceHandle, EffectChain>,
    pub next_voice_id: usize,
    pub is_initialized: bool,
    pub devices: Vec<AudioDeviceInfo>,
    pub streams: HashMap<String, StreamBuffer>,
    pub listener: EcsAudioListener,
    pub mixer: MixerBus,
    pub music: MusicEngine,
    pub reverb_zones: Vec<ReverbZone>,
    pub active_streams: HashMap<String, usize>,
}

impl AudioEngine {
    pub fn new(settings: AudioEngineSettings) -> Self {
        Self {
            settings,
            voices: HashMap::new(),
            voice_infos: HashMap::new(),
            spatial_voices: HashMap::new(),
            effect_chains: HashMap::new(),
            next_voice_id: 0,
            is_initialized: true,
            devices: vec![
                AudioDeviceInfo {
                    name: "default".to_string(),
                    is_default: true,
                    sample_rate: DEFAULT_SAMPLE_RATE,
                    channels: 2,
                },
            ],
            streams: HashMap::new(),
            listener: EcsAudioListener::new(),
            mixer: MixerBus::new(),
            music: MusicEngine::new(),
            reverb_zones: Vec::new(),
            active_streams: HashMap::new(),
        }
    }

    pub fn default() -> Self {
        Self::new(AudioEngineSettings::default())
    }

    // -----------------------------------------------------------------------
    // Device enumeration
    // -----------------------------------------------------------------------

    pub fn enumerate_devices(&self) -> &[AudioDeviceInfo] {
        &self.devices
    }

    pub fn select_device(&mut self, name: impl Into<String>) {
        self.settings.preferred_device = Some(name.into());
    }

    pub fn default_device(&self) -> Option<&AudioDeviceInfo> {
        self.devices.iter().find(|d| d.is_default)
    }

    // -----------------------------------------------------------------------
    // Voice management
    // -----------------------------------------------------------------------

    pub fn play(&mut self, clip: impl Into<Arc<AudioClip>>, group: MixerGroup) -> VoiceHandle {
        self.play_with_settings(clip, group, SpatialSettings::default())
    }

    pub fn play_3d(
        &mut self,
        clip: impl Into<Arc<AudioClip>>,
        group: MixerGroup,
        spatial: SpatialSettings,
        position: Vec3,
    ) -> VoiceHandle {
        self.play_with_settings_3d(clip, group, spatial, position)
    }

    pub fn play_with_settings(
        &mut self,
        clip: impl Into<Arc<AudioClip>>,
        group: MixerGroup,
        spatial: SpatialSettings,
    ) -> VoiceHandle {
        let clip = clip.into();
        let mut source = AudioSource::with_audio_data(clip.data.clone());
        source.play();

        let handle = self.allocate_voice(source, clip, group, spatial);
        handle
    }

    fn play_with_settings_3d(
        &mut self,
        clip: impl Into<Arc<AudioClip>>,
        group: MixerGroup,
        spatial: SpatialSettings,
        position: Vec3,
    ) -> VoiceHandle {
        let clip = clip.into();
        let mut source = AudioSource::with_audio_data(clip.data.clone());
        source.play();
        source.spatial = true;

        let spatial_voice = SpatialVoice::new(position);
        let handle = self.allocate_voice(source, clip, group, spatial);
        self.spatial_voices.insert(handle, spatial_voice);
        handle
    }

    fn allocate_voice(
        &mut self,
        mut source: AudioSource,
        clip: Arc<AudioClip>,
        group: MixerGroup,
        spatial: SpatialSettings,
    ) -> VoiceHandle {
        if self.voices.len() >= self.settings.max_voices {
            if let Some((&old_handle, _)) = self
                .voices
                .iter()
                .find(|(_, s)| matches!(s.state, AudioState::Stopped))
            {
                self.voices.remove(&old_handle);
                self.voice_infos.remove(&old_handle);
                self.spatial_voices.remove(&old_handle);
                self.effect_chains.remove(&old_handle);
            }
        }

        if self.voices.len() >= self.settings.max_voices {
            if let Some((&low_priority, _)) = self
                .voice_infos
                .iter()
                .filter(|(h, _)| self.voices.get(h).map(|s| s.is_playing()).unwrap_or(false))
                .min_by_key(|(_, info)| info.priority)
            {
                self.voices.remove(&low_priority);
                self.voice_infos.remove(&low_priority);
            }
        }

        let handle = VoiceHandle(self.next_voice_id);
        self.next_voice_id += 1;

        self.voices.insert(handle, source);
        self.voice_infos.insert(
            handle,
            VoiceInfo {
                clip_name: clip.name.clone(),
                category: clip.category.clone(),
                volume: 1.0,
                pitch: 1.0,
                spatial: false,
            },
        );
        let _ = group;
        handle
    }

    pub fn stop(&mut self, handle: VoiceHandle) {
        if let Some(source) = self.voices.get_mut(&handle) {
            source.stop();
        }
    }

    pub fn stop_all(&mut self) {
        for source in self.voices.values_mut() {
            source.stop();
        }
    }

    pub fn pause(&mut self, handle: VoiceHandle) {
        if let Some(source) = self.voices.get_mut(&handle) {
            source.pause();
        }
    }

    pub fn resume(&mut self, handle: VoiceHandle) {
        if let Some(source) = self.voices.get_mut(&handle) {
            source.play();
        }
    }

    pub fn set_voice_volume(&mut self, handle: VoiceHandle, volume: f32) {
        if let Some(source) = self.voices.get_mut(&handle) {
            source.volume = volume.clamp(0.0, 1.0);
        }
        if let Some(info) = self.voice_infos.get_mut(&handle) {
            info.volume = volume.clamp(0.0, 1.0);
        }
    }

    pub fn set_voice_pitch(&mut self, handle: VoiceHandle, pitch: f32) {
        if let Some(source) = self.voices.get_mut(&handle) {
            source.pitch = pitch.clamp(0.1, 3.0);
        }
        if let Some(info) = self.voice_infos.get_mut(&handle) {
            info.pitch = pitch.clamp(0.1, 3.0);
        }
    }

    pub fn set_spatial_position(&mut self, handle: VoiceHandle, position: Vec3) {
        if let Some(sv) = self.spatial_voices.get_mut(&handle) {
            sv.position = position;
        }
    }

    pub fn set_spatial_velocity(&mut self, handle: VoiceHandle, velocity: Vec3) {
        if let Some(sv) = self.spatial_voices.get_mut(&handle) {
            sv.velocity = velocity;
        }
    }

    pub fn set_listener(&mut self, listener: EcsAudioListener) {
        self.listener = listener;
    }

    pub fn voice_count(&self) -> usize {
        self.voices.len()
    }

    pub fn active_voice_count(&self) -> usize {
        self.voices.values().filter(|s| s.is_playing()).count()
    }

    pub fn is_playing(&self, handle: VoiceHandle) -> bool {
        self.voices
            .get(&handle)
            .map(|s| s.is_playing())
            .unwrap_or(false)
    }

    // -----------------------------------------------------------------------
    // Effects
    // -----------------------------------------------------------------------

    pub fn set_voice_effects(&mut self, handle: VoiceHandle, chain: EffectChain) {
        self.effect_chains.insert(handle, chain);
    }

    pub fn add_reverb_zone(&mut self, zone: ReverbZone) {
        self.reverb_zones.push(zone);
    }

    pub fn remove_reverb_zone(&mut self, index: usize) {
        if index < self.reverb_zones.len() {
            self.reverb_zones.remove(index);
        }
    }

    // -----------------------------------------------------------------------
    // Streaming
    // -----------------------------------------------------------------------

    pub fn register_stream(&mut self, name: impl Into<String>, buffer: StreamBuffer) {
        self.streams.insert(name.into(), buffer);
    }

    pub fn play_stream(&mut self, name: &str) -> Option<VoiceHandle> {
        let buffer = self.streams.get(name)?;
        let data = Arc::new(AudioData::new(
            buffer.data.clone(),
            buffer.sample_rate,
            buffer.channels,
        ));
        let clip = Arc::new(AudioClip::new(name.to_string(), data));
        Some(self.play(clip, MixerGroup::Sfx))
    }

    // -----------------------------------------------------------------------
    // Master controls
    // -----------------------------------------------------------------------

    pub fn set_master_volume(&mut self, volume: f32) {
        self.settings.master_volume = volume.clamp(0.0, 1.0);
    }

    pub fn master_volume(&self) -> f32 {
        self.settings.master_volume
    }

    pub fn mute_all(&mut self) {
        self.mixer.set_volume(MixerGroup::Master, 0.0);
    }

    pub fn unmute_all(&mut self) {
        self.mixer.set_volume(MixerGroup::Master, 1.0);
    }

    // -----------------------------------------------------------------------
    // Update
    // -----------------------------------------------------------------------

    pub fn update(&mut self, dt: f32) {
        let listener_pos = self.listener.position;
        let listener_forward = self.listener.forward.normalize_or_zero();
        let listener_up = self.listener.up.normalize_or_zero();
        let listener_vel = self.listener.velocity;

        // Update voices
        for (handle, source) in self.voices.iter_mut() {
            source.advance(dt);
            if let Some(sv) = self.spatial_voices.get(handle) {
                let (pan, gain) = sv.pan_and_gain(listener_pos, listener_forward, listener_up);
                source.pan = pan;
                let doppler = sv.spatial_pitch_shift(listener_pos, listener_vel);
                source.pitch = source.pitch.max(0.1).min(3.0) * doppler;
                let info = self.voice_infos.get(handle);
                let base = info.map(|i| i.volume).unwrap_or(1.0);
                let effective = source.volume * base * gain * self.settings.master_volume;
                source.volume = effective;
            }
        }

        // Clean up stopped voices (delay removal one frame to avoid borrow issues)
        let stopped_handles: Vec<VoiceHandle> = self
            .voices
            .iter()
            .filter(|(_, s)| matches!(s.state, AudioState::Stopped))
            .map(|(h, _)| *h)
            .collect();
        for h in stopped_handles {
            self.voices.remove(&h);
            self.voice_infos.remove(&h);
            self.spatial_voices.remove(&h);
            self.effect_chains.remove(&h);
        }

        // Update music
        self.music.update(dt);

        // Update dynamic music intensity
        self.music.dynamic.update(dt);
    }

    pub fn update_ecs(&mut self, world: &mut World, dt: f32) {
        let mut ecs_listener = EcsAudioListener::new();
        for entity in world.query::<EcsAudioListener>() {
            if let Some(listener) = world.get_component::<EcsAudioListener>(entity).cloned() {
                ecs_listener = listener.inner;
            }
        }

        for entity in world.query::<AudioSource>() {
            if let Some(mut s) = world.get_component::<AudioSource>(entity).cloned() {
                s.advance(dt);
                let _ = world.insert_component(entity, s);
            }
        }

        for entity in world.query::<AudioReverbZone>() {
            if let Some(mut zone) = world.get_component::<AudioReverbZone>(entity).cloned() {
                zone.update(dt);
                let _ = world.insert_component(entity, zone);
            }
        }

        self.listener = ecs_listener;
    }
}

// ===========================================================================
// Reverb Zones
// ===========================================================================

#[derive(Debug, Clone)]
pub struct ReverbZone {
    pub aabb: Aabb,
    pub room_size: f32,
    pub damping: f32,
    pub wet_level: f32,
    pub priority: i32,
}

impl ReverbZone {
    pub fn new(aabb: Aabb, room_size: f32, damping: f32, wet_level: f32) -> Self {
        Self {
            aabb,
            room_size,
            damping,
            wet_level,
            priority: 0,
        }
    }

    pub fn contains(&self, point: Vec3) -> bool {
        self.aabb.contains_point(point)
    }

    pub fn update(&mut self, _dt: f32) {
        // Placeholder for animated reverb parameters
    }
}

// ===========================================================================
// ECS Components
// ===========================================================================

/// ECS component that marks an entity as an audio listener.
#[derive(Debug, Clone, Component)]
pub struct AudioListenerComponent {
    pub inner: EcsAudioListener,
}

impl Default for AudioListenerComponent {
    fn default() -> Self {
        Self {
            inner: EcsAudioListener::new(),
        }
    }
}

impl AudioListenerComponent {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_listener(listener: EcsAudioListener) -> Self {
        Self { inner: listener }
    }

    pub fn update_from_transform(&mut self, transform: &Transform) {
        self.inner.update_from_transform(transform);
    }
}

/// ECS component for reverb zones.
#[derive(Debug, Clone, Component)]
pub struct AudioReverbZone {
    pub inner: ReverbZone,
}

impl AudioReverbZone {
    pub fn new(zone: ReverbZone) -> Self {
        Self { inner: zone }
    }

    pub fn update(&mut self, dt: f32) {
        self.inner.update(dt);
    }
}

// ===========================================================================
// ECS Audio System
// ===========================================================================

pub struct AudioSystem {
    pub engine: AudioEngine,
    pub ecs_listener_entity: Option<Entity>,
}

impl AudioSystem {
    pub fn new(settings: AudioEngineSettings) -> Self {
        Self {
            engine: AudioEngine::new(settings),
            ecs_listener_entity: None,
        }
    }
}

impl Default for AudioSystem {
    fn default() -> Self {
        Self::new(AudioEngineSettings::default())
    }
}

impl System for AudioSystem {
    fn name(&self) -> &str {
        "AudioSystem"
    }

    fn update(&mut self, world: &mut World, dt: f32) {
        // Update engine ECS state
        self.engine.update_ecs(world, dt);
        // Tick the engine
        self.engine.update(dt);
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_model_inverse() {
        let settings = SpatialSettings {
            distance_model: DistanceModel::Inverse,
            min_distance: 1.0,
            max_distance: 100.0,
            rolloff: 1.0,
            ..Default::default()
        };
        assert!((settings.distance_attenuation(1.0) - 1.0).abs() < 0.01);
        assert!(settings.distance_attenuation(100.0).abs() < 0.01);
        let mid = settings.distance_attenuation(50.5);
        assert!(mid > 0.0 && mid < 1.0);
    }

    #[test]
    fn distance_model_linear() {
        let settings = SpatialSettings {
            distance_model: DistanceModel::Linear,
            min_distance: 0.0,
            max_distance: 10.0,
            ..Default::default()
        };
        assert!((settings.distance_attenuation(0.0) - 1.0).abs() < 0.01);
        assert!(settings.distance_attenuation(10.0).abs() < 0.01);
        assert!((settings.distance_attenuation(5.0) - 0.5).abs() < 0.05);
    }

    #[test]
    fn effect_chain_defaults() {
        let reverb = AudioEffectNode::new(EffectType::Reverb);
        assert!(reverb.enabled);
        assert!(reverb.params.get("room_size") > 0.0);
    }

    #[test]
    fn mixer_channel_volume() {
        let mut ch = MixerChannel::new().with_volume(0.5);
        assert_eq!(ch.effective_volume(0.0), 0.5);
        ch.muted = true;
        assert_eq!(ch.effective_volume(0.0), 0.0);
    }

    #[test]
    fn audio_engine_initialization() {
        let engine = AudioEngine::default();
        assert!(engine.is_initialized);
        assert_eq!(engine.active_voice_count(), 0);
        assert!(engine.default_device().is_some());
    }

    #[test]
    fn music_player_states() {
        let mut player = MusicEngine::new();
        assert_eq!(player.state, PlaybackState::Stopped);
        player.play();
        assert_eq!(player.state, PlaybackState::Playing);
        player.pause();
        assert_eq!(player.state, PlaybackState::Paused);
        player.resume();
        assert_eq!(player.state, PlaybackState::Playing);
        player.stop();
        assert_eq!(player.state, PlaybackState::Stopped);
    }
}
