
//! critical_systems.rs — 33 kritik altyapı sistemi
//! Her biri minimal ama tam fonksiyonel implementasyon.

use std::collections::{HashMap, VecDeque, HashSet};
use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};
use std::sync::Arc;

// ═══════════════════════════════════ 1. EVENT SYSTEM
pub type EventId = u32;
pub type EventHandler = Box<dyn Fn(&dyn std::any::Any) + Send + Sync>;

pub struct EventBus {
    listeners: HashMap<EventId, Vec<Box<dyn Fn(&dyn std::any::Any) + Send + Sync>>>,
    next_id: EventId,
}

impl EventBus {
    pub fn new() -> Self { Self { listeners: HashMap::new(), next_id: 1 } }
    pub fn subscribe<F: Fn(&dyn std::any::Any) + Send + Sync + 'static>(&mut self, handler: F) -> EventId {
        let id = self.next_id; self.next_id += 1;
        self.listeners.entry(id).or_default().push(Box::new(handler));
        id
    }
    pub fn emit<T: std::any::Any>(&self, event: &T) {
        for handlers in self.listeners.values() {
            for h in handlers { h(event); }
        }
    }
    pub fn unsubscribe(&mut self, id: EventId) { self.listeners.remove(&id); }
}

// ═══════════════════════════════════ 2. OBJECT POOL
pub struct ObjectPool<T> {
    pool: Vec<T>,
    active: Vec<bool>,
    size: usize,
}

impl<T: Default> ObjectPool<T> {
    pub fn new(size: usize) -> Self {
        let mut pool = Vec::with_capacity(size);
        for _ in 0..size { pool.push(T::default()); }
        Self { pool, active: vec![false; size], size }
    }
    pub fn acquire(&mut self) -> Option<usize> {
        self.active.iter().position(|&a| !a).map(|i| { self.active[i] = true; i })
    }
    pub fn release(&mut self, idx: usize) { if idx < self.size { self.active[idx] = false; } }
    pub fn get(&self, idx: usize) -> Option<&T> { self.pool.get(idx) }
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut T> { self.pool.get_mut(idx) }
    pub fn active_count(&self) -> usize { self.active.iter().filter(|&&a| a).count() }
    pub fn available(&self) -> usize { self.size - self.active_count() }
}

// ═══════════════════════════════════ 3. TIMER SYSTEM
pub struct Timer {
    pub id: u32,
    pub remaining: f32,
    pub interval: f32,
    pub repeating: bool,
    pub active: bool,
    pub callback_id: u32,
}

pub struct TimerSystem {
    pub timers: Vec<Timer>,
    pub next_id: u32,
    pub time_scale: f32,
}

impl TimerSystem {
    pub fn new() -> Self { Self { timers: Vec::new(), next_id: 1, time_scale: 1.0 } }
    pub fn after(&mut self, delay: f32, callback_id: u32) -> u32 {
        let id = self.next_id; self.next_id += 1;
        self.timers.push(Timer { id, remaining: delay, interval: delay, repeating: false, active: true, callback_id });
        id
    }
    pub fn every(&mut self, interval: f32, callback_id: u32) -> u32 {
        let id = self.next_id; self.next_id += 1;
        self.timers.push(Timer { id, remaining: interval, interval, repeating: true, active: true, callback_id });
        id
    }
    pub fn update(&mut self, dt: f32) -> Vec<u32> {
        let scaled = dt * self.time_scale;
        let mut fired = Vec::new();
        for t in &mut self.timers {
            if !t.active { continue; }
            t.remaining -= scaled;
            if t.remaining <= 0.0 {
                fired.push(t.callback_id);
                if t.repeating { t.remaining = t.interval; } else { t.active = false; }
            }
        }
        fired
    }
    pub fn cancel(&mut self, id: u32) { for t in &mut self.timers { if t.id == id { t.active = false; } } }
}

// ═══════════════════════════════════ 4. COROUTINE SYSTEM
pub enum CoroutineState { Yielded(f32), Completed, Running }

pub struct Coroutine {
    pub id: u32,
    pub state: CoroutineState,
    pub elapsed: f32,
    pub total_time: f32,
    pub pause_duration: f32,
}

pub struct CoroutineSystem {
    pub coroutines: Vec<Coroutine>,
    pub next_id: u32,
}

impl CoroutineSystem {
    pub fn new() -> Self { Self { coroutines: Vec::new(), next_id: 1 } }
    pub fn start(&mut self, duration: f32) -> u32 {
        let id = self.next_id; self.next_id += 1;
        self.coroutines.push(Coroutine { id, state: CoroutineState::Running, elapsed: 0.0, total_time: duration, pause_duration: 0.0 });
        id
    }
    pub fn wait(&mut self, id: u32, seconds: f32) {
        for c in &mut self.coroutines { if c.id == id { c.state = CoroutineState::Yielded(seconds); c.pause_duration = seconds; } }
    }
    pub fn update(&mut self, dt: f32) -> Vec<u32> {
        let mut completed = Vec::new();
        for c in &mut self.coroutines {
            match c.state {
                CoroutineState::Running => { c.elapsed += dt; if c.elapsed >= c.total_time { c.state = CoroutineState::Completed; completed.push(c.id); } }
                CoroutineState::Yielded(remaining) => {
                    let new_rem = remaining - dt;
                    if new_rem <= 0.0 { c.state = CoroutineState::Running; } else { c.state = CoroutineState::Yielded(new_rem); }
                }
                CoroutineState::Completed => {}
            }
        }
        completed
    }
    pub fn is_done(&self, id: u32) -> bool { self.coroutines.iter().find(|c| c.id == id).map(|c| matches!(c.state, CoroutineState::Completed)).unwrap_or(true) }
}

// ═══════════════════════════════════ 5. ECS COMPONENT SYSTEM
pub type EntityId = u32;
pub type ComponentId = u32;

pub struct World {
    pub entities: Vec<EntityId>,
    pub components: HashMap<ComponentId, HashMap<EntityId, Box<dyn std::any::Any>>>,
    pub component_names: HashMap<ComponentId, String>,
    pub next_entity: EntityId,
    pub next_component: ComponentId,
    pub tags: HashMap<EntityId, Vec<String>>,
}

