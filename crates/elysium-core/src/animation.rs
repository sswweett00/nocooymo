use crate::{Component, Transform};
use std::collections::HashMap;
use nalgebra::Vector3;

// Animasyon kanalları için temel enum
#[derive(Debug, Clone, PartialEq)]
pub enum AnimationChannel {
    TranslationX,
    TranslationY,
    TranslationZ,
    RotationX,
    RotationY,
    RotationZ,
    ScaleX,
    ScaleY,
    ScaleZ,
    Custom(String),
}

// Anahtar kare verisi
#[derive(Debug, Clone)]
pub struct Keyframe {
    pub time: f32,
    pub value: f32,
    pub interpolation: InterpolationType,
}

#[derive(Debug, Clone)]
pub enum InterpolationType {
    Linear,
    Step,
    CubicSpline { in_tangent: f32, out_tangent: f32 },
}

// Animasyon kanalı
#[derive(Debug, Clone)]
pub struct AnimationTrack {
    pub channel: AnimationChannel,
    pub keyframes: Vec<Keyframe>,
}

impl AnimationTrack {
    pub fn new(channel: AnimationChannel) -> Self {
        Self {
            channel,
            keyframes: Vec::new(),
        }
    }
    
    pub fn add_keyframe(&mut self, time: f32, value: f32, interpolation: InterpolationType) {
        let keyframe = Keyframe {
            time,
            value,
            interpolation,
        };
        
        // Zaman sırasına göre ekle — aynı zamanda varsa güncelle
        let pos = self.keyframes.binary_search_by(|k| k.time.partial_cmp(&time).unwrap_or(std::cmp::Ordering::Less));
        match pos {
            Ok(idx) => self.keyframes[idx] = keyframe,
            Err(pos) => self.keyframes.insert(pos, keyframe),
        }
    }
    
    pub fn evaluate(&self, time: f32) -> Option<f32> {
        if self.keyframes.is_empty() {
            return None;
        }
        
        // Sadece bir anahtar kare varsa
        if self.keyframes.len() == 1 {
            return Some(self.keyframes[0].value);
        }
        
        // Zaman aralığını bul
        let last_frame = self.keyframes.last()?;
        if time >= last_frame.time {
            return Some(last_frame.value);
        }
        
        let first_frame = self.keyframes.first()?;
        if time <= first_frame.time {
            return Some(first_frame.value);
        }
        
        // İki anahtar kare arasında interpolasyon yap
        for i in 0..self.keyframes.len() - 1 {
            let frame1 = &self.keyframes[i];
            let frame2 = &self.keyframes[i + 1];
            
            if time >= frame1.time && time < frame2.time {
                let t = (time - frame1.time) / (frame2.time - frame1.time);
                
                match &frame1.interpolation {
                    InterpolationType::Linear => {
                        return Some(frame1.value + t * (frame2.value - frame1.value));
                    }
                    InterpolationType::Step => {
                        return Some(frame1.value);
                    }
                    InterpolationType::CubicSpline { out_tangent, .. } => {
                        // Cubic Hermite spline
                        let t2 = t * t;
                        let t3 = t2 * t;
                        
                        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
                        let h10 = t3 - 2.0 * t2 + t;
                        let h01 = -2.0 * t3 + 3.0 * t2;
                        let h11 = t3 - t2;
                        
                        // out_tangent değerini frame2'den almak gerekir
                        let tangent1 = match &frame1.interpolation {
                            InterpolationType::CubicSpline { out_tangent, .. } => *out_tangent,
                            _ => 0.0,
                        };
                        let tangent2 = match &frame2.interpolation {
                            InterpolationType::CubicSpline { in_tangent, .. } => *in_tangent,
                            _ => 0.0,
                        };
                        
                        let dt = frame2.time - frame1.time;
                        let m1 = tangent1 * dt;
                        let m2 = tangent2 * dt;
                        
                        return Some(
                            h00 * frame1.value + 
                            h10 * m1 + 
                            h01 * frame2.value + 
                            h11 * m2
                        );
                    }
                }
            }
        }
        
        None
    }
}

// Animasyon klip verisi
#[derive(Debug, Clone)]
pub struct AnimationClip {
    pub name: String,
    pub tracks: Vec<AnimationTrack>,
    pub duration: f32,
    pub loop_enabled: bool,
}

impl AnimationClip {
    pub fn new(name: String) -> Self {
        Self {
            name,
            tracks: Vec::new(),
            duration: 0.0,
            loop_enabled: false,
        }
    }
    
