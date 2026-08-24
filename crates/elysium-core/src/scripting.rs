use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::any::Any;

/// Script durumu
#[derive(Debug, Clone, PartialEq)]
pub enum ScriptStatus {
    Loading,
    Loaded,
    Compiling,
    Compiled,
    Running,
    Paused,
    Stopped,
    Error(String),
}

/// Script türü
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScriptLanguage {
    Lua,
    JavaScript,
    Python,
    Rust,
    Custom(String),
}

/// Script bileşeni
#[derive(Debug, Clone)]
pub struct ScriptComponent {
    pub name: String,
    pub language: ScriptLanguage,
    pub source_code: String,
    pub status: ScriptStatus,
    pub auto_start: bool,
    pub persistent: bool,
}

impl Default for ScriptComponent {
    fn default() -> Self {
        Self {
            name: String::new(),
            language: ScriptLanguage::Lua,
            source_code: String::new(),
            status: ScriptStatus::Loading,
            auto_start: true,
            persistent: false,
        }
    }
}

/// Script değişkeni
#[derive(Debug)]
pub enum ScriptValue {
    Nil,
    Boolean(bool),
    Number(f64),
    String(String),
    Table(HashMap<String, ScriptValue>),
    Function(String), // Function name
    UserData(Box<dyn Any + Send + Sync>),
}

impl Clone for ScriptValue {
    fn clone(&self) -> Self {
        match self {
            ScriptValue::Nil => ScriptValue::Nil,
            ScriptValue::Boolean(b) => ScriptValue::Boolean(*b),
            ScriptValue::Number(n) => ScriptValue::Number(*n),
            ScriptValue::String(s) => ScriptValue::String(s.clone()),
            ScriptValue::Table(t) => ScriptValue::Table(t.clone()),
            ScriptValue::Function(f) => ScriptValue::Function(f.clone()),
            ScriptValue::UserData(_) => ScriptValue::Nil,
        }
    }
}

impl ScriptValue {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ScriptValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            ScriptValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<String> {
        match self {
            ScriptValue::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    pub fn as_table(&self) -> Option<HashMap<String, ScriptValue>> {
        match self {
            ScriptValue::Table(t) => Some(t.clone()),
            _ => None,
        }
    }
}

/// Script bağlamı
#[derive(Clone)]
pub struct ScriptContext {
    pub variables: HashMap<String, ScriptValue>,
    pub functions: HashMap<String, Arc<dyn Fn(&[ScriptValue]) -> Result<ScriptValue, String> + Send + Sync>>,
    pub entity_id: Option<crate::Entity>,
}

impl Default for ScriptContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptContext {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            functions: HashMap::new(),
            entity_id: None,
        }
    }

    pub fn set_variable(&mut self, name: String, value: ScriptValue) {
        self.variables.insert(name, value);
    }

    pub fn get_variable(&self, name: &str) -> Option<&ScriptValue> {
        self.variables.get(name)
    }

    pub fn register_function<F>(&mut self, name: String, func: F) 
    where
        F: Fn(&[ScriptValue]) -> Result<ScriptValue, String> + Send + Sync + 'static,
    {
        self.functions.insert(name, Arc::new(func));
    }
}

