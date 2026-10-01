//! Cinematics module — cutscene, camera animation, timeline, and director system.
//!
//! This module integrates with the existing `camera`, `transform`, `animation_system`,
//! `input_manager`, and `math` modules.

use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::math::Vec3;
use crate::transform::Transform;

// ===========================================================================
// 1. MATH HELPERS — Easing & Spline Interpolation
// ===========================================================================

/// Easing function applied to a normalized `t` in [0, 1].
pub type EaseFn = fn(f32) -> f32;

pub fn linear(t: f32) -> f32 {
    t
}

pub fn ease_in(t: f32) -> f32 {
    t * t
}

pub fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(2)
}

pub fn ease_in_out(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

/// Catmull-Rom spline interpolation between four control points.
pub fn catmull_rom(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * (2.0 * p1 + (p2 - p0) * t + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2 + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

pub fn catmull_rom_vec3(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    Vec3::new(
        catmull_rom(p0.x, p1.x, p2.x, p3.x, t),
        catmull_rom(p0.y, p1.y, p2.y, p3.y, t),
        catmull_rom(p0.z, p1.z, p2.z, p3.z, t),
    )
}

pub fn catmull_rom_quat(q0: crate::math::Quat, q1: crate::math::Quat, q2: crate::math::Quat, q3: crate::math::Quat, t: f32) -> crate::math::Quat {
    // Sample four quaternions via slerp: treat as two blend pairs
    let a = q1.slerp(q2, t);
    let b = q0.slerp(q3, t);
    b.slerp(a, ease_in_out(t))
}

/// Bezier interpolation for scalar values.
pub fn bezier(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let t2 = t * t;
    mt2 * mt * p0 + 3.0 * mt2 * t * p1 + 3.0 * mt * t2 * p2 + t2 * t * p3
}

pub fn bezier_vec3(p0: Vec3, p1: Vec3, p2: Vec3, p3: Vec3, t: f32) -> Vec3 {
    Vec3::new(
        bezier(p0.x, p1.x, p2.x, p3.x, t),
        bezier(p0.y, p1.y, p2.y, p3.y, t),
        bezier(p0.z, p1.z, p2.z, p3.z, t),
    )
}

// ===========================================================================
// 2. TIMELINE — Keyframes, Tracks, and Playback
// ===========================================================================

/// Interpolation mode for keyframes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyframeInterpolation {
    Linear,
    Step,
    Bezier,
    CatmullRom,
    EaseIn,
    EaseOut,
    EaseInOut,
}

impl Default for KeyframeInterpolation {
    fn default() -> Self {
        Self::Linear
    }
}

/// A scalar or vector keyframe on a track.
#[derive(Debug, Clone)]
pub enum KeyframeValue {
    Float(f32),
    Vec3(Vec3),
    Quat(crate::math::Quat),
    Bool(bool),
    Color([f32; 4]),
}

impl KeyframeValue {
    pub fn lerp(&self, other: &Self, t: f32) -> Option<Self> {
        match (self, other) {
            (KeyframeValue::Float(a), KeyframeValue::Float(b)) => Some(KeyframeValue::Float(a + (b - a) * t)),
            (KeyframeValue::Vec3(a), KeyframeValue::Vec3(b)) => Some(KeyframeValue::Vec3(a.lerp(*b, t))),
            (KeyframeValue::Quat(a), KeyframeValue::Quat(b)) => Some(KeyframeValue::Quat(a.slerp(*b, t))),
            (KeyframeValue::Color(a), KeyframeValue::Color(b)) => Some(KeyframeValue::Color([
                a[0] + (b[0] - a[0]) * t,
                a[1] + (b[1] - a[1]) * t,
                a[2] + (b[2] - a[2]) * t,
                a[3] + (b[3] - a[3]) * t,
            ])),
            (KeyframeValue::Bool(_), KeyframeValue::Bool(_)) => {
                if t < 0.5 { Some(self.clone()) } else { Some(other.clone()) }
            }
            _ => None,
        }
    }

    pub fn apply_ease(&self, ease: EaseFn) -> Option<Self> {
        match self {
            KeyframeValue::Float(f) => Some(KeyframeValue::Float(ease(*f))),
            _ => Some(self.clone()),
        }
    }
}

/// A single keyframe in time.
#[derive(Debug, Clone)]
pub struct Keyframe {
    pub time: f32,
    pub value: KeyframeValue,
    pub interpolation: KeyframeInterpolation,
    pub in_tangent: Option<Vec3>,
    pub out_tangent: Option<Vec3>,
}

impl Keyframe {
    pub fn new_float(time: f32, value: f32) -> Self {
        Self {
            time,
            value: KeyframeValue::Float(value),
            interpolation: KeyframeInterpolation::Linear,
            in_tangent: None,
            out_tangent: None,
        }
    }

    pub fn new_vec3(time: f32, value: Vec3) -> Self {
        Self {
            time,
            value: KeyframeValue::Vec3(value),
            interpolation: KeyframeInterpolation::Linear,
            in_tangent: None,
            out_tangent: None,
        }
    }

    pub fn new_quat(time: f32, value: crate::math::Quat) -> Self {
        Self {
            time,
            value: KeyframeValue::Quat(value),
            interpolation: KeyframeInterpolation::Linear,
            in_tangent: None,
            out_tangent: None,
        }
    }

    pub fn with_interpolation(mut self, interp: KeyframeInterpolation) -> Self {
        self.interpolation = interp;
        self
    }
}

/// The kind of track in the timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackKind {
    CameraPosition,
    CameraRotation,
    CameraFov,
    CameraShake,
    CameraTransition,
    LookAtTarget,
    Audio,
    Animation,
    Vfx,
    Event,
    Custom,
}

impl std::fmt::Display for TrackKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrackKind::CameraPosition => write!(f, "CameraPosition"),
            TrackKind::CameraRotation => write!(f, "CameraRotation"),
            TrackKind::CameraFov => write!(f, "CameraFov"),
            TrackKind::CameraShake => write!(f, "CameraShake"),
            TrackKind::CameraTransition => write!(f, "CameraTransition"),
            TrackKind::LookAtTarget => write!(f, "LookAtTarget"),
            TrackKind::Audio => write!(f, "Audio"),
            TrackKind::Animation => write!(f, "Animation"),
            TrackKind::Vfx => write!(f, "Vfx"),
            TrackKind::Event => write!(f, "Event"),
            TrackKind::Custom => write!(f, "Custom"),
        }
    }
}