    pub fn add_track(&mut self, track: AnimationTrack) {
        // Süreyi güncelle — track taşınmadan önce yap
        for keyframe in &track.keyframes {
            if keyframe.time > self.duration {
                self.duration = keyframe.time;
            }
        }
        self.tracks.push(track);
    }
    
    pub fn evaluate(&self, time: f32) -> HashMap<AnimationChannel, f32> {
        let mut values = HashMap::new();
        
        for track in &self.tracks {
            if let Some(value) = track.evaluate(time) {
                values.insert(track.channel.clone(), value);
            }
        }
        
        values
    }
    
    pub fn evaluate_transform(&self, time: f32) -> Option<Transform> {
        let values = self.evaluate(time);
        
        let mut translation = crate::math::Vec3::new(0.0, 0.0, 0.0);
        let mut rotation_euler = crate::math::Vec3::new(0.0, 0.0, 0.0);
        let mut scale = crate::math::Vec3::new(1.0, 1.0, 1.0);
        
        let mut has_any_transform = false;
        
        for (channel, value) in values {
            match channel {
                AnimationChannel::TranslationX => {
                    translation.x = value;
                    has_any_transform = true;
                }
                AnimationChannel::TranslationY => {
                    translation.y = value;
                    has_any_transform = true;
                }
                AnimationChannel::TranslationZ => {
                    translation.z = value;
                    has_any_transform = true;
                }
                AnimationChannel::RotationX => {
                    rotation_euler.x = value;
                    has_any_transform = true;
                }
                AnimationChannel::RotationY => {
                    rotation_euler.y = value;
                    has_any_transform = true;
                }
                AnimationChannel::RotationZ => {
                    rotation_euler.z = value;
                    has_any_transform = true;
                }
                AnimationChannel::ScaleX => {
                    scale.x = value;
                    has_any_transform = true;
                }
                AnimationChannel::ScaleY => {
                    scale.y = value;
                    has_any_transform = true;
                }
                AnimationChannel::ScaleZ => {
                    scale.z = value;
                    has_any_transform = true;
                }
                _ => {}
            }
        }
        
        if has_any_transform {
            let rotation = crate::math::Quat::from_euler(crate::math::EulerRot::XYZ, rotation_euler.x, rotation_euler.y, rotation_euler.z);
            Some(Transform {
                translation,
                rotation,
                scale,
                ..Transform::identity()
            })
        } else {
            None
        }
    }
}

// Skeletal animasyon için kemik verisi
#[derive(Debug, Clone)]
pub struct Bone {
    pub name: String,
    pub parent_index: Option<usize>,
    pub local_transform: Transform,
    pub global_transform: Transform,
    pub inverse_bind_pose: Transform,
}

impl Bone {
    pub fn new(name: String, parent_index: Option<usize>) -> Self {
        Self {
            name,
            parent_index,
            local_transform: Transform::default(),
            global_transform: Transform::default(),
            inverse_bind_pose: Transform::default(),
        }
    }
}

// Skeletal animasyon verisi
#[derive(Debug, Clone)]
pub struct Skeleton {
    pub bones: Vec<Bone>,
    pub root_bones: Vec<usize>,
}

impl Skeleton {
    pub fn new() -> Self {
        Self {
            bones: Vec::new(),
            root_bones: Vec::new(),
        }
    }
    
    pub fn add_bone(&mut self, bone: Bone) -> usize {
        let index = self.bones.len();
        self.bones.push(bone);
        
        // Eğer parent yoksa root bone olarak ekle
        if self.bones[index].parent_index.is_none() {
            self.root_bones.push(index);
        }
        
        index
    }
    
    pub fn calculate_global_transforms(&mut self) {
        for &root_idx in &self.root_bones {
            self.calculate_bone_transform_recursive(root_idx, Transform::default());
        }
    }
    
    fn calculate_bone_transform_recursive(&mut self, bone_idx: usize, parent_transform: Transform) {
        let local_transform = self.bones[bone_idx].local_transform;
        let global_transform = parent_transform * local_transform;
        
        self.bones[bone_idx].global_transform = global_transform;
        
        // Child kemikleri hesapla
        for (idx, bone) in self.bones.iter().enumerate() {
            if bone.parent_index == Some(bone_idx) {
                self.calculate_bone_transform_recursive(idx, global_transform);
            }
        }
    }
    
    pub fn get_bone_index_by_name(&self, name: &str) -> Option<usize> {
        self.bones.iter().position(|bone| bone.name == name)
    }
}

// Skeletal animasyon kanalı
#[derive(Debug, Clone)]
pub struct SkeletalAnimationTrack {
    pub bone_name: String,
    pub translation_track: Option<AnimationTrack>,
    pub rotation_track: Option<AnimationTrack>,
    pub scale_track: Option<AnimationTrack>,
}

