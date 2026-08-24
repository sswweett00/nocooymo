use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::any::Any;

/// Komut durumu
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommandStatus {
    Pending,
    Executing,
    Success,
    Failed,
    Cancelled,
}

/// Komut türü
#[derive(Debug, Clone, PartialEq)]
pub enum CommandType {
    Immediate,
    Delayed,
    Conditional,
    Sequential,
    Parallel,
    UndoRedo,
}

/// Komut tanımı
pub trait Command: Send + Sync {
    fn execute(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn undo(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn can_undo(&self) -> bool;
    fn status(&self) -> CommandStatus;
}

/// Basit bir komut yapısı
pub struct SimpleCommand<F, U>
where
    F: Fn() -> Result<(), Box<dyn std::error::Error>> + Send + Sync,
    U: Fn() -> Result<(), Box<dyn std::error::Error>> + Send + Sync,
{
    pub name: String,
    pub description: String,
    pub execute_func: F,
    pub undo_func: U,
    pub status: CommandStatus,
    pub can_be_undone: bool,
}

impl<F, U> SimpleCommand<F, U>
where
    F: Fn() -> Result<(), Box<dyn std::error::Error>> + Send + Sync,
    U: Fn() -> Result<(), Box<dyn std::error::Error>> + Send + Sync,
{
    pub fn new(name: String, description: String, execute_func: F, undo_func: U, can_be_undone: bool) -> Self {
        Self {
            name,
            description,
            execute_func,
            undo_func,
            status: CommandStatus::Pending,
            can_be_undone: can_be_undone,
        }
    }
}

impl<F, U> Command for SimpleCommand<F, U>
where
    F: Fn() -> Result<(), Box<dyn std::error::Error>> + Send + Sync,
    U: Fn() -> Result<(), Box<dyn std::error::Error>> + Send + Sync,
{
    fn execute(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.status = CommandStatus::Executing;
        let result = (self.execute_func)();
        self.status = match result {
            Ok(()) => {
                CommandStatus::Success
            }
            Err(_) => {
                CommandStatus::Failed
            }
        };
        result
    }

    fn undo(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.can_be_undone {
            return Err("Command cannot be undone".into());
        }
        (self.undo_func)()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn can_undo(&self) -> bool {
        self.can_be_undone
    }

    fn status(&self) -> CommandStatus {
        self.status
    }
}

/// Komut argümanları
#[derive(Debug, Clone)]
pub struct CommandArgs {
    pub params: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl CommandArgs {
    pub fn new() -> Self {
        Self {
            params: HashMap::new(),
        }
    }

    pub fn add<T: 'static + Send + Sync>(&mut self, key: String, value: T) {
        self.params.insert(key, Box::new(value));
    }

    pub fn get<T: 'static>(&self, key: &str) -> Option<&T> {
        self.params.get(key).and_then(|boxed| boxed.downcast_ref::<T>())
    }
}

/// Komut işleyici
pub type CommandHandler = Box<dyn Fn(&CommandArgs) -> Result<(), Box<dyn std::error::Error>> + Send + Sync>;

/// Komut sistemi yapılandırması
#[derive(Debug, Clone)]
pub struct CommandSystemConfig {
    pub max_history_size: usize,
    pub enable_undo_redo: bool,
    pub enable_command_queue: bool,
    pub enable_logging: bool,
}

impl Default for CommandSystemConfig {
    fn default() -> Self {
        Self {
            max_history_size: 100,
            enable_undo_redo: true,
            enable_command_queue: true,
            enable_logging: true,
        }
    }
}

/// Komut sistemi
pub struct CommandSystem {
    pub commands: HashMap<String, Box<dyn Command>>,
    pub command_queue: Vec<String>,
    pub history: Vec<String>,
    pub undo_stack: Vec<String>,
    pub redo_stack: Vec<String>,
    pub config: CommandSystemConfig,
    pub command_handlers: HashMap<String, CommandHandler>,
    pub aliases: HashMap<String, String>,
    pub executing_command: Option<String>,
    pub command_results: HashMap<String, Result<(), Box<dyn std::error::Error>>>,
}

impl Default for CommandSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandSystem {
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
            command_queue: Vec::new(),
            history: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            config: CommandSystemConfig::default(),
            command_handlers: HashMap::new(),
            aliases: HashMap::new(),
            executing_command: None,
            command_results: HashMap::new(),
        }
    }