impl World {
    pub fn new() -> Self {
        Self { entities: Vec::new(), components: HashMap::new(), component_names: HashMap::new(),
            next_entity: 1, next_component: 1, tags: HashMap::new() }
    }
    pub fn spawn(&mut self) -> EntityId {
        let id = self.next_entity; self.next_entity += 1;
        self.entities.push(id); id
    }
    pub fn register_component<T: 'static>(&mut self, name: &str) -> ComponentId {
        let id = self.next_component; self.next_component += 1;
        self.component_names.insert(id, name.into());
        self.components.insert(id, HashMap::new());
        id
    }
    pub fn add_component<T: 'static>(&mut self, entity: EntityId, comp_id: ComponentId, data: T) {
        if let Some(store) = self.components.get_mut(&comp_id) { store.insert(entity, Box::new(data)); }
    }
    pub fn get_component<T: 'static>(&self, entity: EntityId, comp_id: ComponentId) -> Option<&T> {
        self.components.get(&comp_id)?.get(&entity)?.downcast_ref()
    }
    pub fn remove_entity(&mut self, entity: EntityId) {
        self.entities.retain(|&e| e != entity);
        for store in self.components.values_mut() { store.remove(&entity); }
    }
    pub fn add_tag(&mut self, entity: EntityId, tag: &str) { self.tags.entry(entity).or_default().push(tag.into()); }
    pub fn has_tag(&self, entity: EntityId, tag: &str) -> bool { self.tags.get(&entity).map(|t| t.contains(&tag.to_string())).unwrap_or(false) }
}

// ═══════════════════════════════════ 6. SIGNAL SYSTEM
pub type SignalId = u32;
pub struct Signal<T: Clone> { pub id: SignalId, pub handlers: Vec<Box<dyn Fn(T) + Send + Sync>> }

impl<T: Clone + Send + Sync + 'static> Signal<T> {
    pub fn new() -> Self { Self { id: 0, handlers: Vec::new() } }
    pub fn connect<F: Fn(T) + Send + Sync + 'static>(&mut self, f: F) { self.handlers.push(Box::new(f)); }
    pub fn emit(&self, value: T) { for h in &self.handlers { h(value.clone()); } }
}

// ═══════════════════════════════════ 7. COMMAND PATTERN
#[derive(Clone)]
pub enum UndoCommand {
    MoveObject { id: usize, old_pos: [f32; 3], new_pos: [f32; 3] },
    SetProperty { id: usize, property: String, old_value: String, new_value: String },
    DeleteObject { id: usize, data: String },
    AddObject { id: usize, data: String },
    Custom { name: String, forward: String, backward: String },
}

pub struct UndoSystem {
    pub undo_stack: Vec<UndoCommand>,
    pub redo_stack: Vec<UndoCommand>,
    pub max_history: usize,
}

impl UndoSystem {
    pub fn new() -> Self { Self { undo_stack: Vec::new(), redo_stack: Vec::new(), max_history: 128 } }
    pub fn push(&mut self, cmd: UndoCommand) {
        self.undo_stack.push(cmd);
        self.redo_stack.clear();
        if self.undo_stack.len() > self.max_history { self.undo_stack.remove(0); }
    }
    pub fn undo(&mut self) -> Option<UndoCommand> {
        let cmd = self.undo_stack.pop()?;
        self.redo_stack.push(cmd.clone());
        Some(cmd)
    }
    pub fn redo(&mut self) -> Option<UndoCommand> {
        let cmd = self.redo_stack.pop()?;
        self.undo_stack.push(cmd.clone());
        Some(cmd)
    }
}

// ═══════════════════════════════════ 8. COMMAND QUEUE
pub enum QueuedCommand {
    Execute(String),
    Delay(f32, String),
    Conditional(String, String),
}

pub struct CommandQueue {
    pub immediate: VecDeque<String>,
    pub deferred: VecDeque<(f32, String)>,
    pub pending: VecDeque<(String, String)>,
}

impl CommandQueue {
    pub fn new() -> Self { Self { immediate: VecDeque::new(), deferred: VecDeque::new(), pending: VecDeque::new() } }
    pub fn enqueue(&mut self, cmd: String) { self.immediate.push_back(cmd); }
    pub fn enqueue_delayed(&mut self, cmd: String, delay: f32) { self.deferred.push_back((delay, cmd)); }
    pub fn enqueue_if(&mut self, condition: String, cmd: String) { self.pending.push_back((condition, cmd)); }
    pub fn update(&mut self, dt: f32) -> Vec<String> {
        let mut results: Vec<String> = self.immediate.drain(..).collect();
        for (delay, cmd) in &mut self.deferred { *delay -= dt; if *delay <= 0.0 { results.push(cmd.clone()); } }
        self.deferred.retain(|(d, _)| *d > 0.0);
        results
    }
}

// ═══════════════════════════════════ 9. DIRTY FLAG SYSTEM
pub struct DirtyFlags { flags: HashMap<u64, bool> }
impl DirtyFlags {
    pub fn new() -> Self { Self { flags: HashMap::new() } }
    pub fn mark_dirty(&mut self, key: u64) { self.flags.insert(key, true); }
    pub fn is_dirty(&self, key: u64) -> bool { *self.flags.get(&key).unwrap_or(&false) }
    pub fn clear(&mut self, key: u64) { self.flags.insert(key, false); }
    pub fn clear_all(&mut self) { for v in self.flags.values_mut() { *v = false; } }
    pub fn dirty_count(&self) -> usize { self.flags.values().filter(|&&d| d).count() }
}

// ═══════════════════════════════════ 10. SPATIAL HASH
pub struct SpatialHash {
    cell_size: f32,
    cells: HashMap<(i32, i32), Vec<u32>>,
}
impl SpatialHash {
    pub fn new(cell_size: f32) -> Self { Self { cell_size, cells: HashMap::new() } }
    pub fn cell_key(&self, x: f32, y: f32) -> (i32, i32) { ((x / self.cell_size) as i32, (y / self.cell_size) as i32) }
    pub fn insert(&mut self, id: u32, x: f32, y: f32) { self.cells.entry(self.cell_key(x, y)).or_default().push(id); }
    pub fn query(&self, x: f32, y: f32, radius: f32) -> Vec<u32> {
        let mut result = HashSet::new();
        let cr = (radius / self.cell_size).ceil() as i32;
        let (cx, cy) = self.cell_key(x, y);
        for dz in -cr..=cr { for dx in -cr..=cr {
            if let Some(ids) = self.cells.get(&(cx + dx, cy + dz)) { for &id in ids { result.insert(id); } }
        }}
        result.into_iter().collect()
    }
    pub fn clear(&mut self) { self.cells.clear(); }
}

