//! Elysium ağ altyapısı — Weave CRDT + TCP/UDP transport (Mimari §8).

pub mod crdt;
pub mod network;

pub use crdt::*;
pub use network::*;

use elysium_core::{World, System};

// ─────────────────────────────────────────────── Weave Network System

/// Hem sunucu hem istemci tarafını yöneten üst-düzey ağ sistemi.
pub struct WeaveNetworkSystem {
    pub server: Option<NetworkServer>,
    pub client: Option<NetworkClient>,
    pub crdt_manager: CrdtManager,
    /// Giden paket kuyruğu (bincode serialize edilmiş).
    outgoing: std::collections::VecDeque<Vec<u8>>,
}

impl Default for WeaveNetworkSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl WeaveNetworkSystem {
    pub fn new() -> Self {
        Self {
            server: None,
            client: None,
            crdt_manager: CrdtManager::new(),
            outgoing: std::collections::VecDeque::new(),
        }
    }

    pub fn init_server(&mut self, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let mut cfg = NetworkConfig::default();
        cfg.tcp_port = port;
        cfg.udp_port = port.wrapping_add(1);
        self.server = Some(NetworkServer::new_sync(cfg));
        Ok(())
    }

    pub fn init_client(&mut self, address: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.client = Some(NetworkClient::new(address)?);
        Ok(())
    }

    /// Gönderilecek ham paketi kuyruğa al.
    pub fn enqueue_packet(&mut self, data: Vec<u8>) {
        self.outgoing.push_back(data);
    }

    /// Kuyruktan bir sonraki paketi al.
    pub fn dequeue_packet(&mut self) -> Option<Vec<u8>> {
        self.outgoing.pop_front()
    }
}

impl System for WeaveNetworkSystem {
    fn update(&mut self, _world: &mut World, _dt: f32) {
        // Her frame'de giden paketleri işle (gerçek async bağlantı yoksa no-op).
        while let Some(_packet) = self.dequeue_packet() {
            // İleride: tokio runtime üzerinden UDP/TCP gönderimi yapılacak.
        }
    }

    fn name(&self) -> &str {
        "WeaveNetworkSystem"
    }
}

// ─────────────────────────────────────────────── Schedule builder

pub fn build_network_schedule(schedule: &mut elysium_core::scheduler::Schedule) {
    let sys = elysium_core::scheduler::FunctionSystem::new("network_sync_fn", |_world: &mut World, _dt: f32| {});
    schedule.add_system_to_stage(elysium_core::scheduler::Stage::Update, Box::new(sys));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weave_creation() {
        let sys = WeaveNetworkSystem::new();
        assert!(sys.crdt_manager.entities.is_empty());
        assert!(sys.server.is_none());
        assert!(sys.client.is_none());
    }

    #[test]
    fn test_packet_queue() {
        let mut sys = WeaveNetworkSystem::new();
        sys.enqueue_packet(vec![1, 2, 3]);
        sys.enqueue_packet(vec![4, 5, 6]);
        assert_eq!(sys.dequeue_packet(), Some(vec![1, 2, 3]));
        assert_eq!(sys.dequeue_packet(), Some(vec![4, 5, 6]));
        assert_eq!(sys.dequeue_packet(), None);
    }
}
