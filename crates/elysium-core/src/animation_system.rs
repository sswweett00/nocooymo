use std::collections::HashMap;
use crate::math::{Vec2, Vec3, Quat, Mat4};
use crate::scheduler::System;
use crate::world::World;

// ============================================================================
// Animasyon kanalı türleri
// ============================================================================

/// Animasyon kanalı türü
#[derive(Debug, Clone, PartialEq)]
pub enum AnimationChannelType {
    Translation,
    Rotation,
    Scale,
    MorphTarget,
    Float,
    Color,
    Custom(String),
}

// ============================================================================
// İnterpolasyon türleri
// ============================================================================

/// İnterpolasyon türü
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InterpolationType {
    Linear,
    Step,
    Bezier,
    Hermite,
}

// ============================================================================
// Döngü modları
// ============================================================================

/// Animasyon döngü modu
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoopMode {
    /// Klamp: son karedə saxla
    Clamp,
    /// Döngü: sonsuz döngü
    Loop,
    /// Ping-pong: iləri-geri
    PingPong,
}

impl Default for LoopMode {
    fn default() -> Self {
        Self::Clamp
    }
}

// ============================================================================
// Anahtar kare
// ============================================================================

/// Bezier kontrol nöqtələri
#[derive(Debug, Clone, Copy)]
pub struct BezierControlPoints {
    pub in_tangent: f32,
    pub out_tangent: f32,
}

/// Hermite interpolasyon üçün tənzimlər
#[derive(Debug, Clone, Copy)]
pub struct HermiteTangents {
    pub in_tangent: f32,
    pub out_tangent: f32,
}

/// Anahtar kare
#[derive(Debug, Clone)]
pub struct Keyframe {
    pub time: f32,
    pub value: AnimationValue,
    pub interpolation: InterpolationType,
    pub bezier_controls: Option<BezierControlPoints>,
    pub hermite_tangents: Option<HermiteTangents>,
}

impl Keyframe {
    pub fn new(time: f32, value: AnimationValue) -> Self {
        Self {
            time,
            value,
            interpolation: InterpolationType::Linear,
            bezier_controls: None,
            hermite_tangents: None,
        }
    }

    pub fn with_interpolation(mut self, interpolation: InterpolationType) -> Self {
        self.interpolation = interpolation;
        self
    }
}

#[derive(Debug, Clone)]
pub enum AnimationValue {
    Vec3(Vec3),
    Quat(Quat),
    Float(f32),
    Color([f32; 4]),
}

