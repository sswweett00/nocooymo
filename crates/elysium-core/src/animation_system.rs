use std::collections::HashMap;
use std::time::Duration;
use crate::math::{Vec3, Quat, Mat4};

/// Animasyon kanalı türü
#[derive(Debug, Clone, PartialEq)]
pub enum AnimationChannelType {
    Translation,
    Rotation,
    Scale,
    MorphTarget,
    Custom(String),
}

/// Anahtar kare
#[derive(Debug, Clone)]
pub struct Keyframe {
    pub time: f32,
    pub value: AnimationValue,
    pub interpolation: InterpolationType,
}

#[derive(Debug, Clone)]
pub enum AnimationValue {
    Vec3(Vec3),
    Quat(Quat),
    Float(f32),
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
}

/// İnterpolasyon türü
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InterpolationType {
    Linear,
    Step,
    CubicSpline,
}

/// Animasyon kanalı
#[derive(Debug, Clone)]
pub struct AnimationChannel {
    pub channel_type: AnimationChannelType,
    pub target_property: String, // "transform.translation", "material.color", vb.
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

    /// Zaman verilen değer için interpolasyon yap
    pub fn sample(&self, time: f32) -> Option<AnimationValue> {
        if self.keyframes.is_empty() {
            return None;
        }

        // Zaman döngüselse normalize et
        let mut clamped_time = time;
        if let Some(last_frame) = self.keyframes.last() {
            if clamped_time > last_frame.time {
                clamped_time = last_frame.time;
            }
        }

        // Uygun anahtar kareleri bul
        let mut prev_frame = None;
        let mut next_frame = None;

        for (i, keyframe) in self.keyframes.iter().enumerate() {
            if keyframe.time <= clamped_time {
                prev_frame = Some(i);
            } else if keyframe.time > clamped_time {
                next_frame = Some(i);
                break;
            }
        }

        match (prev_frame, next_frame) {
            (Some(prev_idx), Some(next_idx)) => {
                let prev = &self.keyframes[prev_idx];
                let next = &self.keyframes[next_idx];

                match prev.interpolation {
                    InterpolationType::Step => Some(prev.value.clone()),
                    InterpolationType::Linear => {
                        let t = (clamped_time - prev.time) / (next.time - prev.time);
                        self.interpolate_linear(&prev.value, &next.value, t)
                    }
                    InterpolationType::CubicSpline => {
                        // Basit bir cubic spline interpolasyonu
                        let t = (clamped_time - prev.time) / (next.time - prev.time);
                        self.interpolate_cubic(&prev.value, &next.value, t)
                    }
                }
            }
            (Some(prev_idx), None) => {
                // Son anahtar kareye ulaşmışız
                Some(self.keyframes[prev_idx].value.clone())
            }
            (None, Some(_)) => {
                // Başlangıçtan önceyiz, ilk anahtar kareyi kullan
                Some(self.keyframes[0].value.clone())
            }
            (None, None) => None,
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
            _ => None,
        }
    }

    fn interpolate_cubic(&self, _a: &AnimationValue, _b: &AnimationValue, _t: f32) -> Option<AnimationValue> {
        // Gerçek cubic spline implementasyonu burada olurdu
        // Şimdilik linear olarak uygulayalım
        None
    }
}

/// Animasyon klip
#[derive(Debug, Clone)]
pub struct AnimationClip {
    pub name: String,
    pub channels: Vec<AnimationChannel>,
    pub duration: f32,
    pub fps: f32,
    pub loop_count: i32, // -1 sonsuz döngü
}

impl AnimationClip {
    pub fn new(name: String) -> Self {
        Self {
            name,
            channels: Vec::new(),
            duration: 0.0,
            fps: 30.0,
            loop_count: 0,
        }
    }

    pub fn add_channel(&mut self, channel: AnimationChannel) {
        self.channels.push(channel);
        
        // Süreyi güncelle
        if let Some(last_keyframe) = self.channels
            .iter()
            .flat_map(|ch| &ch.keyframes)
            .max_by(|a, b| a.time.partial_cmp(&b.time).unwrap()) {
            self.duration = last_keyframe.time;
        }
    }

