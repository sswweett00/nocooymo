use std::any::Any;
use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;

use crate::math::Vec3;
use crate::{Aabb, Entity};

// ============================================================================
// Navigation Mesh (NavMesh)
// ============================================================================

/// Area type for navmesh polygons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AreaType {
    Walk,
    Jump,
    Swim,
    Crawl,
    Fly,
    Climb,
    Custom(u32),
}

/// 3D vertex in a navigation mesh.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NavMeshVertex {
    pub position: Vec3,
    pub area: AreaType,
}

/// Triangular polygon in a navigation mesh.
#[derive(Debug, Clone, PartialEq)]
pub struct NavMeshPolygon {
    pub vertices: [u32; 3],
    pub area: AreaType,
    pub normal: Vec3,
    pub centroid: Vec3,
}

/// Adjacency link between two polygons sharing an edge.
#[derive(Debug, Clone, PartialEq)]
pub struct NavMeshLink {
    pub polygon_from: u32,
    pub polygon_to: u32,
    pub edge_from: [u32; 2],
    pub edge_to: [u32; 2],
    pub cost: f32,
}

/// Off-mesh connection for portals, ladders, etc.
#[derive(Debug, Clone, PartialEq)]
pub struct OffMeshConnection {
    pub start: Vec3,
    pub end: Vec3,
    pub area: AreaType,
    pub cost: f32,
    pub is_bidirectional: bool,
}

/// Navigation mesh built from geometry.
#[derive(Debug, Clone, Default)]
pub struct NavMesh {
    pub polygons: Vec<NavMeshPolygon>,
    pub links: Vec<NavMeshLink>,
    pub off_mesh_connections: Vec<OffMeshConnection>,
    pub bounds: Aabb,
}

impl NavMesh {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            polygons: Vec::new(),
            links: Vec::new(),
            off_mesh_connections: Vec::new(),
            bounds: Aabb::default(),
        }
    }

    /// Adds a triangle to the mesh. Returns the polygon index.
    pub fn add_triangle(&mut self, v0: Vec3, v1: Vec3, v2: Vec3, area: AreaType) -> u32 {
        let idx = self.vertices.len() as u32;
        let normal = (v1 - v0).cross(v2 - v0);
        let n_len = normal.length();
        let normal = if n_len > 1e-6 { normal / n_len } else { Vec3::Y };
        let centroid = (v0 + v1 + v2) / 3.0;
        self.vertices.push(NavMeshVertex { position: v0, area });
        self.vertices.push(NavMeshVertex { position: v1, area });
        self.vertices.push(NavMeshVertex { position: v2, area });
        let poly_idx = self.polygons.len() as u32;
        self.polygons.push(NavMeshPolygon {
            vertices: [idx, idx + 1, idx + 2],
            area,
            normal,
            centroid,
        });
        self.bounds = self
            .bounds
            .expand(v0)
            .expand(v1)
            .expand(v2);
        poly_idx
    }

    /// Returns the centroid of a polygon by index.
    pub fn polygon_centroid(&self, poly_idx: u32) -> Vec3 {
        self.polygons
            .get(poly_idx as usize)
            .map(|p| p.centroid)
            .unwrap_or(Vec3::ZERO)
    }

    /// Builds adjacency links between adjacent polygons.
    pub fn build_links(&mut self) {
        self.links.clear();
        let mut edge_map: HashMap<[u32; 2], u32> = HashMap::new();
        for (poly_idx, poly) in self.polygons.iter().enumerate() {
            let v = poly.vertices;
            let edges = [[v[0], v[1]], [v[1], v[2]], [v[2], v[0]]];
            for edge in edges {
                let sorted = if edge[0] < edge[1] { [edge[0], edge[1]] } else { [edge[1], edge[0]] };
                if let Some(&other) = edge_map.get(&sorted) {
                    let a = poly.centroid;
                    let b = self.polygons[other].centroid;
                    let cost = (a - b).length();
                    self.links.push(NavMeshLink {
                        polygon_from: poly_idx as u32,
                        polygon_to: other,
                        edge_from: edge,
                        edge_to: sorted,
                        cost,
                    });
                    self.links.push(NavMeshLink {
                        polygon_from: other,
                        polygon_to: poly_idx as u32,
                        edge_from: sorted,
                        edge_to: edge,
                        cost,
                    });
                } else {
                    edge_map.insert(sorted, poly_idx as u32);
                }
            }
        }
    }

    /// Adds an off-mesh connection. Returns its index.
    pub fn add_off_mesh_connection(&mut self, conn: OffMeshConnection) -> u32 {
        let idx = self.off_mesh_connections.len() as u32;
        self.off_mesh_connections.push(conn);
        idx
    }

    /// Finds the polygon whose centroid is nearest to the point.
    pub fn get_nearest_polygon(&self, point: Vec3) -> Option<u32> {
        let mut best: Option<(u32, f32)> = None;
        for (i, poly) in self.polygons.iter().enumerate() {
            let dist = (point - poly.centroid).length_squared();
            match best {
                Some((_, bd)) if dist < bd => best = Some((i as u32, dist)),
                None => best = Some((i as u32, dist)),
                _ => {}
            }
        }
        best.map(|(idx, _)| idx)
    }
}