impl AnimationValue {
    pub fn as_vec3(&self) -> Option<Vec3> {
        match self {
            AnimationValue::Vec3(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_quat(&self) -> Option<Quat> {
        match self {
            AnimationValue::Quat(q) => Some(*q),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f32> {
        match self {
            AnimationValue::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_color(&self) -> Option<[f32; 4]> {
        match self {
            AnimationValue::Color(c) => Some(*c),
            _ => None,
        }
    }
}

// ============================================================================
// Animasyon kanalı
// ============================================================================

/// Animasyon kanalı
#[derive(Debug, Clone)]
pub struct AnimationChannel {
    pub channel_type: AnimationChannelType,
    pub target_property: String,
    pub keyframes: Vec<Keyframe>,
}

impl AnimationChannel {
    pub fn new(channel_type: AnimationChannelType, target_property: String) -> Self {
        Self {
            channel_type,
            target_property,
            keyframes: Vec::new(),
        }
    }

    pub fn push_keyframe(&mut self, keyframe: Keyframe) {
        self.keyframes.push(keyframe);
        self.keyframes.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    }

    /// Zaman verilən dəyər üçün interpolasiya et
    pub fn sample(&self, time: f32, loop_mode: LoopMode) -> Option<AnimationValue> {
        if self.keyframes.is_empty() {
            return None;
        }

        let mut t = time;
        if let Some(last) = self.keyframes.last() {
            match loop_mode {
                LoopMode::Loop => {
                    if last.time > 0.0 {
                        t = time % last.time;
                    }
                }
                LoopMode::PingPong => {
                    if last.time > 0.0 {
                        let cycle = last.time * 2.0;
                        let pos = time % cycle;
                        t = if pos < last.time { pos } else { cycle - pos };
                    }
                }
                LoopMode::Clamp => {
                    if t > last.time {
                        t = last.time;
                    }
                }
            }
        }

        // Uyğun əsas kadrları tap
        let mut prev_idx = None;
        let mut next_idx = None;

        for (i, kf) in self.keyframes.iter().enumerate() {
            if kf.time <= t {
                prev_idx = Some(i);
            } else {
                next_idx = Some(i);
                break;
            }
        }

        match (prev_idx, next_idx) {
            (Some(p), Some(n)) => {
                let prev = &self.keyframes[p];
                let next = &self.keyframes[n];
                let local_t = (t - prev.time) / (next.time - prev.time).max(f32::EPSILON);
                self.interpolate_between(prev, next, local_t)
            }
            (Some(p), None) => Some(self.keyframes[p].value.clone()),
            (None, Some(_)) => Some(self.keyframes[0].value.clone()),
            (None, None) => None,
        }
    }

    fn interpolate_between(
        &self,
        a: &Keyframe,
        b: &Keyframe,
        t: f32,
    ) -> Option<AnimationValue> {
        match a.interpolation {
            InterpolationType::Step => Some(a.value.clone()),
            InterpolationType::Linear => self.interpolate_linear(&a.value, &b.value, t),
            InterpolationType::Bezier => self.interpolate_bezier(&a.value, &b.value, t, a.bezier_controls, b.bezier_controls),
            InterpolationType::Hermite => self.interpolate_hermite(&a.value, &b.value, t, a.hermite_tangents, b.hermite_tangents),
        }
    }

    fn interpolate_linear(&self, a: &AnimationValue, b: &AnimationValue, t: f32) -> Option<AnimationValue> {
        match (a, b) {
            (AnimationValue::Vec3(va), AnimationValue::Vec3(vb)) => {
                Some(AnimationValue::Vec3(va.lerp(*vb, t)))
            }
            (AnimationValue::Quat(qa), AnimationValue::Quat(qb)) => {
                Some(AnimationValue::Quat(qa.slerp(*qb, t)))
            }
            (AnimationValue::Float(fa), AnimationValue::Float(fb)) => {
                Some(AnimationValue::Float(fa + (fb - fa) * t))
            }
            (AnimationValue::Color(ca), AnimationValue::Color(cb)) => {
                let r = ca[0] + (cb[0] - ca[0]) * t;
                let g = ca[1] + (cb[1] - ca[1]) * t;
                let b = ca[2] + (cb[2] - ca[2]) * t;
                let a = ca[3] + (cb[3] - ca[3]) * t;
                Some(AnimationValue::Color([r, g, b, a]))
            }
            _ => None,
        }
    }

    fn interpolate_bezier(
        &self,
        a: &AnimationValue,
        b: &AnimationValue,
        t: f32,
        _in_cp: Option<BezierControlPoints>,
        _out_cp: Option<BezierControlPoints>,
    ) -> Option<AnimationValue> {
        // Basit bezier yaxınlaşdırması: həmişə lineara düşür
        self.interpolate_linear(a, b, t)
    }

    fn interpolate_hermite(
        &self,
        a: &AnimationValue,
        b: &AnimationValue,
        t: f32,
        _t_in: Option<HermiteTangents>,
        _t_out: Option<HermiteTangents>,
    ) -> Option<AnimationValue> {
        // Basit Hermite yaxınlaşdırması: həmişə lineara düşür
        self.interpolate_linear(a, b, t)
    }
}

// ============================================================================
// Animasyon klibi
// ============================================================================

/// Animasyon hadisəsi
#[derive(Debug, Clone)]
pub struct AnimationEvent {
    pub time: f32,
    pub event_type: String,
    pub data: HashMap<String, String>,
}

/// Animasyon klip
#[derive(Debug, Clone)]
pub struct AnimationClip {
    pub name: String,
    pub channels: Vec<AnimationChannel>,
    pub duration: f32,
    pub fps: f32,
    pub loop_count: i32,
    pub loop_mode: LoopMode,
    pub events: Vec<AnimationEvent>,
}

impl AnimationClip {
    pub fn new(name: String) -> Self {
        Self {
            name,
            channels: Vec::new(),
            duration: 0.0,
            fps: 30.0,
            loop_count: 0,
            loop_mode: LoopMode::Clamp,
            events: Vec::new(),
        }
    }

    pub fn add_channel(&mut self, channel: AnimationChannel) {
        self.channels.push(channel);

        if let Some(last_kf) = self.channels
            .iter()
            .flat_map(|ch| &ch.keyframes)
            .max_by(|a, b| a.time.partial_cmp(&b.time).unwrap())
        {
            self.duration = last_kf.time;
        }
    }

    pub fn add_event(&mut self, event: AnimationEvent) {
        self.events.push(event);
        self.events.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    }

    /// Klibi örnekle
    pub fn sample(&self, time: f32) -> HashMap<String, AnimationValue> {
        let mut result = HashMap::new();

        for channel in &self.channels {
            if let Some(value) = channel.sample(time, self.loop_mode) {
                result.insert(channel.target_property.clone(), value);
            }
        }

        result
    }

    /// Aktiv hadisələri tap
    pub fn sample_events(&self, prev_time: f32, curr_time: f32) -> Vec<AnimationEvent> {
        let min_t = prev_time.min(curr_time);
        let max_t = prev_time.max(curr_time);

        self.events
            .iter()
            .filter(|e| e.time >= min_t && e.time <= max_t)
            .cloned()
            .collect()
    }

    /// Sıkışdırma: lazımsız əsas kadrları sil
    pub fn compress(&mut self, tolerance: f32) {
        for channel in &mut self.channels {
            if channel.keyframes.len() <= 2 {
                continue;
            }

            let mut reduced = vec![channel.keyframes[0].clone()];
            let mut last = &channel.keyframes[0];

            for kf in channel.keyframes.iter().skip(1) {
                if let (Some(lv), Some(cv)) = (last.value.as_vec3(), kf.value.as_vec3()) {
                    if (lv - cv).length() > tolerance {
                        reduced.push(kf.clone());
                        last = kf;
                    }
                } else if let (Some(lq), Some(cq)) = (last.value.as_quat(), kf.value.as_quat()) {
                    let dot = lq.dot(*cq);
                    if dot < 0.999 {
                        reduced.push(kf.clone());
                        last = kf;
                    }
                } else {
                    reduced.push(kf.clone());
                    last = kf;
                }
            }

            channel.keyframes = reduced;
        }
    }
}

/// Animasyon durumu
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimationState {
    Stopped,
    Playing,
    Paused,
    Blending,
}

// ============================================================================
// Morph Target (Blend Shape)
// ============================================================================

/// Morph target
#[derive(Debug, Clone)]
pub struct MorphTarget {
    pub name: String,
    pub weight: f32,
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
}

impl Default for MorphTarget {
    fn default() -> Self {
        Self {
            name: String::new(),
            weight: 0.0,
            vertices: Vec::new(),
            normals: Vec::new(),
        }
    }
}

/// Morph target kanalı
#[derive(Debug, Clone)]
pub struct MorphTargetChannel {
    pub target_name: String,
    pub keyframes: Vec<Keyframe>,
}

impl MorphTargetChannel {
    pub fn new(target_name: String) -> Self {
        Self {
            target_name,
            keyframes: Vec::new(),
        }
    }

    pub fn sample(&self, time: f32) -> f32 {
        if self.keyframes.is_empty() {
            return 0.0;
        }

        let mut prev = None;
        let mut next = None;

        for (i, kf) in self.keyframes.iter().enumerate() {
            if kf.time <= time {
                prev = Some(i);
            } else {
                next = Some(i);
                break;
            }
        }

        match (prev, next) {
            (Some(p), Some(n)) => {
                let a = &self.keyframes[p];
                let b = &self.keyframes[n];
                let t = (time - a.time) / (b.time - a.time).max(f32::EPSILON);
                if let (AnimationValue::Float(fa), AnimationValue::Float(fb)) = (&a.value, &b.value) {
                    fa + (fb - fa) * t
                } else {
                    0.0
                }
            }
            (Some(p), None) => {
                if let AnimationValue::Float(f) = &self.keyframes[p].value {
                    *f
                } else {
                    0.0
                }
            }
            _ => 0.0,
        }
    }
}

/// Morph target animasyon klipi
#[derive(Debug, Clone)]
pub struct MorphTargetClip {
    pub name: String,
    pub channels: Vec<MorphTargetChannel>,
    pub duration: f32,
}

impl MorphTargetClip {
    pub fn new(name: String) -> Self {
        Self {
            name,
            channels: Vec::new(),
            duration: 0.0,
        }
    }

    pub fn sample(&self, time: f32) -> HashMap<String, f32> {
        let mut result = HashMap::new();
        for ch in &self.channels {
            result.insert(ch.target_name.clone(), ch.sample(time));
        }
        result
    }
}

// ============================================================================
// Kök hərəkət (Root Motion)
// ============================================================================

/// Kök hərəkət məlumatları
#[derive(Debug, Clone, Copy)]
pub struct RootMotion {
    pub translation: Vec3,
    pub rotation: Quat,
}

impl Default for RootMotion {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        }
    }
}

impl RootMotion {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_zero(&self) -> bool {
        self.translation == Vec3::ZERO && self.rotation == Quat::IDENTITY
    }

    pub fn reset(&mut self) {
        self.translation = Vec3::ZERO;
        self.rotation = Quat::IDENTITY;
    }
}

/// Kök hərəkət kanalı
#[derive(Debug, Clone)]
pub struct RootMotionChannel {
    pub translation_keyframes: Vec<Keyframe>,
    pub rotation_keyframes: Vec<Keyframe>,
}

impl RootMotionChannel {
    pub fn new() -> Self {
        Self {
            translation_keyframes: Vec::new(),
            rotation_keyframes: Vec::new(),
        }
    }

    pub fn sample(&self, time: f32, loop_mode: LoopMode) -> RootMotion {
        let mut translation = Vec3::ZERO;
        let mut rotation = Quat::IDENTITY;

        if !self.translation_keyframes.is_empty() {
            let ch = AnimationChannel {
                channel_type: AnimationChannelType::Translation,
                target_property: String::new(),
                keyframes: self.translation_keyframes.clone(),
            };
            if let Some(AnimationValue::Vec3(v)) = ch.sample(time, loop_mode) {
                translation = v;
            }
        }

        if !self.rotation_keyframes.is_empty() {
            let ch = AnimationChannel {
                channel_type: AnimationChannelType::Rotation,
                target_property: String::new(),
                keyframes: self.rotation_keyframes.clone(),
            };
            if let Some(AnimationValue::Quat(q)) = ch.sample(time, loop_mode) {
                rotation = q;
            }
        }

        RootMotion { translation, rotation }
    }
}

// ============================================================================
// Animasyon vəziyyət maşını (State Machine)
// ============================================================================

/// Keçid şərti növü
#[derive(Debug, Clone, PartialEq)]
pub enum TransitionConditionType {
    Speed(f32, ComparisonOp),
    Distance(f32, ComparisonOp),
    Trigger(String),
    Bool(String, bool),
    Float(String, f32, ComparisonOp),
    Int(String, i32, ComparisonOp),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ComparisonOp {
    Greater,
    Less,
    Equal,
    GreaterEqual,
    LessEqual,
    NotEqual,
}

impl ComparisonOp {
    pub fn evaluate(&self, lhs: f32, rhs: f32) -> bool {
        match self {
            ComparisonOp::Greater => lhs > rhs,
            ComparisonOp::Less => lhs < rhs,
            ComparisonOp::Equal => (lhs - rhs).abs() < f32::EPSILON,
            ComparisonOp::GreaterEqual => lhs >= rhs,
            ComparisonOp::LessEqual => lhs <= rhs,
            ComparisonOp::NotEqual => (lhs - rhs).abs() >= f32::EPSILON,
        }
    }
}

/// Keçid şərti
#[derive(Debug, Clone)]
pub struct TransitionCondition {
    pub condition_type: TransitionConditionType,
}

impl TransitionCondition {
    pub fn speed(speed: f32, op: ComparisonOp) -> Self {
        Self {
            condition_type: TransitionConditionType::Speed(speed, op),
        }
    }

    pub fn trigger(name: impl Into<String>) -> Self {
        Self {
            condition_type: TransitionConditionType::Trigger(name.into()),
        }
    }

    pub fn bool_param(name: impl Into<String>, value: bool) -> Self {
        Self {
            condition_type: TransitionConditionType::Bool(name.into(), value),
        }
    }

    pub fn evaluate(&self, params: &HashMap<String, AnimationParameterValue>) -> bool {
        match &self.condition_type {
            TransitionConditionType::Speed(val, op) => {
                if let Some(AnimationParameterValue::Float(s)) = params.get("speed") {
                    op.evaluate(*s, *val)
                } else {
                    false
                }
            }
            TransitionConditionType::Trigger(name) => {
                params.get(name).map_or(false, |v| matches!(v, AnimationParameterValue::Bool(true)))
            }
            TransitionConditionType::Bool(name, expected) => {
                params.get(name).map_or(false, |v| {
                    if let AnimationParameterValue::Bool(b) = v {
                        *b == *expected
                    } else {
                        false
                    }
                })
            }
            TransitionConditionType::Float(name, val, op) => {
                params.get(name).map_or(false, |v| {
                    if let AnimationParameterValue::Float(f) = v {
                        op.evaluate(*f, *val)
                    } else {
                        false
                    }
                })
            }
            TransitionConditionType::Distance(dist, op) => {
                params.get("distance").map_or(false, |v| {
                    if let AnimationParameterValue::Float(d) = v {
                        op.evaluate(*d, *dist)
                    } else {
                        false
                    }
                })
            }
            TransitionConditionType::Int(name, val, op) => {
                params.get(name).map_or(false, |v| {
                    if let AnimationParameterValue::Int(i) = v {
                        op.evaluate(*i as f32, *val as f32)
                    } else {
                        false
                    }
                })
            }
        }
    }
}

/// Animasyon parametri dəyəri
#[derive(Debug, Clone, PartialEq)]
pub enum AnimationParameterValue {
    Float(f32),
    Bool(bool),
    Int(i32),
    Trigger(String),
}

/// Keçid (transition)
#[derive(Debug, Clone)]
pub struct Transition {
    pub from_state: String,
    pub to_state: String,
    pub conditions: Vec<TransitionCondition>,
    pub duration: f32,
    pub offset: f32,
    pub has_exit_time: bool,
    pub exit_time: f32,
}

impl Transition {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from_state: from.into(),
            to_state: to.into(),
            conditions: Vec::new(),
            duration: 0.3,
            offset: 0.0,
            has_exit_time: false,
            exit_time: 1.0,
        }
    }

    pub fn with_duration(mut self, duration: f32) -> Self {
        self.duration = duration;
        self
    }

    pub fn with_offset(mut self, offset: f32) -> Self {
        self.offset = offset;
        self
    }

    pub fn with_condition(mut self, condition: TransitionCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    pub fn with_exit_time(mut self, exit_time: f32) -> Self {
        self.has_exit_time = true;
        self.exit_time = exit_time;
        self
    }

    pub fn can_transition(&self, params: &HashMap<String, AnimationParameterValue>, exit_time: f32) -> bool {
        if self.has_exit_time && exit_time >= self.exit_time {
            return true;
        }
        if self.conditions.is_empty() {
            return true;
        }
        self.conditions.iter().all(|c| c.evaluate(params))
    }
}

/// Animator vəziyyəti
#[derive(Debug, Clone)]
pub struct AnimatorState {
    pub name: String,
    pub clip: Option<String>,
    pub speed: f32,
    pub loop_mode: LoopMode,
    pub weight: f32,
    pub is_entry: bool,
    pub is_exit: bool,
    pub sub_state_machine: Option<String>,
}

impl AnimatorState {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            clip: None,
            speed: 1.0,
            loop_mode: LoopMode::Clamp,
            weight: 1.0,
            is_entry: false,
            is_exit: false,
            sub_state_machine: None,
        }
    }

    pub fn with_clip(mut self, clip: impl Into<String>) -> Self {
        self.clip = Some(clip.into());
        self
    }

    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }
}

/// Alt vəziyyət maşını
#[derive(Debug, Clone)]
pub struct SubStateMachine {
    pub name: String,
    pub states: HashMap<String, AnimatorState>,
    pub transitions: Vec<Transition>,
    pub entry_state: Option<String>,
    pub exit_state: Option<String>,
}

impl SubStateMachine {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            states: HashMap::new(),
            transitions: Vec::new(),
            entry_state: None,
            exit_state: None,
        }
    }