    /// Klipi örnekle
    pub fn sample(&self, time: f32) -> HashMap<String, AnimationValue> {
        let mut result = HashMap::new();

        for channel in &self.channels {
            if let Some(value) = channel.sample(time) {
                result.insert(channel.target_property.clone(), value);
            }
        }

        result
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

/// Animasyon oynatıcı
#[derive(Debug, Clone)]
pub struct AnimationPlayer {
    pub clip: String, // Clip adı
    pub state: AnimationState,
    pub current_time: f32,
    pub speed: f32,
    pub weight: f32, // Karıştırma ağırlığı (0.0 - 1.0)
    pub loop_count: i32,
    pub played_loops: i32,
    pub start_time: Option<std::time::Instant>,
}

impl AnimationPlayer {
    pub fn new(clip: String) -> Self {
        Self {
            clip,
            state: AnimationState::Stopped,
            current_time: 0.0,
            speed: 1.0,
            weight: 1.0,
            loop_count: 0,
            played_loops: 0,
            start_time: None,
        }
    }

    pub fn play(&mut self) {
        self.state = AnimationState::Playing;
        self.current_time = 0.0;
        self.start_time = Some(std::time::Instant::now());
    }

    pub fn pause(&mut self) {
        if self.state == AnimationState::Playing {
            self.state = AnimationState::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.state == AnimationState::Paused {
            self.state = AnimationState::Playing;
            self.start_time = Some(std::time::Instant::now());
        }
    }

    pub fn stop(&mut self) {
        self.state = AnimationState::Stopped;
        self.current_time = 0.0;
        self.start_time = None;
    }

    pub fn update(&mut self, clip_duration: f32, delta_time: f32) -> bool {
        if self.state != AnimationState::Playing {
            return false;
        }

        self.current_time += delta_time * self.speed;

        // Döngü kontrolü
        if self.current_time >= clip_duration && clip_duration > 0.0 {
            if self.loop_count < 0 || self.played_loops < self.loop_count {
                self.current_time = 0.0;
                self.played_loops += 1;
                return true;
            } else {
                self.current_time = clip_duration;
                self.state = AnimationState::Stopped;
                return false;
            }
        }

        true
    }

    pub fn set_time(&mut self, time: f32) {
        self.current_time = time.max(0.0);
    }

    pub fn get_normalized_time(&self, clip_duration: f32) -> f32 {
        if clip_duration > 0.0 {
            (self.current_time % clip_duration) / clip_duration
        } else {
            0.0
        }
    }
}

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

    pub fn play_animation(&mut self, clip_name: &str, blend: bool) {
        let target_index = self.players.iter().position(|p| p.clip == clip_name);
        
        if let Some(index) = target_index {
            if blend && self.transitioning {
                // Mevcut animasyonları azalt
                for i in 0..self.players.len() {
                    if i != index && self.players[i].weight > 0.0 {
                        self.players[i].weight = (self.players[i].weight - 0.1).max(0.0);
                    }
                }
            }
            
            self.players[index].play();
            self.players[index].weight = 1.0;
        }
    }

    pub fn update(&mut self, clips: &HashMap<String, AnimationClip>, delta_time: f32) {
        for player in &mut self.players {
            if let Some(clip) = clips.get(&player.clip) {
                player.update(clip.duration, delta_time);
            }
        }

        // Karıştırma ağırlıklarını yönet
        if self.transitioning {
            self.current_transition_time += delta_time;
            if self.current_transition_time >= self.transition_duration {
                self.transitioning = false;
                self.current_transition_time = 0.0;
            }
        }
    }
}

/// Skeletal animasyon için kemik
#[derive(Debug, Clone)]
pub struct Bone {
    pub id: u32,
    pub name: String,
    pub parent_id: Option<u32>,
    pub local_transform: Mat4,
    pub global_transform: Mat4,
    pub inverse_bind_pose: Mat4,
}

/// Skeletal animasyon için iskelet
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
}

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
}