// ============================================================================
// Pathfinding
// ============================================================================

/// A single point on a navmesh path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NavPathPoint {
    pub position: Vec3,
    pub polygon: u32,
}

/// Complete navigation path.
#[derive(Debug, Clone, Default)]
pub struct NavPath {
    pub points: Vec<NavPathPoint>,
    pub cost: f32,
}

impl NavPath {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn empty() -> Self {
        Self::new()
    }
}

/// Generic navmesh graph interface for pathfinding algorithms.
pub trait NavGraph {
    fn node_count(&self) -> usize;
    fn neighbors(&self, node: u32) -> Vec<u32>;
    fn cost(&self, from: u32, to: u32) -> f32;
    fn heuristic(&self, from: u32, to: u32) -> f32;
    fn position(&self, node: u32) -> Vec3;
}

impl NavGraph for NavMesh {
    fn node_count(&self) -> usize {
        self.polygons.len()
    }

    fn neighbors(&self, node: u32) -> Vec<u32> {
        self.links
            .iter()
            .filter(|l| l.polygon_from == node)
            .map(|l| l.polygon_to)
            .collect()
    }

    fn cost(&self, from: u32, to: u32) -> f32 {
        self.links
            .iter()
            .find(|l| l.polygon_from == from && l.polygon_to == to)
            .map(|l| l.cost)
            .unwrap_or(f32::INFINITY)
    }

    fn heuristic(&self, from: u32, to: u32) -> f32 {
        (self.position(from) - self.position(to)).length()
    }

    fn position(&self, node: u32) -> Vec3 {
        self.polygon_centroid(node)
    }
}

/// Priority queue entry for search algorithms.
#[derive(Debug, Clone, Copy, PartialEq)]
struct State {
    node: u32,
    f: f32,
}

impl Eq for State {}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f.partial_cmp(&self.f).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A* pathfinding on any NavGraph.
pub fn astar<G: NavGraph>(graph: &G, start: u32, goal: u32) -> Option<NavPath> {
    if start == goal {
        return Some(NavPath {
            points: vec![NavPathPoint { position: graph.position(start), polygon: start }],
            cost: 0.0,
        });
    }
    let mut open = BinaryHeap::new();
    let mut g_score = vec![f32::INFINITY; graph.node_count()];
    let mut f_score = vec![f32::INFINITY; graph.node_count()];
    let mut came_from = vec![None; graph.node_count()];
    g_score[start as usize] = 0.0;
    f_score[start as usize] = graph.heuristic(start, goal);
    open.push(State { node: start, f: f_score[start as usize] });

    while let Some(State { node, .. }) = open.pop() {
        if node == goal {
            let mut path = Vec::new();
            let mut curr = goal;
            while let Some(prev) = came_from[curr as usize] {
                path.push(NavPathPoint { position: graph.position(curr), polygon: curr });
                curr = prev;
            }
            path.push(NavPathPoint { position: graph.position(start), polygon: start });
            path.reverse();
            return Some(NavPath { points: path, cost: g_score[goal as usize] });
        }
        for &neighbor in &graph.neighbors(node) {
            let tentative_g = g_score[node as usize] + graph.cost(node, neighbor);
            if tentative_g < g_score[neighbor as usize] {
                came_from[neighbor as usize] = Some(node);
                g_score[neighbor as usize] = tentative_g;
                f_score[neighbor as usize] = tentative_g + graph.heuristic(neighbor, goal);
                open.push(State { node: neighbor, f: f_score[neighbor as usize] });
            }
        }
    }
    None
}

/// Dijkstra's algorithm (A* with zero heuristic).
pub fn dijkstra<G: NavGraph>(graph: &G, start: u32, goal: u32) -> Option<NavPath> {
    astar_with_heuristic(graph, start, goal, |_, _| 0.0)
}

