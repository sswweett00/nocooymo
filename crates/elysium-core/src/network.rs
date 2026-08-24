use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::time::timeout;
use serde::{Serialize, Deserialize};
use rand::Rng;

/// Ağ bağlantı durumu
#[derive(Debug, Clone, PartialEq)]
pub enum NetworkStatus {
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
    Error(String),
}

/// Ağ protokolü türü
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NetworkProtocol {
    Tcp,
    Udp,
    ReliableUdp, // Güvenilir UDP implementasyonu
}

/// Ağ rolü
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NetworkRole {
    Client,
    Server,
    Host, // Hem client hem server
}

/// Bağlantı bilgisi
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub id: u64,
    pub address: SocketAddr,
    pub connected_at: std::time::Instant,
    pub last_ping: Option<std::time::Instant>,
    pub ping: Duration,
    pub protocol: NetworkProtocol,
    pub role: NetworkRole,
}

/// Ağ mesajı türleri
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetworkMessageType {
    Reliable,
    Unreliable,
    Ordered,
    Acknowledged,
    Connect,
    Disconnect,
    Ping,
    Pong,
    Data,
}

/// Ağ mesajı
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMessage {
    pub message_type: NetworkMessageType,
    pub channel: u8,
    pub payload: Vec<u8>,
    pub sender_id: Option<u64>,
    pub recipient_id: Option<u64>,
    pub timestamp: u64,
    pub reliable_id: Option<u64>,
}

impl NetworkMessage {
    pub fn new(message_type: NetworkMessageType, payload: Vec<u8>) -> Self {
        Self {
            message_type,
            channel: 0,
            payload,
            sender_id: None,
            recipient_id: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            reliable_id: None,
        }
    }
    
    pub fn with_channel(mut self, channel: u8) -> Self {
        self.channel = channel;
        self
    }
    
    pub fn with_sender(mut self, sender_id: u64) -> Self {
        self.sender_id = Some(sender_id);
        self
    }
    
    pub fn with_recipient(mut self, recipient_id: u64) -> Self {
        self.recipient_id = Some(recipient_id);
        self
    }
    
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).expect("Serialization failed")
    }
    
    pub fn deserialize(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(bincode::deserialize(data)?)
    }
}

/// Ağ sunucusu yapılandırması
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub port: u16,
    pub max_connections: usize,
    pub heartbeat_interval: Duration,
    pub timeout_duration: Duration,
    pub packet_size_limit: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 7777,
            max_connections: 128,
            heartbeat_interval: Duration::from_secs(5),
            timeout_duration: Duration::from_secs(30),
            packet_size_limit: 1024 * 1024, // 1MB
        }
    }
}

/// Ağ istemcisi yapılandırması
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub server_address: SocketAddr,
    pub reconnect_attempts: u32,
    pub reconnect_delay: Duration,
    pub heartbeat_interval: Duration,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            server_address: "127.0.0.1:7777".parse().unwrap(),
            reconnect_attempts: 5,
            reconnect_delay: Duration::from_secs(2),
            heartbeat_interval: Duration::from_secs(5),
        }
    }
}

// Ağ olayları
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    Connected { client_id: u64, address: SocketAddr },
    Disconnected { client_id: u64 },
    MessageReceived { client_id: u64, message: NetworkMessage },
    Error { error: String },
    ServerStarted { address: SocketAddr },
    ServerStopped,
}

// Ağ olay dinleyicisi
pub type NetworkEventHandler = Box<dyn Fn(&NetworkEvent) + Send + Sync>;
pub type NetworkMessageHandler = Box<dyn Fn(&NetworkMessage) -> Result<(), Box<dyn std::error::Error>> + Send + Sync>;

/// Oyuncu bilgileri
#[derive(Debug, Clone)]
pub struct NetworkPlayer {
    pub id: u64,
    pub username: String,
    pub address: SocketAddr,
    pub connected_at: std::time::SystemTime,
    pub last_ping: std::time::SystemTime,
    pub ping: u64, // Milisaniye cinsinden
    pub is_host: bool,
}

impl NetworkPlayer {
    pub fn new(id: u64, username: String, address: SocketAddr, is_host: bool) -> Self {
        Self {
            id,
            username,
            address,
            connected_at: std::time::SystemTime::now(),
            last_ping: std::time::SystemTime::now(),
            ping: 0,
            is_host: is_host,
        }
    }
}

