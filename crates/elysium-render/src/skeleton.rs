use elysium_core::math::{Vec2, Vec3, Mat4, Quat};
use std::collections::HashMap;

/// İskelet sistemi için kemik tanımı
#[derive(Debug, Clone)]
pub struct Bone {
    pub id: u32,
    pub name: String,
    pub parent_id: Option<u32>,
    pub local_transform: Mat4,
    pub world_transform: Mat4,
    pub inverse_bind_matrix: Mat4,
}

/// İskelet sistemi
#[derive(Debug, Clone)]
pub struct Skeleton {
    pub bones: Vec<Bone>,
    pub bone_map: HashMap<String, u32>,
    pub root_bones: Vec<u32>,
}

impl Skeleton {
    pub fn new() -> Self {
        Self {
            bones: Vec::new(),
            bone_map: HashMap::new(),
            root_bones: Vec::new(),
        }
    }

    pub fn add_bone(&mut self, name: String, parent_name: Option<String>, local_transform: Mat4, inverse_bind_matrix: Mat4) -> u32 {
        let parent_id = parent_name.and_then(|pname| self.bone_map.get(&pname).copied());
        let bone_id = self.bones.len() as u32;
        
        let bone = Bone {
            id: bone_id,
            name: name.clone(),
            parent_id,
            local_transform,
            world_transform: local_transform,
            inverse_bind_matrix,
        };
        
        self.bones.push(bone);
        self.bone_map.insert(name, bone_id);
        
        if parent_id.is_none() {
            self.root_bones.push(bone_id);
        }
        
        bone_id
    }

    pub fn update_bone_transforms(&mut self) {
        let root_bones = self.root_bones.clone();
        for root_bone_id in root_bones {
            self.update_bone_recursive(root_bone_id, Mat4::IDENTITY);
        }
    }

    fn update_bone_recursive(&mut self, bone_id: u32, parent_transform: Mat4) {
        let world_transform = {
            let bone = &mut self.bones[bone_id as usize];
            bone.world_transform = parent_transform * bone.local_transform;
            bone.world_transform
        };

        let children: Vec<u32> = self
            .bones
            .iter()
            .enumerate()
            .filter(|(_, b)| b.parent_id == Some(bone_id))
            .map(|(idx, _)| idx as u32)
            .collect();

        for child_id in children {
            self.update_bone_recursive(child_id, world_transform);
        }
    }

    pub fn get_bone_transforms(&self) -> Vec<Mat4> {
        self.bones.iter()
            .map(|bone| bone.world_transform * bone.inverse_bind_matrix)
            .collect()
    }
}

/// Skinned mesh rendering için veri yapısı
#[derive(Debug, Clone)]
pub struct SkinnedMeshData {
    pub vertex_positions: Vec<Vec3>,
    pub vertex_normals: Vec<Vec3>,
    pub vertex_uvs: Vec<Vec2>,
    pub joint_indices: Vec<[u32; 4]>, // Her vertex en fazla 4 kemiğe ait olabilir
    pub joint_weights: Vec<[f32; 4]>,
    pub skeleton: Skeleton,
}

/// Animasyon kanalı
#[derive(Debug, Clone)]
pub struct AnimationChannel {
    pub target_bone: String,
    pub translation_keys: Vec<(f32, Vec3)>, // (zaman, konum)
    pub rotation_keys: Vec<(f32, Quat)>,    // (zaman, rotasyon)
    pub scale_keys: Vec<(f32, Vec3)>,       // (zaman, ölçek)
}

/// Animasyon verisi
#[derive(Debug, Clone)]
pub struct AnimationClip {
    pub name: String,
    pub duration: f32,
    pub channels: Vec<AnimationChannel>,
    pub fps: f32,
}

impl AnimationClip {
    pub fn new(name: String, duration: f32, fps: f32) -> Self {
        Self {
            name,
            duration,
            channels: Vec::new(),
            fps,
        }
    }

    pub fn sample_animation(&self, time: f32, skeleton: &mut Skeleton) {
        for channel in &self.channels {
            if let Some(&bone_id) = skeleton.bone_map.get(&channel.target_bone) {
                let bone = &mut skeleton.bones[bone_id as usize];

                // Konum interpolasyonu
                if !channel.translation_keys.is_empty() {
                    let translation = self.interpolate_translation(time, &channel.translation_keys);
                    let (scale, rotation, _) = bone.local_transform.to_scale_rotation_translation();
                    bone.local_transform = Mat4::from_scale_rotation_translation(scale, rotation, translation);
                }

                // Rotasyon interpolasyonu
                if !channel.rotation_keys.is_empty() {
                    let rotation = self.interpolate_rotation(time, &channel.rotation_keys);
                    let (scale, _, translation) = bone.local_transform.to_scale_rotation_translation();
                    bone.local_transform = Mat4::from_scale_rotation_translation(scale, rotation, translation);
                }

                // Ölçek interpolasyonu
                if !channel.scale_keys.is_empty() {
                    let scale = self.interpolate_scale(time, &channel.scale_keys);
                    let (_, rotation, translation) = bone.local_transform.to_scale_rotation_translation();
                    bone.local_transform = Mat4::from_scale_rotation_translation(scale, rotation, translation);
                }
            }
        }
    }