fn astar_with_heuristic<G: NavGraph, F>(graph: &G, start: u32, goal: u32, h: F) -> Option<NavPath>
where
    F: Fn(u32, u32) -> f32,
{
    if start == goal {
        return Some(NavPath {
            points: vec![NavPathPoint { position: graph.position(start), polygon: start }],
            cost: 0.0,
        });
    }
    let mut open = BinaryHeap::new();
    let mut g_score = vec![f32::INFINITY; graph.node_count()];
    let mut came_from = vec![None; graph.node_count()];
    g_score[start as usize] = 0.0;
    open.push(State { node: start, f: 0.0 });

    while let Some(State { node, .. }) = open.pop() {
        if node == goal {
            let mut path = Vec::new();
            let mut curr = goal;
            while let Some(prev) = came_from[curr as usize] {
                path.push(NavPathPoint { position: graph.position(curr), polygon: curr });
                curr = prev;
            }
            path.push(NavPathPoint { position: graph.position(start), polygon: start });
            path.reverse();
            return Some(NavPath { points: path, cost: g_score[goal as usize] });
        }
        for &neighbor in &graph.neighbors(node) {
            let tentative_g = g_score[node as usize] + graph.cost(node, neighbor);
            if tentative_g < g_score[neighbor as usize] {
                came_from[neighbor as usize] = Some(node);
                g_score[neighbor as usize] = tentative_g;
                open.push(State { node: neighbor, f: tentative_g });
            }
        }
    }
    None
}

/// D* Lite incremental replanner.
pub struct DStarLite {
    graph: NavMesh,
    start: u32,
    goal: u32,
    rhs: Vec<f32>,
    g: Vec<f32>,
    queue: BinaryHeap<State>,
    km: f32,
    last_start: Vec3,
}

impl DStarLite {
    pub fn new(graph: NavMesh, start: u32, goal: u32) -> Self {
        let node_count = graph.node_count();
        Self {
            graph,
            start,
            goal,
            rhs: vec![f32::INFINITY; node_count],
            g: vec![f32::INFINITY; node_count],
            queue: BinaryHeap::new(),
            km: 0.0,
            last_start: Vec3::ZERO,
        }
    }

    pub fn replan(&mut self, new_start: Vec3) -> Option<NavPath> {
        self.km += (new_start - self.last_start).length();
        self.last_start = new_start;
        self.start = self.graph.get_nearest_polygon(new_start).unwrap_or(self.start);
        self.rhs[self.goal as usize] = 0.0;
        self.queue
            .push(State { node: self.goal, f: self.key(self.goal) });
        self.compute_shortest_path();
        if self.g[self.start as usize] == f32::INFINITY {
            return None;
        }
        let mut path = Vec::new();
        let mut curr = self.start;
        while curr != self.goal {
            path.push(NavPathPoint { position: self.graph.position(curr), polygon: curr });
            let mut min_rhs = f32::INFINITY;
            let mut next = curr;
            for &s in &self.graph.neighbors(curr) {
                let val = self.graph.cost(curr, s) + self.g[s as usize];
                if val < min_rhs {
                    min_rhs = val;
                    next = s;
                }
            }
            curr = next;
        }
        path.push(NavPathPoint { position: self.graph.position(self.goal), polygon: self.goal });
        Some(NavPath { points: path, cost: self.g[self.start as usize] })
    }

    fn compute_shortest_path(&mut self) {
        while let Some(top) = self.queue.peek().cloned() {
            if self.g[top.node as usize] <= self.rhs[top.node as usize] && top.node != self.goal {
                break;
            }
            self.queue.pop();
            if self.g[top.node as usize] > self.rhs[top.node as usize] {
                self.g[top.node as usize] = self.rhs[top.node as usize];
                for &pred in &self.predecessors(top.node) {
                    self.update_vertex(pred);
                }
            } else {
                self.g[top.node as usize] = f32::INFINITY;
                self.update_vertex(top.node);
                for &pred in &self.predecessors(top.node) {
                    self.update_vertex(pred);
                }
            }
        }
    }

    fn predecessors(&self, node: u32) -> Vec<u32> {
        self.graph
            .links
            .iter()
            .filter(|l| l.polygon_to == node)
            .map(|l| l.polygon_from)
            .collect()
    }

    fn update_vertex(&mut self, u: u32) {
        if u != self.goal {
            self.rhs[u as usize] = f32::INFINITY;
            for &s in &self.graph.neighbors(u) {
                self.rhs[u as usize] =
                    self.rhs[u as usize].min(self.graph.cost(u, s) + self.g[s as usize]);
            }
        }
        if !self.queue.iter().any(|st| st.node == u) {
            let key = self.key(u);
            self.queue.push(State { node: u, f: key });
        }
    }

    fn key(&self, u: u32) -> f32 {
        self.rhs[u as usize]
            .min(self.g[u as usize])
            .max(self.km + self.graph.position(u).distance(self.graph.position(self.start)))
    }
}

/// Cluster used by HPA* hierarchical pathfinding.
#[derive(Debug, Clone)]
pub struct HpaCluster {
    pub polygons: Vec<u32>,
    pub entrances: Vec<u32>,
}

/// HPA* hierarchical pathfinding structure.
#[derive(Debug, Clone)]
pub struct HpaStar {
    pub clusters: Vec<HpaCluster>,
    pub abstract_graph: HashMap<u32, Vec<(u32, f32)>>,
    pub nav_mesh: NavMesh,
}

