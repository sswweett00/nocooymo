use crate::math::Vec3;
use crate::{Component, Transform};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

// Ses veri yapısı
#[derive(Debug, Clone)]
pub struct AudioData {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration: f32,
}

impl AudioData {
    pub fn new(samples: Vec<f32>, sample_rate: u32, channels: u16) -> Self {
        let duration = samples.len() as f32 / sample_rate as f32 / channels as f32;
        Self {
            samples,
            sample_rate,
            channels,
            duration,
        }
    }
    
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        // Bu sadece bir placeholder - gerçek implementasyon farklı kütüphaneler gerektirir
        println!("Loading audio from: {:?}", path.as_ref());
        Ok(AudioData {
            samples: vec![0.0; 44100], // 1 saniyelik örnek veri
            sample_rate: 44100,
            channels: 2,
            duration: 1.0,
        })
    }
}

// Ses çalma durumu
#[derive(Debug, Clone, PartialEq)]
pub enum AudioState {
    Stopped,
    Playing,
    Paused,
}

/// Alias kept for backwards compatibility with code expecting `PlayState`.
pub type PlayState = AudioState;

// Ses kaynağı bileşeni
pub struct AudioSource {
    pub audio_data: Option<Arc<AudioData>>,
    pub volume: f32,
    pub pitch: f32,
    pub loop_enabled: bool,
    pub spatial: bool, // 3D ses mi?
        pub min_distance: f32,
    pub max_distance: f32,
    pub rolloff_factor: f32,
    pub rolloff: f32,
    pub position: Vec3,
    pub velocity: Vec3,
    pub state: AudioState,
    pub playback_position: f32,
    pub playback_speed: f32,
    pub pan: f32, // -1.0 sol, 1.0 sağ
    pub priority: u8, // 0-255, yüksek sayı daha yüksek öncelik
}

impl AudioSource {
    pub fn new() -> Self {
        Self {
            audio_data: None,
            volume: 1.0,
            pitch: 1.0,
            loop_enabled: false,
            spatial: false,
            min_distance: 1.0,
            max_distance: 100.0,
            rolloff_factor: 1.0,
            rolloff: 1.0,
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            state: AudioState::Stopped,
            playback_position: 0.0,
            playback_speed: 1.0,
            pan: 0.0,
            priority: 128,
        }
    }
    
    pub fn with_audio_data(audio_data: Arc<AudioData>) -> Self {
        Self {
            audio_data: Some(audio_data),
            volume: 1.0,
            pitch: 1.0,
            loop_enabled: false,
            spatial: false,
            min_distance: 1.0,
            max_distance: 100.0,
            rolloff_factor: 1.0,
            rolloff: 1.0,
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            state: AudioState::Stopped,
            playback_position: 0.0,
            playback_speed: 1.0,
            pan: 0.0,
            priority: 128,
        }
    }
    
    pub fn play(&mut self) {
        self.state = AudioState::Playing;
    }
    
    pub fn pause(&mut self) {
        self.state = AudioState::Paused;
    }
    
    pub fn stop(&mut self) {
        self.state = AudioState::Stopped;
        self.playback_position = 0.0;
    }
    
    pub fn is_playing(&self) -> bool {
        self.state == AudioState::Playing
    }
    
    pub fn advance(&mut self, delta_time: f32) -> bool {
        self.update(delta_time);
        self.state == AudioState::Playing
    }
    
