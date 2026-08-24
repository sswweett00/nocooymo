use crate::{World, Entity, Component};
use std::collections::VecDeque;
use std::any::{Any, TypeId};
use std::sync::{Arc, Mutex};

// Komut sistemi için temel trait
pub trait Command: Send + Sync {
    fn execute(&mut self, world: &mut World);
}

// Komut arabelleği
pub struct CommandBuffer {
    pub commands: VecDeque<Box<dyn Command>>,
    pub world: Arc<Mutex<World>>,
}

impl CommandBuffer {
    pub fn new() -> Self {
        Self {
            commands: VecDeque::new(),
            world: Arc::new(Mutex::new(World::new())),
        }
    }
    
    pub fn new_with_world(world: Arc<Mutex<World>>) -> Self {
        Self {
            commands: VecDeque::new(),
            world,
        }
    }
    
    pub fn add_command<C>(&mut self, command: C) -> &mut Self
    where
        C: Command + 'static,
    {
        self.commands.push_back(Box::new(command));
        self
    }
    
    pub fn add<F>(&mut self, f: F) -> &mut Self
    where
        F: FnOnce(&mut World) + Send + Sync + 'static,
    {
        self.commands.push_back(Box::new(FunctionCommand::new(f)));
        self
    }
    
    pub fn spawn<F>(&mut self, f: F) -> &mut Self
    where
        F: FnOnce(&mut World, Entity) + Send + Sync + 'static,
    {
        self.commands.push_back(Box::new(SpawnCommand::new(f)));
        self
    }
    
    pub fn despawn(&mut self, entity: Entity) -> &mut Self {
        self.commands.push_back(Box::new(DespawnCommand::new(entity)));
        self
    }
    
    pub fn insert_component<T: Component + Clone>(&mut self, entity: Entity, component: T) -> &mut Self {
        self.commands.push_back(Box::new(InsertComponentCommand::new(entity, component)));
        self
    }

    pub fn remove_component<T: Component + Clone>(&mut self, entity: Entity) -> &mut Self {
        self.commands.push_back(Box::new(RemoveComponentCommand::<T>::new(entity)));
        self
    }
    
    pub fn execute_all(&mut self) {
        while let Some(mut command) = self.commands.pop_front() {
            if let Ok(mut world) = self.world.lock() {
                command.execute(&mut world);
            }
        }
    }
    
    pub fn execute_one(&mut self) -> bool {
        if let Some(mut command) = self.commands.pop_front() {
            if let Ok(mut world) = self.world.lock() {
                command.execute(&mut world);
            }
            true
        } else {
            false
        }
    }
    
    pub fn clear(&mut self) {
        self.commands.clear();
    }
    