impl HpaStar {
    pub fn new(nav_mesh: NavMesh, cluster_size: u32) -> Self {
        let mut clusters = Vec::new();
        let polygon_count = nav_mesh.polygons.len() as u32;
        let num_clusters = (polygon_count + cluster_size - 1) / cluster_size;
        for c in 0..num_clusters {
            let start = c * cluster_size;
            let end = ((c + 1) * cluster_size).min(polygon_count);
            let polys: Vec<u32> = (start..end).collect();
            clusters.push(HpaCluster { polygons: polys, entrances: Vec::new() });
        }
        let abstract_graph = HashMap::new();
        Self { clusters, abstract_graph, nav_mesh }
    }

    pub fn find_path(&self, start: u32, goal: u32) -> Option<NavPath> {
        astar(&self.nav_mesh, start, goal)
    }
}

/// Funnel algorithm for navmesh path straightening.
pub fn funnel_algorithm(_nav_mesh: &NavMesh, path: &NavPath) -> Vec<Vec3> {
    if path.points.len() < 2 {
        return path.points.iter().map(|p| p.position).collect();
    }
    let mut result = Vec::new();
    result.push(path.points[0].position);
    if path.points.len() == 2 {
        result.push(path.points[1].position);
        return result;
    }
    let mut apex = path.points[0].position;
    let mut left = path.points[1].position;
    let mut right = path.points[1].position;
    let mut i = 1;
    while i < path.points.len() - 1 {
        let p = path.points[i].position;
        let p_next = path.points[i + 1].position;
        if (left - apex).cross(p - apex).dot(Vec3::Y) < 0.0 {
            left = p;
        }
        if (right - apex).cross(p - apex).dot(Vec3::Y) > 0.0 {
            right = p;
        }
        if (left - right).length() < 1e-4 {
            i += 1;
            continue;
        }
        let dir = (p_next - apex).normalize();
        let mut perp = Vec3::Y.cross(dir).normalize();
        if perp.length_squared() < 1e-4 {
            perp = Vec3::X.cross(dir).normalize();
        }
        let new_left = p - perp * 0.5;
        let new_right = p + perp * 0.5;
        if (new_left - apex).cross(new_right - apex).dot(Vec3::Y) < 0.0 {
            result.push(apex);
            apex = p;
            left = p;
            right = p;
        }
        i += 1;
    }
    result.push(path.points.last().unwrap().position);
    result
}

// ============================================================================
// Behavior Trees
// ============================================================================

/// Result of ticking a behavior node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BehaviorStatus {
    Success,
    Failure,
    Running,
}

/// Blackboard for sharing data between behavior tree nodes.
#[derive(Debug, Default)]
pub struct Blackboard {
    data: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl Blackboard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set<T: Any + Send + Sync>(&mut self, key: impl Into<String>, value: T) {
        self.data.insert(key.into(), Box::new(value));
    }

    pub fn get<T: Any + Send + Sync>(&self, key: &str) -> Option<&T> {
        self.data.get(key).and_then(|b| b.downcast_ref::<T>())
    }

    pub fn get_mut<T: Any + Send + Sync>(&mut self, key: &str) -> Option<&mut T> {
        self.data.get_mut(key).and_then(|b| b.downcast_mut::<T>())
    }

    pub fn remove<T: Any + Send + Sync>(&mut self, key: &str) -> Option<Box<dyn Any + Send + Sync>> {
        self.data.remove(key)
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }
}

/// Trait implemented by all behavior tree nodes.
pub trait BehaviorNode: Send + Sync {
    fn tick(&self, blackboard: &mut Blackboard) -> BehaviorStatus;
    fn reset(&self, _blackboard: &mut Blackboard) {}
}

/// Composite node that runs children in sequence.
#[derive(Debug, Default)]
pub struct SequenceNode {
    pub children: Vec<Box<dyn BehaviorNode>>,
}

impl BehaviorNode for SequenceNode {
    fn tick(&self, blackboard: &mut Blackboard) -> BehaviorStatus {
        for child in &self.children {
            match child.tick(blackboard) {
                BehaviorStatus::Failure => return BehaviorStatus::Failure,
                BehaviorStatus::Running => return BehaviorStatus::Running,
                BehaviorStatus::Success => {}
            }
        }
        BehaviorStatus::Success
    }
}

/// Composite node that runs children until one succeeds.
#[derive(Debug, Default)]
pub struct SelectorNode {
    pub children: Vec<Box<dyn BehaviorNode>>,
}