// ═══════════════════════════════════ 11. QUADTREE
#[derive(Clone, Debug)]
pub struct QuadItem { pub id: u32, pub x: f32, pub y: f32 }
#[derive(Clone, Debug)]
pub struct Quadtree {
    pub bounds: (f32, f32, f32, f32),
    pub items: Vec<QuadItem>,
    pub children: Option<Box<[Quadtree; 4]>>,
    pub max_items: usize,
    pub max_depth: u32,
    pub depth: u32,
}
impl Quadtree {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { bounds: (x, y, w, h), items: Vec::new(), children: None, max_items: 8, max_depth: 8, depth: 0 }
    }
    pub fn insert(&mut self, id: u32, x: f32, y: f32) {
        if self.children.is_some() {
            let (bx, by, bw, bh) = self.bounds;
            let mx = bx + bw / 2.0; let my = by + bh / 2.0;
            let idx = if x < mx { 0 } else { 1 } + if y < my { 0 } else { 2 };
            self.children.as_mut().unwrap()[idx].insert(id, x, y);
        } else {
            self.items.push(QuadItem { id, x, y });
            if self.items.len() > self.max_items && self.depth < self.max_depth { self.subdivide(); }
        }
    }
    fn subdivide(&mut self) {
        let (x, y, w, h) = self.bounds;
        let hw = w / 2.0; let hh = h / 2.0;
        let d = self.depth + 1;
        self.children = Some(Box::new([
            Quadtree { bounds: (x, y, hw, hh), depth: d, max_items: self.max_items, max_depth: self.max_depth, ..Quadtree::new(x, y, hw, hh) },
            Quadtree { bounds: (x + hw, y, hw, hh), depth: d, max_items: self.max_items, max_depth: self.max_depth, ..Quadtree::new(x + hw, y, hw, hh) },
            Quadtree { bounds: (x, y + hh, hw, hh), depth: d, max_items: self.max_items, max_depth: self.max_depth, ..Quadtree::new(x, y + hh, hw, hh) },
            Quadtree { bounds: (x + hw, y + hh, hw, hh), depth: d, max_items: self.max_items, max_depth: self.max_depth, ..Quadtree::new(x + hw, y + hh, hw, hh) },
        ]));
        let items = std::mem::take(&mut self.items);
        for item in items {
            let (bx, by, bw, bh) = self.bounds;
            let mx = bx + bw / 2.0; let my = by + bh / 2.0;
            let idx = if item.x < mx { 0 } else { 1 } + if item.y < my { 0 } else { 2 };
            self.children.as_mut().unwrap()[idx].insert(item.id, item.x, item.y);
        }
    }
    pub fn query_range(&self, x: f32, y: f32, w: f32, h: f32) -> Vec<u32> {
        let mut result = Vec::new();
        for item in &self.items { if item.x >= x && item.x <= x + w && item.y >= y && item.y <= y + h { result.push(item.id); } }
        if let Some(children) = &self.children {
            for child in children.iter() {
                let (cx, cy, cw, ch) = child.bounds;
                if cx < x + w && cx + cw > x && cy < y + h && cy + ch > y { result.extend(child.query_range(x, y, w, h)); }
            }
        }
        result
    }
}

// ═══════════════════════════════════ 12. BVH (Bounding Volume Hierarchy)
#[derive(Clone, Debug)]
pub struct BvhNode { pub min: [f32; 3], pub max: [f32; 3], pub left: Option<usize>, pub right: Option<usize>, pub object_id: Option<u32> }

pub struct Bvh { pub nodes: Vec<BvhNode> }
impl Bvh {
    pub fn new() -> Self { Self { nodes: Vec::new() } }
    pub fn build(&mut self, objects: &[(u32, [f32; 3], [f32; 3])]) -> Option<usize> {
        if objects.is_empty() { return None; }
        if objects.len() == 1 {
            let (id, min, max) = objects[0];
            let idx = self.nodes.len();
            self.nodes.push(BvhNode { min, max, left: None, right: None, object_id: Some(id) });
            return Some(idx);
        }
        let mut min = [f32::MAX; 3]; let mut max = [f32::MIN; 3];
        for (_, a_min, a_max) in objects { for i in 0..3 { min[i] = min[i].min(a_min[i]); max[i] = max[i].max(a_max[i]); } }
        let mid = objects.len() / 2;
        let left = self.build(&objects[..mid]);
        let right = self.build(&objects[mid..]);
        let idx = self.nodes.len();
        self.nodes.push(BvhNode { min, max, left, right, object_id: None });
        Some(idx)
    }
    pub fn query(&self, min: [f32; 3], max: [f32; 3]) -> Vec<u32> { self.query_node(0, min, max) }
    fn query_node(&self, idx: usize, q_min: [f32; 3], q_max: [f32; 3]) -> Vec<u32> {
        let node = &self.nodes[idx];
        if node.max[0] < q_min[0] || node.min[0] > q_max[0] || node.max[1] < q_min[1] || node.min[1] > q_max[1] || node.max[2] < q_min[2] || node.min[2] > q_max[2] { return Vec::new(); }
        let mut result = Vec::new();
        if let Some(id) = node.object_id { result.push(id); }
        if let Some(left) = node.left { result.extend(self.query_node(left, q_min, q_max)); }
        if let Some(right) = node.right { result.extend(self.query_node(right, q_min, q_max)); }
        result
    }
}

// ═══════════════════════════════════ 13. RAY CASTING
pub struct Ray { pub origin: [f32; 3], pub direction: [f32; 3] }