    pub fn add_state(&mut self, state: AnimatorState) {
        self.states.insert(state.name.clone(), state);
    }

    pub fn add_transition(&mut self, transition: Transition) {
        self.transitions.push(transition);
    }
}

/// Animasyon vəziyyət maşını (layer)
#[derive(Debug, Clone)]
pub struct AnimationStateMachine {
    pub name: String,
    pub states: HashMap<String, AnimatorState>,
    pub transitions: Vec<Transition>,
    pub parameters: HashMap<String, AnimationParameterValue>,
    pub entry_state: Option<String>,
    pub exit_state: Option<String>,
    pub sub_state_machines: HashMap<String, SubStateMachine>,
    pub current_state: Option<String>,
    pub previous_state: Option<String>,
    pub state_time: f32,
    pub transition_time: f32,
    pub is_transitioning: bool,
}

impl AnimationStateMachine {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            states: HashMap::new(),
            transitions: Vec::new(),
            parameters: HashMap::new(),
            entry_state: None,
            exit_state: None,
            sub_state_machines: HashMap::new(),
            current_state: None,
            previous_state: None,
            state_time: 0.0,
            transition_time: 0.0,
            is_transitioning: false,
        }
    }

    pub fn add_state(&mut self, state: AnimatorState) {
        self.states.insert(state.name.clone(), state);
    }

    pub fn add_transition(&mut self, transition: Transition) {
        self.transitions.push(transition);
    }

    pub fn set_parameter(&mut self, name: impl Into<String>, value: AnimationParameterValue) {
        self.parameters.insert(name.into(), value);
    }

    pub fn get_parameter(&self, name: &str) -> Option<&AnimationParameterValue> {
        self.parameters.get(name)
    }

    pub fn set_trigger(&mut self, name: impl Into<String>) {
        self.parameters.insert(name.into(), AnimationParameterValue::Trigger(name.into().clone()));
    }

    pub fn enter(&mut self, state_name: impl Into<String>) {
        self.previous_state = self.current_state.clone();
        self.current_state = Some(state_name.into());
        self.state_time = 0.0;
        self.transition_time = 0.0;
        self.is_transitioning = false;
    }

    pub fn update(&mut self, delta_time: f32, clips: &HashMap<String, AnimationClip>) {
        self.state_time += delta_time;

        if self.is_transitioning {
            self.transition_time += delta_time;
            if self.transition_time >= self.get_active_transition().map(|t| t.duration).unwrap_or(0.0) {
                self.is_transitioning = false;
                self.transition_time = 0.0;
            }
        }

        // Check for valid transitions
        if let Some(curr) = &self.current_state {
            for trans in &self.transitions {
                if trans.from_state == *curr && trans.can_transition(&self.parameters, self.state_time) {
                    self.previous_state = Some(curr.clone());
                    self.current_state = Some(trans.to_state.clone());
                    self.state_time = 0.0;
                    self.transition_time = 0.0;
                    self.is_transitioning = true;
                    break;
                }
            }
        }
    }

    fn get_active_transition(&self) -> Option<&Transition> {
        self.transitions.iter().find(|t| {
            t.from_state == self.previous_state.as_deref().unwrap_or_default()
                && t.to_state == self.current_state.as_deref().unwrap_or_default()
        })
    }

    pub fn get_current_clip(&self) -> Option<&String> {
        self.current_state.as_ref().and_then(|name| {
            self.states.get(name).and_then(|s| s.clip.as_ref())
        })
    }
}