impl BehaviorNode for SelectorNode {
    fn tick(&self, blackboard: &mut Blackboard) -> BehaviorStatus {
        for child in &self.children {
            match child.tick(blackboard) {
                BehaviorStatus::Success => return BehaviorStatus::Success,
                BehaviorStatus::Running => return BehaviorStatus::Running,
                BehaviorStatus::Failure => {}
            }
        }
        BehaviorStatus::Failure
    }
}

/// Composite node that runs all children in parallel.
#[derive(Debug, Default)]
pub struct ParallelNode {
    pub children: Vec<Box<dyn BehaviorNode>>,
    pub policy: ParallelPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParallelPolicy {
    RequireAll,
    RequireOne,
}

impl BehaviorNode for ParallelNode {
    fn tick(&self, blackboard: &mut Blackboard) -> BehaviorStatus {
        let mut successes = 0;
        let mut failures = 0;
        let mut running = false;
        for child in &self.children {
            match child.tick(blackboard) {
                BehaviorStatus::Success => successes += 1,
                BehaviorStatus::Failure => failures += 1,
                BehaviorStatus::Running => running = true,
            }
        }
        match self.policy {
            ParallelPolicy::RequireAll => {
                if running { BehaviorStatus::Running } else if failures == 0 { BehaviorStatus::Success } else { BehaviorStatus::Failure }
            }
            ParallelPolicy::RequireOne => {
                if successes > 0 { BehaviorStatus::Success } else if running { BehaviorStatus::Running } else { BehaviorStatus::Failure }
            }
        }
    }
}

/// Decorator node that inverts the child's result.
#[derive(Debug, Default)]
pub struct InverterNode {
    pub child: Option<Box<dyn BehaviorNode>>,
}

impl BehaviorNode for InverterNode {
    fn tick(&self, blackboard: &mut Blackboard) -> BehaviorStatus {
        match &self.child {
            Some(c) => match c.tick(blackboard) {
                BehaviorStatus::Success => BehaviorStatus::Failure,
                BehaviorStatus::Failure => BehaviorStatus::Success,
                BehaviorStatus::Running => BehaviorStatus::Running,
            },
            None => BehaviorStatus::Failure,
        }
    }
}

/// Repeater decorator that retries a child N times.
#[derive(Debug, Default)]
pub struct RepeaterNode {
    pub child: Option<Box<dyn BehaviorNode>>,
    pub max_loops: u32,
}

impl BehaviorNode for RepeaterNode {
    fn tick(&self, blackboard: &mut Blackboard) -> BehaviorStatus {
        let count = blackboard.get::<u32>("repeater_counter").copied().unwrap_or(0);
        if count >= self.max_loops {
            return BehaviorStatus::Success;
        }
        match &self.child {
            Some(c) => match c.tick(blackboard) {
                BehaviorStatus::Success => {
                    blackboard.set("repeater_counter", count + 1);
                    BehaviorStatus::Success
                }
                BehaviorStatus::Failure => BehaviorStatus::Failure,
                BehaviorStatus::Running => BehaviorStatus::Running,
            },
            None => BehaviorStatus::Failure,
        }
    }
}

/// Action node that runs a custom closure.
#[derive(Debug, Default)]
pub struct ActionNode {
    pub action: Option<Box<dyn Fn(&mut Blackboard) -> BehaviorStatus + Send + Sync>>,
}

impl BehaviorNode for ActionNode {
    fn tick(&self, blackboard: &mut Blackboard) -> BehaviorStatus {
        match &self.action {
            Some(a) => a(blackboard),
            None => BehaviorStatus::Failure,
        }
    }
}

/// Condition node that checks a predicate.
#[derive(Debug, Default)]
pub struct ConditionNode {
    pub condition: Option<Box<dyn Fn(&Blackboard) -> bool + Send + Sync>>,
}

impl BehaviorNode for ConditionNode {
    fn tick(&self, blackboard: &mut Blackboard) -> BehaviorStatus {
        match &self.condition {
            Some(c) if c(blackboard) => BehaviorStatus::Success,
            Some(_) => BehaviorStatus::Failure,
            None => BehaviorStatus::Failure,
        }
    }
}

/// Behavior tree root.
#[derive(Debug, Default)]
pub struct BehaviorTree {
    pub root: Option<Box<dyn BehaviorNode>>,
    pub blackboard: Blackboard,
}

impl BehaviorTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tick(&mut self) -> BehaviorStatus {
        match &self.root {
            Some(root) => root.tick(&mut self.blackboard),
            None => BehaviorStatus::Failure,
        }
    }

    pub fn reset(&mut self) {
        if let Some(root) = &self.root {
            root.reset(&mut self.blackboard);
        }
        self.blackboard = Blackboard::new();
    }
}

/// Compiled behavior tree as a state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BtState {
    Idle,
    Running,
    Success,
    Failure,
}

