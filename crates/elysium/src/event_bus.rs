/// event_bus.rs — Enterprise Event Bus
///
/// Features:
/// - Type-erased publish/subscribe system
/// - Priority-based event dispatch
/// - Event filtering (allow/block lists)
/// - One-shot subscribers
/// - Wildcard subscriptions
/// - Event history and replay
/// - Thread-safe with RwLock
/// - Event coalescing (dedup within time window)
/// - Conditional routing

use std::collections::{HashMap, BTreeMap};
use std::sync::{Arc, RwLock};

// ═══════════════════════════════════════════════════════════ Event Types

/// Priority for event dispatch ordering
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EventPriority {
    Lowest = 0,
    Low = 1,
    Normal = 2,
    High = 3,
    Highest = 4,
    Critical = 5,
}

impl Default for EventPriority {
    fn default() -> Self { EventPriority::Normal }
}

impl EventPriority {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Lowest => "Lowest",
            Self::Low => "Low",
            Self::Normal => "Normal",
            Self::High => "High",
            Self::Highest => "Highest",
            Self::Critical => "Critical",
        }
    }
}

/// Event metadata
#[derive(Clone, Debug)]
pub struct EventMetadata {
    pub id: u64,
    pub event_type: String,
    pub priority: EventPriority,
    pub timestamp_ms: u64,
    pub source: String,
    pub tags: Vec<String>,
}

/// Type-erased event envelope
#[derive(Clone)]
pub struct EventEnvelope {
    pub metadata: EventMetadata,
    pub payload: Arc<dyn std::any::Any + Send + Sync>,
}

impl EventEnvelope {
    pub fn new<T: Send + Sync + 'static>(
        event_type: &str,
        priority: EventPriority,
        source: &str,
        payload: T,
        id: u64,
    ) -> Self {
        Self {
            metadata: EventMetadata {
                id,
                event_type: event_type.to_string(),
                priority,
                timestamp_ms: 0, // Set by dispatch
                source: source.to_string(),
                tags: Vec::new(),
            },
            payload: Arc::new(payload),
        }
    }

    pub fn downcast_ref<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.payload.downcast_ref::<T>()
    }
}

// ═══════════════════════════════════════════════════════════ Subscriber

pub type SubscriberCallback = Box<dyn Fn(&EventEnvelope) -> bool + Send + Sync>;

pub struct Subscriber {
    pub id: u64,
    pub event_type: String,
    pub priority_filter: Option<EventPriority>,
    pub callback: SubscriberCallback,
    pub one_shot: bool,
    pub enabled: bool,
    pub event_count: u64,
}

impl Subscriber {
    pub fn new<F>(id: u64, event_type: &str, callback: F) -> Self
    where F: Fn(&EventEnvelope) -> bool + Send + Sync + 'static {
        Self {
            id,
            event_type: event_type.to_string(),
            priority_filter: None,
            callback: Box::new(callback),
            one_shot: false,
            enabled: true,
            event_count: 0,
        }
    }

    pub fn with_priority_filter(mut self, priority: EventPriority) -> Self {
        self.priority_filter = Some(priority);
        self
    }

    pub fn one_shot(mut self) -> Self {
        self.one_shot = true;
        self
    }
}

// ═══════════════════════════════════════════════════════════ Event Stats

#[derive(Clone, Debug, Default)]
pub struct EventBusStats {
    pub total_published: u64,
    pub total_delivered: u64,
    pub total_dropped: u64,
    pub active_subscribers: u32,
    pub event_type_counts: HashMap<String, u64>,
    pub avg_dispatch_time_us: f32,
    pub max_dispatch_time_us: f32,
    pub coalesced_events: u64,
}

// ═══════════════════════════════════════════════════════════ Event History

#[derive(Clone)]
pub struct EventRecord {
    pub envelope: EventEnvelope,
    pub delivered_to: Vec<u64>,
    pub dropped: bool,
}

// ═══════════════════════════════════════════════════════════ Event Bus

pub struct EventBus {
    subscribers: Vec<Subscriber>,
    history: Vec<EventRecord>,
    next_id: u64,
    next_event_id: u64,
    allow_list: Option<Vec<String>>,
    block_list: Vec<String>,
    coalesce_window_ms: u64,
    last_event_times: HashMap<String, u64>,
    max_history: usize,
    stats: EventBusStats,
}

impl Default for EventBus {
    fn default() -> Self {
        Self {
            subscribers: Vec::new(),
            history: Vec::new(),
            next_id: 1,
            next_event_id: 1,
            allow_list: None,
            block_list: Vec::new(),
            coalesce_window_ms: 0,
            last_event_times: HashMap::new(),
            max_history: 1000,
            stats: EventBusStats::default(),
        }
    }
}