impl SkeletalAnimationTrack {
    pub fn new(bone_name: String) -> Self {
        Self {
            bone_name,
            translation_track: None,
            rotation_track: None,
            scale_track: None,
        }
    }
    
    pub fn evaluate(&self, time: f32, skeleton: &Skeleton) -> Option<Transform> {
        let _bone_idx = skeleton.get_bone_index_by_name(&self.bone_name)?;
        // Transform::rotation Quat tipinde — Euler olarak yorumlanamaz, identity koruyoruz
        // Bu track'ler pozisyon offset'i olarak kullanılıyor; tam iskelet animasyonu
        // için ayrı quaternion track gerekir — şimdilik translation/scale uygula
        let mut transform = Transform::default();
        let mut has_any_transform = false;
        
        if let Some(ref track) = self.translation_track {
            if let Some(val) = track.evaluate(time) {
                // Tek değerli track — tüm eksenlere değil, tek kanala yazılmalı
                // Burada translation'in x bileşeni olarak yorumluyoruz (basitleştirilmiş)
                // Gerçek kullanımda TranslationX/Y/Z ayrı track'ler olmalı
                transform.translation.x = val;
                has_any_transform = true;
            }
        }
        
        if let Some(ref track) = self.rotation_track {
            if let Some(angle) = track.evaluate(time) {
                // Rotation track — Y ekseni etrafında açı (radyan)
                transform.rotation = crate::math::Quat::from_rotation_y(angle);
                has_any_transform = true;
            }
        }
        
        if let Some(ref track) = self.scale_track {
            if let Some(s) = track.evaluate(time) {
                transform.scale = crate::math::Vec3::splat(s);
                has_any_transform = true;
            }
        }
        
        if has_any_transform {
            Some(transform)
        } else {
            None
        }
    }
}

// Skeletal animasyon klip
#[derive(Debug, Clone)]
pub struct SkeletalAnimationClip {
    pub name: String,
    pub tracks: Vec<SkeletalAnimationTrack>,
    pub duration: f32,
    pub loop_enabled: bool,
}

impl SkeletalAnimationClip {
    pub fn new(name: String) -> Self {
        Self {
            name,
            tracks: Vec::new(),
            duration: 0.0,
            loop_enabled: false,
        }
    }
    
    pub fn add_track(&mut self, track: SkeletalAnimationTrack) {
        // Süreyi güncelle — taşınmadan önce
        for sub_track in [
            track.translation_track.as_ref(),
            track.rotation_track.as_ref(),
            track.scale_track.as_ref()
        ].iter().flatten() {
            for keyframe in &sub_track.keyframes {
                if keyframe.time > self.duration {
                    self.duration = keyframe.time;
                }
            }
        }
        self.tracks.push(track);
    }
    
    pub fn evaluate_skeleton(&self, skeleton: &mut Skeleton, time: f32) {
        if self.duration <= f32::EPSILON {
            return;
        }
        let clamped_time = if self.loop_enabled {
            time % self.duration
        } else {
            time.min(self.duration)
        };
        
        for track in &self.tracks {
            if let Some(transform) = track.evaluate(clamped_time, skeleton) {
                if let Some(bone_idx) = skeleton.get_bone_index_by_name(&track.bone_name) {
                    skeleton.bones[bone_idx].local_transform = transform;
                }
            }
        }
        
        skeleton.calculate_global_transforms();
    }
}

// Animasyon oynatıcı bileşeni
pub struct AnimationPlayer {
    pub clips: HashMap<String, AnimationClip>,
    pub active_clip: Option<String>,
    pub time: f32,
    pub speed: f32,
    pub weight: f32,
    pub playing: bool,
    pub loop_enabled: bool,
}

impl AnimationPlayer {
    pub fn new() -> Self {
        Self {
            clips: HashMap::new(),
            active_clip: None,
            time: 0.0,
            speed: 1.0,
            weight: 1.0,
            playing: false,
            loop_enabled: true,
        }
    }
    
    pub fn add_clip(&mut self, clip: AnimationClip) {
        self.clips.insert(clip.name.clone(), clip);
    }
    
    pub fn play(&mut self, clip_name: String) {
        if self.clips.contains_key(&clip_name) {
            self.active_clip = Some(clip_name);
            self.time = 0.0;
            self.playing = true;
        }
    }
    
    pub fn pause(&mut self) {
        self.playing = false;
    }
    
    pub fn stop(&mut self) {
        self.playing = false;
        self.time = 0.0;
        self.active_clip = None;
    }
    