#[derive(Debug, Clone, Default)]
pub struct CompiledBehaviorTree {
    pub states: Vec<BtState>,
    pub transitions: Vec<BtTransition>,
    pub current_state: BtState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BtTransition {
    pub from: BtState,
    pub to: BtState,
}

impl CompiledBehaviorTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tick(&mut self) -> BtState {
        self.current_state
    }
}

// ============================================================================
// State Machines
// ============================================================================

/// Trigger for a state transition.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TransitionTrigger {
    Event(String),
    Timeout(f32),
    Condition(String),
}

/// Transition between two states.
#[derive(Debug, Clone)]
pub struct StateTransition {
    pub from: String,
    pub to: String,
    pub trigger: TransitionTrigger,
}

/// A single state.
pub struct State {
    pub name: String,
    pub entry_action: Option<Box<dyn Fn(&mut Blackboard) + Send + Sync>>,
    pub exit_action: Option<Box<dyn Fn(&mut Blackboard) + Send + Sync>>,
    pub transitions: Vec<StateTransition>,
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("State").field("name", &self.name).finish()
    }
}

/// Hierarchical state machine.
#[derive(Debug, Default)]
pub struct HierarchicalStateMachine {
    pub states: HashMap<String, State>,
    pub current: String,
    pub blackboard: Blackboard,
    pub time_in_state: f32,
}

impl HierarchicalStateMachine {
    pub fn new(initial: impl Into<String>) -> Self {
        let initial = initial.into();
        Self {
            states: HashMap::new(),
            current: initial.clone(),
            blackboard: Blackboard::new(),
            time_in_state: 0.0,
        }
    }

    pub fn add_state(&mut self, state: State) {
        self.states.insert(state.name.clone(), state);
    }

    pub fn tick(&mut self, dt: f32) {
        self.time_in_state += dt;
        if let Some(state) = self.states.get(&self.current) {
            for trans in &state.transitions {
                let triggered = match &trans.trigger {
                    TransitionTrigger::Event(e) => {
                        self.blackboard.contains_key(&format!("event_{}", e))
                    }
                    TransitionTrigger::Timeout(t) => self.time_in_state >= *t,
                    TransitionTrigger::Condition(c) => self.blackboard.contains_key(c),
                };
                if triggered {
                    if let Some(from_state) = self.states.get(&self.current) {
                        if let Some(exit) = &from_state.exit_action {
                            exit(&mut self.blackboard);
                        }
                    }
                    self.current = trans.to.clone();
                    self.time_in_state = 0.0;
                    if let Some(to_state) = self.states.get(&self.current) {
                        if let Some(entry) = &to_state.entry_action {
                            entry(&mut self.blackboard);
                        }
                    }
                    break;
                }
            }
        }
    }

    pub fn current_state(&self) -> Option<&State> {
        self.states.get(&self.current)
    }
}

// ============================================================================
// Perception System
// ============================================================================

/// Vision cone parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct VisionCone {
    pub origin: Vec3,
    pub direction: Vec3,
    pub fov: f32,
    pub range: f32,
}

impl VisionCone {
    pub fn new(origin: Vec3, direction: Vec3, fov: f32, range: f32) -> Self {
        Self { origin, direction: direction.normalize(), fov, range }
    }

    /// Checks if a point is inside the vision cone.
    pub fn contains(&self, point: Vec3) -> bool {
        let dir = point - self.origin;
        let dist = dir.length();
        if dist > self.range {
            return false;
        }
        let dot = dir.normalize().dot(self.direction);
        dot > self.fov.cos()
    }
}

/// Hearing parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct Hearing {
    pub origin: Vec3,
    pub range: f32,
}

impl Hearing {
    pub fn new(origin: Vec3, range: f32) -> Self {
        Self { origin, range }
    }

    pub fn can_hear(&self, point: Vec3) -> bool {
        (point - self.origin).length() <= self.range
    }
}

/// Stimulus detected by perception.
#[derive(Debug, Clone)]
pub struct Stimulus {
    pub entity: Entity,
    pub position: Vec3,
    pub strength: f32,
    pub time: f32,
}

impl Stimulus {
    pub fn new(entity: Entity, position: Vec3, strength: f32) -> Self {
        Self { entity, position, strength, time: 0.0 }
    }
}

/// Sensory memory storing past stimuli.
#[derive(Debug, Clone, Default)]
pub struct SensoryMemory {
    pub stimuli: HashMap<Entity, Stimulus>,
    pub memory_duration: f32,
}

impl SensoryMemory {
    pub fn new(memory_duration: f32) -> Self {
        Self { stimuli: HashMap::new(), memory_duration }
    }

    pub fn record(&mut self, stimulus: Stimulus) {
        self.stimuli.insert(stimulus.entity, stimulus);
    }

    pub fn forget_old(&mut self, current_time: f32) {
        self.stimuli.retain(|_, s| current_time - s.time <= self.memory_duration);
    }