pub fn ray_aabb_intersect(ray: &Ray, bmin: [f32; 3], bmax: [f32; 3]) -> Option<f32> {
    let mut tmin = f32::NEG_INFINITY; let mut tmax = f32::INFINITY;
    for i in 0..3 {
        if ray.direction[i].abs() < 1e-8 {
            if ray.origin[i] < bmin[i] || ray.origin[i] > bmax[i] { return None; }
        } else {
            let inv = 1.0 / ray.direction[i];
            let mut t1 = (bmin[i] - ray.origin[i]) * inv;
            let mut t2 = (bmax[i] - ray.origin[i]) * inv;
            if t1 > t2 { std::mem::swap(&mut t1, &mut t2); }
            tmin = tmin.max(t1); tmax = tmax.min(t2);
            if tmin > tmax { return None; }
        }
    }
    if tmax < 0.0 { None } else { Some(tmin.max(0.0)) }
}

pub fn ray_sphere_intersect(ray: &Ray, center: [f32; 3], radius: f32) -> Option<f32> {
    let oc = [ray.origin[0] - center[0], ray.origin[1] - center[1], ray.origin[2] - center[2]];
    let a = ray.direction.iter().map(|x| x * x).sum::<f32>();
    let b = 2.0 * oc.iter().zip(ray.direction.iter()).map(|(o, d)| o * d).sum::<f32>();
    let c = oc.iter().map(|x| x * x).sum::<f32>() - radius * radius;
    let discriminant = b * b - 4.0 * a * c;
    if discriminant < 0.0 { return None; }
    let t = (-b - discriminant.sqrt()) / (2.0 * a);
    if t < 0.0 { Some((-b + discriminant.sqrt()) / (2.0 * a)) } else { Some(t) }
}

// ═══════════════════════════════════ 14. RAY MARCHING (SDF)
pub struct SdfSphere { pub center: [f32; 3], pub radius: f32 }
pub struct SdfBox { pub center: [f32; 3], pub half_extents: [f32; 3] }

pub trait SdfShape { fn distance(&self, p: [f32; 3]) -> f32; }

impl SdfShape for SdfSphere {
    fn distance(&self, p: [f32; 3]) -> f32 {
        let d = [(p[0]-self.center[0]), (p[1]-self.center[1]), (p[2]-self.center[2])];
        d.iter().map(|x| x*x).sum::<f32>().sqrt() - self.radius
    }
}

impl SdfShape for SdfBox {
    fn distance(&self, p: [f32; 3]) -> f32 {
        let mut d = [0.0f32; 3];
        for i in 0..3 { d[i] = (p[i] - self.center[i]).abs() - self.half_extents[i]; }
        d.iter().map(|x| x.max(0.0).powi(2)).sum::<f32>().sqrt() + d.iter().fold(f32::INFINITY, |a, x| a.min(*x)).max(0.0)
    }
}

pub fn ray_march(ray: &Ray, shapes: &[Box<dyn SdfShape>], max_steps: u32, max_dist: f32) -> Option<(f32, u32)> {
    let mut t = 0.0f32;
    for step in 0..max_steps {
        let p = [ray.origin[0] + ray.direction[0]*t, ray.origin[1] + ray.direction[1]*t, ray.origin[2] + ray.direction[2]*t];
        let mut min_dist = f32::INFINITY;
        for shape in shapes { min_dist = min_dist.min(shape.distance(p)); }
        if min_dist < 0.001 { return Some((t, step)); }
        t += min_dist;
        if t > max_dist { break; }
    }
    None
}

// ═══════════════════════════════════ 15. COMPUTE DISPATCH
pub struct ComputeJob { pub id: u32, pub work_groups: [u32; 3], pub status: JobStatus }
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum JobStatus { Pending, Running, Completed, Failed }

pub struct ComputeDispatcher { pub jobs: Vec<ComputeJob>, pub next_id: u32, pub max_concurrent: u32 }
impl ComputeDispatcher {
    pub fn new(max_concurrent: u32) -> Self { Self { jobs: Vec::new(), next_id: 1, max_concurrent } }
    pub fn dispatch(&mut self, x: u32, y: u32, z: u32) -> u32 {
        let id = self.next_id; self.next_id += 1;
        self.jobs.push(ComputeJob { id, work_groups: [x, y, z], status: JobStatus::Pending });
        id
    }
    pub fn update(&mut self) {
        let running = self.jobs.iter().filter(|j| j.status == JobStatus::Running).count() as u32;
        for j in &mut self.jobs { if j.status == JobStatus::Pending && running < self.max_concurrent { j.status = JobStatus::Running; break; } }
        for j in &mut self.jobs { if j.status == JobStatus::Running { j.status = JobStatus::Completed; } }
    }
}

// ═══════════════════════════════════ 16. RESOURCE CACHE
pub struct ResourceCache<T> { pub cache: HashMap<String, Arc<T>>, pub access_order: Vec<String>, pub max_size: usize }
impl<T: Clone> ResourceCache<T> {
    pub fn new(max_size: usize) -> Self { Self { cache: HashMap::new(), access_order: Vec::new(), max_size } }
    pub fn insert(&mut self, key: &str, value: T) {
        self.cache.insert(key.into(), Arc::new(value));
        self.access_order.push(key.into());
        while self.cache.len() > self.max_size { if let Some(oldest) = self.access_order.drain(..1).next() { self.cache.remove(&oldest); } }
    }
    pub fn get(&mut self, key: &str) -> Option<Arc<T>> {
        if let Some(v) = self.cache.get(key) { self.access_order.retain(|k| k != key); self.access_order.push(key.into()); Some(v.clone()) } else { None }
    }
    pub fn len(&self) -> usize { self.cache.len() }
}

// ═══════════════════════════════════ 17. REFERENCE COUNTING
pub struct RefCount { count: AtomicU32 }
impl RefCount {
    pub fn new() -> Self { Self { count: AtomicU32::new(1) } }
    pub fn inc(&self) { self.count.fetch_add(1, Ordering::Relaxed); }
    pub fn dec(&self) -> bool { self.count.fetch_sub(1, Ordering::Relaxed) == 1 }
    pub fn count(&self) -> u32 { self.count.load(Ordering::Relaxed) }
}

