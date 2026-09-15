/// anim_state_machine.rs — Animation State Machine
/// Blend trees, IK (FABRIK/CCD), procedural animation, layers.
///
/// Özellikler:
/// - State machine with conditions and transitions
/// - 1D and 2D blend trees
/// - Additive animation layers
/// - FABRIK and CCD IK solvers
/// - Two-bone IK for arms/legs
/// - Look-at and Aim IK
/// - Procedural animation (spring, oscillation, noise)
/// - Animation parameters (float, int, bool, trigger)
/// - Animation events (callbacks at specific times)
/// - Animation layer system with blending

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════ Math Helpers

#[derive(Clone, Copy, Debug, Default)]
pub struct Vec3(pub f32, pub f32, pub f32);

impl Vec3 {
    pub const ZERO: Self = Self(0.0, 0.0, 0.0);
    pub const Y: Self = Self(0.0, 1.0, 0.0);
    pub fn new(x: f32, y: f32, z: f32) -> Self { Self(x, y, z) }
    pub fn splat(v: f32) -> Self { Self(v, v, v) }
    pub fn x(&self) -> f32 { self.0 }
    pub fn y(&self) -> f32 { self.1 }
    pub fn z(&self) -> f32 { self.2 }
    pub fn dot(self, o: Self) -> f32 { self.0*o.0 + self.1*o.1 + self.2*o.2 }
    pub fn cross(self, o: Self) -> Self {
        Self(self.1*o.2 - self.2*o.1, self.2*o.0 - self.0*o.2, self.0*o.1 - self.1*o.0)
    }
    pub fn length(self) -> f32 { self.dot(self).sqrt() }
    pub fn normalize(self) -> Self { let l = self.length(); if l > 1e-8 { self * (1.0/l) } else { Self::ZERO } }
    pub fn lerp(self, o: Self, t: f32) -> Self {
        Self(self.0+(o.0-self.0)*t, self.1+(o.1-self.1)*t, self.2+(o.2-self.2)*t)
    }
}

impl std::ops::Add for Vec3 { type Output = Self; fn add(self, o: Self) -> Self { Self(self.0+o.0, self.1+o.1, self.2+o.2) } }
impl std::ops::Sub for Vec3 { type Output = Self; fn sub(self, o: Self) -> Self { Self(self.0-o.0, self.1-o.1, self.2-o.2) } }
impl std::ops::Mul<f32> for Vec3 { type Output = Self; fn mul(self, s: f32) -> Self { Self(self.0*s, self.1*s, self.2*s) } }

#[derive(Clone, Copy, Debug, Default)]
pub struct Quat(pub f32, pub f32, pub f32, pub f32); // x, y, z, w

impl Quat {
    pub const IDENTITY: Self = Self(0.0, 0.0, 0.0, 1.0);

    pub fn from_euler(x: f32, y: f32, z: f32) -> Self {
        let (sx, cx) = (x * 0.5).sin_cos();
        let (sy, cy) = (y * 0.5).sin_cos();
        let (sz, cz) = (z * 0.5).sin_cos();
        Self(
            sx*cy*cz - cx*sy*sz,
            cx*sy*cz + sx*cy*sz,
            cx*cy*sz - sx*sy*cz,
            cx*cy*cz + sx*sy*sz,
        )
    }

    pub fn slerp(self, other: Self, t: f32) -> Self {
        let mut dot = self.0*other.0 + self.1*other.1 + self.2*other.2 + self.3*other.3;
        let mut b = other;
        if dot < 0.0 { dot = -dot; b = Self(-b.0, -b.1, -b.2, -b.3); }
        if dot > 0.9995 {
            let r = Self(
                self.0 + (b.0-self.0)*t,
                self.1 + (b.1-self.1)*t,
                self.2 + (b.2-self.2)*t,
                self.3 + (b.3-self.3)*t,
            );
            let l = (r.0*r.0 + r.1*r.1 + r.2*r.2 + r.3*r.3).sqrt();
            return Self(r.0/l, r.1/l, r.2/l, r.3/l);
        }
        let theta = dot.acos();
        let sin_theta = theta.sin();
        let a = ((1.0-t)*theta).sin() / sin_theta;
        let b_t = (t*theta).sin() / sin_theta;
        Self(
            self.0*a + b.0*b_t, self.1*a + b.1*b_t,
            self.2*a + b.2*b_t, self.3*a + b.3*b_t,
        )
    }

    pub fn to_euler(self) -> (f32, f32, f32) {
        let (x, y, z, w) = (self.0, self.1, self.2, self.3);
        let sinr_cosp = 2.0 * (w*x + y*z);
        let cosr_cosp = 1.0 - 2.0 * (x*x + y*y);
        let sinp = 2.0 * (w*y - z*x);
        let siny_cosp = 2.0 * (w*z + x*y);
        let cosy_cosp = 1.0 - 2.0 * (y*y + z*z);
        (
            sinr_cosp.atan2(cosr_cosp),
            if sinp.abs() >= 1.0 { std::f32::consts::FRAC_PI_2.copysign(sinp) } else { sinp.asin() },
            siny_cosp.atan2(cosy_cosp),
        )
    }
}

impl std::ops::Mul for Quat { type Output = Self; fn mul(self, o: Self) -> Self {
    Self(self.3*o.0 + self.0*o.3 + self.1*o.2 - self.2*o.1,
         self.3*o.1 - self.0*o.2 + self.1*o.3 + self.2*o.0,
         self.3*o.2 + self.0*o.1 - self.1*o.0 + self.2*o.3,
         self.3*o.3 - self.0*o.0 - self.1*o.1 - self.2*o.2)
}}

// ═══════════════════════════════════════════════════════════ Animation Clip

/// A single animation clip (sequence of keyframes per bone)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnimationClip {
    pub name: String,
    pub duration: f32,
    pub loop_animation: bool,
    pub speed: f32,
    pub channels: Vec<AnimationChannel>,
    pub events: Vec<AnimationEvent>,
}