// Ağ yapılandırması
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub max_connections: usize,
    pub heartbeat_interval: Duration,
    pub timeout_duration: Duration,
    pub packet_loss_simulation: f32, // 0.0 - 1.0 arası
    pub latency_simulation: Duration,
    pub encryption_enabled: bool,
    pub compression_enabled: bool,
    pub port: u16,
    pub packet_size_limit: usize,
}

impl NetworkConfig {
    pub fn default() -> Self {
        Self {
            max_connections: 32,
            heartbeat_interval: Duration::from_secs(30),
            timeout_duration: Duration::from_secs(60),
            packet_loss_simulation: 0.0,
            latency_simulation: Duration::from_millis(0),
            encryption_enabled: false,
            compression_enabled: false,
            port: 7777,
            packet_size_limit: 1024 * 1024, // 1MB
        }
    }
    
    pub fn server_config(port: u16) -> Self {
        let mut config = Self::default();
        config.port = port;
        config.max_connections = 128;
        config
    }
    
    pub fn client_config() -> Self {
        Self::default()
    }
}

/// Ağ sunucusu
pub struct NetworkServer {
    pub players: HashMap<u64, NetworkPlayer>,
    pub config: NetworkConfig,
    pub event_handlers: Vec<NetworkEventHandler>,
    pub is_running: bool,
    pub host_id: u64,
    pub next_client_id: u64,
    pub message_queue: Arc<Mutex<Vec<NetworkMessage>>>,
    pub reliable_messages: HashMap<u64, NetworkMessage>, // ID -> Mesaj
    pub pending_acks: HashMap<u64, u64>, // Mesaj ID -> Client ID
    pub tcp_listener: Option<TcpListener>,
    pub udp_socket: Option<UdpSocket>,
    pub status: NetworkStatus,
    pub connections: HashMap<u64, ConnectionInfo>,
    pub message_handlers: HashMap<NetworkMessageType, NetworkMessageHandler>,
}

impl NetworkServer {
    pub fn new(config: NetworkConfig) -> Self {
        Self {
            players: HashMap::new(),
            config,
            event_handlers: Vec::new(),
            is_running: false,
            host_id: 0,
            next_client_id: 1, // 0 host ID'si olarak kullanılır
            message_queue: Arc::new(Mutex::new(Vec::new())),
            reliable_messages: HashMap::new(),
            pending_acks: HashMap::new(),
            tcp_listener: None,
            udp_socket: None,
            status: NetworkStatus::Disconnected,
            connections: HashMap::new(),
            message_handlers: HashMap::new(),
        }
    }
    
    pub fn add_player(&mut self, address: SocketAddr, username: String) -> u64 {
        let client_id = self.next_client_id;
        self.next_client_id += 1;
        
        let player = NetworkPlayer::new(client_id, username, address, false);
        self.players.insert(client_id, player);
        
        // Oyuncu bağlandığında olayı yayınla
        self.broadcast_event(&NetworkEvent::Connected { client_id, address });
        
        client_id
    }
    
    pub fn remove_player(&mut self, client_id: u64) {
        if self.players.remove(&client_id).is_some() {
            // Oyuncu ayrıldığında olayı yayınla
            self.broadcast_event(&NetworkEvent::Disconnected { client_id });
        }
    }
    
    pub fn broadcast_message(&self, message: NetworkMessage) {
        // Mesajı tüm oyunculara gönder
        for player in self.players.values() {
            // Burada gerçek gönderim mantığı olurdu
            self.broadcast_event(&NetworkEvent::MessageReceived { 
                client_id: player.id, 
                message: message.clone() 
            });
        }
    }
    
    pub fn send_message_to(&self, client_id: u64, message: NetworkMessage) {
        if self.players.contains_key(&client_id) {
            self.broadcast_event(&NetworkEvent::MessageReceived { 
                client_id, 
                message 
            });
        }
    }
    
