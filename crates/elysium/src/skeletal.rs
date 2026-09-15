//! skeletal.rs — Skeletal animasyon sistemi
//!
//! - Bone hierarchy: parent-child zinciri, global/local transform
//! - Skinning: vertex bone weights, software skinning
//! - Animation clip: keyframe pozisyon/rotasyon/ölçek interpolasyonu
//! - Blend state machine: crossfade, layered blend, additive blend

use std::collections::HashMap;
use glam::{Vec3, Quat, Mat4};

// ═══════════════════════════════════════════════════════════ Bone Hierarchy

/// Bone — iskelet kemik tanımı
#[derive(Clone, Debug)]
pub struct Bone {
    pub id: u32,
    pub name: String,
    pub parent_id: Option<u32>,
    pub local_transform: Mat4,
    pub world_transform: Mat4,
    pub inverse_bind_matrix: Mat4,
    pub rest_position: Vec3,
    pub rest_rotation: Quat,
    pub rest_scale: Vec3,
}

/// İskelet sistemi — tüm kemiklerin hiyerarşisi
#[derive(Clone, Debug)]
pub struct Skeleton {
    pub bones: Vec<Bone>,
    pub bone_map: HashMap<String, u32>,
    pub root_bones: Vec<u32>,
    pub bone_count: usize,
}

impl Skeleton {
    pub fn new() -> Self {
        Self {
            bones: Vec::new(),
            bone_map: HashMap::new(),
            root_bones: Vec::new(),
            bone_count: 0,
        }
    }

    /// Yeni bone ekle
    pub fn add_bone(
        &mut self,
        name: &str,
        parent_name: Option<&str>,
        local_position: Vec3,
        local_rotation: Quat,
        local_scale: Vec3,
    ) -> u32 {
        let parent_id = parent_name.and_then(|pn| self.bone_map.get(pn).copied());
        let bone_id = self.bones.len() as u32;

        let local_transform = Mat4::from_scale_rotation_translation(
            local_scale, local_rotation, local_position,
        );

        let bone = Bone {
            id: bone_id,
            name: name.to_string(),
            parent_id,
            local_transform,
            world_transform: local_transform,
            inverse_bind_matrix: Mat4::IDENTITY,
            rest_position: local_position,
            rest_rotation: local_rotation,
            rest_scale: local_scale,
        };

        self.bones.push(bone);
        self.bone_map.insert(name.to_string(), bone_id);

        if parent_id.is_none() {
            self.root_bones.push(bone_id);
        }

        self.bone_count = self.bones.len();
        bone_id
    }

    /// Inverse bind matrix'leri hesapla (poze göre)
    pub fn compute_inverse_bind_matrices(&mut self) {
        self.update_world_transforms();
        for i in 0..self.bones.len() {
            let world = self.bones[i].world_transform;
            self.bones[i].inverse_bind_matrix = world.inverse();
        }
    }

    /// World transform'ları güncelle (recursive)
    pub fn update_world_transforms(&mut self) {
        let roots = self.root_bones.clone();
        for root_id in roots {
            self.update_bone_recursive(root_id, Mat4::IDENTITY);
        }
    }

    fn update_bone_recursive(&mut self, bone_id: u32, parent_world: Mat4) {
        let world = parent_world * self.bones[bone_id as usize].local_transform;
        self.bones[bone_id as usize].world_transform = world;

        let children: Vec<u32> = self.bones.iter()
            .filter(|b| b.parent_id == Some(bone_id))
            .map(|b| b.id)
            .collect();

        for child_id in children {
            self.update_bone_recursive(child_id, world);
        }
    }

    /// Final bone matrix paleti (skinning için)
    pub fn get_bone_matrices(&self) -> Vec<Mat4> {
        self.bones.iter()
            .map(|b| b.world_transform * b.inverse_bind_matrix)
            .collect()
    }

    /// Bone ID'sinden indeks
    pub fn bone_index(&self, name: &str) -> Option<u32> {
        self.bone_map.get(name).copied()
    }

    /// Bone'u isimle bul
    pub fn find_bone(&self, name: &str) -> Option<&Bone> {
        self.bone_map.get(name).and_then(|&id| self.bones.get(id as usize))
    }

    /// Bone'u isimle bul (mutable)
    pub fn find_bone_mut(&mut self, name: &str) -> Option<&mut Bone> {
        self.bone_map.get(name).copied()
            .and_then(|id| self.bones.get_mut(id as usize))
    }