/// A single animated track in the timeline.
#[derive(Debug, Clone)]
pub struct Track {
    pub name: String,
    pub kind: TrackKind,
    pub keyframes: Vec<Keyframe>,
    pub target: Option<String>,
    pub enabled: bool,
    pub weight: f32,
}

impl Track {
    pub fn new(name: impl Into<String>, kind: TrackKind) -> Self {
        Self {
            name: name.into(),
            kind,
            keyframes: Vec::new(),
            target: None,
            enabled: true,
            weight: 1.0,
        }
    }

    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    pub fn add_keyframe(&mut self, keyframe: Keyframe) {
        self.keyframes.push(keyframe);
        self.keyframes.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    }

    /// Sample the track at `time` and apply `ease` to the resulting `t`.
    pub fn sample(&self, time: f32, ease: EaseFn) -> Option<KeyframeValue> {
        if self.keyframes.is_empty() || !self.enabled {
            return None;
        }

        let mut t_clamped = time;
        if let Some(last) = self.keyframes.last() {
            if t_clamped > last.time {
                t_clamped = last.time;
            }
        }

        // Find bracketing keyframes
        let mut prev_idx = None;
        let mut next_idx = None;

        for (i, kf) in self.keyframes.iter().enumerate() {
            if kf.time <= t_clamped {
                prev_idx = Some(i);
            } else if kf.time > t_clamped && next_idx.is_none() {
                next_idx = Some(i);
                break;
            }
        }

        match (prev_idx, next_idx) {
            (Some(pi), Some(ni)) => {
                let prev = &self.keyframes[pi];
                let next = &self.keyframes[ni];
                let mut t = (t_clamped - prev.time) / (next.time - prev.time).max(f32::EPSILON);

                match prev.interpolation {
                    KeyframeInterpolation::Step => Some(prev.value.clone()),
                    KeyframeInterpolation::EaseIn => {
                        t = ease_in(t);
                        prev.value.lerp(&next.value, t)
                    }
                    KeyframeInterpolation::EaseOut => {
                        t = ease_out(t);
                        prev.value.lerp(&next.value, t)
                    }
                    KeyframeInterpolation::EaseInOut => {
                        t = ease_in_out(t);
                        prev.value.lerp(&next.value, t)
                    }
                    _ => {
                        // Default linear interpolation
                        prev.value.lerp(&next.value, t)
                    }
                }
            }
            (Some(pi), None) => Some(self.keyframes[pi].value.clone()),
            (None, Some(_)) => Some(self.keyframes[0].value.clone()),
            _ => None,
        }
    }

    pub fn duration(&self) -> f32 {
        self.keyframes.last().map(|k| k.time).unwrap_or(0.0)
    }
}

// ===========================================================================
// 3. TIMELINE — Full Timeline with Tracks
// ===========================================================================

/// Playback state of the timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelinePlayback {
    Stopped,
    Playing,
    Paused,
}

/// The master timeline that drives a cutscene or cinematic sequence.
#[derive(Debug, Clone)]
pub struct Timeline {
    pub name: String,
    pub tracks: Vec<Track>,
    pub duration: f32,
    pub playback: TimelinePlayback,
    pub current_time: f32,
    pub speed: f32,
    pub loop_enabled: bool,
    pub loop_count: i32,
    pub loops_played: i32,
}

impl Default for Timeline {
    fn default() -> Self {
        Self::new("Untitled")
    }
}