    pub fn send_reliable_message(&mut self, client_id: u64, mut message: NetworkMessage) -> u64 {
        let message_id = rand::random::<u64>();
        message.reliable_id = Some(message_id);
        
        // Bekleyen onaylar listesine ekle
        self.pending_acks.insert(message_id, client_id);
        
        // Mesajı güvenilir mesajlar listesine ekle
        self.reliable_messages.insert(message_id, message.clone());
        
        // Gönder
        self.send_message_to(client_id, message);
        
        message_id
    }
    
    pub fn acknowledge_message(&mut self, message_id: u64) {
        // Onaylanan mesajı listeden kaldır
        self.reliable_messages.remove(&message_id);
        self.pending_acks.remove(&message_id);
    }
    
    pub fn add_event_handler(&mut self, handler: NetworkEventHandler) {
        self.event_handlers.push(handler);
    }
    
    fn broadcast_event(&self, event: &NetworkEvent) {
        for handler in &self.event_handlers {
            handler(event);
        }
    }
    
    pub fn get_player(&self, client_id: u64) -> Option<&NetworkPlayer> {
        self.players.get(&client_id)
    }
    
    pub fn get_players(&self) -> Vec<&NetworkPlayer> {
        self.players.values().collect()
    }
    
    pub fn get_player_count(&self) -> usize {
        self.players.len()
    }
    
    pub fn update(&mut self) {
        // Zaman aşımına uğramış oyuncuları kaldır
        let now = std::time::SystemTime::now();
        let timeout_duration = self.config.timeout_duration;
        
        self.players.retain(|_, player| {
            if let Ok(elapsed) = now.duration_since(player.last_ping) {
                elapsed < timeout_duration
            } else {
                true // Zaman hatası durumunda oyuncuyu koru
            }
        });
    }

    /// Sunucuyu başlat
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let addr = format!("0.0.0.0:{}", self.config.port);
        
        self.tcp_listener = Some(TcpListener::bind(&addr).await?);
        self.udp_socket = Some(UdpSocket::bind(&addr).await?);
        
        self.status = NetworkStatus::Connected;
        self.is_running = true;
        println!("Sunucu {} adresinde çalışıyor", addr);
        
        // Sunucu başladığında olayı yayınla
        self.broadcast_event(&NetworkEvent::ServerStarted { 
            address: SocketAddr::new(IpAddr::from([127, 0, 0, 1]), self.config.port) 
        });
        