// ============================================================================
// Blend Trees
// ============================================================================

/// Blend tree növü
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BlendTreeType {
    OneD,
    TwoD,
    Direct,
}

/// Blend tree qovşağı
#[derive(Debug, Clone)]
pub struct BlendTreeNode {
    pub clip: String,
    pub position: Vec2,
    pub weight: f32,
    pub children: Vec<usize>,
}

/// Blend tree
#[derive(Debug, Clone)]
pub struct BlendTree {
    pub name: String,
    pub blend_type: BlendTreeType,
    pub parameters: Vec<String>,
    pub nodes: Vec<BlendTreeNode>,
    pub root_nodes: Vec<usize>,
}

impl BlendTree {
    pub fn new(name: impl Into<String>, blend_type: BlendTreeType) -> Self {
        Self {
            name: name.into(),
            blend_type,
            parameters: Vec::new(),
            nodes: Vec::new(),
            root_nodes: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: BlendTreeNode) -> usize {
        let idx = self.nodes.len();
        self.nodes.push(node);
        idx
    }
}

/// 1D blend nöqtəsi
#[derive(Debug, Clone, Copy)]
pub struct Blend1D {
    pub clip: String,
    pub parameter_value: f32,
}

/// 2D blend nöqtəsi
#[derive(Debug, Clone, Copy)]
pub struct Blend2D {
    pub clip: String,
    pub parameter_x: f32,
    pub parameter_y: f32,
}

// ============================================================================
// IK Solvers
// ============================================================================

/// IK hədəfi
#[derive(Debug, Clone, Copy)]
pub struct IKSolverResult {
    pub position: Vec3,
    pub rotation: Quat,
    pub solved: bool,
}

impl Default for IKSolverResult {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            solved: true,
        }
    }
}

/// İki-kemik IK həlledici (qol, ayaq)
#[derive(Debug, Clone)]
pub struct TwoBoneIK {
    pub upper_bone: String,
    pub lower_bone: String,
    pub end_bone: String,
    pub target: Vec3,
    pub mid_chain_rotation: Quat,
}

impl TwoBoneIK {
    pub fn new(upper: impl Into<String>, lower: impl Into<String>, end: impl Into<String>) -> Self {
        Self {
            upper_bone: upper.into(),
            lower_bone: lower.into(),
            end_bone: end.into(),
            target: Vec3::ZERO,
            mid_chain_rotation: Quat::IDENTITY,
        }
    }