    pub fn len(&self) -> usize {
        self.commands.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
    
    pub fn get_world(&self) -> Arc<Mutex<World>> {
        Arc::clone(&self.world)
    }
}

// Fonksiyon tabanlı komut
pub struct FunctionCommand<F>
where
    F: FnOnce(&mut World) + Send + Sync + 'static,
{
    function: Option<F>,
}

impl<F> FunctionCommand<F>
where
    F: FnOnce(&mut World) + Send + Sync + 'static,
{
    pub fn new(function: F) -> Self {
        Self {
            function: Some(function),
        }
    }
}

impl<F> Command for FunctionCommand<F>
where
    F: FnOnce(&mut World) + Send + Sync + 'static,
{
    fn execute(&mut self, world: &mut World) {
        if let Some(func) = self.function.take() {
            func(world);
        }
    }
}

// Varlık oluşturma komutu
pub struct SpawnCommand<F>
where
    F: FnOnce(&mut World, Entity) + Send + Sync + 'static,
{
    function: Option<F>,
}

impl<F> SpawnCommand<F>
where
    F: FnOnce(&mut World, Entity) + Send + Sync + 'static,
{
    pub fn new(function: F) -> Self {
        Self {
            function: Some(function),
        }
    }
}

impl<F> Command for SpawnCommand<F>
where
    F: FnOnce(&mut World, Entity) + Send + Sync + 'static,
{
    fn execute(&mut self, world: &mut World) {
        let entity = world.spawn();
        if let Some(func) = self.function.take() {
            func(world, entity);
        }
    }
}

// Varlık silme komutu
pub struct DespawnCommand {
    entity: Entity,
}

impl DespawnCommand {
    pub fn new(entity: Entity) -> Self {
        Self { entity }
    }
}

impl Command for DespawnCommand {
    fn execute(&mut self, world: &mut World) {
        world.despawn(self.entity);
    }
}

// Bileşen ekleme komutu
pub struct InsertComponentCommand<T: Component + Clone> {
    entity: Entity,
    component: Option<T>,
}

impl<T: Component + Clone> InsertComponentCommand<T> {
    pub fn new(entity: Entity, component: T) -> Self {
        Self {
            entity,
            component: Some(component),
        }
    }
}

impl<T: Component + Clone> Command for InsertComponentCommand<T> {
    fn execute(&mut self, world: &mut World) {
        if let Some(component) = self.component.take() {
            let _ = world.insert_component(self.entity, component);
        }
    }
}

// Bileşen kaldırma komutu
pub struct RemoveComponentCommand<T: Component + Clone> {
    entity: Entity,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Component + Clone> RemoveComponentCommand<T> {
    pub fn new(entity: Entity) -> Self {
        Self {
            entity,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: Component + Clone> Command for RemoveComponentCommand<T> {
    fn execute(&mut self, world: &mut World) {
        let _ = world.remove_component::<T>(self.entity);
    }
}

// Toplu komutlar
pub struct BatchCommand {
    commands: Vec<Box<dyn Command>>,
}

impl BatchCommand {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }
    
    pub fn add<C: Command + 'static>(mut self, command: C) -> Self {
        self.commands.push(Box::new(command));
        self
    }
    
    pub fn add_multiple(mut self, commands: Vec<Box<dyn Command>>) -> Self {
        self.commands.extend(commands);
        self
    }
}

impl Command for BatchCommand {
    fn execute(&mut self, world: &mut World) {
        for mut command in self.commands.drain(..) {
            command.execute(world);
        }
    }
}

// Koşullu komut
pub struct ConditionalCommand<F>
where
    F: Fn(&World) -> bool + Send + Sync + 'static,
{
    condition: F,
    true_command: Option<Box<dyn Command>>,
    false_command: Option<Box<dyn Command>>,
}

impl<F> ConditionalCommand<F>
where
    F: Fn(&World) -> bool + Send + Sync + 'static,
{
    pub fn new(condition: F) -> Self {
        Self {
            condition,
            true_command: None,
            false_command: None,
        }
    }
    
    pub fn then<C: Command + 'static>(mut self, command: C) -> Self {
        self.true_command = Some(Box::new(command));
        self
    }
    
    pub fn else_<C: Command + 'static>(mut self, command: C) -> Self {
        self.false_command = Some(Box::new(command));
        self
    }
}

impl<F> Command for ConditionalCommand<F>
where
    F: Fn(&World) -> bool + Send + Sync + 'static,
{
    fn execute(&mut self, world: &mut World) {
        let condition_result = (self.condition)(world);
        
        if condition_result {
            if let Some(mut command) = self.true_command.take() {
                command.execute(world);
            }
        } else if let Some(mut command) = self.false_command.take() {
            command.execute(world);
        }
    }
}

// Gecikmeli komut
pub struct DelayedCommand {
    delay_frames: u32,
    command: Option<Box<dyn Command>>,
    elapsed_frames: u32,
}

impl DelayedCommand {
    pub fn new(delay_frames: u32, command: Box<dyn Command>) -> Self {
        Self {
            delay_frames,
            command: Some(command),
            elapsed_frames: 0,
        }
    }
}

impl Command for DelayedCommand {
    fn execute(&mut self, world: &mut World) {
        self.elapsed_frames += 1;
        
        if self.elapsed_frames >= self.delay_frames {
            if let Some(mut command) = self.command.take() {
                command.execute(world);
            }
        }
    }
}

// Zincirleme komut
pub struct ChainCommand {
    commands: VecDeque<Box<dyn Command>>,
}

impl ChainCommand {
    pub fn new() -> Self {
        Self {
            commands: VecDeque::new(),
        }
    }
    
    pub fn add<C: Command + 'static>(mut self, command: C) -> Self {
        self.commands.push_back(Box::new(command));
        self
    }
    
    pub fn add_multiple(mut self, commands: Vec<Box<dyn Command>>) -> Self {
        for command in commands {
            self.commands.push_back(command);
        }
        self
    }
}