        Ok(())
    }

    /// Sunucuyu durdur
    pub fn stop(&mut self) {
        self.is_running = false;
        self.status = NetworkStatus::Disconnected;
        
        // Sunucu durduğunda olayı yayınla
        self.broadcast_event(&NetworkEvent::ServerStopped);
    }

    /// TCP üzerinden yeni bağlantıları dinle
    pub async fn listen_tcp(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(listener) = &self.tcp_listener {
            loop {
                let (socket, addr) = listener.accept().await?;
                println!("Yeni TCP bağlantısı: {}", addr);
                
                // Bağlantıyı işlemek için ayrı bir task başlat
                tokio::spawn(async move {
                    // Bağlantı işleme kodu burada olur
                    // Şimdilik sadece bağlantıyı kabul ediyoruz
                    let mut buf = [0; 1024];
                    match socket.peer_addr() {
                        Ok(peer_addr) => println!("Bağlantıdan gelen veri için peer: {}", peer_addr),
                        Err(e) => eprintln!("Peer adresi alınamadı: {}", e),
                    }
                });
            }
        } else {
            return Err("TCP listener not initialized".into());
        }
    }

    /// UDP üzerinden mesajları dinle
    pub async fn listen_udp(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(socket) = &self.udp_socket {
            let mut buf = vec![0u8; self.config.packet_size_limit];
            
            loop {
                let (len, addr) = socket.recv_from(&mut buf).await?;
                println!("UDP mesajı alındı {} bayt, {} adresinden", len, addr);
                
                // Gelen mesajı işle
                if let Ok(message) = NetworkMessage::deserialize(&buf[..len]) {
                    self.handle_message(&message).await?;
                }
            }
        } else {
            return Err("UDP socket not initialized".into());
        }
    }

    /// Mesajı işle
    async fn handle_message(&self, message: &NetworkMessage) -> Result<(), Box<dyn std::error::Error>> {
        // Kayıtlı mesaj işleyicilerini çalıştır
        if let Some(handler) = self.message_handlers.get(&message.message_type) {
            return handler(message);
        }

        match message.message_type {
            NetworkMessageType::Ping => {
                // Pong yanıtını gönder
                let pong_message = NetworkMessage::new(
                    NetworkMessageType::Pong,
                    vec![], // Ekstra veri yok
                );
                
                // Yanıt gönderme işlemi burada olur
                println!("Ping mesajı alındı, Pong yanıtı gönderiliyor");
                let _ = pong_message;
            }
            NetworkMessageType::Connect => {
                println!("Yeni bağlantı isteği alındı");
            }
            NetworkMessageType::Disconnect => {
                println!("Bağlantı kesme isteği alındı");
            }
            NetworkMessageType::Data => {
                println!("Veri mesajı alındı: {} byte", message.payload.len());
            }
            _ => {
                println!("Bilinmeyen mesaj türü: {:?}", message.message_type);
            }
        }
        
        Ok(())
    }

    /// Tüm bağlantılara mesaj gönder
    pub fn broadcast_to_all(&self, message: &NetworkMessage) -> Result<(), Box<dyn std::error::Error>> {
        // Tüm TCP bağlantılarına mesaj gönder
        // UDP ile yayın yap
        // Gerçek implementasyon burada olurdu
        
        println!("Broadcast mesaj gönderiliyor: {:?}", message.message_type);
        Ok(())
    }

    /// Belirli bir bağlantıya mesaj gönder
    pub fn send_to_client(&self, client_id: u64, message: &NetworkMessage) -> Result<(), Box<dyn std::error::Error>> {
        // Belirli TCP bağlantısına veya UDP üzerinden mesaj gönder
        // Gerçek implementasyon burada olurdu
        
        println!("Client {} için mesaj gönderiliyor", client_id);
        Ok(())
    }

    /// Bağlantıyı ekle
    pub fn add_connection(&mut self, address: SocketAddr) -> u64 {
        let id = self.next_client_id;
        self.next_client_id += 1;
        
        let connection_info = ConnectionInfo {
            id,
            address,
            connected_at: std::time::Instant::now(),
            last_ping: None,
            ping: Duration::from_secs(0),
            protocol: NetworkProtocol::Tcp,
            role: NetworkRole::Client,
        };
        
        self.connections.insert(id, connection_info);
        id
    }

    /// Bağlantıyı kaldır
    pub fn remove_connection(&mut self, client_id: u64) -> bool {
        self.connections.remove(&client_id).is_some()
    }

    /// Bağlantı var mı kontrol et
    pub fn is_connected(&self, client_id: u64) -> bool {
        self.connections.contains_key(&client_id)
    }

    /// Mesaj işleyici ekle
    pub fn add_message_handler<F>(&mut self, message_type: NetworkMessageType, handler: F)
    where
        F: Fn(&NetworkMessage) -> Result<(), Box<dyn std::error::Error>> + Send + Sync + 'static,
    {
        self.message_handlers.insert(message_type, Box::new(handler));
    }
}

/// Ağ istemcisi
pub struct NetworkClient {
    pub server_address: SocketAddr,
    pub client_id: Option<u64>,
    pub username: String,
    pub config: NetworkConfig,
    pub connected: bool,
    pub event_handlers: Vec<NetworkEventHandler>,
    pub last_ping: std::time::SystemTime,
    pub server_ping: u64, // ms cinsinden
    pub message_queue: Arc<Mutex<Vec<NetworkMessage>>>,
    pub tcp_stream: Option<TcpStream>,
    pub status: NetworkStatus,
}

impl NetworkClient {
    pub fn new(server_address: SocketAddr, username: String, config: NetworkConfig) -> Self {
        Self {
            server_address,
            client_id: None,
            username,
            config,
            connected: false,
            event_handlers: Vec::new(),
            last_ping: std::time::SystemTime::now(),
            server_ping: 0,
            message_queue: Arc::new(Mutex::new(Vec::new())),
            tcp_stream: None,
            status: NetworkStatus::Disconnected,
        }
    }

    pub async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // TCP bağlantısı kur
        let stream = TcpStream::connect(self.server_address).await?;
        self.tcp_stream = Some(stream);
        