    pub fn solve(&self, upper_length: f32, lower_length: f32, end_pos: Vec3, shoulder_pos: Vec3) -> IKSolverResult {
        let dir = (end_pos - shoulder_pos).normalize();
        let total_dist = (end_pos - shoulder_pos).length();

        let clamped_dist = total_dist.min(upper_length + lower_length - 0.01).max(0.01);

        let cos_angle = (upper_length * upper_length + lower_length * lower_length - clamped_dist * clamped_dist)
            / (2.0 * upper_length * lower_length);
        let angle = cos_angle.clamp(-1.0, 1.0).acos();

        let half_len = (upper_length + (clamped_dist * clamped_dist + upper_length * upper_length - lower_length * lower_length)
            / (2.0 * clamped_dist))
            .max(0.01);

        let mid_pos = shoulder_pos + dir * half_len;

        IKSolverResult {
            position: mid_pos,
            rotation: Quat::IDENTITY,
            solved: true,
        }
    }
}

/// CCD IK (chain IK)
#[derive(Debug, Clone)]
pub struct CCDIK {
    pub bone_chain: Vec<String>,
    pub target: Vec3,
    pub iterations: u32,
    pub tolerance: f32,
}

impl CCDIK {
    pub fn new(bone_chain: Vec<String>) -> Self {
        Self {
            bone_chain,
            target: Vec3::ZERO,
            iterations: 10,
            tolerance: 0.001,
        }
    }

    pub fn solve(&self, positions: &[Vec3], rotations: &mut [Quat]) -> IKSolverResult {
        if positions.len() < 2 {
            return IKSolverResult::default();
        }

        let mut result = IKSolverResult::default();
        let mut pos = positions.to_vec();

        for _ in 0..self.iterations {
            for i in (0..pos.len() - 1).rev() {
                let to_target = (self.target - pos[i]).normalize();
                let to_end = (pos[pos.len() - 1] - pos[i]).normalize();

                if to_target.length_squared() < f32::EPSILON {
                    continue;
                }

                let cos_angle = to_target.dot(to_end).clamp(-1.0, 1.0);
                let angle = cos_angle.acos();

                if angle < self.tolerance {
                    result.solved = true;
                    result.position = pos[pos.len() - 1];
                    return result;
                }

                let axis = to_target.cross(to_end).normalize();
                if axis.length_squared() < f32::EPSILON {
                    continue;
                }

                let rot = Quat::from_axis_angle(axis, angle);
                if i < rotations.len() {
                    rotations[i] = rot * rotations[i];
                }

                for j in i + 1..pos.len() {
                    let offset = pos[j] - pos[i];
                    pos[j] = pos[i] + rot * offset;
                }
            }
        }

        result.position = pos[pos.len() - 1];
        result.rotation = rotations.first().copied().unwrap_or(Quat::IDENTITY);
        result.solved = true;
        result
    }
}

/// FABRIK IK həlledici
#[derive(Debug, Clone)]
pub struct FabrikIK {
    pub bone_chain: Vec<String>,
    pub target: Vec3,
    pub tolerance: f32,
    pub max_iterations: u32,
}

impl FabrikIK {
    pub fn new(bone_chain: Vec<String>) -> Self {
        Self {
            bone_chain,
            target: Vec3::ZERO,
            tolerance: 0.001,
            max_iterations: 20,
        }
    }

    pub fn solve(&self, positions: &[Vec3], lengths: &[f32]) -> IKSolverResult {
        if positions.len() < 2 || lengths.len() != positions.len() - 1 {
            return IKSolverResult::default();
        }

        let mut pos = positions.to_vec();
        let total_len: f32 = lengths.iter().sum();
        let base = pos[0];
        let dist_to_target = (self.target - base).length();

        let mut solved = false;
        let mut result = IKSolverResult::default();

        if dist_to_target > total_len {
            for i in 0..pos.len() - 1 {
                let dir = (self.target - pos[i]).normalize();
                pos[i + 1] = pos[i] + dir * lengths[i];
                result.position = pos[pos.len() - 1];
            }
            result.solved = dist_to_target <= total_len + self.tolerance;
            return result;
        }

        let diff_a = (self.target - base).normalize();

        for _ in 0..self.max_iterations {
            // Forward reaching
            pos[pos.len() - 1] = self.target;
            for i in (0..pos.len() - 1).rev() {
                let dir = (pos[i] - pos[i + 1]).normalize();
                pos[i] = pos[i + 1] + dir * lengths[i];
            }

            // Backward reaching
            pos[0] = base;
            for i in 0..pos.len() - 1 {
                let dir = (pos[i + 1] - pos[i]).normalize();
                pos[i + 1] = pos[i] + dir * lengths[i];
            }

            if (pos[pos.len() - 1] - self.target).length_squared() < self.tolerance * self.tolerance {
                solved = true;
                break;
            }
        }

        result.position = pos[pos.len() - 1];
        result.rotation = Quat::IDENTITY;
        result.solved = solved;
        result
    }
}

/// Baxış (Look-at) IK
#[derive(Debug, Clone)]
pub struct LookAtIK {
    pub eye_bone: String,
    pub target: Vec3,
    pub up_vector: Vec3,
    pub clamp_pitch: Option<f32>,
    pub clamp_yaw: Option<f32>,
}

impl LookAtIK {
    pub fn new(eye_bone: impl Into<String>) -> Self {
        Self {
            eye_bone: eye_bone.into(),
            target: Vec3::ZERO,
            up_vector: Vec3::Y,
            clamp_pitch: None,
            clamp_yaw: None,
        }
    }

    pub fn solve(&self, eye_position: Vec3, current_rotation: Quat) -> IKSolverResult {
        let dir = (self.target - eye_position).normalize();

        let mut look_mat = Mat4::look_to_lh(eye_position, dir, self.up_vector);
        let look_quat = Quat::from_mat4(&look_mat);

        let mut result = IKSolverResult::default();
        result.rotation = current_rotation.slerp(look_quat, 1.0);
        result.position = eye_position;
        result.solved = true;
        result
    }
}

/// Uzuv IK (Limb IK)
#[derive(Debug, Clone)]
pub struct LimbIK {
    pub upper_bone: String,
    pub lower_bone: String,
    pub end_bone: String,
    pub target: Vec3,
    pub hint_position: Option<Vec3>,
}

impl LimbIK {
    pub fn new(upper: impl Into<String>, lower: impl Into<String>, end: impl Into<String>) -> Self {
        Self {
            upper_bone: upper.into(),
            lower_bone: lower.into(),
            end_bone: end.into(),
            target: Vec3::ZERO,
            hint_position: None,
        }
    }

    pub fn solve(&self, upper_length: f32, lower_length: f32, shoulder_pos: Vec3) -> IKSolverResult {
        let two_bone = TwoBoneIK::new(&self.upper_bone, &self.lower_bone, &self.end_bone);
        two_bone.solve(upper_length, lower_length, self.target, shoulder_pos)
    }
}