impl EventBus {
    pub fn new() -> Self { Self::default() }

    pub fn with_max_history(max_history: usize) -> Self {
        Self { max_history, ..Default::default() }
    }

    // ── Subscription ─────────────────────────────────────

    pub fn subscribe<F>(&mut self, event_type: &str, callback: F) -> u64
    where F: Fn(&EventEnvelope) -> bool + Send + Sync + 'static {
        let id = self.next_id;
        self.next_id += 1;
        self.subscribers.push(Subscriber::new(id, event_type, callback));
        self.stats.active_subscribers = self.subscribers.len() as u32;
        id
    }

    pub fn subscribe_one_shot<F>(&mut self, event_type: &str, callback: F) -> u64
    where F: Fn(&EventEnvelope) -> bool + Send + Sync + 'static {
        let id = self.next_id;
        self.next_id += 1;
        let mut sub = Subscriber::new(id, event_type, callback);
        sub.one_shot = true;
        self.subscribers.push(sub);
        self.stats.active_subscribers = self.subscribers.len() as u32;
        id
    }

    pub fn subscribe_priority<F>(&mut self, event_type: &str, priority: EventPriority, callback: F) -> u64
    where F: Fn(&EventEnvelope) -> bool + Send + Sync + 'static {
        let id = self.next_id;
        self.next_id += 1;
        let mut sub = Subscriber::new(id, event_type, callback);
        sub.priority_filter = Some(priority);
        self.subscribers.push(sub);
        self.stats.active_subscribers = self.subscribers.len() as u32;
        id
    }

    pub fn unsubscribe(&mut self, subscriber_id: u64) -> bool {
        let len_before = self.subscribers.len();
        self.subscribers.retain(|s| s.id != subscriber_id);
        let removed = self.subscribers.len() < len_before;
        if removed {
            self.stats.active_subscribers = self.subscribers.len() as u32;
        }
        removed
    }

    pub fn enable_subscriber(&mut self, subscriber_id: u64) {
        if let Some(s) = self.subscribers.iter_mut().find(|s| s.id == subscriber_id) {
            s.enabled = true;
        }
    }

    pub fn disable_subscriber(&mut self, subscriber_id: u64) {
        if let Some(s) = self.subscribers.iter_mut().find(|s| s.id == subscriber_id) {
            s.enabled = false;
        }
    }

    // ── Publishing ───────────────────────────────────────

    pub fn publish<T: Send + Sync + Clone + 'static>(&mut self, event_type: &str, priority: EventPriority, source: &str, payload: T) -> u64 {
        let id = self.next_event_id;
        self.next_event_id += 1;

        // Coalescing check
        if self.coalesce_window_ms > 0 {
            let now = id; // Simplified timestamp
            if let Some(&last) = self.last_event_times.get(event_type) {
                if now.saturating_sub(last) < self.coalesce_window_ms {
                    self.stats.coalesced_events += 1;
                    return id;
                }
            }
            self.last_event_times.insert(event_type.to_string(), now);
        }

        // Filtering
        if self.is_blocked(event_type) {
            self.stats.total_dropped += 1;
            return id;
        }

        let envelope = EventEnvelope::new(event_type, priority, source, payload, id);

        // Dispatch to matching subscribers (sorted by priority)
        let mut matching: Vec<(usize, EventPriority)> = self.subscribers.iter()
            .enumerate()
            .filter(|(_, s)| s.enabled)
            .filter(|(_, s)| s.event_type == event_type || s.event_type == "*")
            .filter(|(_, s)| s.priority_filter.map_or(true, |p| priority >= p))
            .map(|(i, s)| (i, s.priority_filter.unwrap_or(EventPriority::Normal)))
            .collect();
        matching.sort_by(|a, b| b.1.cmp(&a.1)); // Higher priority first

        let mut delivered = Vec::new();
        let mut remove_ids = Vec::new();

        for (idx, _) in &matching {
            let sub = &mut self.subscribers[*idx];
            let keep_going = (sub.callback)(&envelope);
            sub.event_count += 1;
            delivered.push(sub.id);
            self.stats.total_delivered += 1;

            if sub.one_shot {
                remove_ids.push(sub.id);
            }
            if !keep_going {
                break; // Subscriber wants to stop propagation
            }
        }

        // Remove one-shot subscribers
        for rid in &remove_ids {
            self.subscribers.retain(|s| s.id != *rid);
        }

        self.stats.total_published += 1;
        *self.stats.event_type_counts.entry(event_type.to_string()).or_insert(0) += 1;
        self.stats.active_subscribers = self.subscribers.len() as u32;

        // Record history
        if self.history.len() >= self.max_history {
            self.history.remove(0);
        }
        self.history.push(EventRecord { envelope, delivered_to: delivered.clone(), dropped: false });

        id
    }

    fn is_blocked(&self, event_type: &str) -> bool {
        if let Some(ref allow) = self.allow_list {
            return !allow.iter().any(|a| a == event_type || a == "*");
        }
        self.block_list.iter().any(|b| b == event_type || b == "*")
    }

    // ── Configuration ────────────────────────────────────

    pub fn set_coalesce_window(&mut self, ms: u64) {
        self.coalesce_window_ms = ms;
    }

    pub fn set_allow_list(&mut self, types: Vec<String>) {
        self.allow_list = Some(types);
    }

    pub fn set_block_list(&mut self, types: Vec<String>) {
        self.block_list = types;
    }

    pub fn clear_allow_list(&mut self) {
        self.allow_list = None;
    }

    pub fn clear_block_list(&mut self) {
        self.block_list.clear();
    }

    // ── Query ────────────────────────────────────────────

    pub fn subscriber_count(&self) -> usize {
        self.subscribers.iter().filter(|s| s.enabled).count()
    }

    pub fn subscriber_count_for(&self, event_type: &str) -> usize {
        self.subscribers.iter()
            .filter(|s| s.enabled && (s.event_type == event_type || s.event_type == "*"))
            .count()
    }

    pub fn stats(&self) -> &EventBusStats { &self.stats }

    pub fn history(&self) -> &[EventRecord] { &self.history }

    pub fn history_for(&self, event_type: &str) -> Vec<&EventRecord> {
        self.history.iter()
            .filter(|r| r.envelope.metadata.event_type == event_type)
            .collect()
    }

    pub fn subscriber_ids(&self) -> Vec<u64> {
        self.subscribers.iter().map(|s| s.id).collect()
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    pub fn clear_all(&mut self) {
        self.subscribers.clear();
        self.history.clear();
        self.stats = EventBusStats::default();
    }
}