        // Bağlantı kurulduğunda kimlik doğrulama mesajı gönder
        let auth_msg = NetworkMessage::new(
            NetworkMessageType::Reliable,
            format!("AUTH:{}", self.username).into_bytes()
        );
        
        // Gerçek gönderim burada olurdu
        // self.send_message(auth_msg).await?;
        let _ = auth_msg;
        
        self.connected = true;
        self.status = NetworkStatus::Connected;
        
        Ok(())
    }

    /// Bağlantıyı kes
    pub async fn disconnect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(_stream) = self.tcp_stream.take() {
            let _disconnect_msg = NetworkMessage::new(
                NetworkMessageType::Disconnect,
                vec![], // Ekstra veri yok
            );
        }
        
        self.connected = false;
        self.status = NetworkStatus::Disconnected;
        Ok(())
    }

    pub async fn send_message(&mut self, message: NetworkMessage) -> Result<(), Box<dyn std::error::Error>> {
        if !self.connected {
            return Err("Not connected to server".into());
        }
        
        // Gerçek gönderim mantığı burada olurdu
        // Burada sadece örnek
        
        Ok(())
    }
    
    pub async fn send_reliable_message(&mut self, message: NetworkMessage) -> Result<u64, Box<dyn std::error::Error>> {
        if !self.connected {
            return Err("Not connected to server".into());
        }
        
        let message_id = rand::random::<u64>();
        let mut msg = message;
        msg.reliable_id = Some(message_id);
        
        // Gerçek gönderim burada olurdu
        // self.send_message(msg).await?;
        
        Ok(message_id)
    }

    pub async fn ping_server(&mut self) -> Result<u64, Box<dyn std::error::Error>> {
        let start_time = std::time::Instant::now();
        
        // Ping mesajı gönder
        let ping_msg = NetworkMessage::new(NetworkMessageType::Unreliable, b"PING".to_vec());
        self.send_message(ping_msg).await?;
        
        let elapsed = start_time.elapsed().as_millis() as u64;
        self.server_ping = elapsed;
        
        Ok(elapsed)
    }
    
    pub fn get_server_ping(&self) -> u64 {
        self.server_ping
    }
}

// Ağ yetkilendirme sistemi
#[derive(Debug, Clone)]
pub struct NetworkAuth {
    pub token: String,
    pub permissions: Vec<String>,
    pub expires_at: Option<std::time::SystemTime>,
}

impl NetworkAuth {
    pub fn new(token: String, permissions: Vec<String>) -> Self {
        Self {
            token,
            permissions,
            expires_at: None,
        }
    }
    
    pub fn with_expiration(mut self, duration: Duration) -> Self {
        self.expires_at = Some(std::time::SystemTime::now() + duration);
        self
    }
    
    pub fn is_valid(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            std::time::SystemTime::now() < expires_at
        } else {
            true
        }
    }
    
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
    }
}

// Ağ senkronizasyon bileşeni
pub struct NetworkSync {
    pub network_id: u64,
    pub sync_frequency: Duration, // Ne sıklıkla senkronize edileceği
    pub is_owner: bool,          // Bu istemcinin sahibi olup olmadığı
    pub authority_mode: AuthorityMode,
    pub last_sync: std::time::SystemTime,
    pub interpolate: bool,       // Konum interpolasyonu
    pub extrapolate: bool,       // Konum ekstrapolasyonu
}

#[derive(Debug, Clone)]
pub enum AuthorityMode {
    ServerAuthoritative,  // Sunucu yetkili
    ClientAuthoritative,  // İstemci yetkili
    Predictive,           // Tahmini yetki
}

impl NetworkSync {
    pub fn new(network_id: u64, authority_mode: AuthorityMode) -> Self {
        Self {
            network_id,
            sync_frequency: Duration::from_millis(33), // ~30 FPS
            is_owner: false,
            authority_mode,
            last_sync: std::time::SystemTime::now(),
            interpolate: true,
            extrapolate: false,
        }
    }
    
    pub fn should_sync(&self) -> bool {
        if let Ok(elapsed) = std::time::SystemTime::now().duration_since(self.last_sync) {
            elapsed >= self.sync_frequency
        } else {
            true
        }
    }
    
    pub fn mark_synced(&mut self) {
        self.last_sync = std::time::SystemTime::now();
    }
}