    pub fn get(&self, entity: Entity) -> Option<&Stimulus> {
        self.stimuli.get(&entity)
    }
}

/// Perception system aggregating vision and hearing.
#[derive(Debug, Clone, Default)]
pub struct PerceptionSystem {
    pub vision_cones: Vec<VisionCone>,
    pub hearing: Vec<Hearing>,
    pub memory: SensoryMemory,
    pub current_time: f32,
}

impl PerceptionSystem {
    pub fn new(memory_duration: f32) -> Self {
        Self {
            vision_cones: Vec::new(),
            hearing: Vec::new(),
            memory: SensoryMemory::new(memory_duration),
            current_time: 0.0,
        }
    }

    pub fn add_vision_cone(&mut self, cone: VisionCone) {
        self.vision_cones.push(cone);
    }

    pub fn add_hearing(&mut self, hear: Hearing) {
        self.hearing.push(hear);
    }

    pub fn update(&mut self, dt: f32) {
        self.current_time += dt;
        self.memory.forget_old(self.current_time);
    }

    /// Performs a line-of-sight check using a simple raycast against bounds.
    pub fn line_of_sight(&self, from: Vec3, to: Vec3, obstacles: &[Aabb]) -> bool {
        let dir = to - from;
        let dist = dir.length();
        if dist < 1e-4 {
            return true;
        }
        let ray = crate::math::Ray::new(from, dir / dist);
        for aabb in obstacles {
            if ray.intersects_aabb(aabb).is_some() {
                return false;
            }
        }
        true
    }

    /// Queries vision cones for a target position.
    pub fn query_vision(&self, target: Vec3) -> bool {
        self.vision_cones.iter().any(|cone| cone.contains(target))
    }
}

// ============================================================================
// Group AI
// ============================================================================

/// Formation shape definitions.
#[derive(Debug, Clone, PartialEq)]
pub enum FormationShape {
    Line { count: usize, spacing: f32 },
    Circle { count: usize, radius: f32 },
    Wedge { count: usize, spacing: f32 },
}

/// Formation system for group movement.
#[derive(Debug, Clone, Default)]
pub struct Formation {
    pub shape: FormationShape,
    pub slots: Vec<Vec3>,
    pub center: Vec3,
    pub facing: Vec3,
}

impl Formation {
    pub fn new(shape: FormationShape, center: Vec3, facing: Vec3) -> Self {
        let mut formation = Self { shape, slots: Vec::new(), center, facing };
        formation.recalculate();
        formation
    }

    /// Recalculates slot positions from the formation shape.
    pub fn recalculate(&mut self) {
        self.slots.clear();
        match self.shape {
            FormationShape::Line { count, spacing } => {
                let total = (count - 1) as f32 * spacing;
                for i in 0..count {
                    let offset = (i as f32 * spacing) - total * 0.5;
                    let right = Vec3::Y.cross(self.facing).normalize();
                    self.slots.push(self.center + right * offset);
                }
            }
            FormationShape::Circle { count, radius } => {
                for i in 0..count {
                    let angle = (i as f32 / count as f32) * std::f32::consts::TAU;
                    let dir = Vec3::new(angle.cos(), 0.0, angle.sin());
                    self.slots.push(self.center + dir * radius);
                }
            }
            FormationShape::Wedge { count, spacing } => {
                for i in 0..count {
                    let row = (i / 2) as f32 * spacing;
                    let side = if i % 2 == 0 { -1.0 } else { 1.0 };
                    let col = (i as f32 / 2.0) * spacing * side;
                    let right = Vec3::Y.cross(self.facing).normalize();
                    self.slots.push(self.center + self.facing * row + right * col);
                }
            }
        }
    }

    /// Returns the slot for a given unit index.
    pub fn get_slot(&self, index: usize) -> Option<Vec3> {
        self.slots.get(index).copied()
    }

    /// Updates the formation center and facing.
    pub fn update_transform(&mut self, center: Vec3, facing: Vec3) {
        self.center = center;
        self.facing = facing;
        self.recalculate();
    }
}

/// Single boid used in flocking.
#[derive(Debug, Clone, PartialEq)]
pub struct Boid {
    pub position: Vec3,
    pub velocity: Vec3,
    pub acceleration: Vec3,
}

impl Boid {
    pub fn new(position: Vec3, velocity: Vec3) -> Self {
        Self { position, velocity, acceleration: Vec3::ZERO }
    }
}

/// Flocking (boids) system.
#[derive(Debug, Clone, Default)]
pub struct Flock {
    pub boids: Vec<Boid>,
    pub separation_weight: f32,
    pub alignment_weight: f32,
    pub cohesion_weight: f32,
    pub max_speed: f32,
    pub max_force: f32,
    pub neighbor_radius: f32,
}