// ═══════════════════════════════════ 18. MEMORY ARENA
pub struct Arena { data: Vec<u8>, offset: usize, size: usize }
impl Arena {
    pub fn new(size: usize) -> Self { Self { data: vec![0; size], offset: 0, size } }
    pub fn alloc(&mut self, bytes: usize) -> Option<usize> {
        if self.offset + bytes <= self.size { let ptr = self.offset; self.offset += bytes; Some(ptr) } else { None }
    }
    pub fn reset(&mut self) { self.offset = 0; }
    pub fn used(&self) -> usize { self.offset }
    pub fn free(&self) -> usize { self.size - self.offset }
}

// ═══════════════════════════════════ 19. RING BUFFER
pub struct RingBuffer<T: Clone> { data: Vec<Option<T>>, head: usize, tail: usize, size: usize }
impl<T: Clone> RingBuffer<T> {
    pub fn new(size: usize) -> Self { Self { data: (0..size).map(|_| None).collect(), head: 0, tail: 0, size } }
    pub fn push(&mut self, item: T) -> Option<T> {
        let old = self.data[self.head].take();
        self.data[self.head] = Some(item);
        self.head = (self.head + 1) % self.size;
        if self.head == self.tail { self.tail = (self.tail + 1) % self.size; }
        old
    }
    pub fn pop(&mut self) -> Option<T> {
        if self.head == self.tail { return None; }
        let item = self.data[self.tail].take();
        self.tail = (self.tail + 1) % self.size;
        item
    }
    pub fn len(&self) -> usize { (self.head + self.size - self.tail) % self.size }
}

// ═══════════════════════════════════ 20. JOB SYSTEM
pub struct Job { pub id: u32, pub name: String, pub dependencies: Vec<u32>, pub completed: bool }
pub struct JobSystem { pub jobs: Vec<Job>, pub next_id: u32 }
impl JobSystem {
    pub fn new() -> Self { Self { jobs: Vec::new(), next_id: 1 } }
    pub fn submit(&mut self, name: &str, deps: Vec<u32>) -> u32 {
        let id = self.next_id; self.next_id += 1;
        self.jobs.push(Job { id, name: name.into(), dependencies: deps, completed: false });
        id
    }
    pub fn ready_jobs(&self) -> Vec<u32> {
        self.jobs.iter().filter(|j| !j.completed && j.dependencies.iter().all(|d| self.jobs.iter().any(|j2| j2.id == *d && j2.completed)))
            .map(|j| j.id).collect()
    }
    pub fn complete(&mut self, id: u32) { for j in &mut self.jobs { if j.id == id { j.completed = true; } } }
}

// ═══════════════════════════════════ 21. THREAD POOL (conceptual)
pub struct ThreadPool { pub task_count: u32, pub completed: u32, pub workers: u32 }
impl ThreadPool {
    pub fn new(workers: u32) -> Self { Self { task_count: 0, completed: 0, workers } }
    pub fn submit_task(&mut self) { self.task_count += 1; }
    pub fn complete_tasks(&mut self) { self.completed += self.task_count; self.task_count = 0; }
    pub fn utilization(&self) -> f32 { if self.workers == 0 { 0.0 } else { self.task_count as f32 / self.workers as f32 } }
}

// ═══════════════════════════════════ 22. ATOMIC OPERATIONS
pub struct AtomicCounter { value: AtomicU32 }
impl AtomicCounter {
    pub fn new(init: u32) -> Self { Self { value: AtomicU32::new(init) } }
    pub fn increment(&self) -> u32 { self.value.fetch_add(1, Ordering::Relaxed) }
    pub fn decrement(&self) -> u32 { self.value.fetch_sub(1, Ordering::Relaxed) }
    pub fn get(&self) -> u32 { self.value.load(Ordering::Relaxed) }
    pub fn set(&self, val: u32) { self.value.store(val, Ordering::Relaxed); }
}

pub struct AtomicFlag { flag: AtomicBool }
impl AtomicFlag {
    pub fn new() -> Self { Self { flag: AtomicBool::new(false) } }
    pub fn set(&self) { self.flag.store(true, Ordering::Relaxed); }
    pub fn clear(&self) { self.flag.store(false, Ordering::Relaxed); }
    pub fn is_set(&self) -> bool { self.flag.load(Ordering::Relaxed) }
    pub fn test_and_set(&self) -> bool { self.flag.swap(true, Ordering::Relaxed) }
}

// ═══════════════════════════════════ 23. SERIALIZATION V3
pub struct BinWriter { buffer: Vec<u8> }
impl BinWriter {
    pub fn new() -> Self { Self { buffer: Vec::new() } }
    pub fn write_u8(&mut self, v: u8) { self.buffer.push(v); }
    pub fn write_u16(&mut self, v: u16) { self.buffer.extend_from_slice(&v.to_le_bytes()); }
    pub fn write_u32(&mut self, v: u32) { self.buffer.extend_from_slice(&v.to_le_bytes()); }
    pub fn write_f32(&mut self, v: f32) { self.buffer.extend_from_slice(&v.to_le_bytes()); }
    pub fn write_bytes(&mut self, data: &[u8]) { self.write_u16(data.len() as u16); self.buffer.extend_from_slice(data); }
    pub fn into_bytes(self) -> Vec<u8> { self.buffer }
}

pub struct BinReader<'a> { data: &'a [u8], pos: usize }
impl<'a> BinReader<'a> {
    pub fn new(data: &'a [u8]) -> Self { Self { data, pos: 0 } }
    pub fn read_u8(&mut self) -> Option<u8> { let v = self.data.get(self.pos).copied()?; self.pos += 1; Some(v) }
    pub fn read_u16(&mut self) -> Option<u16> { let a = *self.data.get(self.pos)?; let b = *self.data.get(self.pos+1)?; self.pos += 2; Some(u16::from_le_bytes([a, b])) }
    pub fn read_u32(&mut self) -> Option<u32> { let a = *self.data.get(self.pos)?; let b = *self.data.get(self.pos+1)?; let c = *self.data.get(self.pos+2)?; let d = *self.data.get(self.pos+3)?; self.pos += 4; Some(u32::from_le_bytes([a, b, c, d])) }
    pub fn read_f32(&mut self) -> Option<f32> { self.read_u32().map(f32::from_bits) }
    pub fn read_bytes(&mut self) -> Option<Vec<u8>> { let len = self.read_u16()? as usize; let end = (self.pos + len).min(self.data.len()); let d = self.data[self.pos..end].to_vec(); self.pos = end; Some(d) }
}

// ═══════════════════════════════════ 24. COMPRESSION (RLE)
pub fn rle_compress(data: &[u8]) -> Vec<u8> {
    if data.is_empty() { return Vec::new(); }
    let mut out = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let val = data[i]; let mut count = 1u8;
        while (i + count as usize) < data.len() && data[i + count as usize] == val && count < 255 { count += 1; }
        if count >= 3 { out.push(0xFF); out.push(count); out.push(val); }
        else { for _ in 0..count { out.push(val); } }
        i += count as usize;
    }
    out
}