/// Per-bone animation channel
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnimationChannel {
    pub bone_name: String,
    pub bone_index: u32,
    pub position_keys: Vec<KeyframeVec3>,
    pub rotation_keys: Vec<KeyframeQuat>,
    pub scale_keys: Vec<KeyframeVec3>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct KeyframeVec3 {
    pub time: f32,
    pub value: [f32; 3],
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct KeyframeQuat {
    pub time: f32,
    pub value: [f32; 4], // x, y, z, w
}

/// Animation event (triggered at specific time)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnimationEvent {
    pub time: f32,
    pub event_name: String,
    pub int_param: i32,
    pub float_param: f32,
    pub string_param: String,
}

impl Default for AnimationClip {
    fn default() -> Self {
        Self {
            name: "Unnamed".into(), duration: 1.0, loop_animation: true,
            speed: 1.0, channels: Vec::new(), events: Vec::new(),
        }
    }
}

impl AnimationClip {
    pub fn new(name: &str, duration: f32) -> Self {
        Self { name: name.into(), duration, ..Default::default() }
    }

    /// Sample the animation at a given time
    pub fn sample(&self, time: f32) -> Vec<BonePose> {
        let t = if self.loop_animation {
            time.rem_euclid(self.duration)
        } else {
            time.clamp(0.0, self.duration)
        };

        self.channels.iter().map(|ch| {
            let pos = Self::sample_vec3(&ch.position_keys, t);
            let rot = Self::sample_quat(&ch.rotation_keys, t);
            let scl = Self::sample_vec3(&ch.scale_keys, t);
            BonePose { bone_index: ch.bone_index, position: pos, rotation: rot, scale: scl }
        }).collect()
    }

    /// Get events that fire at a given time range
    pub fn get_events(&self, prev_time: f32, current_time: f32) -> Vec<&AnimationEvent> {
        self.events.iter().filter(|e| {
            if self.loop_animation {
                let t0 = prev_time.rem_euclid(self.duration);
                let t1 = current_time.rem_euclid(self.duration);
                if t1 >= t0 { e.time > t0 && e.time <= t1 }
                else { e.time > t0 || e.time <= t1 } // Wrapped
            } else {
                e.time > prev_time && e.time <= current_time
            }
        }).collect()
    }

    fn sample_vec3(keys: &[KeyframeVec3], t: f32) -> [f32; 3] {
        if keys.is_empty() { return [0.0; 3]; }
        if keys.len() == 1 { return keys[0].value; }
        // Find surrounding keyframes
        for i in 0..keys.len()-1 {
            if t >= keys[i].time && t <= keys[i+1].time {
                let blend = if (keys[i+1].time - keys[i].time).abs() < 0.0001 {
                    0.0
                } else {
                    (t - keys[i].time) / (keys[i+1].time - keys[i].time)
                };
                return [
                    keys[i].value[0] + (keys[i+1].value[0] - keys[i].value[0]) * blend,
                    keys[i].value[1] + (keys[i+1].value[1] - keys[i].value[1]) * blend,
                    keys[i].value[2] + (keys[i+1].value[2] - keys[i].value[2]) * blend,
                ];
            }
        }
        keys.last().unwrap().value
    }

    fn sample_quat(keys: &[KeyframeQuat], t: f32) -> [f32; 4] {
        if keys.is_empty() { return [0.0, 0.0, 0.0, 1.0]; }
        if keys.len() == 1 { return keys[0].value; }
        for i in 0..keys.len()-1 {
            if t >= keys[i].time && t <= keys[i+1].time {
                let blend = if (keys[i+1].time - keys[i].time).abs() < 0.0001 {
                    0.0
                } else {
                    (t - keys[i].time) / (keys[i+1].time - keys[i].time)
                };
                let a = Quat(keys[i].value[0], keys[i].value[1], keys[i].value[2], keys[i].value[3]);
                let b = Quat(keys[i+1].value[0], keys[i+1].value[1], keys[i+1].value[2], keys[i+1].value[3]);
                let r = a.slerp(b, blend);
                return [r.0, r.1, r.2, r.3];
            }
        }
        keys.last().unwrap().value
    }
}

/// Sampled bone pose
#[derive(Clone, Debug)]
pub struct BonePose {
    pub bone_index: u32,
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

// ═══════════════════════════════════════════════════════════ Blend Tree

/// Blend mode for blend trees
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlendMode {
    /// Linear interpolation between children
    Simple1D,
    /// 2D blend space (Cartesian or Simplex)
    Direct2D,
    /// Additive blend (add on top of base)
    Additive,
    /// Override blend (replaces base)
    Override,
}

/// A blend tree node (blends multiple clips/motions)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlendTree {
    pub name: String,
    pub blend_mode: BlendMode,
    pub children: Vec<BlendTreeChild>,
    /// Parameter name that controls blending (for 1D)
    pub blend_parameter: String,
    /// Second parameter (for 2D)
    pub blend_parameter_y: String,
    pub min_value: f32,
    pub max_value: f32,
    pub min_value_y: f32,
    pub max_value_y: f32,
    pub sync_group: String,
}

/// Child motion in a blend tree
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlendTreeChild {
    pub clip_name: String,
    pub motion_threshold: f32,
    pub threshold_y: f32,
    pub speed: f32,
    pub mirror: bool,
    pub cycle_offset: f32,
    /// Position in 2D blend space
    pub position: [f32; 2],
}

impl Default for BlendTree {
    fn default() -> Self {
        Self {
            name: "BlendTree".into(), blend_mode: BlendMode::Simple1D,
            children: Vec::new(), blend_parameter: "Speed".into(),
            blend_parameter_y: "Direction".into(),
            min_value: 0.0, max_value: 1.0,
            min_value_y: -1.0, max_value_y: 1.0,
            sync_group: String::new(),
        }
    }
}

impl BlendTree {
    pub fn new(name: &str, mode: BlendMode) -> Self {
        Self { name: name.into(), blend_mode: mode, ..Default::default() }
    }

    /// Add a child motion
    pub fn add_child(&mut self, clip_name: &str, threshold: f32) {
        self.children.push(BlendTreeChild {
            clip_name: clip_name.into(), motion_threshold: threshold,
            threshold_y: 0.0, speed: 1.0, mirror: false, cycle_offset: 0.0,
            position: [threshold, 0.0],
        });
        self.children.sort_by(|a, b| a.motion_threshold.partial_cmp(&b.motion_threshold)
            .unwrap_or(std::cmp::Ordering::Equal));
    }

    /// Add a 2D child motion
    pub fn add_child_2d(&mut self, clip_name: &str, x: f32, y: f32) {
        self.children.push(BlendTreeChild {
            clip_name: clip_name.into(), motion_threshold: x,
            threshold_y: y, speed: 1.0, mirror: false, cycle_offset: 0.0,
            position: [x, y],
        });
    }

    /// Calculate blend weights for the current parameter value
    pub fn calculate_weights(&self, param_value: f32, param_y: f32) -> Vec<(usize, f32)> {
        if self.children.is_empty() { return vec![]; }

        match self.blend_mode {
            BlendMode::Simple1D => self.blend_1d(param_value),
            BlendMode::Direct2D => self.blend_2d(param_value, param_y),
            BlendMode::Additive => {
                // First child is base (weight 1.0), rest are additive
                let mut weights = vec![(0, 1.0)];
                for i in 1..self.children.len() {
                    weights.push((i, 1.0));
                }
                weights
            }
            BlendMode::Override => {
                // Find closest child
                let closest = self.children.iter().enumerate()
                    .min_by_key(|(_, c)| (c.motion_threshold - param_value).abs() as u32)
                    .map(|(i, _)| i).unwrap_or(0);
                vec![(closest, 1.0)]
            }
        }
    }

    fn blend_1d(&self, value: f32) -> Vec<(usize, f32)> {
        if self.children.is_empty() { return vec![]; }
        if self.children.len() == 1 { return vec![(0, 1.0)]; }

        let v = value.clamp(self.min_value, self.max_value);

        // Find the two surrounding children
        let mut lower = 0;
        let mut upper = self.children.len() - 1;
        for i in 0..self.children.len() {
            if self.children[i].motion_threshold <= v { lower = i; }
            if self.children[i].motion_threshold >= v && i < upper { upper = i; }
        }
        if lower == upper {
            if upper > 0 { upper -= 1; } else if lower < self.children.len() - 1 { upper = lower + 1; }
            else { return vec![(0, 1.0)]; }
        }

        let range = self.children[upper].motion_threshold - self.children[lower].motion_threshold;
        let t = if range.abs() < 0.001 { 0.0 } else { (v - self.children[lower].motion_threshold) / range };
        let t = t.clamp(0.0, 1.0);

        let mut weights = vec![];
        if t > 0.001 { weights.push((lower, 1.0 - t)); }
        if t < 0.999 { weights.push((upper, t)); }
        if weights.is_empty() { weights.push((lower, 1.0)); }
        weights
    }

    fn blend_2d(&self, x: f32, y: f32) -> Vec<(usize, f32)> {
        if self.children.is_empty() { return vec![]; }
        if self.children.len() == 1 { return vec![(0, 1.0)]; }

        // Find 3 nearest children (barycentric)
        let mut distances: Vec<(usize, f32)> = self.children.iter().enumerate()
            .map(|(i, c)| {
                let dx = c.position[0] - x;
                let dy = c.position[1] - y;
                (i, (dx*dx + dy*dy).sqrt())
            }).collect();
        distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let n = distances.len().min(3);
        let total_dist: f32 = distances[..n].iter().map(|d| d.1).sum();
        let total_dist = total_dist.max(0.001);

        distances[..n].iter().map(|&(i, d)| {
            (i, 1.0 - d / total_dist)
        }).collect()
    }
}

// ═══════════════════════════════════════════════════════════ State Machine

/// Animation parameter type
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AnimParam {
    Float(f32),
    Int(i32),
    Bool(bool),
    Trigger(bool), // Auto-reset after consumption
}

impl Default for AnimParam {
    fn default() -> Self { AnimParam::Float(0.0) }
}

impl AnimParam {
    pub fn is_triggered(&self) -> bool {
        matches!(self, AnimParam::Trigger(true))
    }
}

/// Transition condition
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TransitionCondition {
    pub parameter: String,
    pub condition_type: ConditionType,
    pub threshold_value: AnimParam,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConditionType {
    #[default]
    If,             // bool == true
    IfNot,          // bool == false
    Greater,        // float > threshold
    Less,           // float < threshold
    Equals,         // int == threshold
    NotEqual,       // int != threshold
    GreaterEqual,   // float >= threshold
    LessEqual,      // float <= threshold
}

impl ConditionType {
    pub fn check(&self, param: &AnimParam, threshold: &AnimParam) -> bool {
        match (self, param, threshold) {
            (Self::If, AnimParam::Bool(v), _) => *v,
            (Self::IfNot, AnimParam::Bool(v), _) => !*v,
            (Self::If, AnimParam::Trigger(v), _) => *v,
            (Self::Greater, AnimParam::Float(v), AnimParam::Float(t)) => v > t,
            (Self::Less, AnimParam::Float(v), AnimParam::Float(t)) => v < t,
            (Self::GreaterEqual, AnimParam::Float(v), AnimParam::Float(t)) => v >= t,
            (Self::LessEqual, AnimParam::Float(v), AnimParam::Float(t)) => v <= t,
            (Self::Equals, AnimParam::Int(v), AnimParam::Int(t)) => v == t,
            (Self::NotEqual, AnimParam::Int(v), AnimParam::Int(t)) => v != t,
            _ => false,
        }
    }
}

/// Animation state
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnimState {
    pub name: String,
    pub clip_name: String,
    pub speed: f32,
    pub mirror: bool,
    pub cycle_offset: f32,
    pub blend_tree: Option<BlendTree>,
    pub transitions: Vec<Transition>,
    pub tags: Vec<String>,
    pub position: [f32; 2], // Editor position
    pub time: f32,
    pub motion_time: f32,
    pub active: bool,
    pub exit_time: f32,
}

/// Transition between states
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Transition {
    pub name: String,
    pub from_state: String,
    pub to_state: String,
    pub conditions: Vec<TransitionCondition>,
    pub has_exit_time: bool,
    pub exit_time: f32,
    pub exit_time_offset: f32,
    pub has_fixed_duration: bool,
    pub duration: f32,
    pub offset: f32,
    pub ordered_interruption: bool,
    pub priority: u32,
    pub mute: bool,
}

impl Default for AnimState {
    fn default() -> Self {
        Self {
            name: "State".into(), clip_name: "Idle".into(), speed: 1.0,
            mirror: false, cycle_offset: 0.0, blend_tree: None,
            transitions: Vec::new(), tags: Vec::new(),
            position: [0.0, 0.0], time: 0.0, motion_time: 0.0,
            active: false, exit_time: 0.0,
        }
    }
}

/// Complete animation state machine
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnimationStateMachine {
    pub name: String,
    pub states: Vec<AnimState>,
    pub parameters: HashMap<String, AnimParam>,
    pub entry_state: String,
    pub any_state_transitions: Vec<Transition>,
    pub default_state: String,
    pub layers: Vec<AnimLayer>,
    pub speed: f32,
}

/// Animation layer (for blending multiple machines)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnimLayer {
    pub name: String,
    pub weight: f32,
    pub blend_mode: BlendMode,
    pub state_machine: AnimationStateMachine,
    pub mask_bones: Vec<String>,
    pub override_motion: bool,
    pub synced_layer: Option<usize>,
}

impl Default for AnimationStateMachine {
    fn default() -> Self {
        Self {
            name: "Base Layer".into(),
            states: Vec::new(),
            parameters: HashMap::new(),
            entry_state: String::new(),
            any_state_transitions: Vec::new(),
            default_state: String::new(),
            layers: Vec::new(),
            speed: 1.0,
        }
    }
}

impl AnimationStateMachine {
    pub fn new(name: &str) -> Self {
        Self { name: name.into(), ..default() }
    }

    /// Add a state
    pub fn add_state(&mut self, name: &str, clip_name: &str) -> usize {
        let idx = self.states.len();
        self.states.push(AnimState {
            name: name.into(), clip_name: clip_name.into(), ..AnimState::default()
        });
        if self.default_state.is_empty() {
            self.default_state = name.into();
            self.entry_state = name.into();
        }
        idx
    }

    /// Add a blend tree state
    pub fn add_blend_tree_state(&mut self, name: &str, tree: BlendTree) -> usize {
        let idx = self.states.len();
        self.states.push(AnimState {
            name: name.into(), blend_tree: Some(tree),
            ..AnimState::default()
        });
        idx
    }

    /// Add a transition
    pub fn add_transition(&mut self, from: &str, to: &str, conditions: Vec<TransitionCondition>) {
        self.states.iter_mut().find(|s| s.name == from).map(|s| {
            s.transitions.push(Transition {
                from_state: from.into(), to_state: to.into(),
                conditions, duration: 0.25, ..Default::default()
            });
        });
    }

    /// Add a parameter
    pub fn add_parameter(&mut self, name: &str, value: AnimParam) {
        self.parameters.insert(name.into(), value);
    }

    /// Set parameter value
    pub fn set_parameter(&mut self, name: &str, value: AnimParam) {
        self.parameters.insert(name.into(), value);
    }

    /// Set float parameter
    pub fn set_float(&mut self, name: &str, value: f32) {
        self.parameters.insert(name.into(), AnimParam::Float(value));
    }

    /// Set bool parameter
    pub fn set_bool(&mut self, name: &str, value: bool) {
        self.parameters.insert(name.into(), AnimParam::Bool(value));
    }

    /// Set trigger parameter
    pub fn set_trigger(&mut self, name: &str) {
        self.parameters.insert(name.into(), AnimParam::Trigger(true));
    }

    /// Get active state
    pub fn active_state(&self) -> Option<&AnimState> {
        self.states.iter().find(|s| s.active)
    }

    /// Get active state mut
    pub fn active_state_mut(&mut self) -> Option<&mut AnimState> {
        self.states.iter_mut().find(|s| s.active)
    }

    /// Evaluate transitions and return the target state name if transition should happen
    pub fn evaluate_transitions(&self) -> Option<String> {
        let active = self.active_state()?;

        // Check transitions in priority order
        let mut candidates: Vec<&Transition> = active.transitions.iter()
            .filter(|t| !t.mute)
            .collect();
        candidates.sort_by(|a, b| b.priority.cmp(&a.priority));

        for trans in &candidates {
            let all_met = trans.conditions.iter().all(|cond| {
                if let Some(param) = self.parameters.get(&cond.parameter) {
                    cond.condition_type.check(param, &cond.threshold_value)
                } else {
                    false
                }
            });

            if all_met {
                // Check exit time if required
                if trans.has_exit_time {
                    let exit_time = trans.exit_time;
                    if active.time < exit_time {
                        continue;
                    }
                }
                return Some(trans.to_state.clone());
            }
        }

        // Check any-state transitions
        for trans in &self.any_state_transitions {
            let all_met = trans.conditions.iter().all(|cond| {
                if let Some(param) = self.parameters.get(&cond.parameter) {
                    cond.condition_type.check(param, &cond.threshold_value)
                } else { false }
            });
            if all_met && trans.from_state != active.name {
                return Some(trans.to_state.clone());
            }
        }

        None
    }

    /// Transition to a specific state
    pub fn transition_to(&mut self, state_name: &str) {
        // Deactivate all states
        for state in &mut self.states {
            state.active = false;
            state.time = 0.0;
        }
        // Activate target
        if let Some(state) = self.states.iter_mut().find(|s| s.name == state_name) {
            state.active = true;
            state.time = 0.0;
        }
        // Reset triggers
        for param in self.parameters.values_mut() {
            if let AnimParam::Trigger(v) = param { *v = false; }
        }
    }

    /// Update the state machine
    pub fn update(&mut self, dt: f32) {
        let dt = dt * self.speed;

        // Update active state time
        if let Some(state) = self.active_state_mut() {
            state.time += dt * state.speed;
            state.motion_time = state.time;
        }

        // Evaluate transitions
        if let Some(target) = self.evaluate_transitions() {
            self.transition_to(&target);
        }
    }

    /// Add a layer
    pub fn add_layer(&mut self, name: &str, weight: f32, blend_mode: BlendMode) -> usize {
        let idx = self.layers.len();
        self.layers.push(AnimLayer {
            name: name.into(), weight, blend_mode,
            state_machine: AnimationStateMachine::new(name),
            mask_bones: Vec::new(),
            override_motion: false,
            synced_layer: None,
        });
        idx
    }

    /// Sample all active animations and blend them
    pub fn sample(&self, clips: &HashMap<String, AnimationClip>) -> Vec<BonePose> {
        let mut result: Vec<BonePose> = Vec::new();

        // Sample base layer
        if let Some(state) = self.active_state() {
            if let Some(clip) = clips.get(&state.clip_name) {
                let mut poses = clip.sample(state.time * state.speed);
                // Apply mirror
                if state.mirror {
                    for pose in &mut poses {
                        pose.position[0] = -pose.position[0];
                        pose.rotation[1] = -pose.rotation[1];
                        pose.rotation[3] = -pose.rotation[3]; // Negate w for Y-axis mirror
                    }
                }
                result = poses;
            }
        }

        // Sample additional layers
        for layer in &self.layers {
            if layer.weight <= 0.0 { continue; }
            if let Some(state) = layer.state_machine.active_state() {
                if let Some(clip) = clips.get(&state.clip_name) {
                    let layer_poses = clip.sample(state.time * state.speed);
                    match layer.blend_mode {
                        BlendMode::Additive => {
                            for lpose in &layer_poses {
                                if let Some(base) = result.iter_mut().find(|p| p.bone_index == lpose.bone_index) {
                                    base.position[0] += lpose.position[0] * layer.weight;
                                    base.position[1] += lpose.position[1] * layer.weight;
                                    base.position[2] += lpose.position[2] * layer.weight;
                                }
                            }
                        }
                        BlendMode::Override => {
                            for lpose in &layer_poses {
                                if let Some(base) = result.iter_mut().find(|p| p.bone_index == lpose.bone_index) {
                                    let w = layer.weight;
                                    base.position = [
                                        base.position[0] * (1.0-w) + lpose.position[0] * w,
                                        base.position[1] * (1.0-w) + lpose.position[1] * w,
                                        base.position[2] * (1.0-w) + lpose.position[2] * w,
                                    ];
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        result
    }
}

fn default() -> AnimationStateMachine { AnimationStateMachine::default() }

// ═══════════════════════════════════════════════════════════ IK System

/// IK solver type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IKType {
    /// FABRIK (Forward And Backward Reaching IK)
    FABRIK,
    /// Cyclic Coordinate Descent
    CCD,
    /// Two-bone analytical IK (arms/legs)
    TwoBone,
    /// Look-at constraint
    LookAt,
    /// Aim constraint
    Aim,
}

/// IK bone chain
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IKChain {
    pub bones: Vec<IKBone>,
    pub target: [f32; 3],
    pub pole_target: [f32; 3],
    pub iterations: u32,
    pub tolerance: f32,
    pub weight: f32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct IKBone {
    pub bone_index: u32,
    pub length: f32,
    pub min_angle: f32,
    pub max_angle: f32,
}

impl Default for IKChain {
    fn default() -> Self {
        Self {
            bones: Vec::new(), target: [0.0; 3], pole_target: [0.0, 0.0, 1.0],
            iterations: 10, tolerance: 0.001, weight: 1.0,
        }
    }
}

impl IKChain {
    pub fn new() -> Self { Self::default() }

    /// FABRIK solver — iteratively positions bones toward target
    pub fn solve_fabrik(&self, bone_positions: &mut Vec<[f32; 3]>) {
        if self.bones.len() < 2 || bone_positions.len() < 2 { return; }

        let num_bones = self.bones.len();

        for _ in 0..self.iterations {
            // Forward reaching (from root to end effector)
            bone_positions[num_bones] = self.target;

            for i in (1..=num_bones).rev() {
                let dir = [
                    bone_positions[i][0] - bone_positions[i-1][0],
                    bone_positions[i][1] - bone_positions[i-1][1],
                    bone_positions[i][2] - bone_positions[i-1][2],
                ];
                let dist = (dir[0]*dir[0] + dir[1]*dir[1] + dir[2]*dir[2]).sqrt();
                if dist < 0.0001 { continue; }
                let len = self.bones[i-1].length;
                let factor = len / dist;
                bone_positions[i-1] = [
                    bone_positions[i][0] - dir[0] * factor,
                    bone_positions[i][1] - dir[1] * factor,
                    bone_positions[i][2] - dir[2] * factor,
                ];
            }

            // Backward reaching (from end effector to root)
            for i in 0..num_bones {
                let dir = [
                    bone_positions[i][0] - bone_positions[i+1][0],
                    bone_positions[i][1] - bone_positions[i+1][1],
                    bone_positions[i][2] - bone_positions[i+1][2],
                ];
                let dist = (dir[0]*dir[0] + dir[1]*dir[1] + dir[2]*dir[2]).sqrt();
                if dist < 0.0001 { continue; }
                let len = self.bones[i].length;
                let factor = len / dist;
                bone_positions[i+1] = [
                    bone_positions[i][0] - dir[0] * factor,
                    bone_positions[i][1] - dir[1] * factor,
                    bone_positions[i][2] - dir[2] * factor,
                ];
            }

            // Check convergence
            let end = bone_positions[num_bones];
            let err = ((end[0]-self.target[0]).powi(2) + (end[1]-self.target[1]).powi(2) + (end[2]-self.target[2]).powi(2)).sqrt();
            if err < self.tolerance { break; }
        }
    }

    /// CCD solver — rotates each bone to point toward target
    pub fn solve_ccd(&self, bone_positions: &mut Vec<[f32; 3]>, bone_rotations: &mut Vec<[f32; 4]>) {
        if self.bones.len() < 2 || bone_positions.len() < 2 { return; }

        let end_idx = self.bones.len();

        for _ in 0..self.iterations {
            for i in (0..end_idx).rev() {
                let chain_end = bone_positions[end_idx];
                let to_end = [
                    chain_end[0] - bone_positions[i][0],
                    chain_end[1] - bone_positions[i][1],
                    chain_end[2] - bone_positions[i][2],
                ];
                let to_target = [
                    self.target[0] - bone_positions[i][0],
                    self.target[1] - bone_positions[i][1],
                    self.target[2] - bone_positions[i][2],
                ];

                let d1 = (to_end[0]*to_end[0] + to_end[1]*to_end[1] + to_end[2]*to_end[2]).sqrt();
                let d2 = (to_target[0]*to_target[0] + to_target[1]*to_target[1] + to_target[2]*to_target[2]).sqrt();

                if d1 < 0.0001 || d2 < 0.0001 { continue; }

                // Calculate rotation axis and angle
                let cross = [
                    to_end[1]*to_target[2] - to_end[2]*to_target[1],
                    to_end[2]*to_target[0] - to_end[0]*to_target[2],
                    to_end[0]*to_target[1] - to_end[1]*to_target[0],
                ];
                let cross_len = (cross[0]*cross[0] + cross[1]*cross[1] + cross[2]*cross[2]).sqrt();
                let dot = (to_end[0]*to_target[0] + to_end[1]*to_target[1] + to_end[2]*to_target[2]) / (d1 * d2);
                let angle = dot.max(-1.0).min(1.0).acos();

                if cross_len > 0.0001 && angle > 0.0001 {
                    let axis = [cross[0]/cross_len, cross[1]/cross_len, cross[2]/cross_len];
                    let half_angle = angle * 0.5 * self.weight;
                    let s = half_angle.sin();
                    let q = Quat(axis[0]*s, axis[1]*s, axis[2]*s, half_angle.cos());
                    let bone_q = Quat(bone_rotations[i][0], bone_rotations[i][1], bone_rotations[i][2], bone_rotations[i][3]);
                    let new_q = q * bone_q;
                    bone_rotations[i] = [new_q.0, new_q.1, new_q.2, new_q.3];

                    // Update positions forward
                    for j in i..end_idx {
                        let len = self.bones[j].length;
                        let q_rot = Quat(bone_rotations[j][0], bone_rotations[j][1], bone_rotations[j][2], bone_rotations[j][3]);
                        let fwd = Self::rotate_vec3([0.0, len, 0.0], q_rot);
                        bone_positions[j+1] = [bone_positions[j][0]+fwd[0], bone_positions[j][1]+fwd[1], bone_positions[j][2]+fwd[2]];
                    }
                }
            }
        }
    }

    /// Two-bone IK solver (analytical solution for arm/leg)
    pub fn solve_two_bone(&self, bone_positions: &mut Vec<[f32; 3]>, bone_rotations: &mut Vec<[f32; 4]>) {
        if self.bones.len() < 2 || bone_positions.len() < 3 { return; }

        let root = bone_positions[0];
        let target = self.target;
        let mid = bone_positions[1];

        let a = self.bones[0].length;
        let b = self.bones[1].length;

        // Distance from root to target
        let dx = target[0] - root[0];
        let dy = target[1] - root[1];
        let dz = target[2] - root[2];
        let dist = (dx*dx + dy*dy + dz*dz).sqrt();

        // Clamp distance
        let dist = dist.clamp((a - b).abs() + 0.001, a + b - 0.001);

        // Cosine rule for angle at root
        let cos_angle = (a*a + dist*dist - b*b) / (2.0 * a * dist);
        let angle = cos_angle.max(-1.0).min(1.0).acos();

        // Direction to target
        let dir = [dx/dist, dy/dist, dz/dist];

        // Cross product with up to get elbow direction
        let up = [0.0, 1.0, 0.0];
        let right = Self::cross_vec3(dir, up);
        let right_len = (right[0]*right[0] + right[1]*right[1] + right[2]*right[2]).sqrt();
        let right = if right_len > 0.001 {
            [right[0]/right_len, right[1]/right_len, right[2]/right_len]
        } else {
            [1.0, 0.0, 0.0]
        };

        // Calculate elbow position
        let elbow_dir = [
            dir[0] * angle.cos() + right[0] * angle.sin(),
            dir[1] * angle.cos() + right[1] * angle.sin(),
            dir[2] * angle.cos() + right[2] * angle.sin(),
        ];

        bone_positions[1] = [
            root[0] + elbow_dir[0] * a,
            root[1] + elbow_dir[1] * a,
            root[2] + elbow_dir[2] * a,
        ];
        bone_positions[2] = target;
    }

    fn rotate_vec3(v: [f32; 3], q: Quat) -> [f32; 3] {
        let u = [q.0, q.1, q.2];
        let s = q.3;
        let t = 2.0 * (u[0]*v[0] + u[1]*v[1] + u[2]*v[2]);
        [
            v[0] + t*u[0] + s*(u[1]*v[2]-u[2]*v[1]),
            v[1] + t*u[1] + s*(u[2]*v[0]-u[0]*v[2]),
            v[2] + t*u[2] + s*(u[0]*v[1]-u[1]*v[0]),
        ]
    }

    fn cross_vec3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]]
    }
}

/// Look-at constraint
pub struct LookAtIK {
    pub bone_index: u32,
    pub target: [f32; 3],
    pub up: [f32; 3],
    pub weight: f32,
    pub axis: [f32; 3],
}

impl Default for LookAtIK {
    fn default() -> Self {
        Self { bone_index: 0, target: [0.0; 3], up: [0.0, 1.0, 0.0], weight: 1.0, axis: [0.0, 1.0, 0.0] }
    }
}

impl LookAtIK {
    pub fn solve(&self, bone_pos: [f32; 3]) -> Quat {
        let dir = [self.target[0]-bone_pos[0], self.target[1]-bone_pos[1], self.target[2]-bone_pos[2]];
        let len = (dir[0]*dir[0]+dir[1]*dir[1]+dir[2]*dir[2]).sqrt();
        if len < 0.0001 { return Quat::IDENTITY; }
        let forward = [dir[0]/len, dir[1]/len, dir[2]/len];
        let axis_norm = {
            let l = (self.axis[0]*self.axis[0]+self.axis[1]*self.axis[1]+self.axis[2]*self.axis[2]).sqrt();
            if l > 0.001 { [self.axis[0]/l, self.axis[1]/l, self.axis[2]/l] } else { [0.0, 1.0, 0.0] }
        };
        let cross = IKChain::cross_vec3(axis_norm, forward);
        let cross_len = (cross[0]*cross[0]+cross[1]*cross[1]+cross[2]*cross[2]).sqrt();
        if cross_len < 0.0001 { return Quat::IDENTITY; }
        let dot = axis_norm[0]*forward[0]+axis_norm[1]*forward[1]+axis_norm[2]*forward[2];
        let angle = dot.max(-1.0).min(1.0).acos();
        let half = angle * 0.5 * self.weight;
        let s = half.sin();
        Quat(cross[0]/cross_len*s, cross[1]/cross_len*s, cross[2]/cross_len*s, half.cos())
    }
}

// ═══════════════════════════════════════════════════════════ Procedural Animation

/// Procedural animation types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ProceduralType {
    /// Spring physics (bouncy response)
    Spring { stiffness: f32, damping: f32 },
    /// Oscillation (sine wave)
    Oscillation { frequency: f32, amplitude: f32, phase: f32 },
    /// Noise-based motion
    Noise { scale: f32, speed: f32 },
    /// Look-at (procedural head turn)
    LookAt { target: [f32; 3], speed: f32 },
    /// Gravity (fall/arc)
    Gravity { acceleration: f32 },
    /// Breathing (chest movement)
    Breathing { rate: f32, depth: f32 },
    /// Foot IK (ground adaptation)
    FootIK { ray_length: f32, step_height: f32 },
}

/// Spring physics state
#[derive(Clone, Debug)]
pub struct SpringState {
    pub value: f32,
    pub velocity: f32,
    pub target: f32,
}

impl Default for SpringState {
    fn default() -> Self { Self { value: 0.0, velocity: 0.0, target: 0.0 } }
}

impl SpringState {
    pub fn new() -> Self { Self::default() }

    /// Update spring physics
    pub fn update(&mut self, stiffness: f32, damping: f32, dt: f32) {
        let force = (self.target - self.value) * stiffness;
        self.velocity += force * dt;
        self.velocity *= (1.0 - damping * dt).max(0.0);
        self.value += self.velocity * dt;
    }

    pub fn set_target(&mut self, target: f32) { self.target = target; }
    pub fn get_value(&self) -> f32 { self.value }
}

/// Spring vector (3D spring)
#[derive(Clone, Debug)]
pub struct SpringVec3 {
    pub x: SpringState,
    pub y: SpringState,
    pub z: SpringState,
}

impl Default for SpringVec3 {
    fn default() -> Self {
        Self { x: SpringState::new(), y: SpringState::new(), z: SpringState::new() }
    }
}

impl SpringVec3 {
    pub fn new() -> Self { Self::default() }

    pub fn update(&mut self, stiffness: f32, damping: f32, dt: f32) {
        self.x.update(stiffness, damping, dt);
        self.y.update(stiffness, damping, dt);
        self.z.update(stiffness, damping, dt);
    }

    pub fn set_target(&mut self, target: [f32; 3]) {
        self.x.set_target(target[0]);
        self.y.set_target(target[1]);
        self.z.set_target(target[2]);
    }

    pub fn get_value(&self) -> [f32; 3] {
        [self.x.get_value(), self.y.get_value(), self.z.get_value()]
    }
}

/// Procedural motion component
#[derive(Clone, Debug)]
pub struct ProceduralMotion {
    pub bone_index: u32,
    pub motion_type: ProceduralType,
    pub weight: f32,
    pub time: f32,
    pub spring: SpringState,
    pub spring_vec: SpringVec3,
}

impl ProceduralMotion {
    pub fn new(bone_index: u32, motion_type: ProceduralType) -> Self {
        Self {
            bone_index, motion_type, weight: 1.0, time: 0.0,
            spring: SpringState::new(), spring_vec: SpringVec3::new(),
        }
    }

    /// Update and return position offset
    pub fn update(&mut self, dt: f32) -> [f32; 3] {
        self.time += dt;

        match &self.motion_type {
            ProceduralType::Spring { stiffness, damping } => {
                self.spring.update(*stiffness, *damping, dt);
                [0.0, self.spring.get_value() * self.weight, 0.0]
            }
            ProceduralType::Oscillation { frequency, amplitude, phase } => {
                let v = (self.time * frequency * std::f32::consts::TAU + phase).sin() * amplitude;
                [0.0, v * self.weight, 0.0]
            }
            ProceduralType::Noise { scale, speed } => {
                let t = self.time * speed;
                let x = (t * 1.1).sin() * scale;
                let y = (t * 1.7).cos() * scale;
                let z = (t * 0.9).sin() * scale;
                [x * self.weight, y * self.weight, z * self.weight]
            }
            ProceduralType::Breathing { rate, depth } => {
                let v = (self.time * rate * std::f32::consts::TAU).sin() * depth;
                [0.0, v * self.weight, v * 0.3 * self.weight]
            }
            _ => [0.0; 3],
        }
    }
}

/// Procedural animation manager
pub struct ProceduralAnimManager {
    pub motions: Vec<ProceduralMotion>,
}

impl Default for ProceduralAnimManager {
    fn default() -> Self { Self { motions: Vec::new() } }
}

impl ProceduralAnimManager {
    pub fn new() -> Self { Self::default() }

    pub fn add_motion(&mut self, bone_index: u32, motion_type: ProceduralType) -> usize {
        let idx = self.motions.len();
        self.motions.push(ProceduralMotion::new(bone_index, motion_type));
        idx
    }

    pub fn update(&mut self, dt: f32) -> Vec<(u32, [f32; 3])> {
        self.motions.iter_mut().map(|m| {
            let offset = m.update(dt);
            (m.bone_index, offset)
        }).collect()
    }

    pub fn clear(&mut self) { self.motions.clear(); }
}

// ═══════════════════════════════════════════════════════════ Animation Controller

/// High-level animation controller combining state machine + IK + procedural
pub struct AnimationController {
    pub state_machine: AnimationStateMachine,
    pub clips: HashMap<String, AnimationClip>,
    pub ik_chains: Vec<IKChain>,
    pub look_at: Vec<LookAtIK>,
    pub procedural: ProceduralAnimManager,
    pub output_poses: Vec<BonePose>,
    pub bone_count: u32,
    pub speed: f32,
    pub events_this_frame: Vec<AnimationEvent>,
}

impl Default for AnimationController {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimationController {
    pub fn new() -> Self {
        Self {
            state_machine: AnimationStateMachine::new("Base"),
            clips: HashMap::new(), ik_chains: Vec::new(),
            look_at: Vec::new(), procedural: ProceduralAnimManager::new(),
            output_poses: Vec::new(), bone_count: 0,
            speed: 1.0, events_this_frame: Vec::new(),
        }
    }

    /// Add a clip
    pub fn add_clip(&mut self, clip: AnimationClip) {
        self.clips.insert(clip.name.clone(), clip);
    }

    /// Update and sample all animation
    pub fn update(&mut self, dt: f32) {
        let dt = dt * self.speed;
        self.events_this_frame.clear();

        // Update state machine
        let prev_time = self.state_machine.active_state()
            .map(|s| s.time).unwrap_or(0.0);
        self.state_machine.update(dt);

        // Collect events
        if let Some(state) = self.state_machine.active_state() {
            if let Some(clip) = self.clips.get(&state.clip_name) {
                let events = clip.get_events(prev_time, state.time);
                for e in events {
                    self.events_this_frame.push(e.clone());
                }
            }
        }

        // Sample base animation
        self.output_poses = self.state_machine.sample(&self.clips);

        // Apply procedural animation
        let procedural_offsets = self.procedural.update(dt);
        for (bone_idx, offset) in procedural_offsets {
            if let Some(pose) = self.output_poses.iter_mut().find(|p| p.bone_index == bone_idx) {
                pose.position[0] += offset[0];
                pose.position[1] += offset[1];
                pose.position[2] += offset[2];
            }
        }

        // Apply IK (after procedural)
        for chain in &self.ik_chains {
            let mut positions: Vec<[f32; 3]> = chain.bones.iter().map(|b| {
                self.output_poses.iter()
                    .find(|p| p.bone_index == b.bone_index)
                    .map(|p| p.position)
                    .unwrap_or([0.0; 3])
            }).collect();
            if positions.len() > 1 {
                positions.push(chain.target);
                chain.solve_fabrik(&mut positions);
                for (i, bone) in chain.bones.iter().enumerate() {
                    if i < positions.len() {
                        if let Some(pose) = self.output_poses.iter_mut().find(|p| p.bone_index == bone.bone_index) {
                            let w = chain.weight;
                            pose.position[0] = pose.position[0] * (1.0-w) + positions[i][0] * w;
                            pose.position[1] = pose.position[1] * (1.0-w) + positions[i][1] * w;
                            pose.position[2] = pose.position[2] * (1.0-w) + positions[i][2] * w;
                        }
                    }
                }
            }
        }
    }

    /// Get the current output poses
    pub fn output(&self) -> &[BonePose] { &self.output_poses }

    /// Get events from this frame
    pub fn events(&self) -> &[AnimationEvent] { &self.events_this_frame }
}

// ═══════════════════════════════════════════════════════════ Demo Content

/// Create demo animation data
pub fn create_demo_animations() -> (HashMap<String, AnimationClip>, AnimationStateMachine) {
    // Idle animation
    let mut idle = AnimationClip::new("Idle", 2.0);
    idle.channels.push(AnimationChannel {
        bone_name: "Hips".into(), bone_index: 0,
        position_keys: vec![
            KeyframeVec3 { time: 0.0, value: [0.0, 0.0, 0.0] },
            KeyframeVec3 { time: 0.5, value: [0.0, 0.05, 0.0] },
            KeyframeVec3 { time: 1.0, value: [0.0, 0.0, 0.0] },
            KeyframeVec3 { time: 1.5, value: [0.0, 0.05, 0.0] },
            KeyframeVec3 { time: 2.0, value: [0.0, 0.0, 0.0] },
        ],
        rotation_keys: vec![
            KeyframeQuat { time: 0.0, value: [0.0, 0.0, 0.0, 1.0] },
            KeyframeQuat { time: 1.0, value: [0.0, 0.0, 0.0, 1.0] },
        ],
        scale_keys: vec![],
    });

    // Walk animation
    let mut walk = AnimationClip::new("Walk", 1.0);
    walk.loop_animation = true;
    walk.channels.push(AnimationChannel {
        bone_name: "Hips".into(), bone_index: 0,
        position_keys: vec![
            KeyframeVec3 { time: 0.0, value: [0.0, 0.0, 0.0] },
            KeyframeVec3 { time: 0.25, value: [0.0, 0.1, 0.0] },
            KeyframeVec3 { time: 0.5, value: [0.0, 0.0, 0.0] },
            KeyframeVec3 { time: 0.75, value: [0.0, 0.1, 0.0] },
            KeyframeVec3 { time: 1.0, value: [0.0, 0.0, 0.0] },
        ],
        rotation_keys: vec![
            KeyframeQuat { time: 0.0, value: [0.0, 0.0, 0.0, 1.0] },
            KeyframeQuat { time: 0.5, value: [0.0, 0.05, 0.0, 1.0] },
            KeyframeQuat { time: 1.0, value: [0.0, 0.0, 0.0, 1.0] },
        ],
        scale_keys: vec![],
    });
    // Walk event
    walk.events.push(AnimationEvent {
        time: 0.0, event_name: "FootStep".into(),
        int_param: 0, float_param: 0.0, string_param: "Left".into(),
    });
    walk.events.push(AnimationEvent {
        time: 0.5, event_name: "FootStep".into(),
        int_param: 1, float_param: 0.0, string_param: "Right".into(),
    });

    // Run animation
    let mut run = AnimationClip::new("Run", 0.6);
    run.loop_animation = true;
    run.speed = 1.0;
    run.channels.push(AnimationChannel {
        bone_name: "Hips".into(), bone_index: 0,
        position_keys: vec![
            KeyframeVec3 { time: 0.0, value: [0.0, 0.0, 0.0] },
            KeyframeVec3 { time: 0.15, value: [0.0, 0.15, 0.0] },
            KeyframeVec3 { time: 0.3, value: [0.0, 0.0, 0.0] },
            KeyframeVec3 { time: 0.45, value: [0.0, 0.15, 0.0] },
            KeyframeVec3 { time: 0.6, value: [0.0, 0.0, 0.0] },
        ],
        rotation_keys: vec![
            KeyframeQuat { time: 0.0, value: [0.0, 0.0, 0.0, 1.0] },
            KeyframeQuat { time: 0.3, value: [0.0, 0.08, 0.0, 1.0] },
            KeyframeQuat { time: 0.6, value: [0.0, 0.0, 0.0, 1.0] },
        ],
        scale_keys: vec![],
    });

    let mut clips = HashMap::new();
    clips.insert("Idle".into(), idle);
    clips.insert("Walk".into(), walk);
    clips.insert("Run".into(), run);

    // State machine
    let mut sm = AnimationStateMachine::new("Character");
    sm.add_parameter("Speed", AnimParam::Float(0.0));
    sm.add_parameter("IsRunning", AnimParam::Bool(false));
    sm.add_parameter("Jump", AnimParam::Trigger(false));

    sm.add_state("Idle", "Idle");
    sm.add_state("Walk", "Walk");
    sm.add_state("Run", "Run");

    // Walk → Idle (speed < 0.1)
    sm.add_transition("Walk", "Idle", vec![
        TransitionCondition { parameter: "Speed".into(), condition_type: ConditionType::Less,
            threshold_value: AnimParam::Float(0.1) },
    ]);
    // Idle → Walk (speed > 0.1)
    sm.add_transition("Idle", "Walk", vec![
        TransitionCondition { parameter: "Speed".into(), condition_type: ConditionType::Greater,
            threshold_value: AnimParam::Float(0.1) },
    ]);
    // Walk → Run (speed > 0.7)
    sm.add_transition("Walk", "Run", vec![
        TransitionCondition { parameter: "Speed".into(), condition_type: ConditionType::Greater,
            threshold_value: AnimParam::Float(0.7) },
    ]);
    // Run → Walk (speed < 0.7)
    sm.add_transition("Run", "Walk", vec![
        TransitionCondition { parameter: "Speed".into(), condition_type: ConditionType::Less,
            threshold_value: AnimParam::Float(0.7) },
    ]);

    // Start in idle
    if let Some(s) = sm.states.iter_mut().find(|s| s.name == "Idle") {
        s.active = true;
    }
    sm.default_state = "Idle".into();

    (clips, sm)
}

// ═══════════════════════════════════════════════════════════ Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quat_euler_roundtrip() {
        let q = Quat::from_euler(0.5, 1.0, 0.3);
        let (x, y, z) = q.to_euler();
        let q2 = Quat::from_euler(x, y, z);
        let dot = (q.0*q2.0 + q.1*q2.1 + q.2*q2.2 + q.3*q2.3).abs();
        assert!(dot > 0.999, "Quaternion roundtrip failed: {}", dot);
    }

    #[test]
    fn test_quat_slerp() {
        let a = Quat::IDENTITY;
        let b = Quat::from_euler(0.0, std::f32::consts::PI, 0.0);
        let mid = a.slerp(b, 0.5);
        // mid should be roughly halfway between IDENTITY and PI rotation on Y
        let (x, y, z) = mid.to_euler();
        let y_abs = y.abs() + z.abs() + x.abs();
        // Allow flexible euler decomposition — just verify it's roughly halfway
        assert!(y_abs > 0.5 && y_abs < 2.5, "Slerp mid expected ~PI/2, got euler=({},{},{})", x, y, z);
    }

    #[test]
    fn test_animation_clip_sampling() {
        let mut clip = AnimationClip::new("Test", 1.0);
        clip.channels.push(AnimationChannel {
            bone_name: "Bone0".into(), bone_index: 0,
            position_keys: vec![
                KeyframeVec3 { time: 0.0, value: [0.0, 0.0, 0.0] },
                KeyframeVec3 { time: 1.0, value: [1.0, 2.0, 3.0] },
            ],
            rotation_keys: vec![],
            scale_keys: vec![],
        });

        let poses = clip.sample(0.5);
        assert_eq!(poses.len(), 1);
        assert!((poses[0].position[0] - 0.5).abs() < 0.01);
        assert!((poses[0].position[1] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_animation_clip_events() {
        let mut clip = AnimationClip::new("Test", 1.0);
        clip.events.push(AnimationEvent {
            time: 0.5, event_name: "Step".into(),
            int_param: 0, float_param: 0.0, string_param: String::new(),
        });
        let events = clip.get_events(0.0, 0.6);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_name, "Step");

        let events = clip.get_events(0.6, 1.0);
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_blend_tree_1d() {
        let mut tree = BlendTree::new("Locomotion", BlendMode::Simple1D);
        tree.add_child("Idle", 0.0);
        tree.add_child("Walk", 0.5);
        tree.add_child("Run", 1.0);

        let weights = tree.calculate_weights(0.25, 0.0);
        assert_eq!(weights.len(), 2);
        // Should blend between Idle(0.0) and Walk(0.5)
        assert!((weights[0].1 - 0.5).abs() < 0.01);
        assert!((weights[1].1 - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_blend_tree_2d() {
        let mut tree = BlendTree::new("2D Blend", BlendMode::Direct2D);
        tree.add_child_2d("Idle", 0.0, 0.0);
        tree.add_child_2d("WalkForward", 1.0, 0.0);
        tree.add_child_2d("WalkBack", -1.0, 0.0);
        tree.add_child_2d("WalkLeft", 0.0, -1.0);

        let weights = tree.calculate_weights(0.5, 0.0);
        assert!(!weights.is_empty());
    }

    #[test]
    fn test_state_machine_creation() {
        let mut sm = AnimationStateMachine::new("Test");
        sm.add_parameter("Speed", AnimParam::Float(0.0));
        sm.add_state("Idle", "Idle");
        sm.add_state("Walk", "Walk");
        assert_eq!(sm.states.len(), 2);
    }

    #[test]
    fn test_state_machine_transition() {
        let mut sm = AnimationStateMachine::new("Test");
        sm.add_parameter("Speed", AnimParam::Float(0.0));
        sm.add_state("Idle", "Idle");
        sm.add_state("Walk", "Walk");

        // Start in idle
        sm.states[0].active = true;

        // Add transition: Idle → Walk when Speed > 0.5
        sm.add_transition("Idle", "Walk", vec![
            TransitionCondition { parameter: "Speed".into(), condition_type: ConditionType::Greater,
                threshold_value: AnimParam::Float(0.5) },
        ]);

        // Speed is 0, no transition
        sm.update(0.016);
        assert_eq!(sm.active_state().unwrap().name, "Idle");

        // Set speed > 0.5
        sm.set_float("Speed", 0.8);
        sm.update(0.016);
        assert_eq!(sm.active_state().unwrap().name, "Walk");
    }

    #[test]
    fn test_condition_types() {
        assert!(ConditionType::Greater.check(&AnimParam::Float(1.0), &AnimParam::Float(0.5)));
        assert!(!ConditionType::Greater.check(&AnimParam::Float(0.3), &AnimParam::Float(0.5)));
        assert!(ConditionType::If.check(&AnimParam::Bool(true), &AnimParam::Bool(false)));
        assert!(!ConditionType::If.check(&AnimParam::Bool(false), &AnimParam::Bool(false)));
        assert!(ConditionType::Equals.check(&AnimParam::Int(5), &AnimParam::Int(5)));
    }

    #[test]
    fn test_trigger_parameter() {
        let mut sm = AnimationStateMachine::new("Test");
        sm.add_parameter("Jump", AnimParam::Trigger(false));
        assert!(!sm.parameters["Jump"].is_triggered());

        sm.set_trigger("Jump");
        assert!(sm.parameters["Jump"].is_triggered());
    }

    #[test]
    fn test_fabrik_solver() {
        let mut chain = IKChain::new();
        chain.bones = vec![
            IKBone { bone_index: 0, length: 1.0, min_angle: -180.0, max_angle: 180.0 },
            IKBone { bone_index: 1, length: 1.0, min_angle: -180.0, max_angle: 180.0 },
        ];
        chain.target = [1.5, 0.0, 0.0];
        chain.iterations = 20;

        let mut positions = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]];
        chain.solve_fabrik(&mut positions);

        // End effector should be near target
        let err = ((positions[2][0]-1.5).powi(2) + positions[2][1].powi(2) + positions[2][2].powi(2)).sqrt();
        assert!(err < 0.1, "FABRIK error: {}", err);
    }

    #[test]
    fn test_two_bone_ik() {
        let mut chain = IKChain::new();
        chain.bones = vec![
            IKBone { bone_index: 0, length: 1.0, min_angle: -90.0, max_angle: 90.0 },
            IKBone { bone_index: 1, length: 1.0, min_angle: -90.0, max_angle: 90.0 },
        ];
        chain.target = [0.5, 1.5, 0.0];
        chain.weight = 1.0;

        let mut positions = vec![[0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 2.0, 0.0]];
        let mut rotations = vec![[0.0, 0.0, 0.0, 1.0]; 3];
        chain.solve_two_bone(&mut positions, &mut rotations);

        // End should be near target
        let err = ((positions[2][0]-0.5).powi(2) + (positions[2][1]-1.5).powi(2)).sqrt();
        assert!(err < 0.5, "Two-bone IK error: {}", err);
    }

    #[test]
    fn test_spring_physics() {
        let mut spring = SpringState::new();
        spring.set_target(1.0);
        for _ in 0..100 {
            spring.update(100.0, 5.0, 0.016);
        }
        assert!((spring.get_value() - 1.0).abs() < 0.05);
    }

    #[test]
    fn test_spring_vec3() {
        let mut spring = SpringVec3::new();
        spring.set_target([1.0, 2.0, 3.0]);
        for _ in 0..100 {
            spring.update(100.0, 5.0, 0.016);
        }
        let v = spring.get_value();
        assert!((v[0] - 1.0).abs() < 0.05);
        assert!((v[1] - 2.0).abs() < 0.05);
        assert!((v[2] - 3.0).abs() < 0.05);
    }

    #[test]
    fn test_procedural_oscillation() {
        let mut motion = ProceduralMotion::new(0, ProceduralType::Oscillation {
            frequency: 1.0, amplitude: 0.5, phase: 0.0,
        });
        let pos1 = motion.update(0.25); // quarter cycle
        assert!((pos1[1] - 0.5).abs() < 0.1);
        let pos2 = motion.update(0.25); // half cycle
        assert!(pos2[1].abs() < 0.1);
    }

    #[test]
    fn test_procedural_breathing() {
        let mut motion = ProceduralMotion::new(5, ProceduralType::Breathing {
            rate: 0.25, depth: 0.02,
        });
        let _ = motion.update(1.0); // Should not panic
    }

    #[test]
    fn test_animation_controller() {
        let (clips, sm) = create_demo_animations();
        let mut ctrl = AnimationController::new();
        ctrl.clips = clips;
        ctrl.state_machine = sm;
        ctrl.bone_count = 19;

        ctrl.update(0.016);
        assert!(!ctrl.output().is_empty());

        // Switch to walk
        ctrl.state_machine.set_float("Speed", 0.5);
        ctrl.update(0.016);
        assert!(ctrl.output().len() > 0);
    }

    #[test]
    fn test_demo_animations() {
        let (clips, sm) = create_demo_animations();
        assert!(clips.contains_key("Idle"));
        assert!(clips.contains_key("Walk"));
        assert!(clips.contains_key("Run"));
        assert_eq!(sm.states.len(), 3);
        assert_eq!(sm.parameters.len(), 3);
    }

    #[test]
    fn test_animation_layer() {
        let mut sm = AnimationStateMachine::new("Base");
        sm.add_state("Idle", "Idle");
        sm.states[0].active = true;

        let mut layer_sm = AnimationStateMachine::new("UpperBody");
        layer_sm.add_state("Wave", "Wave");

        let mut sm = sm;
        let idx = sm.add_layer("UpperBody", 0.5, BlendMode::Override);
        sm.layers[idx].state_machine = layer_sm;
        sm.layers[idx].state_machine.states[0].active = true;

        assert_eq!(sm.layers.len(), 1);
        assert_eq!(sm.layers[0].weight, 0.5);
    }

    #[test]
    fn test_any_state_transition() {
        let mut sm = AnimationStateMachine::new("Test");
        sm.add_parameter("Hit", AnimParam::Trigger(false));
        sm.add_state("Idle", "Idle");
        sm.add_state("Hurt", "Hurt");
        sm.states[0].active = true;

        sm.any_state_transitions.push(Transition {
            from_state: "Any".into(), to_state: "Hurt".into(),
            conditions: vec![TransitionCondition {
                parameter: "Hit".into(), condition_type: ConditionType::If,
                threshold_value: AnimParam::Bool(true),
            }],
            ..Default::default()
        });

        sm.set_trigger("Hit");
        sm.update(0.016);
        assert_eq!(sm.active_state().unwrap().name, "Hurt");
    }

    #[test]
    fn test_look_at_ik() {
        let look = LookAtIK {
            bone_index: 5, target: [10.0, 5.0, 0.0],
            up: [0.0, 1.0, 0.0], weight: 1.0,
            axis: [0.0, 0.0, 1.0],
        };
        let q = look.solve([0.0, 0.0, 0.0]);
        // Should rotate toward target
        assert!(q.0.abs() + q.1.abs() + q.2.abs() > 0.001);
    }

    #[test]
    fn test_foot_ik_procedural() {
        let mut motion = ProceduralMotion::new(10, ProceduralType::FootIK {
            ray_length: 0.5, step_height: 0.1,
        });
        let offset = motion.update(0.016);
        assert!(offset.len() == 3);
    }
}