    pub fn set_loop(&mut self, enabled: bool) {
        self.loop_enabled = enabled;
    }
    
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }
    
    pub fn set_pitch(&mut self, pitch: f32) {
        self.pitch = pitch.clamp(0.1, 3.0);
    }
    
    pub fn set_pan(&mut self, pan: f32) {
        self.pan = pan.clamp(-1.0, 1.0);
    }
    
    pub fn set_spatial(&mut self, spatial: bool) {
        self.spatial = spatial;
    }
    
    pub fn update(&mut self, delta_time: f32) {
        if self.state != AudioState::Playing {
            return;
        }
        
        if let Some(ref audio_data) = self.audio_data {
            self.playback_position += delta_time * self.playback_speed * self.pitch;
            
            // Döngü kontrolü
            if self.playback_position >= audio_data.duration {
                if self.loop_enabled {
                    self.playback_position = 0.0;
                } else {
                    self.state = AudioState::Stopped;
                }
            }
        }
    }
    
    pub fn get_playback_progress(&self) -> f32 {
        if let Some(ref audio_data) = self.audio_data {
            self.playback_position / audio_data.duration
        } else {
            0.0
        }
    }
}

// Ses dinleyici bileşeni (kamera vs.)
#[derive(Clone)]
pub struct AudioListener {
    pub position: Vec3,
    pub forward: Vec3,
    pub up: Vec3,
    pub velocity: Vec3,
    pub doppler_factor: f32,
    pub speed_of_sound: f32,
    pub sfx_volume: f32,
}

impl Default for AudioListener {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioListener {
    pub fn new() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 0.0),
            forward: Vec3::new(0.0, 0.0, 1.0),
            up: Vec3::new(0.0, 1.0, 0.0),
            velocity: Vec3::new(0.0, 0.0, 0.0),
            doppler_factor: 1.0,
            speed_of_sound: 343.0, // Metre/saniye
            sfx_volume: 1.0,
        }
    }
    
    pub fn update_from_transform(&mut self, transform: &Transform) {
        self.position = transform.translation;
        // Rotasyona göre yönleri güncelle (basit bir yaklaşım)
        self.forward = Vec3::new(
            transform.rotation.y.sin() * transform.rotation.y.cos(),
            transform.rotation.x.sin(),
            transform.rotation.y.cos() * transform.rotation.y.cos(),
        ).normalize();
        
        // Up vektörünü hesapla
        self.up = Vec3::new(0.0, 1.0, 0.0);
    }
    
    pub fn set_velocity(&mut self, velocity: Vec3) {
        self.velocity = velocity;
    }
}

// Ses klibi
#[derive(Debug, Clone)]
pub struct AudioClip {
    pub name: String,
    pub data: Arc<AudioData>,
    pub category: AudioCategory,
}

#[derive(Debug, Clone)]
pub enum AudioCategory {
    SoundEffect,
    Music,
    Voice,
    Ambient,
    Custom(String),
}

impl AudioClip {
    pub fn new(name: String, data: Arc<AudioData>) -> Self {
        Self {
            name,
            data,
            category: AudioCategory::SoundEffect,
        }
    }
    
    pub fn with_category(name: String, data: Arc<AudioData>, category: AudioCategory) -> Self {
        Self {
            name,
            data,
            category,
        }
    }
    
    pub fn load_from_file<P: AsRef<Path>>(name: String, path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let data = AudioData::load_from_file(path)?;
        Ok(Self::new(name, Arc::new(data)))
    }
}

// Ses oynatıcı
pub struct AudioManager {
    pub clips: HashMap<String, Arc<AudioClip>>,
    pub active_sources: Vec<AudioSource>,
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub voice_volume: f32,
    pub listener: Option<AudioListener>,
    pub enabled: bool,
}

impl AudioManager {
    pub fn new() -> Self {
        Self {
            clips: HashMap::new(),
            active_sources: Vec::new(),
            master_volume: 1.0,
            music_volume: 1.0,
            sfx_volume: 1.0,
            voice_volume: 1.0,
            listener: None,
            enabled: true,
        }
    }
    
    pub fn add_clip(&mut self, clip: AudioClip) {
        self.clips.insert(clip.name.clone(), Arc::new(clip));
    }
    
    pub fn get_clip(&self, name: &str) -> Option<Arc<AudioClip>> {
        self.clips.get(name).cloned()
    }
    