    /// Yeni bir komut ekle
    pub fn add_command(&mut self, name: String, command: Box<dyn Command>) {
        self.commands.insert(name, command);
    }

    /// Komut çalıştır
    pub fn execute_command(&mut self, name: &str, args: Option<CommandArgs>) -> Result<(), Box<dyn std::error::Error>> {
        let actual_name = self.aliases.get(name).unwrap_or(&name.to_string());
        
        if let Some(command) = self.commands.get_mut(actual_name) {
            let result = command.execute();
            
            // Komut geçmişini güncelle
            if self.config.enable_undo_redo && command.can_undo() {
                self.history.push(actual_name.clone());
                if self.history.len() > self.config.max_history_size {
                    self.history.remove(0);
                }
                
                self.undo_stack.push(actual_name.clone());
                self.redo_stack.clear(); // Yeni komut çalıştırıldığında redo geçmişi silinir
            }
            
            self.command_results.insert(actual_name.clone(), result.clone());
            
            if self.config.enable_logging {
                match &result {
                    Ok(()) => println!("Command '{}' executed successfully", actual_name),
                    Err(e) => eprintln!("Command '{}' failed: {}", actual_name, e),
                }
            }
            
            result
        } else if let Some(handler) = self.command_handlers.get(actual_name) {
            // Handler tabanlı komut çalıştır
            let args = args.unwrap_or(CommandArgs::new());
            let result = handler(&args);
            
            self.command_results.insert(actual_name.clone(), result.clone());
            
            if self.config.enable_logging {
                match &result {
                    Ok(()) => println!("Command '{}' executed successfully", actual_name),
                    Err(e) => eprintln!("Command '{}' failed: {}", actual_name, e),
                }
            }
            
            result
        } else {
            Err(format!("Command '{}' not found", actual_name).into())
        }
    }

    /// Komutu sıraya al
    pub fn queue_command(&mut self, name: String, args: Option<CommandArgs>) -> Result<(), Box<dyn std::error::Error>> {
        if !self.config.enable_command_queue {
            return Err("Command queue is disabled".into());
        }
        
        if self.commands.contains_key(&name) || self.command_handlers.contains_key(&name) {
            // Argümanları sakla (bu basitleştirilmiş bir versiyon)
            self.command_queue.push(name);
            Ok(())
        } else {
            Err(format!("Command '{}' not found", name).into())
        }
    }

    /// Sıradaki komutu çalıştır
    pub fn process_next_queued_command(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        if !self.config.enable_command_queue {
            return Err("Command queue is disabled".into());
        }
        
        if let Some(command_name) = self.command_queue.pop() {
            self.execute_command(&command_name, None)?;
            Ok(true)
        } else {
            Ok(false) // Sırada komut yok
        }
    }

    /// Tüm sıradaki komutları işle
    pub fn process_all_queued_commands(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        while self.process_next_queued_command()? {}
        Ok(())
    }

    /// Komutu geri al
    pub fn undo_last_command(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.config.enable_undo_redo {
            return Err("Undo/redo is disabled".into());
        }
        
        if let Some(command_name) = self.undo_stack.pop() {
            if let Some(command) = self.commands.get_mut(&command_name) {
                if command.can_undo() {
                    let result = command.undo();
                    
                    if self.config.enable_logging {
                        match &result {
                            Ok(()) => println!("Command '{}' undone successfully", command_name),
                            Err(e) => eprintln!("Command '{}' undo failed: {}", command_name, e),
                        }
                    }
                    
                    self.redo_stack.push(command_name);
                    result
                } else {
                    self.undo_stack.push(command_name);
                    Err("Last command cannot be undone".into())
                }
            } else {
                self.undo_stack.push(command_name);
                Err("Command not found in registry".into())
            }
        } else {
            Err("No commands to undo".into())
        }
    }