impl Command for ChainCommand {
    fn execute(&mut self, world: &mut World) {
        if let Some(mut command) = self.commands.pop_front() {
            command.execute(world);
        }
    }
}

// Tekrarlayan komut
pub struct RepeatingCommand {
    command: Option<Box<dyn Command>>,
    times: u32,
    executed_times: u32,
    factory: Box<dyn Fn() -> Box<dyn Command> + Send + Sync>,
}

impl RepeatingCommand {
    pub fn new<C: Command + Clone + 'static>(command: C, times: u32) -> Self {
        let factory = {
            let command = command.clone();
            Box::new(move || Box::new(command.clone()) as Box<dyn Command>)
        };
        Self {
            command: Some(Box::new(command)),
            times,
            executed_times: 0,
            factory,
        }
    }
}

impl Command for RepeatingCommand {
    fn execute(&mut self, world: &mut World) {
        if self.executed_times >= self.times {
            return;
        }
        if let Some(mut command) = self.command.take() {
            command.execute(world);
        }
        self.executed_times += 1;

        // Sonraki kullanım için komutu tekrar oluştur
        self.command = Some((self.factory)());
    }
}

// Komut sistemi bileşeni
pub struct CommandSystem {
    pub buffers: Vec<CommandBuffer>,
    pub active_buffer_index: usize,
    pub auto_execute: bool,
}

impl CommandSystem {
    pub fn new() -> Self {
        let mut buffers = Vec::new();
        buffers.push(CommandBuffer::new()); // Varsayılan arabellek
        
        Self {
            buffers,
            active_buffer_index: 0,
            auto_execute: true,
        }
    }
    
    pub fn new_with_world(world: Arc<Mutex<World>>) -> Self {
        let mut buffers = Vec::new();
        buffers.push(CommandBuffer::new_with_world(world)); // Varsayılan arabellek
        
        Self {
            buffers,
            active_buffer_index: 0,
            auto_execute: true,
        }
    }
    
    pub fn create_buffer(&mut self) -> usize {
        let index = self.buffers.len();
        self.buffers.push(CommandBuffer::new());
        index
    }
    
    pub fn create_buffer_with_world(&mut self, world: Arc<Mutex<World>>) -> usize {
        let index = self.buffers.len();
        self.buffers.push(CommandBuffer::new_with_world(world));
        index
    }
    
    pub fn get_buffer(&self, index: usize) -> Option<&CommandBuffer> {
        self.buffers.get(index)
    }
    
    pub fn get_buffer_mut(&mut self, index: usize) -> Option<&mut CommandBuffer> {
        self.buffers.get_mut(index)
    }
    
    pub fn get_active_buffer(&self) -> &CommandBuffer {
        &self.buffers[self.active_buffer_index]
    }
    
    pub fn get_active_buffer_mut(&mut self) -> &mut CommandBuffer {
        &mut self.buffers[self.active_buffer_index]
    }
    
    pub fn set_active_buffer(&mut self, index: usize) -> Result<(), &'static str> {
        if index < self.buffers.len() {
            self.active_buffer_index = index;
            Ok(())
        } else {
            Err("Buffer index out of bounds")
        }
    }
    
    pub fn add_command<C>(&mut self, command: C) -> &mut Self
    where
        C: Command + 'static,
    {
        self.get_active_buffer_mut().add_command(command);
        self
    }
    
    pub fn add<F>(&mut self, f: F) -> &mut Self
    where
        F: FnOnce(&mut World) + Send + Sync + 'static,
    {
        self.get_active_buffer_mut().add(f);
        self
    }
    
    pub fn spawn<F>(&mut self, f: F) -> &mut Self
    where
        F: FnOnce(&mut World, Entity) + Send + Sync + 'static,
    {
        self.get_active_buffer_mut().spawn(f);
        self
    }
    
    pub fn despawn(&mut self, entity: Entity) -> &mut Self {
        self.get_active_buffer_mut().despawn(entity);
        self
    }
    
    pub fn insert_component<T: Component + Clone>(&mut self, entity: Entity, component: T) -> &mut Self {
        self.get_active_buffer_mut().insert_component(entity, component);
        self
    }
    
    pub fn remove_component<T: Component + Clone>(&mut self, entity: Entity) -> &mut Self {
        self.get_active_buffer_mut().remove_component::<T>(entity);
        self
    }
    
    pub fn execute_all(&mut self) {
        for buffer in &mut self.buffers {
            buffer.execute_all();
        }
    }
    
    pub fn execute_active(&mut self) {
        self.get_active_buffer_mut().execute_all();
    }
    
    pub fn clear(&mut self) {
        for buffer in &mut self.buffers {
            buffer.clear();
        }
    }
    
    pub fn clear_active(&mut self) {
        self.get_active_buffer_mut().clear();
    }
    
    pub fn update(&mut self) {
        if self.auto_execute {
            self.execute_active();
        }
    }
    
    pub fn set_auto_execute(&mut self, auto_execute: bool) {
        self.auto_execute = auto_execute;
    }
    
    pub fn get_buffer_count(&self) -> usize {
        self.buffers.len()
    }
    
    pub fn get_active_buffer_index(&self) -> usize {
        self.active_buffer_index
    }
}