/// Tam vücut IK
#[derive(Debug, Clone)]
pub struct FullBodyIK {
    pub fabrik: FabrikIK,
    pub constraints: Vec<IKConstraint>,
    pub root_bone: String,
}

impl FullBodyIK {
    pub fn new(root_bone: impl Into<String>) -> Self {
        Self {
            fabrik: FabrikIK::new(Vec::new()),
            constraints: Vec::new(),
            root_bone: root_bone.into(),
        }
    }

    pub fn add_constraint(&mut self, constraint: IKConstraint) {
        self.constraints.push(constraint);
    }
}

/// IK məhdudiyyəti
#[derive(Debug, Clone)]
pub enum IKConstraint {
    /// Birlik məhdudiyyəti
    Point {
        bone: String,
        position: Vec3,
        weight: f32,
    },
    /// Rotation məhdudiyyəti
    Rotation {
        bone: String,
        rotation: Quat,
        weight: f32,
    },
    /// Hərəkət məhdudiyyəti
    Limit {
        bone: String,
        min_rotation: Quat,
        max_rotation: Quat,
    },
}

// ============================================================================
// Retargeting
// ============================================================================

/// Kemik xəritələşdirməsi
#[derive(Debug, Clone)]
pub struct BoneMapping {
    pub source_bone: String,
    pub target_bone: String,
    pub rotation_offset: Quat,
    pub translation_offset: Vec3,
    pub scale_factor: f32,
}

impl BoneMapping {
    pub fn new(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            source_bone: source.into(),
            target_bone: target.into(),
            rotation_offset: Quat::IDENTITY,
            translation_offset: Vec3::ZERO,
            scale_factor: 1.0,
        }
    }
}

/// İskelet yenidən nəql
#[derive(Debug, Clone)]
pub struct SkeletonRetargeting {
    pub source_skeleton: String,
    pub target_skeleton: String,
    pub bone_mappings: Vec<BoneMapping>,
    pub t_pose_source: HashMap<String, Mat4>,
    pub t_pose_target: HashMap<String, Mat4>,
}

impl SkeletonRetargeting {
    pub fn new(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            source_skeleton: source.into(),
            target_skeleton: target.into(),
            bone_mappings: Vec::new(),
            t_pose_source: HashMap::new(),
            t_pose_target: HashMap::new(),
        }
    }

    pub fn add_bone_mapping(&mut self, mapping: BoneMapping) {
        self.bone_mappings.push(mapping);
    }

    pub fn set_t_pose_source(&mut self, bone_name: impl Into<String>, transform: Mat4) {
        self.t_pose_source.insert(bone_name.into(), transform);
    }

    pub fn set_t_pose_target(&mut self, bone_name: impl Into<String>, transform: Mat4) {
        self.t_pose_target.insert(bone_name.into(), transform);
    }
}

// ============================================================================
// Animasyon komponentləri (ECS)
// ============================================================================

/// Animasiya oynatıcı komponenti
#[derive(Debug, Clone)]
pub struct AnimationPlayer {
    pub current_clip: Option<String>,
    pub state: AnimationState,
    pub current_time: f32,
    pub speed: f32,
    pub weight: f32,
    pub loop_count: i32,
    pub played_loops: i32,
    pub layer: u32,
}

impl Default for AnimationPlayer {
    fn default() -> Self {
        Self {
            current_clip: None,
            state: AnimationState::Stopped,
            current_time: 0.0,
            speed: 1.0,
            weight: 1.0,
            loop_count: 0,
            played_loops: 0,
            layer: 0,
        }
    }
}

impl AnimationPlayer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn play_clip(&mut self, clip_name: impl Into<String>) {
        self.current_clip = Some(clip_name.into());
        self.state = AnimationState::Playing;
        self.current_time = 0.0;
        self.played_loops = 0;
    }

    pub fn stop(&mut self) {
        self.state = AnimationState::Stopped;
        self.current_time = 0.0;
    }

    pub fn is_playing(&self) -> bool {
        matches!(self.state, AnimationState::Playing)
    }
}

/// Animator komponenti (state machine)
#[derive(Debug, Clone)]
pub struct Animator {
    pub state_machine: AnimationStateMachine,
    pub blend_trees: HashMap<String, BlendTree>,
    pub apply_root_motion: bool,
    pub root_motion: RootMotion,
}

impl Default for Animator {
    fn default() -> Self {
        Self {
            state_machine: AnimationStateMachine::new("default"),
            blend_trees: HashMap::new(),
            apply_root_motion: false,
            root_motion: RootMotion::new(),
        }
    }
}

impl Animator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn play(&mut self, state_name: impl Into<String>) {
        self.state_machine.enter(state_name);
    }

    pub fn set_trigger(&mut self, name: impl Into<String>) {
        self.state_machine.set_trigger(name);
    }

    pub fn set_bool(&mut self, name: impl Into<String>, value: bool) {
        self.state_machine.set_parameter(name, AnimationParameterValue::Bool(value));
    }

    pub fn set_float(&mut self, name: impl Into<String>, value: f32) {
        self.state_machine.set_parameter(name, AnimationParameterValue::Float(value));
    }
}

/// IK hədəfi komponenti
#[derive(Debug, Clone)]
pub struct IKGoal {
    pub goal_name: String,
    pub position: Vec3,
    pub rotation: Quat,
    pub weight: f32,
    pub solver_type: String,
}

impl Default for IKGoal {
    fn default() -> Self {
        Self {
            goal_name: String::new(),
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            weight: 1.0,
            solver_type: String::new(),
        }
    }
}

impl IKGoal {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            goal_name: name.into(),
            ..Default::default()
        }
    }

    pub fn with_position(mut self, position: Vec3) -> Self {
        self.position = position;
        self
    }

    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight.clamp(0.0, 1.0);
        self
    }
}

// ============================================================================
// Animasiya sistemi (ECS System trait)
// ============================================================================

/// Animasiya sistemi — ECS System traitini reallaşdırır
pub struct AnimationSystem {
    pub clips: HashMap<String, AnimationClip>,
    pub skeletal_clips: HashMap<String, SkeletalAnimationClip>,
    pub skeletal_channels: HashMap<String, Vec<SkeletalAnimationChannel>>,
    pub players: HashMap<String, AnimationPlayer>,
    pub blenders: HashMap<String, AnimationBlender>,
    pub skeletons: HashMap<String, Skeleton>,
    pub active_animations: HashMap<String, String>,
}

impl Default for AnimationSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimationSystem {
    pub fn new() -> Self {
        Self {
            clips: HashMap::new(),
            skeletal_clips: HashMap::new(),
            skeletal_channels: HashMap::new(),
            players: HashMap::new(),
            blenders: HashMap::new(),
            skeletons: HashMap::new(),
            active_animations: HashMap::new(),
        }
    }