impl SkeletalAnimationClip {
    pub fn sample_skeleton(&self, skeleton: &mut Skeleton, time: f32) {
        for channel in &self.channels {
            let bone_id = match skeleton.bone_map.get(&channel.bone_name) {
                Some(id) => *id,
                None => continue,
            };

            if let Some(transform) = skeleton.bones.get_mut(bone_id as usize) {
                // Konum interpolasyonu
                if let Some(ref trans_channel) = channel.translation_channel {
                    if let Some(AnimationValue::Vec3(pos)) = trans_channel.sample(time) {
                        transform.local_transform = Mat4::from_translation(pos) * transform.local_transform;
                    }
                }

                // Rotasyon interpolasyonu
                if let Some(ref rot_channel) = channel.rotation_channel {
                    if let Some(AnimationValue::Quat(rot)) = rot_channel.sample(time) {
                        transform.local_transform = Mat4::from_quat(rot) * transform.local_transform;
                    }
                }

                // Ölçek interpolasyonu
                if let Some(ref scale_channel) = channel.scale_channel {
                    if let Some(AnimationValue::Vec3(scale)) = scale_channel.sample(time) {
                        transform.local_transform = Mat4::from_scale(scale) * transform.local_transform;
                    }
                }
            }
        }

        skeleton.update_bone_transforms();
    }
}