impl Timeline {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            name,
            tracks: Vec::new(),
            duration: 0.0,
            playback: TimelinePlayback::Stopped,
            current_time: 0.0,
            speed: 1.0,
            loop_enabled: false,
            loop_count: 1,
            loops_played: 0,
        }
    }

    pub fn add_track(&mut self, track: Track) {
        let track_dur = track.duration();
        if track_dur > self.duration {
            self.duration = track_dur;
        }
        self.tracks.push(track);
    }

    pub fn remove_track(&mut self, name: &str) {
        self.tracks.retain(|t| t.name != name);
        self.recalculate_duration();
    }

    pub fn get_track(&self, name: &str) -> Option<&Track> {
        self.tracks.iter().find(|t| t.name == name)
    }

    pub fn get_track_mut(&mut self, name: &str) -> Option<&mut Track> {
        self.tracks.iter_mut().find(|t| t.name == name)
    }

    fn recalculate_duration(&mut self) {
        self.duration = self.tracks.iter().map(|t| t.duration()).fold(0.0, f32::max);
    }

    pub fn play(&mut self) {
        if self.playback == TimelinePlayback::Paused {
            self.playback = TimelinePlayback::Playing;
            return;
        }
        self.current_time = 0.0;
        self.loops_played = 0;
        self.playback = TimelinePlayback::Playing;
    }

    pub fn pause(&mut self) {
        self.playback = TimelinePlayback::Paused;
    }

    pub fn stop(&mut self) {
        self.playback = TimelinePlayback::Stopped;
        self.current_time = 0.0;
        self.loops_played = 0;
    }

    pub fn seek(&mut self, time: f32) {
        self.current_time = time.clamp(0.0, self.duration);
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
    }

    pub fn set_looping(&mut self, enabled: bool, count: i32) {
        self.loop_enabled = enabled;
        self.loop_count = if count < 0 { -1 } else { count.max(1) };
    }

    /// Update timeline by `delta_time`. Returns `true` if the timeline is still active.
    pub fn update(&mut self, delta_time: f32) -> bool {
        if self.playback != TimelinePlayback::Playing {
            return self.playback == TimelinePlayback::Paused;
        }

        self.current_time += delta_time * self.speed;

        if self.current_time >= self.duration {
            if self.loop_enabled {
                if self.loop_count < 0 || self.loops_played < self.loop_count - 1 {
                    self.current_time -= self.duration;
                    self.loops_played += 1;
                } else {
                    self.current_time = self.duration;
                    self.playback = TimelinePlayback::Stopped;
                    return false;
                }
            } else {
                self.current_time = self.duration;
                self.playback = TimelinePlayback::Stopped;
                return false;
            }
        }

        true
    }

    pub fn is_finished(&self) -> bool {
        self.playback == TimelinePlayback::Stopped && self.current_time >= self.duration
    }

    pub fn is_playing(&self) -> bool {
        self.playback == TimelinePlayback::Playing
    }

    pub fn normalized_time(&self) -> f32 {
        if self.duration > 0.0 {
            (self.current_time / self.duration).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

// ===========================================================================
// 4. CAMERA ANIMATION — Spline Paths, Shake, Transitions
// ===========================================================================

/// Spline type for camera paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplineType {
    Linear,
    CatmullRom,
    Bezier,
}

/// A camera path defined by control points.
#[derive(Debug, Clone)]
pub struct CameraSpline {
    pub name: String,
    pub points: Vec<(f32, Vec3, crate::math::Quat)>, // (time, position, rotation)
    pub spline_type: SplineType,
    pub closed: bool,
}

impl CameraSpline {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            points: Vec::new(),
            spline_type: SplineType::CatmullRom,
            closed: false,
        }
    }

    pub fn add_point(&mut self, time: f32, position: Vec3, rotation: crate::math::Quat) {
        self.points.push((time, position, rotation));
        self.points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    }

    pub fn duration(&self) -> f32 {
        self.points.last().map(|p| p.0).unwrap_or(0.0)
    }

    pub fn sample_position(&self, time: f32) -> Option<Vec3> {
        if self.points.is_empty() {
            return None;
        }

        let t_clamped = time.min(self.duration());

        // Find bracketing points
        let mut prev = 0usize;
        let mut next = 0usize;
        for i in 0..self.points.len() {
            if self.points[i].0 <= t_clamped {
                prev = i;
            } else {
                next = i;
                break;
            }
        }

        if prev == next {
            return Some(self.points[prev].1);
        }

        let (t0, p0, _) = self.points[prev.saturating_sub(1).min(self.points.len() - 1)];
        let (t1, p1, _) = self.points[prev];
        let (t2, p2, _) = self.points[next];
        let (t3, p3, _) = self
            .points
            .get(next + 1)
            .unwrap_or(&self.points[next])
            .clone();

        let segment_dur = (t2 - t1).max(f32::EPSILON);
        let local_t = ((t_clamped - t1) / segment_dur).clamp(0.0, 1.0);

        match self.spline_type {
            SplineType::Linear => Some(p1.lerp(p2, local_t)),
            SplineType::CatmullRom => Some(catmull_rom_vec3(p0, p1, p2, p3, local_t)),
            SplineType::Bezier => Some(bezier_vec3(p0, p1, p2, p3, local_t)),
        }
    }

    pub fn sample_rotation(&self, time: f32) -> Option<crate::math::Quat> {
        if self.points.is_empty() {
            return None;
        }

        let t_clamped = time.min(self.duration());
        let mut prev = 0usize;
        let mut next = 0usize;

        for i in 0..self.points.len() {
            if self.points[i].0 <= t_clamped {
                prev = i;
            } else {
                next = i;
                break;
            }
        }

        if prev == next {
            return Some(self.points[prev].2);
        }

        let (t1, _, q1) = self.points[prev];
        let (t2, _, q2) = self.points[next];
        let segment_dur = (t2 - t1).max(f32::EPSILON);
        let local_t = ((t_clamped - t1) / segment_dur).clamp(0.0, 1.0);

        Some(q1.slerp(q2, local_t))
    }
}

// Camera shake types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShakeType {
    Noise,
    Impulse,
    Sine,
}

impl Default for ShakeType {
    fn default() -> Self {
        Self::Noise
    }
}

/// Active camera shake instance.
#[derive(Debug, Clone)]
pub struct CameraShake {
    pub shake_type: ShakeType,
    pub amplitude: f32,
    pub frequency: f32,
    pub decay: f32,
    pub elapsed: f32,
    pub duration: f32,
    pub seed: u32,
    pub active: bool,
}

impl CameraShake {
    pub fn new(amplitude: f32, duration: f32) -> Self {
        Self {
            shake_type: ShakeType::Noise,
            amplitude,
            frequency: 30.0,
            decay: 1.0 / duration.max(f32::EPSILON),
            elapsed: 0.0,
            duration,
            seed: rand::random::<u32>(),
            active: true,
        }
    }

    pub fn impulse(amplitude: f32) -> Self {
        Self {
            shake_type: ShakeType::Impulse,
            amplitude,
            frequency: 20.0,
            decay: 8.0,
            elapsed: 0.0,
            duration: amplitude / 8.0,
            seed: rand::random::<u32>(),
            active: true,
        }
    }