/// Script motoru
pub struct ScriptEngine {
    pub contexts: HashMap<crate::Entity, ScriptContext>,
    pub global_context: ScriptContext,
    pub registered_scripts: HashMap<String, ScriptComponent>,
    pub running_scripts: HashMap<String, ScriptInstance>,
    pub script_language_support: HashMap<ScriptLanguage, Box<dyn ScriptRunner>>,
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            contexts: HashMap::new(),
            global_context: ScriptContext::new(),
            registered_scripts: HashMap::new(),
            running_scripts: HashMap::new(),
            script_language_support: HashMap::new(),
        };

        // Varsayılan olarak Lua destekleyici ekle
        engine.register_language_support(ScriptLanguage::Lua, Box::new(LuaScriptRunner::new()));

        engine
    }

    /// Script dili desteğini kaydet
    pub fn register_language_support(&mut self, language: ScriptLanguage, runner: Box<dyn ScriptRunner>) {
        self.script_language_support.insert(language, runner);
    }

    /// Script bileşeni oluştur
    pub fn create_script_component(&mut self, name: String, source_code: String, language: ScriptLanguage) -> ScriptComponent {
        ScriptComponent {
            name: name.clone(),
            language: language.clone(),
            source_code,
            status: ScriptStatus::Loaded,
            auto_start: true,
            persistent: false,
        }
    }

    /// Script bileşeni kaydet
    pub fn register_script(&mut self, script: ScriptComponent) -> Result<(), String> {
        let name = script.name.clone();
        self.registered_scripts.insert(name, script);
        Ok(())
    }

    /// Script başlat
    pub fn start_script(&mut self, script_name: &str, entity_id: Option<crate::Entity>) -> Result<(), String> {
        let script = self.registered_scripts.get(script_name).ok_or_else(|| 
            format!("Script not found: {}", script_name)
        )?.clone();

        if script.status != ScriptStatus::Compiled && script.status != ScriptStatus::Loaded {
            return Err(format!("Script is not ready to run: {:?}", script.status));
        }

        let context = entity_id
            .and_then(|id| self.contexts.get(&id))
            .unwrap_or(&self.global_context)
            .clone();

        let instance = ScriptInstance {
            script,
            context,
            entity_id,
            status: ScriptStatus::Running,
        };

        self.running_scripts.insert(script_name.to_string(), instance);
        Ok(())
    }

    /// Script durdur
    pub fn stop_script(&mut self, script_name: &str) -> Result<(), String> {
        if self.running_scripts.remove(script_name).is_some() {
            Ok(())
        } else {
            Err(format!("Script is not running: {}", script_name))
        }
    }

    /// Scripti duraklat
    pub fn pause_script(&mut self, script_name: &str) -> Result<(), String> {
        if let Some(instance) = self.running_scripts.get_mut(script_name) {
            instance.status = ScriptStatus::Paused;
            Ok(())
        } else {
            Err(format!("Script is not running: {}", script_name))
        }
    }

    /// Scripti devam ettir
    pub fn resume_script(&mut self, script_name: &str) -> Result<(), String> {
        if let Some(instance) = self.running_scripts.get_mut(script_name) {
            if instance.status == ScriptStatus::Paused {
                instance.status = ScriptStatus::Running;
                Ok(())
            } else {
                Err(format!("Script is not paused: {}", script_name))
            }
        } else {
            Err(format!("Script is not running: {}", script_name))
        }
    }

    /// Script bağlamı al
    pub fn get_context(&self, entity_id: crate::Entity) -> Option<&ScriptContext> {
        self.contexts.get(&entity_id)
    }

    /// Script bağlamı al (mutable)
    pub fn get_context_mut(&mut self, entity_id: crate::Entity) -> Option<&mut ScriptContext> {
        self.contexts.get_mut(&entity_id)
    }

    /// Script bağlamı oluştur
    pub fn create_context(&mut self, entity_id: crate::Entity) -> &mut ScriptContext {
        self.contexts.entry(entity_id).or_insert_with(ScriptContext::new)
    }

    /// Tüm çalışan scriptleri güncelle
    pub fn update(&mut self, delta_time: f32) -> Result<(), String> {
        let scripts_to_update: Vec<String> = self.running_scripts
            .iter()
            .filter(|(_, instance)| instance.status == ScriptStatus::Running)
            .map(|(name, _)| name.clone())
            .collect();

        for script_name in scripts_to_update {
            self.update_script(&script_name, delta_time)?;
        }

        Ok(())
    }

    /// Tek bir scripti güncelle
    pub fn update_script(&mut self, script_name: &str, delta_time: f32) -> Result<(), String> {
        if let Some(instance) = self.running_scripts.get_mut(script_name) {
            if instance.status == ScriptStatus::Running {
                // Gerçek script motoru burada çalıştırılacaktı
                // Şimdilik sadece durumu kontrol ediyoruz
                Ok(())
            } else {
                Err(format!("Script is not running: {}", script_name))
            }
        } else {
            Err(format!("Script not found: {}", script_name))
        }
    }
}