    /// Komutu yeniden yap
    pub fn redo_last_command(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.config.enable_undo_redo {
            return Err("Undo/redo is disabled".into());
        }
        
        if let Some(command_name) = self.redo_stack.pop() {
            if let Some(command) = self.commands.get_mut(&command_name) {
                let result = command.execute();
                
                if self.config.enable_logging {
                    match &result {
                        Ok(()) => println!("Command '{}' redone successfully", command_name),
                        Err(e) => eprintln!("Command '{}' redo failed: {}", command_name, e),
                    }
                }
                
                self.undo_stack.push(command_name);
                result
            } else {
                self.redo_stack.push(command_name);
                Err("Command not found in registry".into())
            }
        } else {
            Err("No commands to redo".into())
        }
    }

    /// Komut işleyici ekle
    pub fn add_command_handler(&mut self, name: String, handler: CommandHandler) {
        self.command_handlers.insert(name, handler);
    }

    /// Komut için takma ad ekle
    pub fn add_alias(&mut self, alias: String, command_name: String) {
        self.aliases.insert(alias, command_name);
    }

    /// Komutu iptal et
    pub fn cancel_command(&mut self, name: &str) -> bool {
        if let Some(command) = self.commands.get_mut(name) {
            if command.status() == CommandStatus::Executing {
                // Komut iptal edilemiyor çünkü bu örnek implementasyonda sadece statüyü güncelliyoruz
                // Gerçek implementasyonda komutun kendisi iptal mekanizmasına sahip olmalı
            }
            true
        } else {
            false
        }
    }

    /// Komutun durumunu al
    pub fn get_command_status(&self, name: &str) -> Option<CommandStatus> {
        if let Some(command) = self.commands.get(name) {
            Some(command.status())
        } else {
            self.command_results.get(name).map(|result| {
                match result {
                    Ok(()) => CommandStatus::Success,
                    Err(_) => CommandStatus::Failed,
                }
            })
        }
    }

    /// Komut geçmişini temizle
    pub fn clear_history(&mut self) {
        self.history.clear();
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Komut sayısını al
    pub fn command_count(&self) -> usize {
        self.commands.len()
    }

    /// Komut isimlerini al
    pub fn command_names(&self) -> Vec<String> {
        self.commands.keys().cloned().collect()
    }

    /// Komut açıklamasını al
    pub fn get_command_description(&self, name: &str) -> Option<String> {
        if let Some(command) = self.commands.get(name) {
            Some(command.description().to_string())
        } else {
            None
        }
    }

    /// Komutun geri alınabilir olup olmadığını kontrol et
    pub fn can_undo_command(&self, name: &str) -> bool {
        if let Some(command) = self.commands.get(name) {
            command.can_undo()
        } else {
            false
        }
    }

    /// Sistem yapılandırmasını güncelle
    pub fn set_config(&mut self, config: CommandSystemConfig) {
        self.config = config;
    }

    /// Komut sonuçlarını al
    pub fn get_command_result(&self, name: &str) -> Option<&Result<(), Box<dyn std::error::Error>>> {
        self.command_results.get(name)
    }
}

/// Komut zinciri (komutları sırayla çalıştıran özel komut)
pub struct CommandChain {
    pub commands: Vec<String>,
    pub current_index: usize,
    pub name: String,
    pub description: String,
}

impl CommandChain {
    pub fn new(name: String, description: String) -> Self {
        Self {
            commands: Vec::new(),
            current_index: 0,
            name,
            description,
        }
    }

    pub fn add_command(&mut self, command_name: String) {
        self.commands.push(command_name);
    }
}

impl Command for CommandChain {
    fn execute(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Tüm komutları sırayla çalıştır
        for command_name in &self.commands {
            println!("Executing chained command: {}", command_name);
            // Gerçek komut çalıştırma burada olurdu
        }
        Ok(())
    }