    pub fn update(&mut self, delta_time: f32) -> Vec3 {
        if !self.active {
            return Vec3::ZERO;
        }

        self.elapsed += delta_time;

        if self.elapsed >= self.duration {
            self.active = false;
            return Vec3::ZERO;
        }

        let progress = self.elapsed / self.duration;
        let current_amp = self.amplitude * (1.0 - progress * self.decay);

        let t = self.elapsed * self.frequency;
        let seed = self.seed as f32;

        match self.shake_type {
            ShakeType::Noise => {
                // Pseudo-random noise based on seed and time
                let x = ((t * 1.1 + seed * 0.37).sin() * 43758.5453).fract() * 2.0 - 1.0;
                let y = ((t * 1.3 + seed * 0.71).sin() * 22578.1459).fract() * 2.0 - 1.0;
                let z = ((t * 1.7 + seed * 0.53).sin() * 12345.6789).fract() * 2.0 - 1.0;
                Vec3::new(x, y, z) * current_amp
            }
            ShakeType::Impulse => {
                let decay_curve = (-self.elapsed * self.decay * 2.0).exp();
                let x = ((t * 2.1 + seed).sin()).copysign(decay_curve);
                let y = ((t * 2.3 + seed * 0.7).sin()).copysign(decay_curve);
                let z = ((t * 2.7 + seed * 1.3).sin()).copysign(decay_curve);
                Vec3::new(x, y, z) * current_amp * decay_curve
            }
            ShakeType::Sine => {
                let x = (t * 1.0).sin() * current_amp;
                let y = (t * 1.4).cos() * current_amp;
                let z = (t * 1.8).sin() * current_amp * 0.5;
                Vec3::new(x, y, z)
            }
        }
    }
}

impl Default for CameraShake {
    fn default() -> Self {
        Self::new(0.5, 1.0)
    }
}

/// Camera transition mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraTransitionMode {
    Cut,
    Fade,
    Blend,
    BlendWithEase { ease: EaseFn },
}

impl Default for CameraTransitionMode {
    fn default() -> Self {
        Self::Cut
    }
}

/// A camera transition between two camera states.
#[derive(Debug, Clone)]
pub struct CameraTransition {
    pub mode: CameraTransitionMode,
    pub start_transform: Transform,
    pub end_transform: Transform,
    pub start_fov: f32,
    pub end_fov: f32,
    pub progress: f32,
    pub duration: f32,
    pub active: bool,
}

impl CameraTransition {
    pub fn new(mode: CameraTransitionMode, start: Transform, end: Transform, start_fov: f32, end_fov: f32, duration: f32) -> Self {
        Self {
            mode,
            start_transform: start,
            end_transform: end,
            start_fov,
            end_fov,
            progress: 0.0,
            duration,
            active: true,
        }
    }

    pub fn update(&mut self, delta_time: f32) -> Option<(Transform, f32)> {
        if !self.active {
            return None;
        }

        self.progress += delta_time / self.duration;

        if self.progress >= 1.0 {
            self.progress = 1.0;
            self.active = false;
        }

        let eased_t = match self.mode {
            CameraTransitionMode::Cut => {
                if self.progress >= 1.0 { 1.0 } else { 0.0 }
            }
            CameraTransitionMode::Blend => linear(self.progress),
            CameraTransitionMode::Fade => {
                // Fade-out then fade-in
                if self.progress < 0.5 {
                    ease_in(self.progress * 2.0)
                } else {
                    ease_out((self.progress - 0.5) * 2.0)
                }
            }
            CameraTransitionMode::BlendWithEase { ease } => ease(self.progress),
        };

        let translation = self
            .start_transform
            .translation
            .lerp(self.end_transform.translation, eased_t);
        let rotation = self
            .start_transform
            .rotation
            .slerp(self.end_transform.rotation, eased_t);
        let scale = self
            .start_transform
            .scale
            .lerp(self.end_transform.scale, eased_t);
        let fov = self.start_fov + (self.end_fov - self.start_fov) * eased_t;

        let transform = Transform::from_trs(translation, rotation, scale);
        Some((transform, fov))
    }
}

// ===========================================================================
// 5. DIRECTOR SYSTEM — Shot Management, Focus Pulling, Framing
// ===========================================================================

/// A single camera shot definition.
#[derive(Debug, Clone)]
pub struct Shot {
    pub name: String,
    pub camera_entity: Option<u32>,
    pub start_transform: Transform,
    pub end_transform: Transform,
    pub start_fov: f32,
    pub end_fov: f32,
    pub duration: f32,
    pub transition_mode: CameraTransitionMode,
    pub look_at: Option<Vec3>,
    pub shake: Option<CameraShake>,
    pub enabled: bool,
    pub weight: f32,
    pub framing: FramingComposition,
}

impl Shot {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            camera_entity: None,
            start_transform: Transform::identity(),
            end_transform: Transform::identity(),
            start_fov: 45.0f32.to_radians(),
            end_fov: 45.0f32.to_radians(),
            duration: 2.0,
            transition_mode: CameraTransitionMode::Blend,
            look_at: None,
            shake: None,
            enabled: true,
            weight: 1.0,
            framing: FramingComposition::default(),
        }
    }

    pub fn with_transform(mut self, start: Transform, end: Transform) -> Self {
        self.start_transform = start;
        self.end_transform = end;
        self
    }

    pub fn with_fov(mut self, start_fov: f32, end_fov: f32) -> Self {
        self.start_fov = start_fov;
        self.end_fov = end_fov;
        self
    }

    pub fn with_duration(mut self, duration: f32) -> Self {
        self.duration = duration;
        self
    }

    pub fn with_look_at(mut self, target: Vec3) -> Self {
        self.look_at = Some(target);
        self
    }

    pub fn with_transition(mut self, mode: CameraTransitionMode) -> Self {
        self.transition_mode = mode;
        self
    }
}

/// Framing composition rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FramingComposition {
    /// Rule of thirds — position subjects at 1/3 and 2/3 of frame
    RuleOfThirds,
    /// Symmetric center framing
    Centered,
    /// Wide shot — full body/environment visible
    Wide,
    /// Medium shot — from waist up
    Medium,
    /// Close-up — head and shoulders
    CloseUp,
    /// Dutch angle — rotated frame
    DutchAngle { roll: f32 },
}

impl Default for FramingComposition {
    fn default() -> Self {
        Self::RuleOfThirds
    }
}