    pub fn play_clip(&mut self, name: &str, transform: Option<&Transform>) -> Option<usize> {
        if !self.enabled {
            return None;
        }
        
        if let Some(clip) = self.clips.get(name) {
            let mut source = AudioSource::with_audio_data(clip.data.clone());
            source.play();
            
            // 3D ses ayarları
            if let Some(transform) = transform {
                source.spatial = true;
                // 3D pozisyon ayarları burada yapılabilir
            }
            
            // Kategoriye göre ses seviyesi ayarı
            match clip.category {
                AudioCategory::Music => source.volume *= self.music_volume,
                AudioCategory::SoundEffect => source.volume *= self.sfx_volume,
                AudioCategory::Voice => source.volume *= self.voice_volume,
                AudioCategory::Ambient => source.volume *= self.sfx_volume * 0.8,
                AudioCategory::Custom(_) => source.volume *= self.master_volume,
            }
            
            self.active_sources.push(source);
            Some(self.active_sources.len() - 1)
        } else {
            None
        }
    }
    
    pub fn play_3d_clip(&mut self, name: &str, position: Vec3) -> Option<usize> {
        if !self.enabled {
            return None;
        }
        
        if let Some(clip) = self.clips.get(name) {
            let mut source = AudioSource::with_audio_data(clip.data.clone());
            source.play();
            source.spatial = true;
            
            // 3D pozisyon ayarları
            // Bu bilgiler ses motoruna aktarılacaktır
            
            self.active_sources.push(source);
            Some(self.active_sources.len() - 1)
        } else {
            None
        }
    }
    
    pub fn stop_source(&mut self, index: usize) {
        if index < self.active_sources.len() {
            self.active_sources[index].stop();
        }
    }
    
    pub fn pause_source(&mut self, index: usize) {
        if index < self.active_sources.len() {
            self.active_sources[index].pause();
        }
    }
    
    pub fn resume_source(&mut self, index: usize) {
        if index < self.active_sources.len() {
            if self.active_sources[index].state == AudioState::Paused {
                self.active_sources[index].play();
            }
        }
    }
    
    pub fn set_source_volume(&mut self, index: usize, volume: f32) {
        if index < self.active_sources.len() {
            self.active_sources[index].set_volume(volume);
        }
    }
    
    pub fn update_listener(&mut self, listener: AudioListener) {
        self.listener = Some(listener);
    }
    
    pub fn update(&mut self, delta_time: f32) {
        if !self.enabled {
            return;
        }
        
        // Aktif kaynakları güncelle
        self.active_sources.retain_mut(|source| {
            source.update(delta_time);
            // Oynatımı bitmişse kaldır
            source.state != AudioState::Stopped
        });
        
        // 3D ses hesaplamaları
        if let Some(listener) = self.listener.clone() {
            self.update_3d_audio(&listener);
        }
    }
    
    fn update_3d_audio(&mut self, listener: &AudioListener) {
        for source in &mut self.active_sources {
            if !source.spatial {
                continue;
            }
            
            // Burada 3D ses hesaplamaları yapılır
            // Mesafe, yön, doppler efekti vb.
            
            // Basit mesafe tabanlı ses azalması
            if let Some(ref data) = source.audio_data {
                // Kaynağın pozisyonuna göre hesapla
                // Bu sadece placeholder - gerçek implementasyon daha karmaşık
            }
        }
    }
    
    pub fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = volume.clamp(0.0, 1.0);
    }
    
    pub fn set_music_volume(&mut self, volume: f32) {
        self.music_volume = volume.clamp(0.0, 1.0);
    }
    
    pub fn set_sfx_volume(&mut self, volume: f32) {
        self.sfx_volume = volume.clamp(0.0, 1.0);
    }
    
    pub fn set_voice_volume(&mut self, volume: f32) {
        self.voice_volume = volume.clamp(0.0, 1.0);
    }
    
    pub fn stop_all(&mut self) {
        for source in &mut self.active_sources {
            source.stop();
        }
    }
    
    pub fn pause_all(&mut self) {
        for source in &mut self.active_sources {
            if source.state == AudioState::Playing {
                source.pause();
            }
        }
    }
    
    pub fn resume_all(&mut self) {
        for source in &mut self.active_sources {
            if source.state == AudioState::Paused {
                source.play();
            }
        }
    }
}