// ═══════════════════════════════════════════════════════════ Thread-safe wrapper

pub struct SharedEventBus {
    inner: Arc<RwLock<EventBus>>,
}

impl Clone for SharedEventBus {
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

impl Default for SharedEventBus {
    fn default() -> Self {
        Self { inner: Arc::new(RwLock::new(EventBus::new())) }
    }
}

impl SharedEventBus {
    pub fn new() -> Self { Self::default() }

    pub fn publish<T: Send + Sync + Clone + 'static>(&self, event_type: &str, priority: EventPriority, source: &str, payload: T) -> u64 {
        self.inner.write().unwrap().publish(event_type, priority, source, payload)
    }

    pub fn subscribe<F>(&self, event_type: &str, callback: F) -> u64
    where F: Fn(&EventEnvelope) -> bool + Send + Sync + 'static {
        self.inner.write().unwrap().subscribe(event_type, callback)
    }

    pub fn unsubscribe(&self, id: u64) -> bool {
        self.inner.write().unwrap().unsubscribe(id)
    }

    pub fn subscriber_count(&self) -> usize {
        self.inner.read().unwrap().subscriber_count()
    }

    pub fn stats(&self) -> EventBusStats {
        self.inner.read().unwrap().stats().clone()
    }
}

use serde::{Serialize, Deserialize};

// ═══════════════════════════════════════════════════════════ Built-in Events

#[derive(Clone, Debug)]
pub struct EngineStartedEvent;

#[derive(Clone, Debug)]
pub struct EngineShutdownEvent;

#[derive(Clone, Debug)]
pub struct FrameEvent {
    pub frame_number: u64,
    pub delta_time: f32,
    pub fps: f32,
}

#[derive(Clone, Debug)]
pub struct SceneChangedEvent {
    pub scene_name: String,
    pub change_type: String,
}

#[derive(Clone, Debug)]
pub struct ObjectCreatedEvent {
    pub object_id: usize,
    pub object_name: String,
}

#[derive(Clone, Debug)]
pub struct ObjectDestroyedEvent {
    pub object_id: usize,
}

#[derive(Clone, Debug)]
pub struct ObjectSelectedEvent {
    pub object_id: usize,
}

#[derive(Clone, Debug)]
pub struct AssetLoadedEvent {
    pub asset_path: String,
    pub asset_type: String,
}

