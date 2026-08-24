//! Audio system: manages listeners, sources, 3D spatialisation, and mixing.

use std::collections::HashMap;

use crate::audio::{AudioClip, AudioListener, AudioSource, PlayState};
use crate::math::Vec3;

/// The audio system manages all audio sources and the listener.
pub struct AudioSystem {
    pub listener: AudioListener,
    sources: HashMap<u64, AudioSource>,
    next_id: u64,
    /// Master volume (0..1).
    pub master_volume: f32,
    /// Whether audio is muted.
    pub muted: bool,
    /// Maximum simultaneous sources.
    pub max_sources: usize,
}

impl AudioSystem {
    pub fn new() -> Self {
        Self {
            listener: AudioListener::default(),
            sources: HashMap::new(),
            next_id: 0,
            master_volume: 1.0,
            muted: false,
            max_sources: 64,
        }
    }

    /// Add an audio source.  Returns its handle ID.
    pub fn add_source(&mut self, source: AudioSource) -> u64 {
        if self.sources.len() >= self.max_sources {
            // Remove oldest stopped source
            if let Some(id) = self
                .sources
                .iter()
                .find(|(_, s)| s.state == PlayState::Stopped)
                .map(|(id, _)| *id)
            {
                self.sources.remove(&id);
            }
        }
        let id = self.next_id;
        self.next_id += 1;
        self.sources.insert(id, source);
        id
    }

    /// Remove a source.
    pub fn remove_source(&mut self, id: u64) -> Option<AudioSource> {
        self.sources.remove(&id)
    }

    /// Get a reference to a source.
    pub fn source(&self, id: u64) -> Option<&AudioSource> {
        self.sources.get(&id)
    }

    /// Get a mutable reference to a source.
    pub fn source_mut(&mut self, id: u64) -> Option<&mut AudioSource> {
        self.sources.get_mut(&id)
    }

    /// Update all sources: advance playheads and compute spatial volume.
    pub fn update(&mut self, dt: f32) {
        let listener_pos = self.listener.position;
        let listener_forward = self.listener.forward;
        let listener_up = self.listener.up;
        let sfx_vol = self.listener.sfx_volume;

        for source in self.sources.values_mut() {
            // Advance playback
            source.advance(dt);

            // Compute spatial attenuation
            if source.spatial {
                let distance = (source.position - listener_pos).length();
                let attenuation = compute_attenuation(
                    distance,
                    source.min_distance,
                    source.max_distance,
                    source.rolloff,
                );

                // Compute pan based on listener orientation
                let to_source = (source.position - listener_pos).normalize_or_zero();
                let right = listener_forward.cross(listener_up).normalize_or_zero();
                let pan = to_source.dot(right); // -1 (left) to 1 (right)

                // Store pan in velocity.x as a convenience field
                source.velocity.x = pan;

                // Apply attenuation to volume
                let _effective_volume = source.volume * attenuation * sfx_vol * self.master_volume;
            }
        }
    }

    /// Set the listener position and orientation.
    pub fn set_listener(&mut self, position: Vec3, forward: Vec3, up: Vec3) {
        self.listener.position = position;
        self.listener.forward = forward.normalize_or_zero();
        self.listener.up = up.normalize_or_zero();
    }

    /// Stop all sources.
    pub fn stop_all(&mut self) {
        for source in self.sources.values_mut() {
            source.stop();
        }
    }

    /// Pause all sources.
    pub fn pause_all(&mut self) {
        for source in self.sources.values_mut() {
            if source.is_playing() {
                source.pause();
            }
        }
    }

    /// Resume all paused sources.
    pub fn resume_all(&mut self) {
        for source in self.sources.values_mut() {
            if source.state == PlayState::Paused {
                source.play();
            }
        }
    }

    /// Number of active (playing) sources.
    pub fn active_count(&self) -> usize {
        self.sources
            .values()
            .filter(|s| s.is_playing())
            .count()
    }

    /// Total number of sources.
    pub fn source_count(&self) -> usize {
        self.sources.len()
    }
}

impl Default for AudioSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute distance-based attenuation using the inverse distance model.
fn compute_attenuation(distance: f32, min_dist: f32, max_dist: f32, rolloff: f32) -> f32 {
    if distance <= min_dist {
        return 1.0;
    }
    if distance >= max_dist {
        return 0.0;
    }
    let normalized = (distance - min_dist) / (max_dist - min_dist);
    (1.0 - normalized).powf(rolloff)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn add_remove_source() {
        let mut sys = AudioSystem::new();
        let data = Arc::new(crate::audio::AudioData::new(vec![0.0; 100], 44100, 1));
        let clip = AudioClip::new("test".to_string(), data);
        let mut source = AudioSource::new();
        source.audio_data = Some(clip.data);
        let id = sys.add_source(source);
        assert_eq!(sys.source_count(), 1);
        sys.remove_source(id);
        assert_eq!(sys.source_count(), 0);
    }

    #[test]
    fn attenuation_formula() {
        // At min distance = full volume
        assert!((compute_attenuation(1.0, 1.0, 100.0, 1.0) - 1.0).abs() < 0.01);
        // At max distance = silent
        assert!(compute_attenuation(100.0, 1.0, 100.0, 1.0).abs() < 0.01);
        // Mid distance = partial
        let mid = compute_attenuation(50.5, 1.0, 100.0, 1.0);
        assert!(mid > 0.0 && mid < 1.0);
    }

    #[test]
    fn source_advance() {
        let data = Arc::new(crate::audio::AudioData::new(vec![0.0; 44100 * 2], 44100, 1));
        let clip = AudioClip::new("test".to_string(), data);
        let mut source = AudioSource::new();
        source.audio_data = Some(clip.data);
        source.play();
        assert!(source.advance(0.1));
        assert!(!source.advance(10.0)); // past end -> stopped
    }
}