    fn undo(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Komutları tersten geri al
        for i in (0..self.commands.len()).rev() {
            println!("Undoing chained command: {}", self.commands[i]);
            // Gerçek komut geri alma burada olurdu
        }
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn can_undo(&self) -> bool {
        true // Zincir komutları genellikle geri alınabilir olur
    }

    fn status(&self) -> CommandStatus {
        if self.current_index >= self.commands.len() {
            CommandStatus::Success
        } else {
            CommandStatus::Executing
        }
    }
}

/// Koşullu komut (belirli bir koşul sağlandığında çalışan komut)
pub struct ConditionalCommand {
    pub condition: Box<dyn Fn() -> bool + Send + Sync>,
    pub positive_command: Box<dyn Command>,
    pub negative_command: Option<Box<dyn Command>>,
    pub name: String,
    pub description: String,
}

impl ConditionalCommand {
    pub fn new(
        name: String,
        description: String,
        condition: Box<dyn Fn() -> bool + Send + Sync>,
        positive_command: Box<dyn Command>,
        negative_command: Option<Box<dyn Command>>,
    ) -> Self {
        Self {
            condition,
            positive_command,
            negative_command,
            name,
            description,
        }
    }
}

impl Command for ConditionalCommand {
    fn execute(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if (self.condition)() {
            self.positive_command.execute()
        } else if let Some(ref mut negative_cmd) = self.negative_command {
            negative_cmd.execute()
        } else {
            Ok(())
        }
    }