// Müzik çalar bileşeni
pub struct MusicPlayer {
    pub playlist: Vec<String>,
    pub current_track: Option<String>,
    pub current_index: usize,
    pub shuffle: bool,
    pub repeat: bool,
    pub volume: f32,
    pub fade_duration: f32,
    pub crossfade_enabled: bool,
    pub state: AudioState,
}

impl MusicPlayer {
    pub fn new() -> Self {
        Self {
            playlist: Vec::new(),
            current_track: None,
            current_index: 0,
            shuffle: false,
            repeat: false,
            volume: 1.0,
            fade_duration: 1.0,
            crossfade_enabled: false,
            state: AudioState::Stopped,
        }
    }
    
    pub fn add_track(&mut self, track_name: String) {
        self.playlist.push(track_name);
    }
    
    pub fn add_tracks(&mut self, tracks: Vec<String>) {
        self.playlist.extend(tracks);
    }
    
    pub fn play(&mut self) {
        if self.playlist.is_empty() {
            return;
        }
        
        if self.shuffle {
            // Basit karıştırma
            use rand::seq::SliceRandom;
            self.playlist.shuffle(&mut rand::thread_rng());
            self.current_index = 0;
        } else if self.current_index >= self.playlist.len() {
            self.current_index = 0;
        }
        
        self.current_track = Some(self.playlist[self.current_index].clone());
        self.state = AudioState::Playing;
    }
    
    pub fn play_track(&mut self, track_name: String) {
        if self.playlist.contains(&track_name) {
            self.current_track = Some(track_name);
            self.current_index = self.playlist.iter().position(|x| x == self.current_track.as_ref().unwrap()).unwrap();
            self.state = AudioState::Playing;
        }
    }
    
    pub fn next_track(&mut self) {
        if self.playlist.is_empty() {
            return;
        }
        
        self.current_index = (self.current_index + 1) % self.playlist.len();
        self.current_track = Some(self.playlist[self.current_index].clone());
        self.state = AudioState::Playing;
    }
    
    pub fn prev_track(&mut self) {
        if self.playlist.is_empty() {
            return;
        }
        
        if self.current_index == 0 {
            self.current_index = self.playlist.len() - 1;
        } else {
            self.current_index -= 1;
        }
        
        self.current_track = Some(self.playlist[self.current_index].clone());
        self.state = AudioState::Playing;
    }
    
    pub fn pause(&mut self) {
        self.state = AudioState::Paused;
    }
    
    pub fn stop(&mut self) {
        self.state = AudioState::Stopped;
        self.current_track = None;
    }
    
    pub fn set_shuffle(&mut self, shuffle: bool) {
        self.shuffle = shuffle;
    }
    
    pub fn set_repeat(&mut self, repeat: bool) {
        self.repeat = repeat;
    }
    
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }
    
    pub fn set_crossfade(&mut self, enabled: bool, duration: f32) {
        self.crossfade_enabled = enabled;
        self.fade_duration = duration;
    }
    
    pub fn update(&mut self, audio_manager: &mut AudioManager, delta_time: f32) {
        if self.state != AudioState::Playing {
            return;
        }
        
        // Geçerli parçanın durumunu kontrol et
        if let Some(ref track_name) = self.current_track {
            // Burada geçerli parçanın bittiğini tespit etmek için
            // AudioSource durumunu kontrol etmek gerekir
            // Bu sadece placeholder implementasyon
        }
        
        // Gerekirse sonraki parçaya geç
        if self.should_change_track() {
            if self.repeat {
                self.play(); // Aynı parçayı tekrar oynat
            } else {
                self.next_track(); // Sonraki parçaya geç
            }
        }
    }
    
    fn should_change_track(&self) -> bool {
        // Placeholder: 실제 구현에서는 현재 트랙의 상태를 확인해야 함
        false
    }
}