// Ağ obje havuzu
pub struct NetworkObjectPool {
    pub objects: HashMap<u64, NetworkSync>,
    pub next_id: u64,
}

impl NetworkObjectPool {
    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
            next_id: 1,
        }
    }
    
    pub fn register_object(&mut self, mut sync_component: NetworkSync) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        
        sync_component.network_id = id;
        self.objects.insert(id, sync_component);
        
        id
    }
    
    pub fn unregister_object(&mut self, id: u64) {
        self.objects.remove(&id);
    }
    
    pub fn get_object(&self, id: u64) -> Option<&NetworkSync> {
        self.objects.get(&id)
    }
    
    pub fn get_object_mut(&mut self, id: u64) -> Option<&mut NetworkSync> {
        self.objects.get_mut(&id)
    }
    
    pub fn update(&mut self) {
        // Burada nesnelerin senkronizasyon kontrolleri yapılabilir
    }
}

// CRDT (Conflict-free Replicated Data Type) sistemi
pub struct CrdtSystem {
    pub objects: HashMap<u64, CrdtObject>,
    pub local_clock: u64,
    pub peer_clocks: HashMap<u64, u64>, // Peer ID -> Clock
}

#[derive(Debug, Clone)]
pub struct CrdtObject {
    pub id: u64,
    pub vector_clock: HashMap<u64, u64>, // Peer ID -> Count
    pub data: Vec<u8>,
    pub last_updated: std::time::SystemTime,
}

impl CrdtSystem {
    pub fn new() -> Self {
        Self {
            objects: HashMap::new(),
            local_clock: 0,
            peer_clocks: HashMap::new(),
        }
    }
    
    pub fn update_local_clock(&mut self) {
        self.local_clock += 1;
    }
    
    pub fn create_object(&mut self, data: Vec<u8>) -> u64 {
        self.update_local_clock();
        
        let id = rand::random::<u64>();
        let mut vector_clock = HashMap::new();
        vector_clock.insert(0, self.local_clock); // 0 bizim peer ID'miz
        
        let object = CrdtObject {
            id,
            vector_clock,
            data,
            last_updated: std::time::SystemTime::now(),
        };
        
        self.objects.insert(id, object);
        
        id
    }
    
    pub fn update_object(&mut self, id: u64, data: Vec<u8>) -> bool {
        self.update_local_clock();
        let local_clock = self.local_clock;
        
        if let Some(obj) = self.objects.get_mut(&id) {
            obj.vector_clock.insert(0, local_clock);
            obj.data = data;
            obj.last_updated = std::time::SystemTime::now();
            true
        } else {
            false
        }
    }
    
    pub fn merge_object(&mut self, remote_obj: CrdtObject) -> bool {
        // Clone necessary data before mutable borrow
        let remote_vector_clock = remote_obj.vector_clock.clone();
        let remote_data = remote_obj.data.clone();
        let remote_id = remote_obj.id;

        if let Some(local_obj) = self.objects.get(&remote_id) {
            if self.should_update_object(local_obj, &remote_obj) {
                let merged_clocks =
                    self.merge_vector_clocks(&local_obj.vector_clock, &remote_vector_clock);
                if let Some(local_obj) = self.objects.get_mut(&remote_id) {
                    // Verileri birleştir
                    local_obj.data = remote_data;
                    local_obj.vector_clock = merged_clocks;
                    local_obj.last_updated = std::time::SystemTime::now();
                }
            }
        } else {
            // Yeni nesne olarak ekle
            self.objects.insert(remote_id, remote_obj);
        }
        true
    }
    
    fn should_update_object(&self, local: &CrdtObject, remote: &CrdtObject) -> bool {
        // Vektör saatlerini karşılaştır
        let local_clock_sum: u64 = local.vector_clock.values().sum();
        let remote_clock_sum: u64 = remote.vector_clock.values().sum();
        
        // Uzak nesne daha güncelse veya eşitse ve ID'si daha büyükse güncelle
        remote_clock_sum > local_clock_sum || 
        (remote_clock_sum == local_clock_sum && remote.id > local.id)
    }
    