    fn undo(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if (self.condition)() {
            self.positive_command.undo()
        } else if let Some(ref mut negative_cmd) = self.negative_command {
            negative_cmd.undo()
        } else {
            Ok(())
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn can_undo(&self) -> bool {
        self.positive_command.can_undo() && 
        self.negative_command.as_ref().map_or(true, |cmd| cmd.can_undo())
    }

    fn status(&self) -> CommandStatus {
        CommandStatus::Pending
    }
}

/// Komut yardımcı fonksiyonları
pub mod helpers {
    use super::*;

    /// Basit bir komut oluşturucu
    pub fn create_simple_command<F, U>(
        name: String,
        description: String,
        execute_func: F,
        undo_func: U,
        can_be_undone: bool,
    ) -> Box<dyn Command>
    where
        F: Fn() -> Result<(), Box<dyn std::error::Error>> + Send + Sync + 'static,
        U: Fn() -> Result<(), Box<dyn std::error::Error>> + Send + Sync + 'static,
    {
        Box::new(SimpleCommand::new(name, description, execute_func, undo_func, can_be_undone))
    }

    /// Koşullu komut oluşturucu
    pub fn create_conditional_command(
        name: String,
        description: String,
        condition: Box<dyn Fn() -> bool + Send + Sync>,
        positive_command: Box<dyn Command>,
        negative_command: Option<Box<dyn Command>>,
    ) -> Box<dyn Command> {
        Box::new(ConditionalCommand::new(name, description, condition, positive_command, negative_command))
    }

    /// Komut zinciri oluşturucu
    pub fn create_command_chain(name: String, description: String) -> Box<dyn Command> {
        Box::new(CommandChain::new(name, description))
    }

    /// Komut argümanı oluşturucu
    pub fn create_command_args() -> CommandArgs {
        CommandArgs::new()
    }
}

/// Komut sistemleri için önceden tanımlanmış komutlar
pub mod presets {
    use super::*;

    /// Test komutu
    pub fn test_command() -> Box<dyn Command> {
        helpers::create_simple_command(
            "test".to_string(),
            "Test command for verification".to_string(),
            || {
                println!("Test command executed!");
                Ok(())
            },
            || {
                println!("Test command undone!");
                Ok(())
            },
            true,
        )
    }

    /// Kaydet komutu
    pub fn save_command() -> Box<dyn Command> {
        helpers::create_simple_command(
            "save".to_string(),
            "Save current state".to_string(),
            || {
                println!("Saving current state...");
                // Gerçek kaydetme işlemi burada olurdu
                Ok(())
            },
            || {
                println!("Save operation undone");
                Ok(())
            },
            false, // Kaydetme işlemi genellikle geri alınamaz
        )
    }

    /// Yükle komutu
    pub fn load_command() -> Box<dyn Command> {
        helpers::create_simple_command(
            "load".to_string(),
            "Load saved state".to_string(),
            || {
                println!("Loading saved state...");
                // Gerçek yükleme işlemi burada olurdu
                Ok(())
            },
            || {
                println!("Load operation undone");
                Ok(())
            },
            false,
        )
    }

    /// Hareket komutu
    pub fn move_command(x: f32, y: f32, z: f32) -> Box<dyn Command> {
        let pos = (x, y, z);
        helpers::create_simple_command(
            format!("move_{}_{}_{}", x, y, z),
            format!("Move to position ({}, {}, {})", x, y, z),
            move || {
                println!("Moving to position: ({}, {}, {})", pos.0, pos.1, pos.2);
                Ok(())
            },
            move || {
                println!("Move operation to ({}, {}, {}) undone", pos.0, pos.1, pos.2);
                Ok(())
            },
            true,
        )
    }
}

/// Komut sistemini başlatan yardımcı fonksiyon
pub fn initialize_command_system() -> CommandSystem {
    let mut system = CommandSystem::new();
    
    // Bazı öntanımlı komutları ekle
    system.add_command("test".to_string(), presets::test_command());
    system.add_command("save".to_string(), presets::save_command());
    system.add_command("load".to_string(), presets::load_command());
    
    // Takma adlar ekle
    system.add_alias("s".to_string(), "save".to_string());
    system.add_alias("l".to_string(), "load".to_string());
    
    system
}

use crate::{CommandBuffer, World, System, Entity};

pub struct CommandBufferSystem;

impl System for CommandBufferSystem {
    fn run(&mut self, world: &mut World) {
        // World'deki tüm CommandBuffer'ları işle
        let mut command_buffers = Vec::new();
        
        // Aktif command buffer'ları topla
        if let Some(active_buffer) = world.get_active_command_buffer() {
            command_buffers.push(active_buffer.clone());
        }
        
        // Her bir command buffer'ı işle
        for mut cmd_buf in command_buffers {
            // Spawn komutlarını işle
            for spawn_cmd in cmd_buf.spawn_commands.drain(..) {
                let entity = world.spawn();
                // Component eklemelerini uygula
                for (component_id, component_data) in spawn_cmd.components {
                    world.insert_component_by_id(entity, component_id, component_data);
                }
            }
            
            // Despawn komutlarını işle
            for entity in cmd_buf.despawn_commands.drain(..) {
                world.despawn(entity);
            }
            
            // Component ekle/kaldır komutlarını işle
            for comp_cmd in cmd_buf.component_commands.drain(..) {
                match comp_cmd.action {
                    ComponentAction::Add => {
                        world.insert_component_by_id(comp_cmd.entity, comp_cmd.component_id, comp_cmd.data);
                    }
                    ComponentAction::Remove => {
                        world.remove_component_by_id(comp_cmd.entity, comp_cmd.component_id);
                    }
                    ComponentAction::Update => {
                        world.update_component_by_id(comp_cmd.entity, comp_cmd.component_id, comp_cmd.data);
                    }
                }
            }
            
            // Hierarchy komutlarını işle
            for hier_cmd in cmd_buf.hierarchy_commands.drain(..) {
                match hier_cmd.action {
                    HierarchyAction::AddChild => {
                        world.add_child(hier_cmd.parent, hier_cmd.child);
                    }
                    HierarchyAction::RemoveChild => {
                        world.remove_child(hier_cmd.parent, hier_cmd.child);
                    }
                    HierarchyAction::SetParent => {
                        world.set_parent(hier_cmd.child, hier_cmd.new_parent);
                    }
                }
            }
        }
    }
}

// Komut sistemi için yardımcı yapılar
#[derive(Debug, Clone)]
pub enum ComponentAction {
    Add,
    Remove,
    Update,
}

#[derive(Debug, Clone)]
pub struct ComponentCommand {
    pub entity: Entity,
    pub component_id: u32,
    pub data: Box<dyn std::any::Any + Send + Sync>,
    pub action: ComponentAction,
}

#[derive(Debug, Clone)]
pub enum HierarchyAction {
    AddChild,
    RemoveChild,
    SetParent,
}

#[derive(Debug, Clone)]
pub struct HierarchyCommand {
    pub parent: Entity,
    pub child: Entity,
    pub new_parent: Option<Entity>,
    pub action: HierarchyAction,
}

// CommandBuffer genişletme
impl CommandBuffer {
    /// Bir varlığı hiyerarşiden kaldırır ama silmez
    pub fn remove_from_hierarchy(&mut self, entity: Entity) -> &mut Self {
        if let Some(parent) = self.world.get_entity_parent(entity) {
            let cmd = HierarchyCommand {
                parent,
                child: entity,
                new_parent: None,
                action: HierarchyAction::RemoveChild,
            };
            self.hierarchy_commands.push(cmd);
        }
        self
    }
    
    /// Varlığın parent'ını değiştirir
    pub fn reparent(&mut self, entity: Entity, new_parent: Entity) -> &mut Self {
        let cmd = HierarchyCommand {
            parent: new_parent, // Eski parent burada değil, ancak sistemde izlenecek
            child: entity,
            new_parent: Some(new_parent),
            action: HierarchyAction::SetParent,
        };
        self.hierarchy_commands.push(cmd);
        self
    }
    
    /// Toplu olarak component ekler
    pub fn insert_components_batch(&mut self, entity: Entity, components: Vec<(u32, Box<dyn std::any::Any + Send + Sync>)>) -> &mut Self {
        for (comp_id, comp_data) in components {
            let cmd = ComponentCommand {
                entity,
                component_id: comp_id,
                data: comp_data,
                action: ComponentAction::Add,
            };
            self.component_commands.push(cmd);
        }
        self
    }
    
    /// Toplu olarak component kaldırır
    pub fn remove_components_batch(&mut self, entity: Entity, component_ids: Vec<u32>) -> &mut Self {
        for comp_id in component_ids {
            let cmd = ComponentCommand {
                entity,
                component_id: comp_id,
                data: Box::new(()), // Boş veri
                action: ComponentAction::Remove,
            };
            self.component_commands.push(cmd);
        }
        self
    }
    
    /// Varlık varsa komutları uygular, yoksa atlar
    pub fn entity_exists_then<F>(&mut self, entity: Entity, f: F) -> &mut Self
    where
        F: FnOnce(&mut CommandBuffer),
    {
        if self.world.entity_exists(entity) {
            f(self);
        }
        self
    }
    
    /// Belirli bir koşul sağlandığında komutları uygular
    pub fn conditional<F>(&mut self, condition: bool, f: F) -> &mut Self
    where
        F: FnOnce(&mut CommandBuffer),
    {
        if condition {
            f(self);
        }
        self
    }
    
    /// Zamanlayıcı komutu ekler (gelecekte çalıştırılacak)
    pub fn delayed<F>(&mut self, delay_frames: u32, f: F) -> &mut Self
    where
        F: FnOnce(&mut CommandBuffer) + Send + Sync + 'static,
    {
        // Zamanlayıcı sistemine komut ekleme mantığı burada olurdu
        // Bu sadece API tanımı, gerçek implementasyon zamanlayıcı sistemde yapılır
        self
    }
}

// Komut sistemi başlatma fonksiyonu
pub fn init_command_system() -> CommandBufferSystem {
    CommandBufferSystem
}
