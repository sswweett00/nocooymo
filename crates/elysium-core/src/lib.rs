pub mod math;
pub mod alloc;
pub mod memory_system;

/// 16 KiB chunk sayfası — WorldAllocator ve Chunk tarafından kullanılır.
pub const CHUNK_SIZE: usize = 16 * 1024;

pub mod archetype;
pub mod audio;
pub mod camera;
pub mod chunk;
pub mod command;
pub mod component;
pub mod entity;
pub mod hierarchy;
pub mod scheduler;
pub mod save_game;
pub mod serialization;
pub mod storage;
pub mod terrain;
pub mod transform;
pub mod world;
pub mod ui;
pub mod asset;
pub mod config;
pub mod logging;
pub mod performance;
pub mod data_structures;
pub mod physics_world;
pub mod scripting;
pub mod network;
pub mod networking;
pub mod ml;
pub mod particle_systems;
pub mod audio_system;
pub mod audio_engine;
pub mod animation_system;
pub mod crash_handler;
pub mod mod_system;
pub mod ai_navigation;
pub mod input_manager;
pub mod localization;
pub mod vr;
pub mod cinematics;
pub mod build_system;
pub mod job_system;
pub mod accessibility;
pub mod test_framework;
pub mod analytics;
pub mod cloud_services;

pub use animation_system::*;
pub use cinematics::*;
pub use crash_handler::*;
pub use ai_navigation::*;
pub use input::*;
pub use input_manager::*;
pub use localization::*;
pub use vr::*;
pub use archetype::*;
pub use audio::*;
pub use camera::*;
pub use chunk::*;
pub use command::*;
pub use component::*;
pub use entity::*;
pub use hierarchy::*;
pub use scheduler::*;
pub use serialization::*;
pub use storage::*;
pub use terrain::*;
pub use transform::*;
pub use world::*;
pub use ui::*;
pub use asset::*;
pub use config::*;
pub use logging::*;
pub use performance::*;
pub use data_structures::*;
pub use physics_world::*;
pub use scripting::*;
pub use network::*;
pub use networking::*;
pub use ml::*;
pub use particle_systems::*;
pub use audio_system::*;
pub use audio_engine::*;
pub use save_game::*;
pub use mod_system::*;
pub use build_system::*;
pub use memory_system::*;
pub use job_system::*;
pub use test_framework::*;
pub use analytics::*;
pub use accessibility::*;
pub use cloud_services::*;