/// Animasyon sistemi
pub struct AnimationSystem {
    pub clips: HashMap<String, AnimationClip>,
    pub skeletal_clips: HashMap<String, SkeletalAnimationClip>,
    pub players: HashMap<String, AnimationPlayer>,
    pub blenders: HashMap<String, AnimationBlender>,
    pub skeletons: HashMap<String, Skeleton>,
    pub active_animations: HashMap<String, String>, // entity_id -> clip_name
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
            players: HashMap::new(),
            blenders: HashMap::new(),
            skeletons: HashMap::new(),
            active_animations: HashMap::new(),
        }
    }

    /// Animasyon klip ekle
    pub fn add_clip(&mut self, clip: AnimationClip) {
        self.clips.insert(clip.name.clone(), clip);
    }

    /// Skeletal animasyon klip ekle
    pub fn add_skeletal_clip(&mut self, clip: SkeletalAnimationClip) {
        self.skeletal_clips.insert(clip.name.clone(), clip);
    }

    /// İskelet ekle
    pub fn add_skeleton(&mut self, name: String, skeleton: Skeleton) {
        self.skeletons.insert(name, skeleton);
    }

    /// Animasyon oynatıcı ekle
    pub fn add_player(&mut self, entity_id: String, player: AnimationPlayer) {
        self.players.insert(entity_id, player);
    }

    /// Animasyon başlat
    pub fn play_animation(&mut self, entity_id: &str, clip_name: &str) -> bool {
        if let Some(clip) = self.clips.get(clip_name) {
            let mut player = AnimationPlayer::new(clip_name.to_string());
            player.play();
            self.players.insert(entity_id.to_string(), player);
            self.active_animations.insert(entity_id.to_string(), clip_name.to_string());
            true
        } else {
            false
        }
    }

    /// Skeletal animasyon başlat
    pub fn play_skeletal_animation(&mut self, skeleton_name: &str, clip_name: &str) -> bool {
        if let Some(skeleton) = self.skeletons.get_mut(skeleton_name) {
            if let Some(clip) = self.skeletal_clips.get(clip_name) {
                // Animasyonu uygula
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

    /// Animasyon durdur
    pub fn stop_animation(&mut self, entity_id: &str) -> bool {
        self.players.remove(entity_id).is_some()
    }

    /// Sistemi güncelle
    pub fn update(&mut self, delta_time: f32) {
        // Klasi̇k ani̇masyonları güncelle
        let mut finished_animations = Vec::new();
        
        for (entity_id, player) in self.players.iter_mut() {
            if let Some(clip) = self.clips.get(&player.clip) {
                if !player.update(clip.duration, delta_time) {
                    finished_animations.push(entity_id.clone());
                }
            }
        }

        for entity_id in finished_animations {
            self.players.remove(&entity_id);
            self.active_animations.remove(&entity_id);
        }

        // Skeletal animasyonları güncelle
        for (skeleton_name, clip_name) in self.active_animations.iter() {
            if let Some(skeleton) = self.skeletons.get_mut(skeleton_name) {
                if let Some(clip) = self.skeletal_clips.get(clip_name) {
                    if let Some(player) = self.players.get(skeleton_name) {
                        clip.sample_skeleton(skeleton, player.current_time);
                    }
                }
            }
        }
    }

    /// Animasyon durumu al
    pub fn get_animation_state(&self, entity_id: &str) -> Option<AnimationState> {
        self.players.get(entity_id).map(|p| p.state)
    }

    /// Animasyon süresi al
    pub fn get_animation_duration(&self, clip_name: &str) -> Option<f32> {
        self.clips.get(clip_name).map(|clip| clip.duration)
    }

    /// Normali̇ze edilmiş zaman al
    pub fn get_normalized_time(&self, entity_id: &str) -> Option<f32> {
        if let Some(player) = self.players.get(entity_id) {
            if let Some(clip) = self.clips.get(&player.clip) {
                Some(player.get_normalized_time(clip.duration))
            } else {
                None
            }
        } else {
            None
        }
    }
}

/// Animasyon yardımcı fonksiyonları
pub mod helpers {
    use super::*;

    /// Lineer interpolasyon
    pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
        a + (b - a) * t
    }

    /// Catmull-Rom spline interpolasyonu
    pub fn catmull_rom(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
        0.5 * (2.0 * p1 + 
               (p2 - p0) * t + 
               (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t * t + 
               (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t * t * t)
    }

    /// Ease-in fonksiyonu
    pub fn ease_in(t: f32) -> f32 {
        t * t
    }

    /// Ease-out fonksiyonu
    pub fn ease_out(t: f32) -> f32 {
        1.0 - (1.0 - t) * (1.0 - t)
    }

    /// Ease-in-out fonksiyonu
    pub fn ease_in_out(t: f32) -> f32 {
        if t < 0.5 {
            2.0 * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
        }
    }
}

/// Animasyon preset'leri
pub mod presets {
    use super::*;

    /// Fade in/out animasyon kanalı oluştur
    pub fn fade_animation(target_property: String, duration: f32) -> AnimationChannel {
        let mut channel = AnimationChannel::new(AnimationChannelType::Custom("alpha".to_string()), target_property);
        
        channel.keyframes.push(Keyframe {
            time: 0.0,
            value: AnimationValue::Float(0.0),
            interpolation: InterpolationType::Linear,
        });
        
        channel.keyframes.push(Keyframe {
            time: duration,
            value: AnimationValue::Float(1.0),
            interpolation: InterpolationType::Linear,
        });
        
        channel
    }

    /// Rotate animasyon kanalı oluştur
    pub fn rotate_animation(target_property: String, start_rotation: f32, end_rotation: f32, duration: f32) -> AnimationChannel {
        let mut channel = AnimationChannel::new(AnimationChannelType::Rotation, target_property);
        
        channel.keyframes.push(Keyframe {
            time: 0.0,
            value: AnimationValue::Float(start_rotation),
            interpolation: InterpolationType::Linear,
        });
        
        channel.keyframes.push(Keyframe {
            time: duration,
            value: AnimationValue::Float(end_rotation),
            interpolation: InterpolationType::Linear,
        });
        
        channel
    }

    /// Scale animasyon kanalı oluştur
    pub fn scale_animation(target_property: String, start_scale: Vec3, end_scale: Vec3, duration: f32) -> AnimationChannel {
        let mut channel = AnimationChannel::new(AnimationChannelType::Scale, target_property);
        
        channel.keyframes.push(Keyframe {
            time: 0.0,
            value: AnimationValue::Vec3(start_scale),
            interpolation: InterpolationType::Linear,
        });
        
        channel.keyframes.push(Keyframe {
            time: duration,
            value: AnimationValue::Vec3(end_scale),
            interpolation: InterpolationType::Linear,
        });
        
        channel
    }
}