impl Flock {
    pub fn new(max_speed: f32, max_force: f32, neighbor_radius: f32) -> Self {
        Self {
            boids: Vec::new(),
            separation_weight: 1.5,
            alignment_weight: 1.0,
            cohesion_weight: 1.0,
            max_speed,
            max_force,
            neighbor_radius,
        }
    }

    pub fn add_boid(&mut self, boid: Boid) {
        self.boids.push(boid);
    }

    pub fn update(&mut self, dt: f32) {
        for boid in &mut self.boids {
            let sep = self.separation(boid);
            let ali = self.alignment(boid);
            let coh = self.cohesion(boid);
            boid.acceleration = sep * self.separation_weight
                + ali * self.alignment_weight
                + coh * self.cohesion_weight;
            boid.velocity = (boid.velocity + boid.acceleration * dt).clamp_length_max(self.max_speed);
            boid.position += boid.velocity * dt;
        }
    }

    fn separation(&self, boid: &Boid) -> Vec3 {
        let mut steer = Vec3::ZERO;
        let mut count = 0;
        for other in &self.boids {
            let d = (other.position - boid.position).length();
            if other.position != boid.position && d < self.neighbor_radius * 0.5 {
                let diff = (boid.position - other.position).normalize() / d;
                steer += diff;
                count += 1;
            }
        }
        if count > 0 {
            steer = (steer / count as f32).normalize() * self.max_speed - boid.velocity;
            steer = steer.clamp_length_max(self.max_force);
        }
        steer
    }

    fn alignment(&self, boid: &Boid) -> Vec3 {
        let mut sum = Vec3::ZERO;
        let mut count = 0;
        for other in &self.boids {
            let d = (other.position - boid.position).length();
            if other.position != boid.position && d < self.neighbor_radius {
                sum += other.velocity;
                count += 1;
            }
        }
        if count > 0 {
            let avg = sum / count as f32;
            let steer = (avg.normalize() * self.max_speed - boid.velocity).clamp_length_max(self.max_force);
            return steer;
        }
        Vec3::ZERO
    }

    fn cohesion(&self, boid: &Boid) -> Vec3 {
        let mut sum = Vec3::ZERO;
        let mut count = 0;
        for other in &self.boids {
            let d = (other.position - boid.position).length();
            if other.position != boid.position && d < self.neighbor_radius {
                sum += other.position;
                count += 1;
            }
        }
        if count > 0 {
            let target = sum / count as f32;
            let desired = (target - boid.position).normalize() * self.max_speed;
            let steer = (desired - boid.velocity).clamp_length_max(self.max_force);
            return steer;
        }
        Vec3::ZERO
    }
}

/// RVO (Reciprocal Velocity Obstacles) collision avoidance agent.
#[derive(Debug, Clone, PartialEq)]
pub struct RvoAgent {
    pub position: Vec3,
    pub velocity: Vec3,
    pub radius: f32,
    pub max_speed: f32,
    pub goal: Vec3,
}

impl RvoAgent {
    pub fn new(position: Vec3, goal: Vec3, radius: f32, max_speed: f32) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            radius,
            max_speed,
            goal,
        }
    }
}

/// RVO collision avoidance system.
#[derive(Debug, Clone, Default)]
pub struct RvoCollisionAvoidance {
    pub agents: Vec<RvoAgent>,
    pub time_horizon: f32,
    pub neighbor_radius: f32,
}

impl RvoCollisionAvoidance {
    pub fn new(time_horizon: f32, neighbor_radius: f32) -> Self {
        Self { agents: Vec::new(), time_horizon, neighbor_radius }
    }

    pub fn add_agent(&mut self, agent: RvoAgent) {
        self.agents.push(agent);
    }

    pub fn update(&mut self, dt: f32) {
        let mut new_velocities = Vec::with_capacity(self.agents.len());
        for agent in &self.agents {
            let mut velocity = (agent.goal - agent.position).normalize() * agent.max_speed;
            for other in &self.agents {
                if other.position == agent.position {
                    continue;
                }
                let diff = other.position - agent.position;
                let dist = diff.length();
                if dist < self.neighbor_radius && dist > 1e-6 {
                    let u = (diff / dist) * (agent.radius + other.radius);
                    let rel_vel = agent.velocity - other.velocity;
                    let u_dot_rel = u.dot(rel_vel);
                    if u_dot_rel > 0.0 {
                        let factor = (1.0 - u_dot_rel / self.time_horizon).max(0.0);
                        velocity -= u * factor / dist;
                    }
                }
            }
            velocity = velocity.clamp_length_max(agent.max_speed);
            new_velocities.push(velocity);
        }
        for (agent, vel) in self.agents.iter_mut().zip(new_velocities) {
            agent.velocity = vel;
            agent.position += agent.velocity * dt;
        }
    }
}