// Ortak olarak kullanılan yardımcı türler ve sabitler
pub mod prelude {
    pub use crate::{
        // Temel yapılar
        Entity,
        EntityAllocator,
        EntityBuilder,
        Transform,
        World,
        CommandBuffer,
        Schedule,
        System,
        FunctionSystem,
        Stage,
        FixedTimestep,
        Component,
        ComponentRegistry,
        TypedStorage,
        ComponentStorage,
        Archetype,
        ArchetypeGraph,
        ArchetypeId,
        Chunk,
        FrameAllocator,
        WorldAllocator,
        SparseSet,
        
        // Hiyerarşi
        HierarchyNode,
        HierarchySystem,
        
        // Girdi sistemi
        InputManager,
        InputState,
        InputRebindManager,
        InputRebindManagerBuilder,
        InputPreset,
        InputAction,
        InputBinding,
        KeyCode,
        KeyState,
        BindingSource,
        RebindResult,
        RebindTarget,
        MouseSettings,
        GamepadSettings,
        AccessibilitySettings,
        ButtonBehavior,
        OneHandedLayout,
        RumbleEffect,
        SmoothedValue,
        ControllerType,
        Platform,
        PresetDescription,
        
        // Kamera sistemi
        Camera,
        CameraProjection,
        FreeCameraController,
        FollowCamera,
        CameraMode,
        CameraModeController,
        
        // UI sistemi
        UiElement,
        UiElementType,
        UiEvent,
        UiSystem,
        UiStyle,
        Rect,
        
        // Ses sistemi
        AudioSource,
        AudioListener,
        AudioManager,
        AudioClip,
        MusicPlayer,
        AudioEngine,
        AudioSystem,
        AudioListenerComponent,
        AudioReverbZone,
        AudioEffectNode,
        EffectChain,
        MixerBus,
        MusicEngine,
        SpatialVoice,
        SpatialSettings,
        DistanceModel,
        VoiceHandle,
        
        // Animasyon sistemi
        AnimationPlayer,
        AnimationClip,
        AnimationBlender,
        Skeleton,
        Bone,
        Animator,
        IKGoal,
        AnimationSystem,
        AnimationState,
        AnimationChannel,
        Keyframe,
        InterpolationType,
        LoopMode,
        MorphTarget,
        RootMotion,
        BlendTree,
        TwoBoneIK,
        FabrikIK,
        CCDIK,
        LookAtIK,
        LimbIK,
        FullBodyIK,
        SkeletonRetargeting,
        BoneMapping,
        AnimationParameterValue,
        TransitionCondition,
        
        // Partikül sistemi
        ParticleSystem,
        Particle,
        ParticleSystemController,
        ParticleEffectFactory,
        
        // Fizik sistemi
        PhysicsWorld,
        PhysicsBody,
        PhysicsState,
        
        // Terrain sistemi
        TerrainManager,
        TerrainCoord,
        HeightProvider,
        
        // Ağ sistemi
        NetworkServer,
        NetworkClient,
        NetworkMessage,

        // Multiplayer / networking
        NetClient,
        NetServer,
        NetMessage,
        NetEvent,
        NetError,
        ClientId,
        ObjectId,
        SequenceNumber,
        RpcId,
        NetworkChannel,
        NetworkRole,
        NetRole,
        ConnectionState,
        EncryptionKey,
        Packet,
        PacketHeader,
        PacketFlags,
        ServerConfig,
        ClientConfig,
        NetServerConfig,
        NetClientConfig,
        ClientConnection,
        ConnectionManager,
        BanList,
        RateLimiter,
        AntiCheatConfig,
        NetInput,
        ClientPredictionBuffer,
        LagCompensationBuffer,
        RpcRegistry,
        ReplicatedState,
        ReplicationPriority,
        ReplicationFlags,
        ReplicationSystem,
        NetworkReplicationSystem,
        RelevanceSystem,
        Lobby,
        LobbyState,
        LobbySummary,
        RoomConfig,
        NatSession,
        Networked,
        NetworkTransform,
        NetworkState,
        PacketValidator,
        
        // ML sistemi
        MlAgent,
        SimpleNeuralNetwork,
        MlSystem,
        
        // Zamanlayıcı sistemi
        TimeSystem,
        
        // Komut sistemi
        CommandSystem,
        Command,
        
        // VR sistemi
        VrManager,
        VrConfig,
        VrRuntime,
        VrFrame,
        VrHand,
        VrController,
        VrTrackingSpace,

        // Yapılandırma
        EngineConfig,
        ConfigManager,
        
        // Kayıt / Profil
        LogLevel,
        LogSink,
        ConsoleSink,
        FileSink,
        FrameTiming,
        ProfileScope,
        FrameStats,
        MemoryStats,

        // Bellek yönetimi sistemi
        MemoryManager,
        MemoryProfiler,
        MemorySnapshot,
        MemoryDebugger,
        AllocationRecord,
        AllocationStats,
        GlobalAllocationTracker,
        GarbageCollector,
        GcObject,
        GcVisitor,
        GcStats,
        TracingGc,
        MarkAndSweepGc,
        GenerationalGc,
        IncrementalGc,
        PoolAllocator,
        LinearAllocator,
        StackAllocator,
        BuddyAllocator,
        SlabAllocator,
        RegionAllocator,
        DebugAllocator,
        ComponentAllocator,
        ArchetypeAllocator,
        ChunkAllocator,
        
        // Olay / Döngü
        GameLoop,
        EventDispatcher,
        ResourceManager,
        
        // Serileştirme
        WorldSnapshot,
        Serializable,
        SerializationFormat,

        // Yerelleştirme
        LocalizationManager,
        LocalizationConfig,
        Locale,
        Gender,
        PluralCategory,
        TranslationContext,
        TranslationEntry,
        TranslationDatabase,
        StringInterpolator,
        LocalizedTextAsset,
        LocalizedFontInfo,
        LocalizedAudioInfo,
        TextToSpeechEngine,
        TranslationExtractor,
        MissingTranslationReporter,
        MissingTranslationReport,
        TranslationValidator,
        UnicodeOptions,
        TextDirection,

        // Accessibility sistemi
        AccessibilitySettings,
        AccessibilityProfile,
        AccessibilityManager,
        ColorblindSettings,
        ColorblindType,
        ContrastMode,
        ColorblindCorrectionMode,
        ColorblindPalette,
        ColorblindSimulationFilter,
        SubtitleSettings,
        SubtitleTrack,
        SubtitleEntry,
        SubtitleStyle,
        CaptionStyle,
        SubtitlePosition,
        SubtitleLanguage,
        SubtitleSize,
        ScreenReaderSettings,
        ScreenReaderEvent,
        TextToSpeechSettings,
        TextToSpeechVoice,
        AriaAttributes,
        AriaRole,
        LiveRegionMode,
        InputAccessibilitySettings,
        InputRemapProfile,
        InputRemapEntry,
        VisualAccessibilitySettings,
        TextScalingMode,
        UIScalingMode,
        ReducedMotionLevel,
        HighContrastColorScheme,
        AudioAccessibilitySettings,
        AudioChannelMode,
        AudioDuckingSettings,
        VisualAudioIndicator,
        FrequencyRange,
        SpeechVerbosity,
        PunctuationLevel,
        AnnouncementPriority,

        // Yardımcılar
        Rng,
        Handle,
        Plane,
        Ray,
        Aabb,
        Frustum,
        Color,

        // Sinema sistemi
        Cutscene,
        CutsceneEffects,
        CinematicManager,
        CinematicEvent,
        CinematicEventType,
        CinematicParticipant,
        CinematicSystemResource,
        CutsceneSaveState,
        Timeline,
        TimelinePlayback,
        Track,
        TrackKind,
        Keyframe,
        KeyframeValue,
        KeyframeInterpolation,
        Director,
        Shot,
        FramingComposition,
        CameraSpline,
        SplineType,
        CameraShake,
        ShakeType,
        CameraTransition,
        CameraTransitionMode,

        // Çökme / Hata yönetimi
        CrashHandler,
        CrashReport,
        CrashUploadConfig,
        ErrorCategory,
        ErrorContext,
        CrashStatistics,
        Watchdog,

        // Job sistemi
        JobId,
        JobPriority,
        JobHandle,
        JobSystem,
        TaskGraph,
        TaskNode,
        SyncPoint,
        ParallelSystem,
        ParallelSystemAdapter,
        Access,
        ParallelIterator,
    };
}