impl FramingComposition {
    /// Compute a desired camera offset from the target for this framing.
    pub fn desired_offset(&self, target: Vec3, direction: Vec3, distance: f32) -> Vec3 {
        match self {
            FramingComposition::Wide => target - direction * distance * 2.0,
            FramingComposition::Medium => target - direction * distance,
            FramingComposition::CloseUp => target - direction * distance * 0.4,
            FramingComposition::RuleOfThirds => target - direction * distance * 0.8,
            FramingComposition::Centered => target - direction * distance * 0.6,
            FramingComposition::DutchAngle { roll: _ } => target - direction * distance * 0.7,
        }
    }
}

/// The director manages multiple cameras and shots.
#[derive(Debug, Clone, Default)]
pub struct Director {
    pub shots: Vec<Shot>,
    pub current_shot_index: usize,
    pub current_transition: Option<CameraTransition>,
    pub active_camera_entity: Option<u32>,
    pub look_at_override: Option<Vec3>,
    pub focus_distance: f32,
    pub focus_speed: f32,
    pub focus_target: Option<f32>,
}

impl Director {
    pub fn new() -> Self {
        Self {
            shots: Vec::new(),
            current_shot_index: 0,
            current_transition: None,
            active_camera_entity: None,
            look_at_override: None,
            focus_distance: 10.0,
            focus_speed: 5.0,
            focus_target: None,
        }
    }

    pub fn add_shot(&mut self, shot: Shot) {
        self.shots.push(shot);
    }

    pub fn current_shot(&self) -> Option<&Shot> {
        self.shots.get(self.current_shot_index)
    }

    pub fn current_shot_mut(&mut self) -> Option<&mut Shot> {
        self.shots.get_mut(self.current_shot_index)
    }

    pub fn advance_shot(&mut self) -> bool {
        if self.current_shot_index + 1 < self.shots.len() {
            self.current_shot_index += 1;
            true
        } else {
            false
        }
    }

    pub fn go_to_shot(&mut self, index: usize) -> bool {
        if index < self.shots.len() {
            self.current_shot_index = index;
            true
        } else {
            false
        }
    }

    pub fn set_focus_target(&mut self, distance: f32) {
        self.focus_target = Some(distance);
    }

    pub fn set_active_camera(&mut self, entity: Option<u32>) {
        self.active_camera_entity = entity;
    }

    /// Update the director and return the current camera transform + FOV if a transition is active.
    pub fn update(&mut self, delta_time: f32) -> Option<(Transform, f32)> {
        // Update focus pulling
        if let Some(target) = self.focus_target {
            self.focus_distance += (target - self.focus_distance).min(self.focus_speed * delta_time).max(-self.focus_speed * delta_time);
            if (self.focus_distance - target).abs() < 0.01 {
                self.focus_distance = target;
                self.focus_target = None;
            }
        }

        // Update active transition
        if let Some(ref mut transition) = self.current_transition {
            let result = transition.update(delta_time);
            if !transition.active {
                self.current_transition = None;
            }
            return result;
        }

        None
    }

    /// Start a transition to a new shot.
    pub fn transition_to_shot(&mut self, shot: &Shot) {
        let start_transform = self
            .current_transition
            .as_ref()
            .map(|t| t.end_transform.clone())
            .unwrap_or_else(|| shot.start_transform.clone());

        let transition = CameraTransition::new(
            shot.transition_mode,
            start_transform,
            shot.end_transform.clone(),
            shot.start_fov,
            shot.end_fov,
            shot.duration,
        );
        self.current_transition = Some(transition);
        self.active_camera_entity = shot.camera_entity;
        self.look_at_override = shot.look_at;
    }

    /// Cut directly to a shot without transition.
    pub fn cut_to_shot(&mut self, shot: &Shot) {
        self.current_transition = None;
        self.active_camera_entity = shot.camera_entity;
        self.look_at_override = shot.look_at;
    }
}

// ===========================================================================
// 6. CINEMATIC EVENTS — Types
// ===========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CinematicEventType {
    CameraMove,
    CameraShake,
    Dialogue,
    Sound,
    Vfx,
    Custom,
}

#[derive(Debug, Clone)]
pub struct CinematicEvent {
    pub event_type: CinematicEventType,
    pub time: f32,
    pub duration: Option<f32>,
    pub name: String,
    pub data: HashMap<String, KeyframeValue>,
}

impl CinematicEvent {
    pub fn new(event_type: CinematicEventType, time: f32, name: impl Into<String>) -> Self {
        Self {
            event_type,
            time,
            duration: None,
            name: name.into(),
            data: HashMap::new(),
        }
    }

    pub fn with_duration(mut self, duration: f32) -> Self {
        self.duration = Some(duration);
        self
    }

    pub fn with_data(mut self, key: impl Into<String>, value: KeyframeValue) -> Self {
        self.data.insert(key.into(), value);
        self
    }
}

// ===========================================================================
// 7. CUTSCENE SYSTEM
// ===========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FadeState {
    In,
    Out,
    None,
}

impl Default for FadeState {
    fn default() -> Self {
        Self::None
    }
}

/// Letterbox state for cinematic aspect ratio bars.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LetterboxState {
    Hidden,
    Showing,
    Visible,
    Hiding,
}

impl Default for LetterboxState {
    fn default() -> Self {
        Self::Hidden
    }
}

/// Cutscene-wide effects state.
#[derive(Debug, Clone)]
pub struct CutsceneEffects {
    pub fade_color: [f32; 4],
    pub fade_alpha: f32,
    pub fade_target: f32,
    pub fade_speed: f32,
    pub fade_state: FadeState,
    pub letterbox_state: LetterboxState,
    pub letterbox_ratio: f32,
    pub letterbox_target_ratio: f32,
    pub letterbox_current_ratio: f32,
    pub letterbox_speed: f32,
}

impl Default for CutsceneEffects {
    fn default() -> Self {
        Self {
            fade_color: [0.0, 0.0, 0.0, 1.0],
            fade_alpha: 0.0,
            fade_target: 0.0,
            fade_speed: 2.0,
            fade_state: FadeState::None,
            letterbox_state: LetterboxState::Hidden,
            letterbox_ratio: 0.0,
            letterbox_target_ratio: 0.0,
            letterbox_current_ratio: 0.0,
            letterbox_speed: 3.0,
        }
    }
}