    fn interpolate_translation(&self, time: f32, keys: &[(f32, Vec3)]) -> Vec3 {
        if keys.is_empty() {
            return Vec3::ZERO;
        }
        
        if keys.len() == 1 {
            return keys[0].1;
        }
        
        // Zaman içinde uygun iki anahtar kareyi bul
        let mut prev_key = &keys[0];
        let mut next_key = &keys[keys.len() - 1];
        
        for i in 0..keys.len() - 1 {
            if time >= keys[i].0 && time <= keys[i + 1].0 {
                prev_key = &keys[i];
                next_key = &keys[i + 1];
                break;
            }
        }
        
        if prev_key.0 == next_key.0 {
            return prev_key.1;
        }
        
        let t = (time - prev_key.0) / (next_key.0 - prev_key.0);
        prev_key.1.lerp(next_key.1, t)
    }

    fn interpolate_rotation(&self, time: f32, keys: &[(f32, Quat)]) -> Quat {
        if keys.is_empty() {
            return Quat::IDENTITY;
        }
        
        if keys.len() == 1 {
            return keys[0].1;
        }
        
        // Zaman içinde uygun iki anahtar kareyi bul
        let mut prev_key = &keys[0];
        let mut next_key = &keys[keys.len() - 1];
        
        for i in 0..keys.len() - 1 {
            if time >= keys[i].0 && time <= keys[i + 1].0 {
                prev_key = &keys[i];
                next_key = &keys[i + 1];
                break;
            }
        }
        
        if prev_key.0 == next_key.0 {
            return prev_key.1;
        }
        
        let t = (time - prev_key.0) / (next_key.0 - prev_key.0);
        prev_key.1.slerp(next_key.1, t)
    }

    fn interpolate_scale(&self, time: f32, keys: &[(f32, Vec3)]) -> Vec3 {
        if keys.is_empty() {
            return Vec3::ONE;
        }
        
        if keys.len() == 1 {
            return keys[0].1;
        }
        
        // Zaman içinde uygun iki anahtar kareyi bul
        let mut prev_key = &keys[0];
        let mut next_key = &keys[keys.len() - 1];
        
        for i in 0..keys.len() - 1 {
            if time >= keys[i].0 && time <= keys[i + 1].0 {
                prev_key = &keys[i];
                next_key = &keys[i + 1];
                break;
            }
        }
        
        if prev_key.0 == next_key.0 {
            return prev_key.1;
        }
        
        let t = (time - prev_key.0) / (next_key.0 - prev_key.0);
        prev_key.1.lerp(next_key.1, t)
    }
}

/// Skeletal animasyon oynatıcı
#[derive(Debug, Clone)]
pub struct AnimationPlayer {
    pub current_clip: Option<String>,
    pub current_time: f32,
    pub playback_speed: f32,
    pub is_playing: bool,
    pub loop_animation: bool,
}

impl AnimationPlayer {
    pub fn new() -> Self {
        Self {
            current_clip: None,
            current_time: 0.0,
            playback_speed: 1.0,
            is_playing: false,
            loop_animation: true,
        }
    }

    pub fn play(&mut self, clip_name: String) {
        self.current_clip = Some(clip_name);
        self.current_time = 0.0;
        self.is_playing = true;
    }

    pub fn pause(&mut self) {
        self.is_playing = false;
    }

    pub fn stop(&mut self) {
        self.current_clip = None;
        self.current_time = 0.0;
        self.is_playing = false;
    }

    pub fn update(&mut self, delta_time: f32, animations: &HashMap<String, AnimationClip>, skeleton: &mut Skeleton) {
        if !self.is_playing {
            return;
        }
        
        if let Some(ref clip_name) = self.current_clip {
            if let Some(clip) = animations.get(clip_name) {
                self.current_time += delta_time * self.playback_speed;
                
                if self.current_time >= clip.duration {
                    if self.loop_animation {
                        self.current_time = 0.0;
                    } else {
                        self.current_time = clip.duration;
                        self.is_playing = false;
                    }
                }
                
                clip.sample_animation(self.current_time, skeleton);
            }
        }
    }
}