// Komut sistemi için yardımcı trait'ler
pub trait CommandExt {
    fn add_spawn_callback<F>(&mut self, f: F) -> &mut Self
    where
        F: FnOnce(&mut World, Entity) + Send + Sync + 'static;
    
    fn add_despawn_callback<F>(&mut self, f: F) -> &mut Self
    where
        F: FnOnce(&mut World, Entity) + Send + Sync + 'static;
    
    fn add_modify_entity<F>(&mut self, entity: Entity, f: F) -> &mut Self
    where
        F: FnOnce(&mut World, Entity) + Send + Sync + 'static;
}

impl CommandExt for CommandBuffer {
    fn add_spawn_callback<F>(&mut self, f: F) -> &mut Self
    where
        F: FnOnce(&mut World, Entity) + Send + Sync + 'static,
    {
        self.spawn(f)
    }
    
    fn add_despawn_callback<F>(&mut self, f: F) -> &mut Self
    where
        F: FnOnce(&mut World, Entity) + Send + Sync + 'static,
    {
        self.add(move |world| {
            f(world, Entity::INVALID); // Geçersiz entity, çünkü siliniyor
        })
    }
    
    fn add_modify_entity<F>(&mut self, entity: Entity, f: F) -> &mut Self
    where
        F: FnOnce(&mut World, Entity) + Send + Sync + 'static,
    {
        self.add(move |world| {
            f(world, entity);
        })
    }
}

// Komut zinciri builder
pub struct CommandChain {
    commands: Vec<Box<dyn Command>>,
}

impl CommandChain {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }
    
    pub fn add<C: Command + 'static>(mut self, command: C) -> Self {
        self.commands.push(Box::new(command));
        self
    }
    
    pub fn build(self) -> ChainCommand {
        ChainCommand::new().add_multiple(self.commands)
    }
}

// Komut sistemi için yardımcı fonksiyonlar
impl CommandSystem {
    pub fn spawn_entity(&mut self) -> Entity {
        let entity = Entity::from_parts(0, 0); // Placeholder - gerçek implementasyon world'den alır
        self.spawn(|world, spawned_entity| {
            // Spawn sonrası işlemler burada olur
        });
        entity
    }
    
    pub fn despawn_entity(&mut self, entity: Entity) -> &mut Self {
        self.despawn(entity)
    }
    
    pub fn add_component_to_entity<T: Component + Clone>(&mut self, entity: Entity, component: T) -> &mut Self {
        self.insert_component(entity, component)
    }
    
    pub fn remove_component_from_entity<T: Component + Clone>(&mut self, entity: Entity) -> &mut Self {
        self.remove_component::<T>(entity)
    }
    
    pub fn execute_when<F>(&mut self, condition: F) -> ConditionalCommand<F>
    where
        F: Fn(&World) -> bool + Send + Sync + 'static,
    {
        ConditionalCommand::new(condition)
    }
    
    pub fn delay_command(&mut self, frames: u32, command: Box<dyn Command>) -> &mut Self {
        self.add_command(DelayedCommand::new(frames, command));
        self
    }
    
    pub fn repeat_command<C: Command + Clone + 'static>(&mut self, command: C, times: u32) -> &mut Self {
        self.add_command(RepeatingCommand::new(command, times));
        self
    }
}