    pub fn update(&mut self, delta_time: f32) {
        if !self.playing {
            return;
        }
        
        if let Some(ref clip_name) = self.active_clip {
            if let Some(clip) = self.clips.get(clip_name) {
                self.time += delta_time * self.speed;
                
                if self.time >= clip.duration {
                    if clip.loop_enabled || self.loop_enabled {
                        self.time = 0.0; // Döngü varsa başa sar
                    } else {
                        self.playing = false; // Döngü yoksa durdur
                    }
                }
            }
        }
    }
    
    pub fn get_current_transform(&self) -> Option<Transform> {
        if let Some(ref clip_name) = self.active_clip {
            if let Some(clip) = self.clips.get(clip_name) {
                return clip.evaluate_transform(self.time);
            }
        }
        None
    }
}

// Skeletal animasyon oynatıcı bileşeni
pub struct SkeletalAnimationPlayer {
    pub clips: HashMap<String, SkeletalAnimationClip>,
    pub active_clip: Option<String>,
    pub time: f32,
    pub speed: f32,
    pub playing: bool,
    pub loop_enabled: bool,
}

impl SkeletalAnimationPlayer {
    pub fn new() -> Self {
        Self {
            clips: HashMap::new(),
            active_clip: None,
            time: 0.0,
            speed: 1.0,
            playing: false,
            loop_enabled: true,
        }
    }
    
    pub fn add_clip(&mut self, clip: SkeletalAnimationClip) {
        self.clips.insert(clip.name.clone(), clip);
    }
    
    pub fn play(&mut self, clip_name: String) {
        if self.clips.contains_key(&clip_name) {
            self.active_clip = Some(clip_name);
            self.time = 0.0;
            self.playing = true;
        }
    }
    
    pub fn update(&mut self, delta_time: f32, skeleton: &mut Skeleton) {
        if !self.playing {
            return;
        }
        
        if let Some(ref clip_name) = self.active_clip {
            if let Some(clip) = self.clips.get_mut(clip_name) {
                self.time += delta_time * self.speed;
                
                if self.time >= clip.duration {
                    if clip.loop_enabled || self.loop_enabled {
                        self.time = 0.0;
                    } else {
                        self.playing = false;
                    }
                }
                
                clip.evaluate_skeleton(skeleton, self.time);
            }
        }
    }
}

// Animasyon blend verisi
#[derive(Debug, Clone)]
pub struct AnimationBlend {
    pub clip_a: String,
    pub clip_b: String,
    pub blend_factor: f32, // 0.0 = clip_a, 1.0 = clip_b
}

impl AnimationBlend {
    pub fn new(clip_a: String, clip_b: String) -> Self {
        Self {
            clip_a,
            clip_b,
            blend_factor: 0.0,
        }
    }
    
    pub fn set_blend_factor(&mut self, factor: f32) {
        self.blend_factor = factor.clamp(0.0, 1.0);
    }
    
    pub fn evaluate(&self, clip_a: &AnimationClip, clip_b: &AnimationClip, time: f32) -> Option<Transform> {
        let transform_a = clip_a.evaluate_transform(time)?;
        let transform_b = clip_b.evaluate_transform(time)?;
        Some(transform_a.lerp(&transform_b, self.blend_factor))
    }
}

// Karma animasyon oynatıcı bileşeni
pub struct BlendedAnimationPlayer {
    pub clips: HashMap<String, AnimationClip>,
    pub active_blends: Vec<AnimationBlend>,
    pub time: f32,
    pub speed: f32,
    pub playing: bool,
}

impl BlendedAnimationPlayer {
    pub fn new() -> Self {
        Self {
            clips: HashMap::new(),
            active_blends: Vec::new(),
            time: 0.0,
            speed: 1.0,
            playing: false,
        }
    }
    
    pub fn add_clip(&mut self, clip: AnimationClip) {
        self.clips.insert(clip.name.clone(), clip);
    }
    
    pub fn blend_animation(&mut self, blend: AnimationBlend) {
        self.active_blends.push(blend);
        self.playing = true;
    }
    
    pub fn update(&mut self, delta_time: f32) {
        if !self.playing {
            return;
        }
        
        self.time += delta_time * self.speed;
        
        // Mevcut tüm blend'leri güncelle
        for blend in &mut self.active_blends {
            if let (Some(clip_a), Some(clip_b)) = (self.clips.get(&blend.clip_a), self.clips.get(&blend.clip_b)) {
                // Zamanı her iki klibe de uygula
                // Not: Gerçek implementasyonda bu daha karmaşık olabilir
            }
        }
    }
}