    pub fn add_clip(&mut self, clip: AnimationClip) {
        self.clips.insert(clip.name.clone(), clip);
    }

    pub fn add_skeletal_clip(&mut self, clip: SkeletalAnimationClip) {
        let name = clip.name.clone();
        let channels = clip.channels.clone();
        self.skeletal_channels.insert(name.clone(), channels);
        self.skeletal_clips.insert(name, clip);
    }

    pub fn add_skeleton(&mut self, name: String, skeleton: Skeleton) {
        self.skeletons.insert(name, skeleton);
    }

    pub fn add_player(&mut self, entity_id: String, player: AnimationPlayer) {
        self.players.insert(entity_id, player);
    }

    pub fn play_animation(&mut self, entity_id: &str, clip_name: &str) -> bool {
        if let Some(_clip) = self.clips.get(clip_name) {
            let mut player = AnimationPlayer::new();
            player.play_clip(clip_name);
            self.players.insert(entity_id.to_string(), player);
            self.active_animations.insert(entity_id.to_string(), clip_name.to_string());
            true
        } else {
            false
        }
    }

    pub fn stop_animation(&mut self, entity_id: &str) -> bool {
        self.players.remove(entity_id).is_some()
    }

    pub fn play_skeletal_animation(&mut self, skeleton_name: &str, clip_name: &str) -> bool {
        if let Some(skeleton) = self.skeletons.get_mut(skeleton_name) {
            if let Some(clip) = self.skeletal_clips.get(clip_name) {
                clip.sample_skeleton(skeleton, 0.0);
                self.active_animations.insert(skeleton_name.to_string(), clip_name.to_string());
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn update(&mut self, delta_time: f32) {
        let mut finished = Vec::new();

        for (entity_id, player) in self.players.iter_mut() {
            if let Some(clip) = self.clips.get(player.current_clip.as_deref().unwrap_or_default()) {
                if player.state == AnimationState::Playing {
                    player.current_time += delta_time * player.speed;

                    let mut clip_duration = clip.duration;
                    let loop_mode = clip.loop_mode;

                    match loop_mode {
                        LoopMode::Clamp => {
                            if player.current_time >= clip_duration && clip_duration > 0.0 {
                                if player.loop_count < 0 || player.played_loops < player.loop_count {
                                    player.current_time = 0.0;
                                    player.played_loops += 1;
                                } else {
                                    player.current_time = clip_duration;
                                    finished.push(entity_id.clone());
                                }
                            }
                        }
                        LoopMode::Loop => {
                            if clip_duration > 0.0 {
                                player.current_time = player.current_time % clip_duration;
                            }
                        }
                        LoopMode::PingPong => {
                            if clip_duration > 0.0 {
                                let cycle = clip_duration * 2.0;
                                player.current_time = player.current_time % cycle;
                                if player.current_time > clip_duration {
                                    player.current_time = cycle - player.current_time;
                                }
                            }
                        }
                    }
                }
            }
        }

        for entity_id in finished {
            self.players.remove(&entity_id);
            self.active_animations.remove(&entity_id);
        }
    }

    pub fn get_animation_state(&self, entity_id: &str) -> Option<AnimationState> {
        self.players.get(entity_id).map(|p| p.state)
    }

    pub fn get_normalized_time(&self, entity_id: &str) -> Option<f32> {
        if let Some(player) = self.players.get(entity_id) {
            if let Some(clip) = self.clips.get(player.current_clip.as_deref()?) {
                Some(player.current_time / clip.duration)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn get_animation_duration(&self, clip_name: &str) -> Option<f32> {
        self.clips.get(clip_name).map(|clip| clip.duration)
    }
}

impl System for AnimationSystem {
    fn name(&self) -> &str {
        "animation_system"
    }

    fn update(&mut self, world: &mut World, dt: f32) {
        self.update(dt);

        // ECS komponentlərini yenilə
        let player_storage = world.components.storage::<AnimationPlayer>();
        let animator_storage = world.components.storage::<Animator>();

        // AnimationPlayer komponentlərini yenilə
        if let Some(storage) = player_storage {
            for (entity, player) in storage.iter_mut() {
                if let Some(clip) = self.clips.get(player.current_clip.as_deref().unwrap_or_default()) {
                    if player.state == AnimationState::Playing {
                        player.current_time += dt * player.speed;
                        player.current_time = player.current_time.min(clip.duration);
                    }
                }
            }
        }

        // Animator komponentlərini yenilə
        if let Some(storage) = animator_storage {
            for (_, animator) in storage.iter_mut() {
                animator.state_machine.update(dt, &self.clips);
            }
        }
    }
}

// ============================================================================
// Animasiya karıştırıcı (Blender)
// ============================================================================

/// Animasyon karıştırıcı (blender)
#[derive(Debug, Clone)]
pub struct AnimationBlender {
    pub players: Vec<AnimationPlayer>,
    pub transition_duration: f32,
    pub current_transition_time: f32,
    pub transitioning: bool,
}

impl AnimationBlender {
    pub fn new() -> Self {
        Self {
            players: Vec::new(),
            transition_duration: 0.3,
            current_transition_time: 0.0,
            transitioning: false,
        }
    }

    pub fn add_player(&mut self, player: AnimationPlayer) {
        self.players.push(player);
    }

    pub fn blend_to(&mut self, target_clip: &str, blend: bool) {
        let target_index = self.players.iter().position(|p| {
            p.current_clip.as_deref() == Some(target_clip)
        });

        if let Some(index) = target_index {
            if blend && self.transitioning {
                for i in 0..self.players.len() {
                    if i != index && self.players[i].weight > 0.0 {
                        self.players[i].weight = (self.players[i].weight - 0.1).max(0.0);
                    }
                }
            }

            self.players[index].play_clip(target_clip);
            self.players[index].weight = 1.0;
            self.transitioning = blend;
            self.current_transition_time = 0.0;
        }
    }

    pub fn update(&mut self, clips: &HashMap<String, AnimationClip>, delta_time: f32) {
        for player in &mut self.players {
            if let Some(clip) = clips.get(player.current_clip.as_deref().unwrap_or_default()) {
                if player.state == AnimationState::Playing {
                    player.current_time += delta_time * player.speed;
                }
            }
        }

        if self.transitioning {
            self.current_transition_time += delta_time;
            if self.current_transition_time >= self.transition_duration {
                self.transitioning = false;
                self.current_transition_time = 0.0;
            }
        }
    }
}

// ============================================================================
// Skeletal Animasiya
// ============================================================================

/// Skeletal animasyon kanalı
#[derive(Debug, Clone)]
pub struct SkeletalAnimationChannel {
    pub bone_name: String,
    pub translation_channel: Option<AnimationChannel>,
    pub rotation_channel: Option<AnimationChannel>,
    pub scale_channel: Option<AnimationChannel>,
}

/// Skeletal animasyon klip
#[derive(Debug, Clone)]
pub struct SkeletalAnimationClip {
    pub name: String,
    pub channels: Vec<SkeletalAnimationChannel>,
    pub duration: f32,
    pub fps: f32,
    pub root_motion: RootMotionChannel,
}

impl SkeletalAnimationClip {
    pub fn sample_skeleton(&self, skeleton: &mut Skeleton, time: f32) {
        for channel in &self.channels {
            let bone_id = match skeleton.bone_map.get(&channel.bone_name) {
                Some(id) => *id,
                None => continue,
            };

            if let Some(transform) = skeleton.bones.get_mut(bone_id as usize) {
                if let Some(ref trans_channel) = channel.translation_channel {
                    if let Some(AnimationValue::Vec3(pos)) = trans_channel.sample(time, LoopMode::Loop) {
                        let trans_mat = Mat4::from_translation(pos);
                        transform.local_transform = trans_mat * transform.local_transform;
                    }
                }

                if let Some(ref rot_channel) = channel.rotation_channel {
                    if let Some(AnimationValue::Quat(rot)) = rot_channel.sample(time, LoopMode::Loop) {
                        let rot_mat = Mat4::from_quat(rot);
                        transform.local_transform = rot_mat * transform.local_transform;
                    }
                }

                if let Some(ref scale_channel) = channel.scale_channel {
                    if let Some(AnimationValue::Vec3(scale)) = scale_channel.sample(time, LoopMode::Loop) {
                        let scale_mat = Mat4::from_scale(scale);
                        transform.local_transform = scale_mat * transform.local_transform;
                    }
                }
            }
        }

        skeleton.update_bone_transforms();
    }

    pub fn sample_root_motion(&self, time: f32) -> RootMotion {
        self.root_motion.sample(time, LoopMode::Loop)
    }
}

// ============================================================================
// Kemik və İskelet
// ============================================================================

/// Skeletal animasyon üçün kemik
#[derive(Debug, Clone)]
pub struct Bone {
    pub id: u32,
    pub name: String,
    pub parent_id: Option<u32>,
    pub local_transform: Mat4,
    pub global_transform: Mat4,
    pub inverse_bind_pose: Mat4,
}

/// Skeletal animasyon üçün iskelet
#[derive(Debug, Clone)]
pub struct Skeleton {
    pub bones: Vec<Bone>,
    pub bone_map: HashMap<String, u32>,
}

impl Skeleton {
    pub fn new() -> Self {
        Self {
            bones: Vec::new(),
            bone_map: HashMap::new(),
        }
    }

    pub fn add_bone(&mut self, name: String, parent_name: Option<String>, local_transform: Mat4, inverse_bind_pose: Mat4) -> u32 {
        let parent_id = parent_name.and_then(|pn| self.bone_map.get(&pn).copied());
        let bone_id = self.bones.len() as u32;

        let bone = Bone {
            id: bone_id,
            name: name.clone(),
            parent_id,
            local_transform,
            global_transform: local_transform,
            inverse_bind_pose,
        };

        self.bones.push(bone);
        self.bone_map.insert(name, bone_id);
        bone_id
    }

    pub fn update_bone_transforms(&mut self) {
        for i in 0..self.bones.len() {
            let parent_id = self.bones[i].parent_id;
            let local_transform = self.bones[i].local_transform;

            if let Some(parent_id) = parent_id {
                self.bones[i].global_transform = self.bones[parent_id as usize].global_transform * local_transform;
            } else {
                self.bones[i].global_transform = local_transform;
            }
        }
    }

    pub fn get_bone_transforms(&self) -> Vec<Mat4> {
        self.bones.iter()
            .map(|bone| bone.global_transform * bone.inverse_bind_pose)
            .collect()
    }

    pub fn get_bone_by_name(&self, name: &str) -> Option<&Bone> {
        let id = self.bone_map.get(name)?;
        self.bones.get(*id as usize)
    }

    pub fn get_bone_mut_by_name(&mut self, name: &str) -> Option<&mut Bone> {
        let id = self.bone_map.get(name)?;
        self.bones.get_mut(*id as usize)
    }
}

// ============================================================================
// Köməkçi funksiyalar
// ============================================================================

pub mod helpers {
    use super::*;

    /// Lineer interpolasiya
    pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
        a + (b - a) * t
    }

    /// Catmull-Rom spline interpolasiyası
    pub fn catmull_rom(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
        0.5 * (2.0 * p1 +
            (p2 - p0) * t +
            (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t * t +
            (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t * t * t)
    }

    /// Ease-in funksiyası
    pub fn ease_in(t: f32) -> f32 {
        t * t
    }

    /// Ease-out funksiyası
    pub fn ease_out(t: f32) -> f32 {
        1.0 - (1.0 - t) * (1.0 - t)
    }

    /// Ease-in-out funksiyası
    pub fn ease_in_out(t: f32) -> f32 {
        if t < 0.5 {
            2.0 * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
        }
    }
}

// ============================================================================
// Presetlər
// ============================================================================

pub mod presets {
    use super::*;

    /// Fade in/out animasiya kanalı yarat
    pub fn fade_animation(target_property: String, duration: f32) -> AnimationChannel {
        let mut channel = AnimationChannel::new(AnimationChannelType::Custom("alpha".to_string()), target_property);

        channel.keyframes.push(Keyframe::new(0.0, AnimationValue::Float(0.0)));
        channel.keyframes.push(Keyframe::new(duration, AnimationValue::Float(1.0)));

        channel
    }

    /// Rotate animasiya kanalı yarat
    pub fn rotate_animation(target_property: String, start_rotation: f32, end_rotation: f32, duration: f32) -> AnimationChannel {
        let mut channel = AnimationChannel::new(AnimationChannelType::Rotation, target_property);

        channel.keyframes.push(Keyframe::new(0.0, AnimationValue::Float(start_rotation)));
        channel.keyframes.push(Keyframe::new(duration, AnimationValue::Float(end_rotation)));

        channel
    }

    /// Scale animasiya kanalı yarat
    pub fn scale_animation(target_property: String, start_scale: Vec3, end_scale: Vec3, duration: f32) -> AnimationChannel {
        let mut channel = AnimationChannel::new(AnimationChannelType::Scale, target_property);

        channel.keyframes.push(Keyframe::new(0.0, AnimationValue::Vec3(start_scale)));
        channel.keyframes.push(Keyframe::new(duration, AnimationValue::Vec3(end_scale)));

        channel
    }
}