impl CutsceneEffects {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn fade_in(&mut self, speed: f32) {
        self.fade_target = 0.0;
        self.fade_speed = speed;
        self.fade_state = FadeState::In;
    }

    pub fn fade_out(&mut self, speed: f32) {
        self.fade_target = 1.0;
        self.fade_speed = speed;
        self.fade_state = FadeState::Out;
    }

    pub fn show_letterbox(&mut self, ratio: f32, speed: f32) {
        self.letterbox_target_ratio = ratio;
        self.letterbox_speed = speed;
        self.letterbox_state = LetterboxState::Showing;
    }

    pub fn hide_letterbox(&mut self, speed: f32) {
        self.letterbox_target_ratio = 0.0;
        self.letterbox_speed = speed;
        self.letterbox_state = LetterboxState::Hiding;
    }

    pub fn update(&mut self, delta_time: f32) {
        // Update fade
        if self.fade_state != FadeState::None {
            let diff = self.fade_target - self.fade_alpha;
            let step = diff.signum() * self.fade_speed.min(diff.abs()) * delta_time;
            self.fade_alpha += step;
            if (self.fade_alpha - self.fade_target).abs() < 0.001 {
                self.fade_alpha = self.fade_target;
                if self.fade_state == FadeState::In {
                    self.fade_state = FadeState::None;
                }
            }
        }

        // Update letterbox
        match self.letterbox_state {
            LetterboxState::Showing => {
                self.letterbox_current_ratio += (self.letterbox_target_ratio - self.letterbox_current_ratio).min(self.letterbox_speed * delta_time).max(-self.letterbox_speed * delta_time);
                if (self.letterbox_current_ratio - self.letterbox_target_ratio).abs() < 0.001 {
                    self.letterbox_current_ratio = self.letterbox_target_ratio;
                    self.letterbox_state = LetterboxState::Visible;
                }
            }
            LetterboxState::Hiding => {
                self.letterbox_current_ratio += (0.0 - self.letterbox_current_ratio).min(self.letterbox_speed * delta_time).max(-self.letterbox_speed * delta_time);
                if self.letterbox_current_ratio.abs() < 0.001 {
                    self.letterbox_current_ratio = 0.0;
                    self.letterbox_state = LetterboxState::Hidden;
                }
            }
            _ => {}
        }
    }

    pub fn is_faded_out(&self) -> bool {
        self.fade_alpha >= 0.99
    }

    pub fn is_faded_in(&self) -> bool {
        self.fade_alpha <= 0.01 && self.fade_state == FadeState::None
    }

    pub fn letterbox_visible(&self) -> bool {
        matches!(self.letterbox_state, LetterboxState::Showing | LetterboxState::Visible) && self.letterbox_current_ratio > 0.01
    }
}

/// Save/load state for a cutscene.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CutsceneSaveState {
    pub timeline_time: f32,
    pub playback: TimelinePlayback,
    pub current_shot_index: usize,
    pub fade_alpha: f32,
    pub letterbox_ratio: f32,
    pub loops_played: i32,
}

/// The main cutscene struct.
#[derive(Debug, Clone, Default)]
pub struct Cutscene {
    pub name: String,
    pub timeline: Timeline,
    pub director: Director,
    pub events: Vec<CinematicEvent>,
    pub effects: CutsceneEffects,
    pub active_shake: Option<CameraShake>,
    pub skip_key: Option<String>,
    pub skip_enabled: bool,
    pub skip_requested: bool,
    pub game_paused: bool,
    pub camera_entity_override: Option<u32>,
    pub played_events: VecDeque<String>,
    pub finished: bool,
    pub on_finish: Option<String>, // callback name
}

