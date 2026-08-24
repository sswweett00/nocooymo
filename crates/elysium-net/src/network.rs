use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};
use elysium_core::{Entity, Transform};
use std::collections::HashMap;
use bincode;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum NetworkMessage {
    EntitySpawn { entity: Entity, transform: Transform },
    EntityUpdate { entity: Entity, transform: Transform },
    EntityDespawn { entity: Entity },
    PlayerInput { player_id: u64, input: PlayerInput },
    SyncState { state: WorldState },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayerInput {
    pub movement: [f32; 3],
    pub rotation: [f32; 3],
    pub actions: Vec<PlayerAction>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PlayerAction {
    Jump,
    Attack,
    Interact,
    UseItem(u32),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WorldState {
    pub entities: Vec<(Entity, Transform)>,
    pub timestamp: u64,
}

pub struct NetworkConfig {
    pub tcp_port: u16,
    pub udp_port: u16,
    pub max_players: usize,
    pub tick_rate: u32,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            tcp_port: 8080,
            udp_port: 8081,
            max_players: 16,
            tick_rate: 60,
        }
    }
}

pub struct NetworkServer {
    pub config: NetworkConfig,
    pub tcp_listener: Option<TcpListener>,
    pub udp_socket: Option<UdpSocket>,
    pub clients: HashMap<u64, ClientConnection>,
    pub next_client_id: u64,
    pub last_broadcast: std::time::Instant,
}

pub struct ClientConnection {
    pub tcp_stream: Arc<Mutex<TcpStream>>,
    pub udp_addr: std::net::SocketAddr,
    pub player_id: u64,
    pub last_heartbeat: std::time::Instant,
    pub ping: u128,
}

impl NetworkServer {
    pub async fn new(config: NetworkConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let tcp_listener = TcpListener::bind(format!("0.0.0.0:{}", config.tcp_port)).await?;
        let udp_socket = UdpSocket::bind(format!("0.0.0.0:{}", config.udp_port)).await?;
        
        Ok(Self {
            config,
            tcp_listener: Some(tcp_listener),
            udp_socket: Some(udp_socket),
            clients: HashMap::new(),
            next_client_id: 1,
            last_broadcast: std::time::Instant::now(),
        })
    }

    pub fn new_sync(config: NetworkConfig) -> Self {
        Self {
            config,
            tcp_listener: None,
            udp_socket: None,
            clients: HashMap::new(),
            next_client_id: 1,
            last_broadcast: std::time::Instant::now(),
        }
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.tcp_listener.is_none() || self.udp_socket.is_none() {
            return Err("TCP listener or UDP socket not initialized".into());
        }
        let mut buf = vec![0u8; 1024];
        loop {
            // Her döngüde listener'ları geçici olarak al, await sırasında self ödünç alınmasın
            let accept_fut = async {
                let listener = self.tcp_listener.as_ref().unwrap();
                listener.accept().await
            };
            let recv_fut = async {
                let sock = self.udp_socket.as_ref().unwrap();
                sock.recv_from(&mut buf).await
            };
            tokio::select! {
                res = accept_fut => {
                    let (tcp_stream, addr) = res?;
                    self.handle_new_connection(tcp_stream, addr).await?;
                }
                res = recv_fut => {
                    let (n, src_addr) = res?;
                    // borrow'u serbest bırakmak için veriyi kopyala
                    let data = buf[..n].to_vec();
                    self.handle_udp_packet(&data, src_addr).await?;
                }
            }
        }
    }

    async fn handle_new_connection(&mut self, tcp_stream: TcpStream, addr: std::net::SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        let client_id = self.next_client_id;
        self.next_client_id += 1;

        let tcp_stream = Arc::new(Mutex::new(tcp_stream));
        let connection = ClientConnection {
            tcp_stream: tcp_stream.clone(),
            udp_addr: addr,
            player_id: client_id,
            last_heartbeat: std::time::Instant::now(),
            ping: 0,
        };

        self.clients.insert(client_id, connection);
        
        println!("New client connected: {} (ID: {})", addr, client_id);
        
        // Send welcome message with client ID
        let welcome_msg = NetworkMessage::PlayerInput {
            player_id: client_id,
            input: PlayerInput {
                movement: [0.0, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0],
                actions: vec![],
            }
        };
        
        self.send_to_client(client_id, &welcome_msg).await?;
        
        Ok(())
    }

    async fn handle_udp_packet(&mut self, data: &[u8], _src_addr: std::net::SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(msg) = bincode::deserialize::<NetworkMessage>(data) {
            match msg {
                NetworkMessage::PlayerInput { player_id, input } => {
                    // Forward player input to game logic
                    self.handle_player_input(player_id, input).await?;
                }
                NetworkMessage::EntitySpawn { .. } |
                NetworkMessage::EntityUpdate { .. } |
                NetworkMessage::EntityDespawn { .. } |
                NetworkMessage::SyncState { .. } => {
                    // These are server-generated messages, ignore incoming
                }
            }
        }
        
        Ok(())
    }

    async fn handle_player_input(&mut self, player_id: u64, input: PlayerInput) -> Result<(), Box<dyn std::error::Error>> {
        // Update client's last heartbeat
        if let Some(client) = self.clients.get_mut(&player_id) {
            client.last_heartbeat = std::time::Instant::now();
        }
        
        // Here you would forward the input to the game state
        // For now, just log it
        println!("Received input from player {}: {:?}", player_id, input.actions);
        
        Ok(())
    }

    pub async fn broadcast_message(&mut self, msg: &NetworkMessage) -> Result<(), Box<dyn std::error::Error>> {
        let serialized = bincode::serialize(msg)?;
        
        let mut disconnected_clients = Vec::new();
        
        for (client_id, connection) in &self.clients {
            match self.send_to_tcp_stream(&connection.tcp_stream, &serialized).await {
                Ok(_) => {},
                Err(_) => {
                    // Mark client for disconnection
                    disconnected_clients.push(*client_id);
                }
            }
        }
        
        // Remove disconnected clients
        for client_id in disconnected_clients {
            self.clients.remove(&client_id);
            println!("Client {} disconnected", client_id);
        }
        
        Ok(())
    }

    async fn send_to_client(&self, client_id: u64, msg: &NetworkMessage) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(connection) = self.clients.get(&client_id) {
            let serialized = bincode::serialize(msg)?;
            self.send_to_tcp_stream(&connection.tcp_stream, &serialized).await?;
        }
        
        Ok(())
    }

    async fn send_to_tcp_stream(
        &self,
        stream: &Arc<Mutex<TcpStream>>, 
        data: &[u8]
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut guard = stream.lock().await;
        guard.write_all(&(data.len() as u32).to_le_bytes()).await?;
        guard.write_all(data).await?;
        Ok(())
    }

    pub async fn sync_world_state(&mut self, world_entities: &[(Entity, Transform)]) -> Result<(), Box<dyn std::error::Error>> {
        let state = WorldState {
            entities: world_entities.to_vec(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis() as u64,
        };
        
        let msg = NetworkMessage::SyncState { state };
        self.broadcast_message(&msg).await?;
        
        Ok(())
    }
}

pub struct NetworkClient {
    pub server_addr: String,
    pub tcp_stream: Option<Arc<Mutex<TcpStream>>>,
    pub udp_socket: Option<UdpSocket>,
    pub player_id: Option<u64>,
    pub connected: bool,
}

impl NetworkClient {
    pub fn new(server_addr: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            server_addr: server_addr.to_string(),
            tcp_stream: None,
            udp_socket: None,
            player_id: None,
            connected: false,
        })
    }

    pub async fn new_async(server_addr: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Self::new(server_addr)
    }

    pub async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Connect TCP
        let tcp_stream = TcpStream::connect(&self.server_addr).await?;
        tcp_stream.set_nodelay(true)?;
        
        self.tcp_stream = Some(Arc::new(Mutex::new(tcp_stream)));
        
        // Bind UDP socket
        let udp_socket = UdpSocket::bind("0.0.0.0:0").await?;
        self.udp_socket = Some(udp_socket);
        
        self.connected = true;
        
        // Start receiving messages
        self.start_receiving()?;
        
        Ok(())
    }

    fn start_receiving(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(tcp_stream) = &self.tcp_stream {
            let stream_clone = Arc::clone(tcp_stream);
            tokio::spawn(async move {
                let mut buffer = [0; 1024];
                loop {
                    let mut stream_guard = stream_clone.lock().await;
                    match stream_guard.read(&mut buffer).await {
                        Ok(0) => {
                            // Connection closed
                            break;
                        }
                        Ok(n) => {
                            // Process received data
                            println!("Received {} bytes from server", n);
                        }
                        Err(e) => {
                            eprintln!("Error receiving from server: {}", e);
                            break;
                        }
                    }
                }
            });
        }
        
        Ok(())
    }

    pub async fn send_input(&mut self, input: PlayerInput) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(player_id) = self.player_id {
            let msg = NetworkMessage::PlayerInput {
                player_id,
                input,
            };
            
            if let Some(ref stream) = self.tcp_stream {
                let serialized = bincode::serialize(&msg)?;
                let mut guard = stream.lock().await;
                guard.write_all(&(serialized.len() as u32).to_le_bytes()).await?;
                guard.write_all(&serialized).await?;
            }
        }
        
        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.connected = false;
        self.tcp_stream = None;
        self.udp_socket = None;
        Ok(())
    }
}