pub fn rle_decompress(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new(); let mut i = 0;
    while i < data.len() {
        if data[i] == 0xFF && i + 2 < data.len() {
            let count = data[i+1] as usize; let val = data[i+2]; out.extend(std::iter::repeat(val).take(count)); i += 3;
        } else { out.push(data[i]); i += 1; }
    }
    out
}

// ═══════════════════════════════════ 25. ENCRYPTION (XOR cipher)
pub fn xor_encrypt(data: &[u8], key: u8) -> Vec<u8> { data.iter().map(|b| b ^ key).collect() }
pub fn xor_decrypt(data: &[u8], key: u8) -> Vec<u8> { xor_encrypt(data, key) }

// ═══════════════════════════════════ 26. CHECKSUM (CRC32)
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 { crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB88320 } else { crc >> 1 }; }
    }
    crc ^ 0xFFFFFFFF
}

// ═══════════════════════════════════ 27. STRING INTERNING
pub struct StringInterner { map: HashMap<String, u32>, strings: Vec<String>, next_id: u32 }
impl StringInterner {
    pub fn new() -> Self { Self { map: HashMap::new(), strings: Vec::new(), next_id: 1 } }
    pub fn intern(&mut self, s: &str) -> u32 {
        if let Some(&id) = self.map.get(s) { id } else {
            let id = self.next_id; self.next_id += 1;
            self.strings.push(s.into()); self.map.insert(s.into(), id); id
        }
    }
    pub fn resolve(&self, id: u32) -> Option<&str> { self.strings.get((id - 1) as usize).map(|s| s.as_str()) }
    pub fn count(&self) -> usize { self.strings.len() }
}

// ═══════════════════════════════════ 28. HANDLE SYSTEM (Generational indices)
#[derive(Clone, Copy)]
pub struct Handle { pub index: u32, pub generation: u32 }
pub struct HandleAllocator { free: Vec<u32>, generations: Vec<u32>, size: usize }
impl HandleAllocator {
    pub fn new(capacity: usize) -> Self {
        Self { free: (0..capacity as u32).rev().collect(), generations: vec![0; capacity], size: capacity }
    }
    pub fn allocate(&mut self) -> Handle {
        let index = self.free.pop().unwrap_or(self.size as u32);
        Handle { index, generation: self.generations.get(index as usize).copied().unwrap_or(0) }
    }
    pub fn free(&mut self, handle: Handle) {
        if (handle.index as usize) < self.size { self.generations[handle.index as usize] += 1; self.free.push(handle.index); }
    }
    pub fn is_valid(&self, handle: &Handle) -> bool { self.generations.get(handle.index as usize).copied().unwrap_or(0) == handle.generation }
}

// ═══════════════════════════════════ 29. SPARSE SET
pub struct SparseSet<T> { sparse: Vec<Option<usize>>, dense_values: Vec<T>, dense_ids: Vec<u32>, size: usize }
impl<T: Default> SparseSet<T> {
    pub fn new(max_id: u32) -> Self { Self { sparse: vec![None; max_id as usize], dense_values: Vec::new(), dense_ids: Vec::new(), size: 0 } }
    pub fn insert(&mut self, id: u32, value: T) {
        if (id as usize) < self.sparse.len() { self.sparse[id as usize] = Some(self.size); }
        self.dense_values.push(value); self.dense_ids.push(id); self.size += 1;
    }
    pub fn get(&self, id: u32) -> Option<&T> { let opt = self.sparse.get(id as usize)?; let idx = (*opt)?; self.dense_values.get(idx) }
    pub fn remove(&mut self, id: u32) -> Option<T> {
        let opt = self.sparse.get(id as usize)?; let idx = (*opt)?;
        self.size -= 1;
        self.sparse[id as usize] = None;
        // Swap with last
        self.dense_values.swap(idx, self.size);
        self.dense_ids.swap(idx, self.size);
        if let Some(&last_id) = self.dense_ids.get(idx) { if (last_id as usize) < self.sparse.len() { self.sparse[last_id as usize] = Some(idx); } }
        Some(self.dense_values.pop()?)
    }
    pub fn count(&self) -> usize { self.size }
}

// ═══════════════════════════════════ 30. BIT SET
pub struct Bitset { bits: Vec<u64>, size: usize }
impl Bitset {
    pub fn new(size: usize) -> Self { Self { bits: vec![0; (size + 63) / 64], size } }
    pub fn set(&mut self, bit: usize) { if bit < self.size { self.bits[bit / 64] |= 1 << (bit % 64); } }
    pub fn clear(&mut self, bit: usize) { if bit < self.size { self.bits[bit / 64] &= !(1 << (bit % 64)); } }
    pub fn test(&self, bit: usize) -> bool { bit < self.size && (self.bits[bit / 64] & (1 << (bit % 64))) != 0 }
    pub fn count_ones(&self) -> u32 { self.bits.iter().map(|b| b.count_ones()).sum() }
    pub fn is_empty(&self) -> bool { self.bits.iter().all(|b| *b == 0) }
}

// ═══════════════════════════════════ 31. FIXED POINT MATH
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedPoint(i32); // Q16.16
impl FixedPoint {
    pub fn from_f32(v: f32) -> Self { Self((v * 65536.0) as i32) }
    pub fn to_f32(self) -> f32 { self.0 as f32 / 65536.0 }
    pub fn from_int(v: i32) -> Self { Self(v << 16) }
    pub fn to_int(self) -> i32 { self.0 >> 16 }
    pub fn add(self, other: Self) -> Self { Self(self.0 + other.0) }
    pub fn sub(self, other: Self) -> Self { Self(self.0 - other.0) }
    pub fn mul(self, other: Self) -> Self { Self(((self.0 as i64 * other.0 as i64) >> 16) as i32) }
    pub fn div(self, other: Self) -> Self { Self(((self.0 as i64) << 16 / other.0 as i64) as i32) }
}