impl Cutscene {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            timeline: Timeline::new(name.into()),
            director: Director::new(),
            events: Vec::new(),
            effects: CutsceneEffects::new(),
            active_shake: None,
            skip_key: None,
            skip_enabled: true,
            skip_requested: false,
            game_paused: false,
            camera_entity_override: None,
            played_events: VecDeque::new(),
            finished: false,
            on_finish: None,
        }
    }

    // --- Construction ---

    pub fn with_timeline(mut self, timeline: Timeline) -> Self {
        self.timeline = timeline;
        self
    }

    pub fn with_director(mut self, director: Director) -> Self {
        self.director = director;
        self
    }

    pub fn with_events(mut self, events: Vec<CinematicEvent>) -> Self {
        self.events = events;
        self
    }

    pub fn with_skip_key(mut self, key: impl Into<String>) -> Self {
        self.skip_key = Some(key.into());
        self
    }

    pub fn with_game_pause(mut self, pause: bool) -> Self {
        self.game_paused = pause;
        self
    }

    pub fn with_on_finish(mut self, callback: impl Into<String>) -> Self {
        self.on_finish = Some(callback.into());
        self
    }

    pub fn with_letterbox(mut self, ratio: f32, speed: f32) -> Self {
        self.effects.show_letterbox(ratio, speed);
        self
    }

    pub fn with_fade(mut self, fade_in: bool, speed: f32) -> Self {
        if fade_in {
            self.effects.fade_in(speed);
        } else {
            self.effects.fade_out(speed);
        }
        self
    }

    // --- Playback ---

    pub fn play(&mut self) {
        self.timeline.play();
        self.finished = false;
        self.skip_requested = false;
        self.played_events.clear();
    }

    pub fn pause(&mut self) {
        self.timeline.pause();
    }

    pub fn stop(&mut self) {
        self.timeline.stop();
        self.finished = true;
        self.effects = CutsceneEffects::default();
        self.active_shake = None;
        self.skip_requested = false;
    }

    pub fn skip(&mut self) {
        if self.skip_enabled {
            self.skip_requested = true;
        }
    }

    pub fn is_playing(&self) -> bool {
        self.timeline.is_playing()
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    // --- Update ---

    /// Update the cutscene. Returns `Some(camera_transform)` if the director has an active result.
    pub fn update(&mut self, delta_time: f32, input_manager: &crate::input::InputManager) -> Option<(Transform, f32)> {
        if self.finished {
            return None;
        }

        // Check skip input
        if self.skip_enabled && self.skip_requested {
            self.stop();
            return None;
        }

        if let Some(ref skip_key) = self.skip_key {
            if input_manager.get_input_state().is_action_down(skip_key.as_str()) {
                self.skip_requested = true;
            }
        }

        // Update effects
        self.effects.update(delta_time);

        // Update active shake
        if let Some(ref mut shake) = self.active_shake {
            let _ = shake.update(delta_time);
        }

        // Update timeline
        let active = self.timeline.update(delta_time);

        // Fire one-shot events at the right time
        self.fire_events();

        // Update director
        let director_result = self.director.update(delta_time);

        if !active && self.timeline.is_finished() {
            self.finished = true;
        }

        director_result
    }

    fn fire_events(&mut self) {
        let current_time = self.timeline.current_time;
        for event in &self.events {
            if event.time <= current_time && !self.played_events.contains(&event.name) {
                self.played_events.push_back(event.name.clone());
                // Event firing is handled by the renderer / game system
                // reading `played_events` as a signal.
            }
        }
    }

    // --- State ---

    pub fn save_state(&self) -> CutsceneSaveState {
        CutsceneSaveState {
            timeline_time: self.timeline.current_time,
            playback: self.timeline.playback,
            current_shot_index: self.director.current_shot_index,
            fade_alpha: self.effects.fade_alpha,
            letterbox_ratio: self.effects.letterbox_current_ratio,
            loops_played: self.timeline.loops_played,
        }
    }

    pub fn restore_state(&mut self, state: &CutsceneSaveState) {
        self.timeline.current_time = state.timeline_time;
        self.timeline.playback = state.playback;
        self.timeline.loops_played = state.loops_played;
        self.director.current_shot_index = state.current_shot_index;
        self.effects.fade_alpha = state.fade_alpha;
        self.effects.letterbox_current_ratio = state.letterbox_ratio;
    }

    // --- Camera helpers ---

    /// Start a camera shake effect for the duration of the cutscene.
    pub fn add_shake(&mut self, shake: CameraShake) {
        self.active_shake = Some(shake);
    }

    /// Apply shake offset to a base transform.
    pub fn apply_shake(&self, base_transform: &Transform) -> Transform {
        let mut result = base_transform.clone();
        if let Some(ref shake) = self.active_shake {
            let offset = shake.update(0.0); // query current offset
            result.translation += offset;
        }
        result
    }

    /// Get the current effective camera transform from the director + shake.
    pub fn effective_camera_transform(&self) -> Option<Transform> {
        let transition = self.director.current_transition.as_ref()?;
        let (transform, _) = transition;
        let mut result = transform.clone();
        if let Some(ref shake) = self.active_shake {
            result.translation += shake.update(0.0);
        }
        Some(result)
    }
}

// ===========================================================================
// 8. CINEMATIC RESOURCE MANAGER — Registry of Cutscenes
// ===========================================================================

#[derive(Debug, Clone, Default)]
pub struct CinematicManager {
    pub cutscenes: HashMap<String, Cutscene>,
    pub active_cutscene: Option<String>,
    pub cutscene_order: Vec<String>,
}

impl CinematicManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, cutscene: Cutscene) {
        let name = cutscene.name.clone();
        self.cutscene_order.push(name.clone());
        self.cutscenes.insert(name, cutscene);
    }

    pub fn unregister(&mut self, name: &str) {
        self.cutscenes.remove(name);
        self.cutscene_order.retain(|n| n != name);
        if self.active_cutscene.as_deref() == Some(name) {
            self.active_cutscene = None;
        }
    }

    pub fn get(&self, name: &str) -> Option<&Cutscene> {
        self.cutscenes.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Cutscene> {
        self.cutscenes.get_mut(name)
    }

    pub fn play(&mut self, name: &str) -> bool {
        if let Some(cutscene) = self.cutscenes.get_mut(name) {
            cutscene.play();
            self.active_cutscene = Some(name.to_string());
            true
        } else {
            false
        }
    }

    pub fn stop(&mut self, name: Option<&str>) {
        let target = name.or(self.active_cutscene.as_deref());
        if let Some(name) = target {
            if let Some(cutscene) = self.cutscenes.get_mut(name) {
                cutscene.stop();
            }
            if self.active_cutscene.as_deref() == Some(name) {
                self.active_cutscene = None;
            }
        }
    }

    pub fn active(&self) -> Option<&Cutscene> {
        self.active_cutscene
            .as_ref()
            .and_then(|name| self.cutscenes.get(name))
    }

    pub fn active_mut(&mut self) -> Option<&mut Cutscene> {
        if let Some(ref name) = self.active_cutscene {
            self.cutscenes.get_mut(name)
        } else {
            None
        }
    }

    pub fn update_all(&mut self, delta_time: f32, input_manager: &crate::input::InputManager) {
        // Update all cutscenes (but only the active one really matters for rendering)
        for cutscene in self.cutscenes.values_mut() {
            cutscene.update(delta_time, input_manager);
        }

        // Clear active reference if finished
        if let Some(ref active_name) = self.active_cutscene {
            if let Some(cutscene) = self.cutscenes.get(active_name) {
                if cutscene.is_finished() {
                    self.active_cutscene = None;
                }
            }
        }
    }
}

// ===========================================================================
// 9. ECS INTEGRATION — Cutscene System Component
// ===========================================================================

/// Component attached to an entity to mark it as participating in a cinematic.
#[derive(Debug, Clone)]
pub struct CinematicParticipant {
    pub cutscene_name: String,
    pub animation_override: Option<String>,
    pub visible: bool,
    pub frozen: bool,
}