    /// Bone'u姿勢_POSE'a sıfırla (rest pose)
    pub fn reset_to_rest_pose(&mut self) {
        for bone in &mut self.bones {
            bone.local_transform = Mat4::from_scale_rotation_translation(
                bone.rest_scale, bone.rest_rotation, bone.rest_position,
            );
        }
        self.update_world_transforms();
    }

    /// Demo iskelet oluştur (basit humanoid)
    pub fn demo_humanoid() -> Self {
        let mut skel = Skeleton::new();

        // Root / Hips
        skel.add_bone("Hips", None,
            Vec3::new(0.0, 1.0, 0.0), Quat::IDENTITY, Vec3::ONE);

        // Spine
        skel.add_bone("Spine", Some("Hips"),
            Vec3::new(0.0, 0.2, 0.0), Quat::IDENTITY, Vec3::ONE);
        skel.add_bone("Chest", Some("Spine"),
            Vec3::new(0.0, 0.2, 0.0), Quat::IDENTITY, Vec3::ONE);
        skel.add_bone("Neck", Some("Chest"),
            Vec3::new(0.0, 0.15, 0.0), Quat::IDENTITY, Vec3::ONE);
        skel.add_bone("Head", Some("Neck"),
            Vec3::new(0.0, 0.12, 0.0), Quat::IDENTITY, Vec3::ONE);

        // Sol Kol
        skel.add_bone("LeftShoulder", Some("Chest"),
            Vec3::new(0.15, 0.08, 0.0), Quat::IDENTITY, Vec3::ONE);
        skel.add_bone("LeftUpperArm", Some("LeftShoulder"),
            Vec3::new(0.2, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE);
        skel.add_bone("LeftLowerArm", Some("LeftUpperArm"),
            Vec3::new(0.25, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE);
        skel.add_bone("LeftHand", Some("LeftLowerArm"),
            Vec3::new(0.2, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE);

        // Sağ Kol
        skel.add_bone("RightShoulder", Some("Chest"),
            Vec3::new(-0.15, 0.08, 0.0), Quat::IDENTITY, Vec3::ONE);
        skel.add_bone("RightUpperArm", Some("RightShoulder"),
            Vec3::new(-0.2, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE);
        skel.add_bone("RightLowerArm", Some("RightUpperArm"),
            Vec3::new(-0.25, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE);
        skel.add_bone("RightHand", Some("RightLowerArm"),
            Vec3::new(-0.2, 0.0, 0.0), Quat::IDENTITY, Vec3::ONE);

        // Sol Bacak
        skel.add_bone("LeftUpperLeg", Some("Hips"),
            Vec3::new(0.1, -0.05, 0.0), Quat::IDENTITY, Vec3::ONE);
        skel.add_bone("LeftLowerLeg", Some("LeftUpperLeg"),
            Vec3::new(0.0, -0.4, 0.0), Quat::IDENTITY, Vec3::ONE);
        skel.add_bone("LeftFoot", Some("LeftLowerLeg"),
            Vec3::new(0.0, -0.4, 0.0), Quat::IDENTITY, Vec3::ONE);

        // Sağ Bacak
        skel.add_bone("RightUpperLeg", Some("Hips"),
            Vec3::new(-0.1, -0.05, 0.0), Quat::IDENTITY, Vec3::ONE);
        skel.add_bone("RightLowerLeg", Some("RightUpperLeg"),
            Vec3::new(0.0, -0.4, 0.0), Quat::IDENTITY, Vec3::ONE);
        skel.add_bone("RightFoot", Some("RightLowerLeg"),
            Vec3::new(0.0, -0.4, 0.0), Quat::IDENTITY, Vec3::ONE);

        skel.compute_inverse_bind_matrices();
        skel
    }
}

// ═══════════════════════════════════════════════════════════ Skinning

/// Vertex skinning bilgisi — her vertex en fazla 4 bone'a ait olabilir
#[derive(Clone, Debug, Default)]
pub struct SkinningWeights {
    pub bone_indices: [u32; 4],
    pub weights: [f32; 4],
}

impl SkinningWeights {
    pub fn new() -> Self {
        Self {
            bone_indices: [0; 4],
            weights: [0.0; 4],
        }
    }

    /// Tek bone ile ağırlık ata
    pub fn single_bone(bone_id: u32) -> Self {
        let mut sw = Self::new();
        sw.bone_indices[0] = bone_id;
        sw.weights[0] = 1.0;
        sw
    }

    /// İki bone ile ağırlık ata
    pub fn two_bones(id0: u32, w0: f32, id1: u32, w1: f32) -> Self {
        let mut sw = Self::new();
        sw.bone_indices[0] = id0;
        sw.weights[0] = w0;
        sw.bone_indices[1] = id1;
        sw.weights[1] = w1;
        sw
    }

    /// Ağırlıkları normalize et
    pub fn normalize(&mut self) {
        let sum: f32 = self.weights.iter().sum();
        if sum > 0.0001 {
            for w in &mut self.weights {
                *w /= sum;
            }
        }
    }
}

/// Skinned mesh — vertex pozisyonları + skinning weights
#[derive(Clone, Debug)]
pub struct SkinnedMesh {
    pub rest_positions: Vec<Vec3>,
    pub rest_normals: Vec<Vec3>,
    pub skinning_weights: Vec<SkinningWeights>,
    pub triangles: Vec<[usize; 3]>,
}

impl SkinnedMesh {
    pub fn new() -> Self {
        Self {
            rest_positions: Vec::new(),
            rest_normals: Vec::new(),
            skinning_weights: Vec::new(),
            triangles: Vec::new(),
        }
    }

    /// GPU-ready bone matrix paleti hesapla
    pub fn compute_skinned_positions(
        &self,
        bone_matrices: &[Mat4],
    ) -> (Vec<Vec3>, Vec<Vec3>) {
        let vertex_count = self.rest_positions.len();
        let mut skinned_positions = vec![Vec3::ZERO; vertex_count];
        let mut skinned_normals = vec![Vec3::ZERO; vertex_count];

        for (i, (pos, weights)) in self.rest_positions.iter()
            .zip(&self.skinning_weights)
            .enumerate()
        {
            let mut skinned_pos = Vec3::ZERO;
            let mut skinned_norm = Vec3::ZERO;

            for j in 0..4 {
                if weights.weights[j] > 0.0001 {
                    let bone_id = weights.bone_indices[j] as usize;
                    if bone_id < bone_matrices.len() {
                        let bone_mat = bone_matrices[bone_id];
                        skinned_pos += (bone_mat * pos.extend(1.0)).truncate() * weights.weights[j];

                        // Normal için sadece rotasyon bileşenini kullan
                        let normal_mat = bone_mat.inverse().transpose();
                        skinned_norm += (normal_mat * self.rest_normals[i].extend(0.0)).truncate() * weights.weights[j];
                    }
                }
            }

            skinned_positions[i] = skinned_pos;
            skinned_normals[i] = skinned_norm.normalize_or_zero();
        }

        (skinned_positions, skinned_normals)
    }

    /// Demo skinned cube oluştur
    pub fn demo_skinned_cube() -> Self {
        let mut mesh = SkinnedMesh::new();

        // 8 köşe, 2 kemiğe ait (sol ve sağ yarım)
        let positions = vec![
            Vec3::new(-0.5, -0.5, -0.5), Vec3::new(0.5, -0.5, -0.5),
            Vec3::new(0.5, 0.5, -0.5),   Vec3::new(-0.5, 0.5, -0.5),
            Vec3::new(-0.5, -0.5, 0.5),  Vec3::new(0.5, -0.5, 0.5),
            Vec3::new(0.5, 0.5, 0.5),    Vec3::new(-0.5, 0.5, 0.5),
        ];
        let normals = vec![
            Vec3::new(-1.0, -1.0, -1.0).normalize(), Vec3::new(1.0, -1.0, -1.0).normalize(),
            Vec3::new(1.0, 1.0, -1.0).normalize(),   Vec3::new(-1.0, 1.0, -1.0).normalize(),
            Vec3::new(-1.0, -1.0, 1.0).normalize(),  Vec3::new(1.0, -1.0, 1.0).normalize(),
            Vec3::new(1.0, 1.0, 1.0).normalize(),    Vec3::new(-1.0, 1.0, 1.0).normalize(),
        ];

        // Sol taraf (bone 0), sağ taraf (bone 1)
        let weights = vec![
            SkinningWeights::two_bones(0, 1.0, 1, 0.0),  // sol alt arka
            SkinningWeights::two_bones(0, 0.0, 1, 1.0),  // sağ alt arka
            SkinningWeights::two_bones(0, 0.0, 1, 1.0),  // sağ üst arka
            SkinningWeights::two_bones(0, 1.0, 1, 0.0),  // sol üst arka
            SkinningWeights::two_bones(0, 1.0, 1, 0.0),  // sol alt ön
            SkinningWeights::two_bones(0, 0.0, 1, 1.0),  // sağ alt ön
            SkinningWeights::two_bones(0, 0.0, 1, 1.0),  // sağ üst ön
            SkinningWeights::two_bones(0, 1.0, 1, 0.0),  // sol üst ön
        ];

        let triangles = vec![
            [0,1,2], [0,2,3], [4,6,5], [4,7,6],
            [0,4,5], [0,5,1], [2,6,7], [2,7,3],
            [0,3,7], [0,7,4], [1,5,6], [1,6,2],
        ];

        mesh.rest_positions = positions;
        mesh.rest_normals = normals;
        mesh.skinning_weights = weights;
        mesh.triangles = triangles;
        mesh
    }
}

// ═══════════════════════════════════════════════════════════ Animation Clip

/// İnterpolasyon türü
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Interpolation {
    Linear,
    Step,
    CubicSpline,
}

/// Animasyon kanalı — tek bir bone'un tek bir özelliğini animasyonlar
#[derive(Clone, Debug)]
pub struct AnimChannel {
    pub bone_name: String,
    pub position_keys: Vec<(f32, Vec3)>,
    pub rotation_keys: Vec<(f32, Quat)>,
    pub scale_keys: Vec<(f32, Vec3)>,
    pub interpolation: Interpolation,
}

impl AnimChannel {
    pub fn new(bone_name: &str) -> Self {
        Self {
            bone_name: bone_name.to_string(),
            position_keys: Vec::new(),
            rotation_keys: Vec::new(),
            scale_keys: Vec::new(),
            interpolation: Interpolation::Linear,
        }
    }

    /// Pozisyon keyframe ekle
    pub fn add_position_key(&mut self, time: f32, position: Vec3) {
        self.position_keys.push((time, position));
        self.position_keys.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    }

    /// Rotasyon keyframe ekle
    pub fn add_rotation_key(&mut self, time: f32, rotation: Quat) {
        self.rotation_keys.push((time, rotation));
        self.rotation_keys.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    }

    /// Ölçek keyframe ekle
    pub fn add_scale_key(&mut self, time: f32, scale: Vec3) {
        self.scale_keys.push((time, scale));
        self.scale_keys.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    }

    /// Belirli bir zamanda bone transform'unu hesapla
    pub fn sample(&self, time: f32) -> (Vec3, Quat, Vec3) {
        let pos = self.sample_position(time);
        let rot = self.sample_rotation(time);
        let scl = self.sample_scale(time);
        (pos, rot, scl)
    }

    fn sample_position(&self, time: f32) -> Vec3 {
        Self::interpolate_vec3(&self.position_keys, time, Vec3::ZERO)
    }

    fn sample_rotation(&self, time: f32) -> Quat {
        Self::interpolate_quat(&self.rotation_keys, time, Quat::IDENTITY)
    }

    fn sample_scale(&self, time: f32) -> Vec3 {
        Self::interpolate_vec3(&self.scale_keys, time, Vec3::ONE)
    }

    fn interpolate_vec3(keys: &[(f32, Vec3)], time: f32, default: Vec3) -> Vec3 {
        if keys.is_empty() { return default; }
        if keys.len() == 1 { return keys[0].1; }

        if time <= keys[0].0 { return keys[0].1; }
        if time >= keys.last().unwrap().0 { return keys.last().unwrap().1; }

        for i in 0..keys.len() - 1 {
            if time >= keys[i].0 && time <= keys[i + 1].0 {
                let t = (time - keys[i].0) / (keys[i + 1].0 - keys[i].0);
                return keys[i].1.lerp(keys[i + 1].1, t);
            }
        }
        default
    }

    fn interpolate_quat(keys: &[(f32, Quat)], time: f32, default: Quat) -> Quat {
        if keys.is_empty() { return default; }
        if keys.len() == 1 { return keys[0].1; }

        if time <= keys[0].0 { return keys[0].1; }
        if time >= keys.last().unwrap().0 { return keys.last().unwrap().1; }

        for i in 0..keys.len() - 1 {
            if time >= keys[i].0 && time <= keys[i + 1].0 {
                let t = (time - keys[i].0) / (keys[i + 1].0 - keys[i].0);
                return keys[i].1.slerp(keys[i + 1].1, t);
            }
        }
        default
    }
}

/// Animasyon clip — bir dizi kanalın birleşimi
#[derive(Clone, Debug)]
pub struct AnimClip {
    pub name: String,
    pub channels: Vec<AnimChannel>,
    pub duration: f32,
    pub fps: f32,
    pub loop_animation: bool,
}

impl AnimClip {
    pub fn new(name: &str, duration: f32) -> Self {
        Self {
            name: name.to_string(),
            channels: Vec::new(),
            duration,
            fps: 30.0,
            loop_animation: true,
        }
    }

    pub fn add_channel(&mut self, channel: AnimChannel) {
        self.channels.push(channel);
    }

    /// Zamanı normalize et (loop için)
    pub fn normalize_time(&self, time: f32) -> f32 {
        if self.loop_animation && self.duration > 0.0 {
            time % self.duration
        } else {
            time.clamp(0.0, self.duration)
        }
    }

    /// Skeleton'a uygula
    pub fn apply_to_skeleton(&self, time: f32, skeleton: &mut Skeleton) {
        let t = self.normalize_time(time);
        for channel in &self.channels {
            if let Some(bone_id) = skeleton.bone_index(&channel.bone_name) {
                let (pos, rot, scl) = channel.sample(t);
                let bone = &mut skeleton.bones[bone_id as usize];
                bone.local_transform = Mat4::from_scale_rotation_translation(scl, rot, pos);
            }
        }
        skeleton.update_world_transforms();
    }
}

// ═══════════════════════════════════════════════════════════ Blend State Machine

/// Blend modu
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BlendMode {
    /// Crossfade: iki animation arası yumuşak geçiş
    CrossFade,
    /// Layered: üst katman alt katmanın üzerine biner
    Layered,
    /// Additive: animasyon mevcut poza eklenir
    Additive,
}

/// Animasyon durumu (blend için)
#[derive(Clone, Debug)]
pub struct AnimState {
    pub clip_name: String,
    pub time: f32,
    pub speed: f32,
    pub weight: f32,
    pub blend_mode: BlendMode,
    pub is_playing: bool,
    pub loop_animation: bool,
}

impl AnimState {
    pub fn new(clip_name: &str) -> Self {
        Self {
            clip_name: clip_name.to_string(),
            time: 0.0,
            speed: 1.0,
            weight: 1.0,
            blend_mode: BlendMode::CrossFade,
            is_playing: true,
            loop_animation: true,
        }
    }
}

/// Blend state machine — çoklu animasyonun karıştırılması
#[derive(Clone, Debug)]
pub struct BlendStateMachine {
    pub states: Vec<AnimState>,
    pub transition_duration: f32,
    pub current_state_index: usize,
    pub previous_state_index: Option<usize>,
    pub transition_progress: f32,
    pub is_transitioning: bool,
}

impl BlendStateMachine {
    pub fn new() -> Self {
        Self {
            states: Vec::new(),
            transition_duration: 0.3,
            current_state_index: 0,
            previous_state_index: None,
            transition_progress: 1.0,
            is_transitioning: false,
        }
    }

    /// Yeni durum ekle
    pub fn add_state(&mut self, state: AnimState) -> usize {
        let idx = self.states.len();
        self.states.push(state);
        idx
    }

    /// Belirli bir duruma geç (crossfade ile)
    pub fn transition_to(&mut self, state_index: usize) {
        if state_index >= self.states.len() { return; }
        if state_index == self.current_state_index { return; }

        self.previous_state_index = Some(self.current_state_index);
        self.current_state_index = state_index;
        self.transition_progress = 0.0;
        self.is_transitioning = true;

        // Yeni durumu başlat
        self.states[state_index].time = 0.0;
        self.states[state_index].is_playing = true;
        self.states[state_index].weight = 0.0;
    }

    /// Durumu isimle ara
    pub fn find_state_index(&self, name: &str) -> Option<usize> {
        self.states.iter().position(|s| s.clip_name == name)
    }

    /// Geçiş ile duruma geç (isimle)
    pub fn transition_to_by_name(&mut self, name: &str) {
        if let Some(idx) = self.find_state_index(name) {
            self.transition_to(idx);
        }
    }

    /// Güncelle
    pub fn update(&mut self, dt: f32) {
        // Geçiş ilerlemesi
        if self.is_transitioning {
            self.transition_progress += dt / self.transition_duration;
            if self.transition_progress >= 1.0 {
                self.transition_progress = 1.0;
                self.is_transitioning = false;

                // Eski durumu durdur
                if let Some(prev_idx) = self.previous_state_index {
                    if prev_idx < self.states.len() {
                        self.states[prev_idx].is_playing = false;
                    }
                }
            }

            // Ağırlıkları ayarla
            if let Some(prev_idx) = self.previous_state_index {
                if prev_idx < self.states.len() {
                    self.states[prev_idx].weight = 1.0 - self.transition_progress;
                }
            }
            if self.current_state_index < self.states.len() {
                self.states[self.current_state_index].weight = self.transition_progress;
            }
        } else {
            // Geçiş yoksa sadece mevcut durumu güncelle
            for (i, state) in self.states.iter_mut().enumerate() {
                if i == self.current_state_index {
                    state.weight = 1.0;
                } else {
                    state.weight = 0.0;
                }
            }
        }

        // Tüm aktif durumların zamanını güncelle
        for state in &mut self.states {
            if state.is_playing {
                state.time += dt * state.speed;
            }
        }
    }

    /// Tüm durumları skeleton'a uygula (blend ile)
    pub fn apply_to_skeleton(
        &self,
        clips: &HashMap<String, AnimClip>,
        skeleton: &mut Skeleton,
    ) {
        // Önce rest pose'a dön
        skeleton.reset_to_rest_pose();

        // Ağırlıklı olarak uygula
        let mut blended_pos: HashMap<String, Vec3> = HashMap::new();
        let mut blended_rot: HashMap<String, Quat> = HashMap::new();
        let mut blended_scl: HashMap<String, Vec3> = HashMap::new();

        for state in &self.states {
            if !state.is_playing || state.weight < 0.001 {
                continue;
            }

            if let Some(clip) = clips.get(&state.clip_name) {
                let t = clip.normalize_time(state.time);

                for channel in &clip.channels {
                    let (pos, rot, scl) = channel.sample(t);

                    match state.blend_mode {
                        BlendMode::CrossFade | BlendMode::Layered => {
                            // Ağırlıklı ortalama
                            let entry_p = blended_pos.entry(channel.bone_name.clone())
                                .or_insert(Vec3::ZERO);
                            *entry_p += pos * state.weight;

                            let entry_r = blended_rot.entry(channel.bone_name.clone())
                                .or_insert(Quat::IDENTITY);
                            let target_rot = *entry_r;
                            *entry_r = target_rot.slerp(rot, state.weight);

                            let entry_s = blended_scl.entry(channel.bone_name.clone())
                                .or_insert(Vec3::ONE);
                            *entry_s = entry_s.lerp(scl, state.weight);
                        }
                        BlendMode::Additive => {
                            // Mevcut bone'a ekle
                            if let Some(bone_id) = skeleton.bone_index(&channel.bone_name) {
                                let bone = &mut skeleton.bones[bone_id as usize];
                                let current_pos = (bone.local_transform * Vec3::ZERO.extend(1.0)).truncate();
                                let add_pos = pos * state.weight;
                                let (s, r, _) = bone.local_transform.to_scale_rotation_translation();
                                bone.local_transform = Mat4::from_scale_rotation_translation(
                                    s, r, current_pos + add_pos,
                                );
                            }
                            continue;
                        }
                    }
                }
            }
        }

        // Blended değerleri skeleton'a uygula
        for (bone_name, pos) in &blended_pos {
            if let Some(bone_id) = skeleton.bone_index(bone_name) {
                let rot = blended_rot.get(bone_name).copied().unwrap_or(Quat::IDENTITY);
                let scl = blended_scl.get(bone_name).copied().unwrap_or(Vec3::ONE);
                skeleton.bones[bone_id as usize].local_transform =
                    Mat4::from_scale_rotation_translation(scl, rot, *pos);
            }
        }

        skeleton.update_world_transforms();
    }

    /// Mevcut durum bilgisi
    pub fn current_state(&self) -> Option<&AnimState> {
        self.states.get(self.current_state_index)
    }

    /// Tüm durum isimlerini listele
    pub fn state_names(&self) -> Vec<&str> {
        self.states.iter().map(|s| s.clip_name.as_str()).collect()
    }
}

// ═══════════════════════════════════════════════════════════ Demo Animasyonlar

/// Idle animasyonu oluştur (hafif sallanma)
pub fn create_idle_animation() -> AnimClip {
    let mut clip = AnimClip::new("Idle", 2.0);

    let mut spine = AnimChannel::new("Spine");
    spine.add_rotation_key(0.0, Quat::IDENTITY);
    spine.add_rotation_key(1.0, Quat::from_rotation_x(0.05));
    spine.add_rotation_key(2.0, Quat::IDENTITY);
    clip.add_channel(spine);

    let mut head = AnimChannel::new("Head");
    head.add_rotation_key(0.0, Quat::IDENTITY);
    head.add_rotation_key(0.5, Quat::from_rotation_y(0.1));
    head.add_rotation_key(1.5, Quat::from_rotation_y(-0.1));
    head.add_rotation_key(2.0, Quat::IDENTITY);
    clip.add_channel(head);

    clip
}

/// Walk animasyonu oluştur
pub fn create_walk_animation() -> AnimClip {
    let mut clip = AnimClip::new("Walk", 1.0);

    // Hips sallanması
    let mut hips = AnimChannel::new("Hips");
    hips.add_position_key(0.0, Vec3::new(0.0, 1.0, 0.0));
    hips.add_position_key(0.25, Vec3::new(0.0, 1.05, 0.0));
    hips.add_position_key(0.5, Vec3::new(0.0, 1.0, 0.0));
    hips.add_position_key(0.75, Vec3::new(0.0, 1.05, 0.0));
    hips.add_position_key(1.0, Vec3::new(0.0, 1.0, 0.0));
    clip.add_channel(hips);

    // Sol bacak
    let mut left_leg = AnimChannel::new("LeftUpperLeg");
    left_leg.add_rotation_key(0.0, Quat::from_rotation_x(0.3));
    left_leg.add_rotation_key(0.25, Quat::IDENTITY);
    left_leg.add_rotation_key(0.5, Quat::from_rotation_x(-0.3));
    left_leg.add_rotation_key(0.75, Quat::IDENTITY);
    left_leg.add_rotation_key(1.0, Quat::from_rotation_x(0.3));
    clip.add_channel(left_leg);

    // Sağ bacak
    let mut right_leg = AnimChannel::new("RightUpperLeg");
    right_leg.add_rotation_key(0.0, Quat::from_rotation_x(-0.3));
    right_leg.add_rotation_key(0.25, Quat::IDENTITY);
    right_leg.add_rotation_key(0.5, Quat::from_rotation_x(0.3));
    right_leg.add_rotation_key(0.75, Quat::IDENTITY);
    right_leg.add_rotation_key(1.0, Quat::from_rotation_x(-0.3));
    clip.add_channel(right_leg);

    // Sol kol (zıt)
    let mut left_arm = AnimChannel::new("LeftUpperArm");
    left_arm.add_rotation_key(0.0, Quat::from_rotation_x(-0.3));
    left_arm.add_rotation_key(0.5, Quat::from_rotation_x(0.3));
    left_arm.add_rotation_key(1.0, Quat::from_rotation_x(-0.3));
    clip.add_channel(left_arm);

    // Sağ kol (zıt)
    let mut right_arm = AnimChannel::new("RightUpperArm");
    right_arm.add_rotation_key(0.0, Quat::from_rotation_x(0.3));
    right_arm.add_rotation_key(0.5, Quat::from_rotation_x(-0.3));
    right_arm.add_rotation_key(1.0, Quat::from_rotation_x(0.3));
    clip.add_channel(right_arm);

    clip
}

/// Run animasyonu oluştur
pub fn create_run_animation() -> AnimClip {
    let mut clip = AnimClip::new("Run", 0.6);

    let mut hips = AnimChannel::new("Hips");
    hips.add_position_key(0.0, Vec3::new(0.0, 1.0, 0.0));
    hips.add_position_key(0.15, Vec3::new(0.0, 1.15, 0.0));
    hips.add_position_key(0.3, Vec3::new(0.0, 1.0, 0.0));
    hips.add_position_key(0.45, Vec3::new(0.0, 1.15, 0.0));
    hips.add_position_key(0.6, Vec3::new(0.0, 1.0, 0.0));
    clip.add_channel(hips);

    let mut left_leg = AnimChannel::new("LeftUpperLeg");
    left_leg.add_rotation_key(0.0, Quat::from_rotation_x(0.6));
    left_leg.add_rotation_key(0.15, Quat::IDENTITY);
    left_leg.add_rotation_key(0.3, Quat::from_rotation_x(-0.6));
    left_leg.add_rotation_key(0.45, Quat::IDENTITY);
    left_leg.add_rotation_key(0.6, Quat::from_rotation_x(0.6));
    clip.add_channel(left_leg);

    let mut right_leg = AnimChannel::new("RightUpperLeg");
    right_leg.add_rotation_key(0.0, Quat::from_rotation_x(-0.6));
    right_leg.add_rotation_key(0.15, Quat::IDENTITY);
    right_leg.add_rotation_key(0.3, Quat::from_rotation_x(0.6));
    right_leg.add_rotation_key(0.45, Quat::IDENTITY);
    right_leg.add_rotation_key(0.6, Quat::from_rotation_x(-0.6));
    clip.add_channel(right_leg);

    clip
}

// ═══════════════════════════════════════════════════════════ Testler

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skeleton_demo() {
        let skel = Skeleton::demo_humanoid();
        assert!(skel.bones.len() >= 15, "Humanoid skeleton should have at least 15 bones");
        assert_eq!(skel.bones[0].name, "Hips");
    }

    #[test]
    fn test_bone_hierarchy() {
        let mut skel = Skeleton::new();
        skel.add_bone("Root", None, Vec3::ZERO, Quat::IDENTITY, Vec3::ONE);
        skel.add_bone("Child", Some("Root"), Vec3::Y, Quat::IDENTITY, Vec3::ONE);

        skel.update_world_transforms();

        let root_world = skel.bones[0].world_transform;
        let child_world = skel.bones[1].world_transform;

        // Child world = Root world * Child local
        let diff = root_world * skel.bones[1].local_transform - child_world;
        let cols = diff.to_cols_array();
        let mag: f32 = cols.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(mag < 0.001);
    }

    #[test]
    fn test_bone_matrices() {
        let mut skel = Skeleton::demo_humanoid();
        skel.compute_inverse_bind_matrices();
        let matrices = skel.get_bone_matrices();
        assert_eq!(matrices.len(), skel.bones.len());
    }

    #[test]
    fn test_skinning_weights() {
        let mut sw = SkinningWeights::new();
        sw.bone_indices = [0, 1, 2, 3];
        sw.weights = [0.5, 0.3, 0.1, 0.1];
        sw.normalize();

        let sum: f32 = sw.weights.iter().sum();
        assert!((sum - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_skinned_mesh() {
        let mesh = SkinnedMesh::demo_skinned_cube();
        assert_eq!(mesh.rest_positions.len(), 8);
        assert_eq!(mesh.skinning_weights.len(), 8);
        assert_eq!(mesh.triangles.len(), 12);
    }

    #[test]
    fn test_anim_clip() {
        let clip = create_idle_animation();
        assert_eq!(clip.name, "Idle");
        assert!(clip.duration > 0.0);
        assert!(!clip.channels.is_empty());
    }

    #[test]
    fn test_anim_channel_sampling() {
        let mut channel = AnimChannel::new("TestBone");
        channel.add_position_key(0.0, Vec3::ZERO);
        channel.add_position_key(1.0, Vec3::Y);

        let (pos, _, _) = channel.sample(0.5);
        assert!((pos.y - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_blend_state_machine() {
        let mut bsm = BlendStateMachine::new();
        bsm.add_state(AnimState::new("Idle"));
        bsm.add_state(AnimState::new("Walk"));
        bsm.add_state(AnimState::new("Run"));

        assert_eq!(bsm.states.len(), 3);
        assert_eq!(bsm.current_state_index, 0);

        bsm.transition_to(1);
        assert_eq!(bsm.current_state_index, 1);
        assert!(bsm.is_transitioning);
    }

    #[test]
    fn test_blend_transition_progress() {
        let mut bsm = BlendStateMachine::new();
        bsm.transition_duration = 0.5;
        bsm.add_state(AnimState::new("Idle"));
        bsm.add_state(AnimState::new("Walk"));
        bsm.transition_to(1);

        bsm.update(0.25); // %50 geçiş
        assert!((bsm.transition_progress - 0.5).abs() < 0.01);
        assert!((bsm.states[0].weight - 0.5).abs() < 0.01);
        assert!((bsm.states[1].weight - 0.5).abs() < 0.01);

        bsm.update(0.25); // %100 geçiş
        assert!(!bsm.is_transitioning);
        assert!((bsm.states[1].weight - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_demo_animations() {
        let idle = create_idle_animation();
        let walk = create_walk_animation();
        let run = create_run_animation();

        assert_eq!(idle.name, "Idle");
        assert_eq!(walk.name, "Walk");
        assert_eq!(run.name, "Run");
        assert!(walk.duration < idle.duration); // Walk daha hızlı
        assert!(run.duration < walk.duration);  // Run en hızlı
    }

    #[test]
    fn test_reset_to_rest() {
        let mut skel = Skeleton::demo_humanoid();
        let rest_hips_pos = (skel.bones[0].local_transform * Vec3::ZERO.extend(1.0)).truncate();

        // Bone'u değiştir
        skel.bones[0].local_transform = Mat4::from_translation(Vec3::new(100.0, 100.0, 100.0));
        skel.update_world_transforms();

        // Rest pose'a dön
        skel.reset_to_rest_pose();
        let after = (skel.bones[0].local_transform * Vec3::ZERO.extend(1.0)).truncate();

        assert!((rest_hips_pos - after).length() < 0.001);
    }
}