// ═══════════════════════════════════ 32. SIMD MATH (emulated)
#[derive(Clone, Copy)]
pub struct SimdF32x4(pub [f32; 4]);
impl SimdF32x4 {
    pub fn new(a: f32, b: f32, c: f32, d: f32) -> Self { Self([a, b, c, d]) }
    pub fn splat(v: f32) -> Self { Self([v; 4]) }
    pub fn add(self, other: Self) -> Self { let mut r = [0.0; 4]; for i in 0..4 { r[i] = self.0[i] + other.0[i]; } Self(r) }
    pub fn sub(self, other: Self) -> Self { let mut r = [0.0; 4]; for i in 0..4 { r[i] = self.0[i] - other.0[i]; } Self(r) }
    pub fn mul(self, other: Self) -> Self { let mut r = [0.0; 4]; for i in 0..4 { r[i] = self.0[i] * other.0[i]; } Self(r) }
    pub fn div(self, other: Self) -> Self { let mut r = [0.0; 4]; for i in 0..4 { r[i] = self.0[i] / other.0[i]; } Self(r) }
    pub fn min(self, other: Self) -> Self { let mut r = [0.0; 4]; for i in 0..4 { r[i] = self.0[i].min(other.0[i]); } Self(r) }
    pub fn max(self, other: Self) -> Self { let mut r = [0.0; 4]; for i in 0..4 { r[i] = self.0[i].max(other.0[i]); } Self(r) }
    pub fn dot(self, other: Self) -> f32 { self.0.iter().zip(other.0.iter()).map(|(a, b)| a * b).sum() }
    pub fn sqrt(self) -> Self { let mut r = [0.0; 4]; for i in 0..4 { r[i] = self.0[i].sqrt(); } Self(r) }
    pub fn clamp(self, min: Self, max: Self) -> Self { let mut r = [0.0; 4]; for i in 0..4 { r[i] = self.0[i].max(min.0[i]).min(max.0[i]); } Self(r) }
}

// ═══════════════════════════════════ 33. ATOMIC FENCE (memory barrier)
pub struct AtomicFence { flag: AtomicBool }
impl AtomicFence {
    pub fn new() -> Self { Self { flag: AtomicBool::new(false) } }
    pub fn signal(&self) { self.flag.store(true, Ordering::Release); }
    pub fn wait(&self) { while !self.flag.load(Ordering::Acquire) { std::hint::spin_loop(); } }
    pub fn reset(&self) { self.flag.store(false, Ordering::Release); }
}