impl Default for CinematicParticipant {
    fn default() -> Self {
        Self {
            cutscene_name: String::new(),
            animation_override: None,
            visible: true,
            frozen: false,
        }
    }
}

/// Resource stored in the ECS world that drives cinematic playback.
#[derive(Debug, Clone, Default)]
pub struct CinematicSystemResource {
    pub manager: CinematicManager,
    pub default_skip_action: String,
    pub pause_game_on_play: bool,
    pub game_was_paused: bool,
}

impl CinematicSystemResource {
    pub fn new() -> Self {
        Self {
            default_skip_action: "skip_cutscene".to_string(),
            pause_game_on_play: true,
            game_was_paused: false,
        }
    }

    /// ECS system update entry point.
    pub fn update(&mut self, delta_time: f32, input_manager: &crate::input::InputManager) {
        self.manager.update_all(delta_time, input_manager);
    }
}

// ===========================================================================
// 10. PRESET BUILDER — Convenience API
// ===========================================================================

pub mod presets {
    use super::*;

    /// Build a basic pan shot timeline.
    pub fn pan_shot(start: Vec3, end: Vec3, start_look: Vec3, end_look: Vec3, duration: f32) -> Timeline {
        let mut timeline = Timeline::new("pan_shot");
        let mut pos_track = Track::new("camera_position", TrackKind::CameraPosition);
        pos_track.add_keyframe(Keyframe::new_vec3(0.0, start).with_interpolation(KeyframeInterpolation::EaseInOut));
        pos_track.add_keyframe(Keyframe::new_vec3(duration, end).with_interpolation(KeyframeInterpolation::EaseInOut));
        timeline.add_track(pos_track);

        let mut look_track = Track::new("look_at", TrackKind::LookAtTarget);
        look_track.add_keyframe(Keyframe::new_vec3(0.0, start_look).with_interpolation(KeyframeInterpolation::EaseInOut));
        look_track.add_keyframe(Keyframe::new_vec3(duration, end_look).with_interpolation(KeyframeInterpolation::EaseInOut));
        timeline.add_track(look_track);

        timeline
    }

    /// Build a fly-through shot along control points.
    pub fn fly_through(points: &[Vec3], look_targets: &[Vec3], duration: f32) -> Timeline {
        let mut timeline = Timeline::new("fly_through");
        let mut pos_track = Track::new("camera_position", TrackKind::CameraPosition);
        let mut look_track = Track::new("look_at", TrackKind::LookAtTarget);

        let step = duration / points.len() as f32;
        for (i, p) in points.iter().enumerate() {
            let t = i as f32 * step;
            pos_track.add_keyframe(Keyframe::new_vec3(t, *p).with_interpolation(KeyframeInterpolation::EaseInOut));
        }
        if let Some(last) = points.last() {
            pos_track.add_keyframe(Keyframe::new_vec3(duration, *last));
        }

        let step_l = duration / look_targets.len() as f32;
        for (i, l) in look_targets.iter().enumerate() {
            let t = i as f32 * step_l;
            look_track.add_keyframe(Keyframe::new_vec3(t, *l).with_interpolation(KeyframeInterpolation::EaseInOut));
        }
        if let Some(last) = look_targets.last() {
            look_track.add_keyframe(Keyframe::new_vec3(duration, *last));
        }

        timeline.add_track(pos_track);
        timeline.add_track(look_track);
        timeline
    }

    /// Build a dialogue-focused shot.
    pub fn dialogue_shot(speaker_pos: Vec3, listener_pos: Vec3, duration: f32) -> Timeline {
        let mut timeline = Timeline::new("dialogue_shot");
        let midpoint = (speaker_pos + listener_pos) * 0.5;
        let speaker_dir = (speaker_pos - listener_pos).normalize();
        let offset = Vec3::new(speaker_dir.z, 0.2, -speaker_dir.x) * 1.5;

        let mut pos_track = Track::new("camera_position", TrackKind::CameraPosition);
        pos_track.add_keyframe(Keyframe::new_vec3(0.0, midpoint + offset).with_interpolation(KeyframeInterpolation::EaseInOut));
        pos_track.add_keyframe(Keyframe::new_vec3(duration, midpoint + offset * 0.95));
        timeline.add_track(pos_track);

        let mut look_track = Track::new("look_at", TrackKind::LookAtTarget);
        look_track.add_keyframe(Keyframe::new_vec3(0.0, speaker_pos + Vec3::new(0.0, 1.0, 0.0)));
        look_track.add_keyframe(Keyframe::new_vec3(duration, speaker_pos + Vec3::new(0.0, 1.0, 0.0)));
        timeline.add_track(look_track);

        timeline
    }

    /// Build a shake preset event.
    pub fn shake_event(time: f32, amplitude: f32, duration: f32) -> CinematicEvent {
        CinematicEvent::new(CinematicEventType::CameraShake, time, "shake")
            .with_duration(duration)
            .with_data("amplitude", KeyframeValue::Float(amplitude))
    }
}

// ===========================================================================
// 11. PUBLIC RE-EXPORTS
// ===========================================================================

pub mod prelude {
    pub use super::{
        // Core cinematic
        Cutscene, CutsceneEffects, CinematicEvent, CinematicEventType,
        CinematicManager, CinematicSystemResource, CinematicParticipant,
        CutsceneSaveState,
        // Timeline
        Timeline, TimelinePlayback, Track, TrackKind, Keyframe, KeyframeValue, KeyframeInterpolation,
        // Camera animation
        CameraSpline, SplineType, CameraShake, ShakeType, CameraTransition, CameraTransitionMode,
        // Director
        Director, Shot, FramingComposition,
        // Easing
        linear, ease_in, ease_out, ease_in_out, catmull_rom, catmull_rom_vec3, bezier, bezier_vec3,
        // Presets
        presets,
    };
}