// Ses efekti bileşeni
pub struct AudioEffect {
    pub effect_type: AudioEffectType,
    pub parameters: HashMap<String, f32>,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub enum AudioEffectType {
    Reverb {
        room_size: f32,
        damping: f32,
        wet_level: f32,
    },
    Echo {
        delay: f32,
        decay: f32,
        wet_level: f32,
    },
    LowPassFilter {
        cutoff_freq: f32,
    },
    HighPassFilter {
        cutoff_freq: f32,
    },
    Distortion {
        drive: f32,
    },
    Chorus {
        rate: f32,
        depth: f32,
        feedback: f32,
    },
    Flanger {
        rate: f32,
        depth: f32,
        feedback: f32,
    },
    Compressor {
        threshold: f32,
        ratio: f32,
        attack: f32,
        release: f32,
    },
    Custom { name: String },
}

impl AudioEffect {
    pub fn new(effect_type: AudioEffectType) -> Self {
        let mut parameters = HashMap::new();
        
        match &effect_type {
            AudioEffectType::Reverb { room_size, damping, wet_level } => {
                parameters.insert("room_size".to_string(), *room_size);
                parameters.insert("damping".to_string(), *damping);
                parameters.insert("wet_level".to_string(), *wet_level);
            },
            AudioEffectType::Echo { delay, decay, wet_level } => {
                parameters.insert("delay".to_string(), *delay);
                parameters.insert("decay".to_string(), *decay);
                parameters.insert("wet_level".to_string(), *wet_level);
            },
            AudioEffectType::LowPassFilter { cutoff_freq } => {
                parameters.insert("cutoff_freq".to_string(), *cutoff_freq);
            },
            AudioEffectType::HighPassFilter { cutoff_freq } => {
                parameters.insert("cutoff_freq".to_string(), *cutoff_freq);
            },
            AudioEffectType::Distortion { drive } => {
                parameters.insert("drive".to_string(), *drive);
            },
            AudioEffectType::Chorus { rate, depth, feedback } => {
                parameters.insert("rate".to_string(), *rate);
                parameters.insert("depth".to_string(), *depth);
                parameters.insert("feedback".to_string(), *feedback);
            },
            AudioEffectType::Flanger { rate, depth, feedback } => {
                parameters.insert("rate".to_string(), *rate);
                parameters.insert("depth".to_string(), *depth);
                parameters.insert("feedback".to_string(), *feedback);
            },
            AudioEffectType::Compressor { threshold, ratio, attack, release } => {
                parameters.insert("threshold".to_string(), *threshold);
                parameters.insert("ratio".to_string(), *ratio);
                parameters.insert("attack".to_string(), *attack);
                parameters.insert("release".to_string(), *release);
            },
            AudioEffectType::Custom { name } => {
                parameters.insert("name".to_string(), 1.0); // Placeholder
            },
        }
        
        Self {
            effect_type,
            parameters,
            active: true,
        }
    }
    
    pub fn set_parameter(&mut self, param_name: &str, value: f32) {
        self.parameters.insert(param_name.to_string(), value);
    }
    
    pub fn get_parameter(&self, param_name: &str) -> Option<f32> {
        self.parameters.get(param_name).copied()
    }
    
    pub fn activate(&mut self) {
        self.active = true;
    }
    
    pub fn deactivate(&mut self) {
        self.active = false;
    }
    
    pub fn toggle(&mut self) {
        self.active = !self.active;
    }
}

// Global ses yöneticisi
pub static AUDIO_MANAGER: once_cell::sync::Lazy<Arc<Mutex<AudioManager>>> = 
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(AudioManager::new())));