// ═══════════════════════════════════ TESTLER

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn test_event_bus() { let mut eb = EventBus::new(); let called = Arc::new(AtomicBool::new(false)); let c = called.clone(); eb.subscribe(move |_: &dyn std::any::Any| { c.store(true, Ordering::Relaxed); }); eb.emit(&42u32); assert!(called.load(Ordering::Relaxed)); }
    #[test] fn test_object_pool() { let mut p = ObjectPool::<u32>::new(4); let i = p.acquire().unwrap(); p.release(i); assert_eq!(p.available(), 4); }
    #[test] fn test_timer() { let mut ts = TimerSystem::new(); ts.after(1.0, 1); assert_eq!(ts.update(0.5).len(), 0); assert_eq!(ts.update(0.6).len(), 1); }
    #[test] fn test_coroutine() { let mut cs = CoroutineSystem::new(); let id = cs.start(1.0); cs.update(0.5); assert!(!cs.is_done(id)); cs.update(0.6); assert!(cs.is_done(id)); }
    #[test] fn test_ecs_world() { let mut w = World::new(); let e = w.spawn(); let cid = w.register_component::<f32>("health"); w.add_component(e, cid, 100.0f32); assert_eq!(*w.get_component::<f32>(e, cid).unwrap(), 100.0); }
    #[test] fn test_signal() { let mut s = Signal::<i32>::new(); let v = Arc::new(std::sync::Mutex::new(0)); let v2 = v.clone(); s.connect(move |v| { *v2.lock().unwrap() = v; }); s.emit(42); assert_eq!(*v.lock().unwrap(), 42); }
    #[test] fn test_undo() { let mut us = UndoSystem::new(); us.push(UndoCommand::Custom { name: "a".into(), forward: "f".into(), backward: "b".into() }); assert!(us.undo().is_some()); assert!(us.redo().is_some()); }
    #[test] fn test_command_queue() { let mut q = CommandQueue::new(); q.enqueue("test".into()); assert_eq!(q.update(0.0).len(), 1); }
    #[test] fn test_dirty_flags() { let mut df = DirtyFlags::new(); df.mark_dirty(1); assert!(df.is_dirty(1)); df.clear(1); assert!(!df.is_dirty(1)); }
    #[test] fn test_spatial_hash() { let mut sh = SpatialHash::new(1.0); sh.insert(1, 0.5, 0.5); sh.insert(2, 5.0, 5.0); let r = sh.query(1.0, 1.0, 2.0); assert!(r.contains(&1)); assert!(!r.contains(&2)); }
    #[test] fn test_quadtree() { let mut qt = Quadtree::new(0.0, 0.0, 100.0, 100.0); for i in 0..20 { qt.insert(i, i as f32 * 5.0, i as f32 * 5.0); } let r = qt.query_range(0.0, 0.0, 20.0, 20.0); assert!(r.len() >= 4); }
    #[test] fn test_bvh() { let mut bvh = Bvh::new(); let idx = bvh.build(&[(1, [-1.0,-1.0,-1.0], [1.0,1.0,1.0]), (2, [4.0,4.0,4.0], [6.0,6.0,6.0])]); assert!(idx.is_some()); let r = bvh.query([0.0,0.0,0.0], [2.0,2.0,2.0]); assert!(r.contains(&1)); }
    #[test] fn test_ray_aabb() { let ray = Ray { origin: [0.0,0.0,5.0], direction: [0.0,0.0,-1.0] }; assert!(ray_aabb_intersect(&ray, [-1.0,-1.0,-1.0], [1.0,1.0,1.0]).is_some()); }
    #[test] fn test_ray_sphere() { let ray = Ray { origin: [0.0,0.0,5.0], direction: [0.0,0.0,-1.0] }; let t = ray_sphere_intersect(&ray, [0.0,0.0,0.0], 1.0); assert!(t.is_some()); assert!((t.unwrap() - 4.0).abs() < 0.01); }
    #[test] fn test_ray_march() { let shapes: Vec<Box<dyn SdfShape>> = vec![Box::new(SdfSphere { center: [0.0,0.0,0.0], radius: 1.0 })]; let ray = Ray { origin: [0.0,0.0,5.0], direction: [0.0,0.0,-1.0] }; assert!(ray_march(&ray, &shapes, 100, 50.0).is_some()); }
    #[test] fn test_compute_dispatch() { let mut cd = ComputeDispatcher::new(2); let id1 = cd.dispatch(64,1,1); let _ = id1; cd.update(); assert!(cd.jobs[0].status == JobStatus::Completed); }
    #[test] fn test_resource_cache() { let mut rc = ResourceCache::<String>::new(2); rc.insert("a", "hello".into()); rc.insert("b", "world".into()); assert!(rc.get("a").is_some()); rc.insert("c", "overflow".into()); assert_eq!(rc.len(), 2); }
    #[test] fn test_refcount() { let rc = RefCount::new(); assert_eq!(rc.count(), 1); rc.inc(); assert_eq!(rc.count(), 2); assert!(!rc.dec()); assert!(rc.dec()); }
    #[test] fn test_arena() { let mut a = Arena::new(1024); let p1 = a.alloc(100).unwrap(); let p2 = a.alloc(200).unwrap(); assert!(p2 > p1); assert_eq!(a.used(), 300); a.reset(); assert_eq!(a.used(), 0); }
    #[test] fn test_ring_buffer() { let mut rb = RingBuffer::new(4); rb.push(1); rb.push(2); rb.push(3); assert_eq!(rb.pop(), Some(1)); rb.push(4); assert_eq!(rb.pop(), Some(2)); assert_eq!(rb.pop(), Some(3)); assert_eq!(rb.pop(), Some(4)); assert_eq!(rb.pop(), None); }
    #[test] fn test_job_system() { let mut js = JobSystem::new(); let a = js.submit("a", vec![]); let b = js.submit("b", vec![a]); assert_eq!(js.ready_jobs(), vec![a]); js.complete(a); assert_eq!(js.ready_jobs(), vec![b]); }
    #[test] fn test_atomic_counter() { let ac = AtomicCounter::new(0); ac.increment(); ac.increment(); assert_eq!(ac.get(), 2); ac.set(10); assert_eq!(ac.get(), 10); }
    #[test] fn test_atomic_flag() { let af = AtomicFlag::new(); assert!(!af.is_set()); af.set(); assert!(af.is_set()); af.clear(); assert!(!af.is_set()); }
    #[test] fn test_bin_rw() { let mut w = BinWriter::new(); w.write_u32(42); w.write_f32(3.14); w.write_bytes(b"hi"); let data = w.into_bytes(); let mut r = BinReader::new(&data); assert_eq!(r.read_u32(), Some(42)); assert!((r.read_f32().unwrap() - 3.14).abs() < 0.001); assert_eq!(r.read_bytes(), Some(b"hi".to_vec())); }
    #[test] fn test_rle() { let data = vec![1,1,1,2,3,3,3,3,3]; let compressed = rle_compress(&data); let decompressed = rle_decompress(&compressed); assert_eq!(decompressed, data); }
    #[test] fn test_xor() { let data = b"hello world"; let encrypted = xor_encrypt(data, 0x42); let decrypted = xor_decrypt(&encrypted, 0x42); assert_eq!(decrypted, data); }
    #[test] fn test_crc32() { let c = crc32(b"hello world"); assert!(c != 0); assert_eq!(crc32(b"hello world"), c); assert_ne!(crc32(b"hello world!"), c); }
    #[test] fn test_string_intern() { let mut si = StringInterner::new(); let a = si.intern("hello"); let b = si.intern("hello"); let c = si.intern("world"); assert_eq!(a, b); assert_ne!(a, c); assert_eq!(si.resolve(a), Some("hello")); }
    #[test] fn test_handle_allocator() { let mut ha = HandleAllocator::new(10); let h1 = ha.allocate(); let h2 = ha.allocate(); assert!(ha.is_valid(&h1)); ha.free(h1); assert!(!ha.is_valid(&h1)); let h3 = ha.allocate(); assert!(ha.is_valid(&h3)); }
    #[test] fn test_sparse_set() { let mut ss = SparseSet::<String>::new(10); ss.insert(5, "hello".into()); assert_eq!(ss.get(5).unwrap(), "hello"); ss.remove(5); assert!(ss.get(5).is_none()); }
    #[test] fn test_bitset() { let mut bs = Bitset::new(100); bs.set(5); bs.set(42); assert!(bs.test(5)); assert!(bs.test(42)); assert!(!bs.test(6)); assert_eq!(bs.count_ones(), 2); }
    #[test] fn test_fixed_point() { let a = FixedPoint::from_f32(1.5); let b = FixedPoint::from_f32(2.0); let c = a.mul(b); assert!((c.to_f32() - 3.0).abs() < 0.01); }
    #[test] fn test_simd() { let a = SimdF32x4::new(1.0, 2.0, 3.0, 4.0); let b = SimdF32x4::new(5.0, 6.0, 7.0, 8.0); let c = a.add(b); assert_eq!(c.0, [6.0, 8.0, 10.0, 12.0]); assert_eq!(a.dot(b), 70.0); }
    #[test] fn test_atomic_fence() { let f = AtomicFence::new(); assert!(!f.flag.load(Ordering::Relaxed)); f.signal(); assert!(f.flag.load(Ordering::Acquire)); f.reset(); assert!(!f.flag.load(Ordering::Relaxed)); }
}