// Ortak olarak kullanılan yardımcı fonksiyonlar
    pub fn create_default_world() -> World {
        World::new()
    }

// Hata işleme için ortak sonuç türü
#[derive(Debug)]
pub enum EngineResult<T> {
    Ok(T),
    Err(EngineError),
}

#[derive(Debug)]
pub enum EngineError {
    InitializationError(String),
    ResourceError(String),
    IOError(std::io::Error),
    SerializationError(String),
    ValidationError(String),
    RuntimeError(String),
}

impl<T> std::convert::From<EngineError> for EngineResult<T> {
    fn from(error: EngineError) -> Self {
        EngineResult::Err(error)
    }
}

impl<T> EngineResult<T> {
    pub fn is_ok(&self) -> bool {
        matches!(self, EngineResult::Ok(_))
    }
    
    pub fn is_err(&self) -> bool {
        matches!(self, EngineResult::Err(_))
    }
    
    pub fn unwrap(self) -> T {
        match self {
            EngineResult::Ok(value) => value,
            EngineResult::Err(error) => panic!("EngineResult unwrap error: {:?}", error),
        }
    }
    
    pub fn expect(self, msg: &str) -> T {
        match self {
            EngineResult::Ok(value) => value,
            EngineResult::Err(error) => panic!("{}: {:?}", msg, error),
        }
    }
    
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            EngineResult::Ok(value) => value,
            EngineResult::Err(_) => default,
        }
    }
    
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> EngineResult<U> {
        match self {
            EngineResult::Ok(value) => EngineResult::Ok(f(value)),
            EngineResult::Err(error) => EngineResult::Err(error),
        }
    }
    
    pub fn and_then<U, F: FnOnce(T) -> EngineResult<U>>(self, f: F) -> EngineResult<U> {
        match self {
            EngineResult::Ok(value) => f(value),
            EngineResult::Err(error) => EngineResult::Err(error),
        }
    }
}

impl From<std::io::Error> for EngineError {
    fn from(error: std::io::Error) -> Self {
        EngineError::IOError(error)
    }
}

impl From<bincode::Error> for EngineError {
    fn from(error: bincode::Error) -> Self {
        EngineError::SerializationError(format!("{:?}", error))
    }
}

impl From<serde_json::Error> for EngineError {
    fn from(error: serde_json::Error) -> Self {
        EngineError::SerializationError(format!("{:?}", error))
    }
}

// Temel oyun döngüsü için yapı
pub struct GameLoop {
    pub running: bool,
    pub target_fps: f64,
    pub frame_time: std::time::Duration,
    pub last_update: std::time::Instant,
    pub accumulator: std::time::Duration,
    pub max_frame_skip: u32,
}