/// Script çalıştırıcı trait'i
pub trait ScriptRunner: Send + Sync {
    fn compile(&self, source: &str) -> Result<(), String>;
    fn execute(&self, context: &ScriptContext) -> Result<ScriptValue, String>;
    fn call_function(&self, function_name: &str, args: &[ScriptValue]) -> Result<ScriptValue, String>;
    fn set_variable(&self, name: &str, value: ScriptValue) -> Result<(), String>;
    fn get_variable(&self, name: &str) -> Result<ScriptValue, String>;
}

/// Lua script çalıştırıcı (örnek implementasyon)
pub struct LuaScriptRunner {
    // Gerçek Lua bağlamı burada olurdu
    // lua_context: mlua::Lua,
}

impl LuaScriptRunner {
    pub fn new() -> Self {
        Self {}
    }
}

impl ScriptRunner for LuaScriptRunner {
    fn compile(&self, _source: &str) -> Result<(), String> {
        // Gerçek implementasyon burada olurdu
        Ok(())
    }

    fn execute(&self, _context: &ScriptContext) -> Result<ScriptValue, String> {
        // Gerçek implementasyon burada olurdu
        Ok(ScriptValue::Nil)
    }

    fn call_function(&self, _function_name: &str, _args: &[ScriptValue]) -> Result<ScriptValue, String> {
        // Gerçek implementasyon burada olurdu
        Ok(ScriptValue::Nil)
    }

    fn set_variable(&self, _name: &str, _value: ScriptValue) -> Result<(), String> {
        // Gerçek implementasyon burada olurdu
        Ok(())
    }

    fn get_variable(&self, _name: &str) -> Result<ScriptValue, String> {
        // Gerçek implementasyon burada olurdu
        Ok(ScriptValue::Nil)
    }
}

/// Script örneği
pub struct ScriptInstance {
    pub script: ScriptComponent,
    pub context: ScriptContext,
    pub entity_id: Option<crate::Entity>,
    pub status: ScriptStatus,
}

/// Script yardımcı fonksiyonları
pub mod helpers {
    use super::*;

    /// Script ile ECS entegrasyonu için yardımcı fonksiyonlar
    pub fn get_entity_position(engine: &ScriptEngine, entity_id: crate::Entity) -> Option<crate::math::Vec3> {
        // Bu fonksiyon ECS sisteminden pozisyonu alır
        // Gerçek implementasyon burada olurdu
        None
    }

    /// Script ile ECS entegrasyonu için yardımcı fonksiyonlar
    pub fn set_entity_position(engine: &mut ScriptEngine, entity_id: crate::Entity, position: crate::math::Vec3) -> Result<(), String> {
        // Bu fonksiyon ECS sistemine pozisyonu yazar
        // Gerçek implementasyon burada olurdu
        Ok(())
    }

    /// Script ile input sistemi entegrasyonu
    pub fn is_key_pressed(engine: &ScriptEngine, key: &str) -> bool {
        // Gerçek implementasyon burada olurdu
        false
    }

    /// Script ile fizik sistemi entegrasyonu
    pub fn apply_force_to_entity(engine: &mut ScriptEngine, entity_id: crate::Entity, force: crate::math::Vec3) -> Result<(), String> {
        // Gerçek implementasyon burada olurdu
        Ok(())
    }
}

/// Script sistemini başlatan yardımcı fonksiyon
pub fn initialize_script_system() -> ScriptEngine {
    let mut engine = ScriptEngine::new();
    
    // Genel fonksiyonları kaydet
    engine.global_context.register_function("print".to_string(), |args| {
        if let Some(ScriptValue::String(msg)) = args.first() {
            println!("{}", msg);
        }
        Ok(ScriptValue::Nil)
    });
    
    engine.global_context.register_function("wait".to_string(), |args| {
        // Zaman bekleme fonksiyonu (gerçek implementasyon daha karmaşık olurdu)
        Ok(ScriptValue::Nil)
    });
    
    engine
}