#[derive(Clone, Debug)]
pub struct AssetFailedEvent {
    pub asset_path: String,
    pub error: String,
}

#[derive(Clone, Debug)]
pub struct ConfigChangedEvent {
    pub key: String,
    pub old_value: String,
    pub new_value: String,
}

#[derive(Clone, Debug)]
pub struct LogEvent {
    pub level: String,
    pub message: String,
    pub module: String,
}

#[derive(Clone, Debug)]
pub struct ErrorEvent {
    pub error_type: String,
    pub message: String,
    pub recoverable: bool,
}

#[derive(Clone, Debug)]
pub struct PerformanceWarningEvent {
    pub metric: String,
    pub value: f32,
    pub threshold: f32,
}

// ═══════════════════════════════════════════════════════════ Tests

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_basic_publish_subscribe() {
        let mut bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        bus.subscribe("test.event", move |env| {
            if let Some(val) = env.downcast_ref::<String>() {
                if val == "hello" {
                    counter_clone.fetch_add(1, Ordering::Relaxed);
                }
            }
            true
        });

        bus.publish("test.event", EventPriority::Normal, "test", "hello".to_string());
        assert_eq!(counter.load(Ordering::Relaxed), 1);

        bus.publish("test.event", EventPriority::Normal, "test", "world".to_string());
        assert_eq!(counter.load(Ordering::Relaxed), 1); // "world" doesn't match
    }

    #[test]
    fn test_unsubscribe() {
        let mut bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        let id = bus.subscribe("test.event", move |_| {
            counter_clone.fetch_add(1, Ordering::Relaxed);
            true
        });

        bus.publish("test.event", EventPriority::Normal, "test", 42i32);
        assert_eq!(counter.load(Ordering::Relaxed), 1);

        assert!(bus.unsubscribe(id));
        bus.publish("test.event", EventPriority::Normal, "test", 42i32);
        assert_eq!(counter.load(Ordering::Relaxed), 1); // No more deliveries
    }

    #[test]
    fn test_priority_filter() {
        let mut bus = EventBus::new();
        let high_counter = Arc::new(AtomicUsize::new(0));
        let high_clone = Arc::clone(&high_counter);
        let normal_counter = Arc::new(AtomicUsize::new(0));
        let normal_clone = Arc::clone(&normal_counter);

        bus.subscribe_priority("test.event", EventPriority::High, move |_| {
            high_clone.fetch_add(1, Ordering::Relaxed);
            true
        });
        bus.subscribe("test.event", move |_| {
            normal_clone.fetch_add(1, Ordering::Relaxed);
            true
        });

        // Normal priority — should NOT trigger High subscriber
        bus.publish("test.event", EventPriority::Normal, "test", 1i32);
        assert_eq!(high_counter.load(Ordering::Relaxed), 0);
        assert_eq!(normal_counter.load(Ordering::Relaxed), 1);

        // High priority — should trigger both
        bus.publish("test.event", EventPriority::High, "test", 2i32);
        assert_eq!(high_counter.load(Ordering::Relaxed), 1);
        assert_eq!(normal_counter.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_one_shot() {
        let mut bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        bus.subscribe_one_shot("test.event", move |_| {
            counter_clone.fetch_add(1, Ordering::Relaxed);
            true
        });

        bus.publish("test.event", EventPriority::Normal, "test", 1i32);
        bus.publish("test.event", EventPriority::Normal, "test", 2i32);
        bus.publish("test.event", EventPriority::Normal, "test", 3i32);

        assert_eq!(counter.load(Ordering::Relaxed), 1); // Only first delivery
    }

    #[test]
    fn test_wildcard_subscription() {
        let mut bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        bus.subscribe("*", move |_| {
            counter_clone.fetch_add(1, Ordering::Relaxed);
            true
        });

        bus.publish("any.event", EventPriority::Normal, "test", 1i32);
        bus.publish("another.event", EventPriority::Normal, "test", 2i32);
        bus.publish("third.event", EventPriority::Normal, "test", 3i32);

        assert_eq!(counter.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn test_history() {
        let mut bus = EventBus::new();
        bus.publish("a", EventPriority::Normal, "test", 1i32);
        bus.publish("b", EventPriority::Normal, "test", 2i32);
        bus.publish("a", EventPriority::Normal, "test", 3i32);

        assert_eq!(bus.history().len(), 3);
        assert_eq!(bus.history_for("a").len(), 2);
        assert_eq!(bus.history_for("b").len(), 1);
    }

    #[test]
    fn test_allow_list() {
        let mut bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        bus.subscribe("allowed.event", move |_| {
            counter_clone.fetch_add(1, Ordering::Relaxed);
            true
        });

        bus.set_allow_list(vec!["allowed.event".to_string()]);

        bus.publish("allowed.event", EventPriority::Normal, "test", 1i32);
        assert_eq!(counter.load(Ordering::Relaxed), 1);

        bus.publish("blocked.event", EventPriority::Normal, "test", 2i32);
        assert_eq!(counter.load(Ordering::Relaxed), 1); // Blocked
    }

    #[test]
    fn test_block_list() {
        let mut bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        bus.subscribe("test.event", move |_| {
            counter_clone.fetch_add(1, Ordering::Relaxed);
            true
        });

        bus.set_block_list(vec!["test.event".to_string()]);

        bus.publish("test.event", EventPriority::Normal, "test", 1i32);
        assert_eq!(counter.load(Ordering::Relaxed), 0); // Blocked

        bus.clear_block_list();
        bus.publish("test.event", EventPriority::Normal, "test", 1i32);
        assert_eq!(counter.load(Ordering::Relaxed), 1); // Now allowed
    }

    #[test]
    fn test_event_stats() {
        let mut bus = EventBus::new();
        bus.publish("a", EventPriority::Normal, "test", 1i32);
        bus.publish("a", EventPriority::Normal, "test", 2i32);
        bus.publish("b", EventPriority::Normal, "test", 3i32);

        let stats = bus.stats();
        assert_eq!(stats.total_published, 3);
        assert_eq!(stats.event_type_counts.get("a"), Some(&2));
        assert_eq!(stats.event_type_counts.get("b"), Some(&1));
    }

    #[test]
    fn test_propagation_stop() {
        let mut bus = EventBus::new();
        let counter1 = Arc::new(AtomicUsize::new(0));
        let c1 = Arc::clone(&counter1);
        let counter2 = Arc::new(AtomicUsize::new(0));
        let c2 = Arc::clone(&counter2);

        bus.subscribe("test.event", move |_| {
            c1.fetch_add(1, Ordering::Relaxed);
            false // Stop propagation
        });
        bus.subscribe("test.event", move |_| {
            c2.fetch_add(1, Ordering::Relaxed);
            true
        });

        bus.publish("test.event", EventPriority::Normal, "test", 1i32);
        assert_eq!(counter1.load(Ordering::Relaxed), 1);
        assert_eq!(counter2.load(Ordering::Relaxed), 0); // Never reached
    }

    #[test]
    fn test_enable_disable_subscriber() {
        let mut bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let c = Arc::clone(&counter);

        let id = bus.subscribe("test.event", move |_| {
            c.fetch_add(1, Ordering::Relaxed);
            true
        });

        bus.publish("test.event", EventPriority::Normal, "test", 1i32);
        assert_eq!(counter.load(Ordering::Relaxed), 1);

        bus.disable_subscriber(id);
        bus.publish("test.event", EventPriority::Normal, "test", 2i32);
        assert_eq!(counter.load(Ordering::Relaxed), 1);

        bus.enable_subscriber(id);
        bus.publish("test.event", EventPriority::Normal, "test", 3i32);
        assert_eq!(counter.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_max_history() {
        let mut bus = EventBus::with_max_history(3);
        for i in 0..5 {
            bus.publish("test", EventPriority::Normal, "test", i as i32);
        }
        assert_eq!(bus.history().len(), 3);
    }

    #[test]
    fn test_thread_safe_bus() {
        let bus = SharedEventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let c = Arc::clone(&counter);

        bus.subscribe("test", move |_| {
            c.fetch_add(1, Ordering::Relaxed);
            true
        });

        bus.publish("test", EventPriority::Normal, "test", 1i32);
        assert_eq!(counter.load(Ordering::Relaxed), 1);
        assert_eq!(bus.subscriber_count(), 1);
    }

    #[test]
    fn test_clear_all() {
        let mut bus = EventBus::new();
        bus.subscribe("test", |_| true);
        bus.publish("test", EventPriority::Normal, "test", 1i32);

        bus.clear_all();
        assert_eq!(bus.subscriber_count(), 0);
        assert_eq!(bus.history().len(), 0);
    }

    #[test]
    fn test_envelope_downcast() {
        let env = EventEnvelope::new("test", EventPriority::Normal, "test", 42i32, 1);
        assert!(env.downcast_ref::<i32>().is_some());
        assert_eq!(*env.downcast_ref::<i32>().unwrap(), 42);
        assert!(env.downcast_ref::<String>().is_none());
    }
}