    fn merge_vector_clocks(&self, local: &HashMap<u64, u64>, remote: &HashMap<u64, u64>) -> HashMap<u64, u64> {
        let mut merged = local.clone();
        
        for (peer_id, clock) in remote {
            let current = merged.entry(*peer_id).or_insert(0);
            *current = (*current).max(*clock);
        }
        
        merged
    }
    
    pub fn get_object(&self, id: u64) -> Option<&CrdtObject> {
        self.objects.get(&id)
    }
}

// Ağ sistem bileşeni
pub struct NetworkSystem {
    pub server: Option<NetworkServer>,
    pub client: Option<NetworkClient>,
    pub object_pool: NetworkObjectPool,
    pub crdt_system: CrdtSystem,
    pub is_server: bool,
    pub is_client: bool,
}

impl NetworkSystem {
    pub fn new() -> Self {
        Self {
            server: None,
            client: None,
            object_pool: NetworkObjectPool::new(),
            crdt_system: CrdtSystem::new(),
            is_server: false,
            is_client: false,
        }
    }
    
    pub fn initialize_server(&mut self, config: NetworkConfig) {
        self.server = Some(NetworkServer::new(config));
        self.is_server = true;
    }
    
    pub fn initialize_client(&mut self, server_addr: SocketAddr, username: String, config: NetworkConfig) {
        self.client = Some(NetworkClient::new(server_addr, username, config));
        self.is_client = true;
    }
    
    pub fn is_host(&self) -> bool {
        self.is_server && self.is_client
    }
    
    pub fn update(&mut self, delta_time: f32) {
        // Sunucu varsa güncelle
        if let Some(ref mut server) = self.server {
            server.update();
        }
        
        // Nesne havuzunu güncelle
        self.object_pool.update();
        
        // CRDT sistemini güncelle
        // Burada CRDT güncellemeleri yapılabilir
    }
    
    pub fn register_network_object(&mut self, sync_component: NetworkSync) -> u64 {
        self.object_pool.register_object(sync_component)
    }
    
    pub fn create_crdt_object(&mut self, data: Vec<u8>) -> u64 {
        self.crdt_system.create_object(data)
    }
    
    pub fn update_crdt_object(&mut self, id: u64, data: Vec<u8>) -> bool {
        self.crdt_system.update_object(id, data)
    }
}

// Yardımcı fonksiyonlar
pub mod network_utils {
    use super::*;
    
    pub fn serialize_entity_data(entity_data: &dyn std::any::Any) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Gerçek serializasyon mantığı burada olur
        // Bu sadece bir placeholder
        Ok(vec![])
    }
    
    pub fn deserialize_entity_data(data: &[u8]) -> Result<Box<dyn std::any::Any + Send + Sync>, Box<dyn std::error::Error>> {
        // Gerçek deserializasyon mantığı burada olur
        // Bu sadece bir placeholder
        Ok(Box::new(()))
    }
    
    pub fn compress_data(data: &[u8]) -> Vec<u8> {
        // Veri sıkıştırma mantığı
        // Bu sadece bir placeholder
        data.to_vec()
    }
    
    pub fn decompress_data(data: &[u8]) -> Vec<u8> {
        // Veri açma mantığı
        // Bu sadece bir placeholder
        data.to_vec()
    }
}

/// Ağ güvenliği için şifreleme yardımcıları
pub mod security {
    use super::*;

    /// Basit mesaj şifreleme
    pub fn encrypt_message(data: &[u8], key: &[u8]) -> Vec<u8> {
        let mut encrypted = data.to_vec();
        for (i, byte) in encrypted.iter_mut().enumerate() {
            *byte ^= key[i % key.len()];
        }
        encrypted
    }

    /// Basit mesaj çözme
    pub fn decrypt_message(data: &[u8], key: &[u8]) -> Vec<u8> {
        // XOR şifrelemesi simetriktir, aynı işlem hem şifreler hem çözer
        encrypt_message(data, key)
    }

    /// Mesaj imzalama (basit implementasyon)
    pub fn sign_message(data: &[u8], key: &[u8]) -> Vec<u8> {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.update(key);
        hasher.finalize().to_vec()
    }

    /// Mesaj imzası doğrulama
    pub fn verify_signature(data: &[u8], signature: &[u8], key: &[u8]) -> bool {
        let expected_signature = sign_message(data, key);
        expected_signature == signature
    }
}