impl GameLoop {
    pub fn new(target_fps: f64) -> Self {
        Self {
            running: true,
            target_fps,
            frame_time: std::time::Duration::from_secs_f64(1.0 / target_fps),
            last_update: std::time::Instant::now(),
            accumulator: std::time::Duration::new(0, 0),
            max_frame_skip: 5,
        }
    }
    
    pub fn update<F>(&mut self, mut update_func: F) -> bool
    where
        F: FnMut(f32),
    {
        if !self.running {
            return false;
        }
        
        let now = std::time::Instant::now();
        let delta_time = now - self.last_update;
        self.last_update = now;
        
        self.accumulator += delta_time;
        
        let mut updates = 0;
        while self.accumulator >= self.frame_time && updates < self.max_frame_skip {
            let dt = self.frame_time.as_secs_f32();
            update_func(dt);
            self.accumulator -= self.frame_time;
            updates += 1;
        }
        
        true
    }
    
    pub fn stop(&mut self) {
        self.running = false;
    }
    
    pub fn start(&mut self) {
        self.running = true;
        self.last_update = std::time::Instant::now();
    }
    
    pub fn set_target_fps(&mut self, fps: f64) {
        self.target_fps = fps;
        self.frame_time = std::time::Duration::from_secs_f64(1.0 / fps);
    }
    
    pub fn get_target_fps(&self) -> f64 {
        self.target_fps
    }
    
    pub fn get_frame_time(&self) -> std::time::Duration {
        self.frame_time
    }
    
    pub fn get_delta_time(&self) -> f32 {
        self.frame_time.as_secs_f32()
    }
}

// Olay sistemi için ortak yapılar
pub trait EventHandler<T> {
    fn handle(&mut self, event: T);
}

pub struct EventDispatcher<T> {
    handlers: Vec<Box<dyn EventHandler<T>>>,
}

impl<T> EventDispatcher<T> {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }
    
    pub fn add_handler(&mut self, handler: Box<dyn EventHandler<T>>) {
        self.handlers.push(handler);
    }
    
    pub fn dispatch(&mut self, event: T) 
    where
        T: Clone,
    {
        for handler in &mut self.handlers {
            handler.handle(event.clone());
        }
    }
    
    pub fn clear_handlers(&mut self) {
        self.handlers.clear();
    }
    
    pub fn handler_count(&self) -> usize {
        self.handlers.len()
    }
}

// Kaynak yönetimi için ortak yapı
pub struct ResourceManager<T> {
    resources: std::collections::HashMap<String, T>,
    loaders: std::collections::HashMap<String, Box<dyn Fn(&str) -> Option<T>>>,
}

impl<T> ResourceManager<T> {
    pub fn new() -> Self {
        Self {
            resources: std::collections::HashMap::new(),
            loaders: std::collections::HashMap::new(),
        }
    }
    
    pub fn register_loader<F>(&mut self, extension: &str, loader: F)
    where
        F: Fn(&str) -> Option<T> + 'static,
    {
        self.loaders.insert(extension.to_string(), Box::new(loader));
    }
    
        pub fn load(&mut self, name: &str, path: &str) -> Option<&T> {
        if self.resources.contains_key(name) {
            return self.resources.get(name);
        }

        // Uzantıyı al ve uygun yükleyiciyi bul
        if let Some(ext) = std::path::Path::new(path).extension().and_then(|s| s.to_str()) {
            if let Some(loader) = self.loaders.get(ext) {
                if let Some(resource) = loader(path) {
                    self.resources.insert(name.to_string(), resource);
                    return self.resources.get(name);
                }
            }
        }

        None
    }
    
    pub fn get(&self, name: &str) -> Option<&T> {
        self.resources.get(name)
    }
    
    pub fn get_mut(&mut self, name: &str) -> Option<&mut T> {
        self.resources.get_mut(name)
    }
    
    pub fn insert(&mut self, name: String, resource: T) {
        self.resources.insert(name, resource);
    }
    
    pub fn remove(&mut self, name: &str) -> Option<T> {
        self.resources.remove(name)
    }
    
    pub fn contains(&self, name: &str) -> bool {
        self.resources.contains_key(name)
    }
    
    pub fn clear(&mut self) {
        self.resources.clear();
    }
    
    pub fn len(&self) -> usize {
        self.resources.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
    }
    
    pub fn keys(&self) -> std::collections::hash_map::Keys<String, T> {
        self.resources.keys()
    }
}