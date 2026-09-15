/// websocket_transport.rs — WebSocket Transport Layer
/// Platform-agnostic WebSocket transport for multiplayer networking.
///
/// Features:
/// - Native: tokio-tungstenite (async, TLS support)
/// - WASM: web-sys WebSocket (browser API)
/// - Length-prefixed binary framing
/// - Connection management (connect, disconnect, reconnect)
/// - Heartbeat / keepalive
/// - Message queuing (reliable + unreliable channels)
/// - Bandwidth statistics
/// - Callback-based message handling

use serde::{Serialize, Deserialize};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

// ═══════════════════════════════════════════════════════════ Transport Types

/// Connection state
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed(String),
}

/// WebSocket message type
#[derive(Clone, Debug)]
pub enum TransportMessage {
    /// Binary data (serialized network messages)
    Binary(Vec<u8>),
    /// Text data (JSON commands, chat)
    Text(String),
    /// Ping frame
    Ping(Vec<u8>),
    /// Pong frame
    Pong(Vec<u8>),
}

/// Transport configuration
#[derive(Clone, Debug)]
pub struct TransportConfig {
    /// WebSocket server URL
    pub url: String,
    /// Connection timeout in milliseconds
    pub connect_timeout_ms: u32,
    /// Heartbeat interval in milliseconds
    pub heartbeat_interval_ms: u32,
    /// Heartbeat timeout in milliseconds
    pub heartbeat_timeout_ms: u32,
    /// Maximum reconnection attempts
    pub max_reconnect_attempts: u32,
    /// Reconnection delay in milliseconds (doubles on each attempt)
    pub reconnect_base_delay_ms: u32,
    /// Maximum reconnection delay in milliseconds
    pub reconnect_max_delay_ms: u32,
    /// Send buffer size
    pub send_buffer_size: usize,
    /// Receive buffer size
    pub recv_buffer_size: usize,
    /// Enable Nagle's algorithm (disable for low-latency)
    pub enable_nagle: bool,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            url: "ws://127.0.0.1:8080".into(),
            connect_timeout_ms: 5000,
            heartbeat_interval_ms: 5000,
            heartbeat_timeout_ms: 10000,
            max_reconnect_attempts: 10,
            reconnect_base_delay_ms: 500,
            reconnect_max_delay_ms: 30000,
            send_buffer_size: 65536,
            recv_buffer_size: 65536,
            enable_nagle: false,
        }
    }
}

/// Bandwidth statistics
#[derive(Clone, Debug, Default)]
pub struct BandwidthStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub frames_sent: u64,
    pub frames_received: u64,
    pub reconnect_count: u32,
    pub last_send_time: u64,
    pub last_recv_time: u64,
}

/// Connection event (for callbacks)
#[derive(Clone, Debug)]
pub enum ConnectionEvent {
    Connected,
    Disconnected { reason: String },
    Reconnecting { attempt: u32, delay_ms: u32 },
    Message(TransportMessage),
    Error(String),
    HeartbeatSent { timestamp: u64 },
    HeartbeatReceived { rtt_ms: f32 },
}

// ═══════════════════════════════════════════════════════════ Message Framing

/// Length-prefixed binary message framing
/// Format: [length: u32 LE] [payload: bytes]
pub struct MessageFramer {
    send_buffer: Vec<u8>,
    recv_buffer: Vec<u8>,
    current_message_len: Option<u32>,
    max_message_size: u32,
}

impl MessageFramer {
    pub fn new(max_message_size: u32) -> Self {
        Self {
            send_buffer: Vec::new(),
            recv_buffer: Vec::new(),
            current_message_len: None,
            max_message_size,
        }
    }

    /// Frame a message with length prefix
    pub fn frame(&mut self, data: &[u8]) -> Vec<u8> {
        let len = data.len() as u32;
        let mut framed = Vec::with_capacity(4 + data.len());
        framed.extend_from_slice(&len.to_le_bytes());
        framed.extend_from_slice(data);
        framed
    }

    /// Feed incoming bytes and extract complete messages
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<Vec<u8>> {
        self.recv_buffer.extend_from_slice(bytes);
        let mut messages = Vec::new();

        loop {
            // Need at least 4 bytes for length prefix
            if self.recv_buffer.len() < 4 {
                break;
            }

            let len = match self.current_message_len {
                Some(l) => l,
                None => {
                    let bytes = [self.recv_buffer[0], self.recv_buffer[1], self.recv_buffer[2], self.recv_buffer[3]];
                    let l = u32::from_le_bytes(bytes);
                    if l > self.max_message_size {
                        // Protocol error — skip
                        self.recv_buffer.clear();
                        self.current_message_len = None;
                        break;
                    }
                    self.current_message_len = Some(l);
                    l
                }
            };

            let total = 4 + len as usize;
            if self.recv_buffer.len() >= total {
                let msg = self.recv_buffer[4..total].to_vec();
                messages.push(msg);
                self.recv_buffer.drain(..total);
                self.current_message_len = None;
            } else {
                break;
            }
        }

        messages
    }

    /// Reset framing state
    pub fn reset(&mut self) {
        self.send_buffer.clear();
        self.recv_buffer.clear();
        self.current_message_len = None;
    }

    pub fn pending_bytes(&self) -> usize {
        self.recv_buffer.len()
    }
}

// ═══════════════════════════════════════════════════════════ Native WebSocket (tokio-tungstenite)

#[cfg(not(target_arch = "wasm32"))]
pub mod native_transport {
    use super::*;
    use tokio::sync::mpsc;
    use futures::{SinkExt, StreamExt};
    use tokio_tungstenite::{connect_async, tungstenite::Message};
    use std::time::Duration;

    /// Native WebSocket connection using tokio-tungstenite
    pub struct NativeWebSocket {
        config: TransportConfig,
        state: ConnectionState,
        framer: MessageFramer,
        stats: BandwidthStats,
        // Channel for outgoing messages (to the write task)
        tx: Option<mpsc::UnboundedSender<Vec<u8>>>,
        // Channel for incoming messages (from the read task)
        rx: Option<mpsc::UnboundedReceiver<TransportMessage>>,
        // Channel for connection events
        event_tx: Option<mpsc::UnboundedSender<ConnectionEvent>>,
        event_rx: Option<mpsc::UnboundedReceiver<ConnectionEvent>>,
        // Pending outgoing messages (queued before connection)
        pending_outgoing: VecDeque<Vec<u8>>,
        // Heartbeat state
        last_heartbeat_sent: u64,
        last_heartbeat_received: u64,
        // Runtime handle
        runtime_handle: Option<tokio::runtime::Handle>,
    }

    impl NativeWebSocket {
        pub fn new(config: TransportConfig) -> Self {
            Self {
                config,
                state: ConnectionState::Disconnected,
                framer: MessageFramer::new(1024 * 1024), // 1MB max message
                stats: BandwidthStats::default(),
                tx: None,
                rx: None,
                event_tx: None,
                event_rx: None,
                pending_outgoing: VecDeque::new(),
                last_heartbeat_sent: 0,
                last_heartbeat_received: 0,
                runtime_handle: None,
            }
        }

        /// Connect to WebSocket server
        pub fn connect(&mut self) -> Result<(), String> {
            if self.state == ConnectionState::Connected || self.state == ConnectionState::Connecting {
                return Err("Already connected or connecting".into());
            }

            self.state = ConnectionState::Connecting;
            let url = self.config.url.clone();
            let (event_tx, event_rx) = mpsc::unbounded_channel();
            let (msg_tx, msg_rx) = mpsc::unbounded_channel();
            let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Vec<u8>>();

            self.event_tx = Some(event_tx.clone());
            self.event_rx = Some(event_rx);
            self.tx = Some(out_tx);

            let recv_tx = msg_tx;
            let connect_timeout = self.config.connect_timeout_ms;
            let heartbeat_interval = self.config.heartbeat_interval_ms;
            let max_msg_size = self.framer.max_message_size;

            // Spawn connection task
            let handle = tokio::runtime::Handle::current();
            self.runtime_handle = Some(handle.clone());

            tokio::spawn(async move {
                let result = tokio::time::timeout(
                    Duration::from_millis(connect_timeout as u64),
                    connect_async(&url),
                ).await;

                let ws_stream = match result {
                    Ok(Ok((ws, _))) => ws,
                    Ok(Err(e)) => {
                        let _ = event_tx.send(ConnectionEvent::Error(format!("Connect failed: {}", e)));
                        let _ = event_tx.send(ConnectionEvent::Disconnected { reason: e.to_string() });
                        return;
                    }
                    Err(_) => {
                        let _ = event_tx.send(ConnectionEvent::Error("Connection timeout".into()));
                        let _ = event_tx.send(ConnectionEvent::Disconnected { reason: "timeout".into() });
                        return;
                    }
                };

                let _ = event_tx.send(ConnectionEvent::Connected);

                let (mut ws_sink, mut ws_source) = ws_stream.split();
                let mut framer = MessageFramer::new(max_msg_size);
                let mut heartbeat_interval = tokio::time::interval(
                    Duration::from_millis(heartbeat_interval as u64)
                );

                // Send pending messages
                while let Ok(data) = out_rx.try_recv() {
                    let framed = framer.frame(&data);
                    let _ = ws_sink.send(Message::Binary(framed)).await;
                }

                loop {
                    tokio::select! {
                        // Incoming messages from server
                        msg = ws_source.next() => {
                            match msg {
                                Some(Ok(Message::Binary(data))) => {
                                    let messages = framer.feed(&data);
                                    for msg_data in messages {
                                        let _ = recv_tx.send(TransportMessage::Binary(msg_data));
                                    }
                                }
                                Some(Ok(Message::Text(text))) => {
                                    let _ = recv_tx.send(TransportMessage::Text(text));
                                }
                                Some(Ok(Message::Ping(data))) => {
                                    let _ = recv_tx.send(TransportMessage::Ping(data.clone()));
                                    let _ = ws_sink.send(Message::Pong(data)).await;
                                }
                                Some(Ok(Message::Pong(data))) => {
                                    let _ = recv_tx.send(TransportMessage::Pong(data));
                                }
                                Some(Ok(Message::Close(_))) => {
                                    let _ = event_tx.send(ConnectionEvent::Disconnected {
                                        reason: "Server closed".into()
                                    });
                                    break;
                                }
                                Some(Err(e)) => {
                                    let _ = event_tx.send(ConnectionEvent::Error(
                                        format!("Receive error: {}", e)
                                    ));
                                    break;
                                }
                                None => {
                                    let _ = event_tx.send(ConnectionEvent::Disconnected {
                                        reason: "Stream ended".into()
                                    });
                                    break;
                                }
                                _ => {}
                            }
                        }
                        // Outgoing messages to server
                        data = out_rx.recv() => {
                            if let Some(data) = data {
                                let framed = framer.frame(&data);
                                if let Err(e) = ws_sink.send(Message::Binary(framed)).await {
                                    let _ = event_tx.send(ConnectionEvent::Error(
                                        format!("Send error: {}", e)
                                    ));
                                    break;
                                }
                            } else {
                                break; // Channel closed
                            }
                        }
                        // Heartbeat
                        _ = heartbeat_interval.tick() => {
                            let ping_data = (std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64).to_le_bytes().to_vec();
                            let _ = ws_sink.send(Message::Ping(ping_data)).await;
                            let _ = event_tx.send(ConnectionEvent::HeartbeatSent {
                                timestamp: timestamp_ms(),
                            });
                        }
                    }
                }

                // Cleanup
                drop(recv_tx);
                drop(event_tx);
            });

            Ok(())
        }

        /// Disconnect from server
        pub fn disconnect(&mut self) {
            self.state = ConnectionState::Disconnected;
            self.tx = None;
            self.rx = None;
            self.pending_outgoing.clear();
            self.framer.reset();
        }

        /// Send binary data
        pub fn send_binary(&mut self, data: &[u8]) -> Result<(), String> {
            match &self.tx {
                Some(tx) => {
                    if tx.send(data.to_vec()).is_ok() {
                        self.stats.bytes_sent += data.len() as u64;
                        self.stats.messages_sent += 1;
                        self.stats.frames_sent += 1;
                        self.stats.last_send_time = timestamp_ms();
                        Ok(())
                    } else {
                        Err("Channel closed".into())
                    }
                }
                None => {
                    self.pending_outgoing.push_back(data.to_vec());
                    Ok(())
                }
            }
        }

        /// Send text data
        pub fn send_text(&mut self, text: &str) -> Result<(), String> {
            self.send_binary(text.as_bytes())
        }

        /// Poll for incoming messages (non-blocking)
        pub fn poll_messages(&mut self) -> Vec<TransportMessage> {
            let mut messages = Vec::new();
            if let Some(rx) = &mut self.rx {
                while let Ok(msg) = rx.try_recv() {
                    match &msg {
                        TransportMessage::Binary(data) => {
                            self.stats.bytes_received += data.len() as u64;
                            self.stats.messages_received += 1;
                            self.stats.frames_received += 1;
                            self.stats.last_recv_time = timestamp_ms();
                        }
                        TransportMessage::Pong(_) => {
                            let rtt = (timestamp_ms() - self.last_heartbeat_sent) as f32;
                            self.last_heartbeat_received = timestamp_ms();
                            if let Some(etx) = &self.event_tx {
                                let _ = etx.send(ConnectionEvent::HeartbeatReceived { rtt_ms: rtt });
                            }
                        }
                        _ => {}
                    }
                    messages.push(msg);
                }
            }
            messages
        }

        /// Poll connection events
        pub fn poll_events(&mut self) -> Vec<ConnectionEvent> {
            let mut events = Vec::new();
            if let Some(rx) = &mut self.event_rx {
                while let Ok(event) = rx.try_recv() {
                    match &event {
                        ConnectionEvent::Connected => {
                            self.state = ConnectionState::Connected;
                        }
                        ConnectionEvent::Disconnected { .. } => {
                            self.state = ConnectionState::Disconnected;
                        }
                        ConnectionEvent::HeartbeatSent { timestamp } => {
                            self.last_heartbeat_sent = *timestamp;
                        }
                        _ => {}
                    }
                    events.push(event);
                }
            }
            events
        }

        pub fn state(&self) -> &ConnectionState { &self.state }
        pub fn stats(&self) -> &BandwidthStats { &self.stats }
        pub fn is_connected(&self) -> bool { self.state == ConnectionState::Connected }
    }

    fn timestamp_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

// ═══════════════════════════════════════════════════════════ WASM WebSocket (web-sys)

#[cfg(target_arch = "wasm32")]
pub mod wasm_transport {
    use super::*;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use web_sys::{WebSocket, MessageEvent, CloseEvent, ErrorEvent, BinaryType};

    /// Callback type for messages
    type MessageCallback = Box<dyn FnMut(TransportMessage)>;
    type EventCallback = Box<dyn FnMut(ConnectionEvent)>;

    /// WASM WebSocket connection using web-sys
    pub struct WasmWebSocket {
        config: TransportConfig,
        state: ConnectionState,
        framer: MessageFramer,
        stats: BandwidthStats,
        ws: Option<WebSocket>,
        message_callback: Option<MessageCallback>,
        event_callback: Option<EventCallback>,
        pending_outgoing: VecDeque<Vec<u8>>,
        url: String,
    }

    impl WasmWebSocket {
        pub fn new(config: TransportConfig) -> Self {
            let url = config.url.clone();
            Self {
                config,
                state: ConnectionState::Disconnected,
                framer: MessageFramer::new(1024 * 1024),
                stats: BandwidthStats::default(),
                ws: None,
                message_callback: None,
                event_callback: None,
                pending_outgoing: VecDeque::new(),
                url,
            }
        }

        /// Set message callback
        pub fn on_message(&mut self, callback: impl FnMut(TransportMessage) + 'static) {
            self.message_callback = Some(Box::new(callback));
        }

        /// Set connection event callback
        pub fn on_event(&mut self, callback: impl FnMut(ConnectionEvent) + 'static) {
            self.event_callback = Some(Box::new(callback));
        }

        /// Connect to WebSocket server
        pub fn connect(&mut self) -> Result<(), String> {
            if self.state == ConnectionState::Connected || self.state == ConnectionState::Connecting {
                return Err("Already connected or connecting".into());
            }

            self.state = ConnectionState::Connecting;
            let ws = WebSocket::new(&self.url)
                .map_err(|e| format!("WebSocket creation failed: {:?}", e))?;

            ws.set_binary_type(BinaryType::Arraybuffer)
                .map_err(|e| format!("set_binary_type failed: {:?}", e))?;

            // ── onopen ──
            {
                let ws_clone = ws.clone();
                let mut cb = self.event_callback.take();
                // We need to use a shared state approach for WASM callbacks
                // Store the ws reference and use closure-based callbacks
                let state_flag = Arc::new(Mutex::new(ConnectionState::Connecting));
                let state_clone = state_flag.clone();

                let onopen = Closure::wrap(Box::new(move |_: web_sys::Event| {
                    web_sys::console::log_1(&"WebSocket connected".into());
                    if let Some(ref mut cb) = cb {
                        cb(ConnectionEvent::Connected);
                    }
                    *state_clone.lock().unwrap() = ConnectionState::Connected;
                }) as Box<dyn FnMut(_)>);
                ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
                onopen.forget();
                self.event_callback = cb;
            }

            // ── onmessage ──
            {
                let mut framer = MessageFramer::new(self.framer.max_message_size);
                let mut cb = self.message_callback.take();
                let mut stats = BandwidthStats::default();

                let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
                    if let Ok(data) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                        let bytes = unsafe {
                            let view = js_sys::Uint8Array::view(&data.byte_length().to_be_bytes());
                            let full_view = js_sys::Uint8Array::new_with_byte_offset_and_length(
                                &data, 0, data.byte_length() as u32,
                            );
                            full_view.to_vec()
                        };
                        let messages = framer.feed(&bytes);
                        for msg_data in messages {
                            stats.bytes_received += msg_data.len() as u64;
                            stats.messages_received += 1;
                            if let Some(ref mut cb) = cb {
                                cb(TransportMessage::Binary(msg_data));
                            }
                        }
                    } else if let Ok(text) = e.data().dyn_into::<js_sys::JsString>() {
                        if let Some(s) = text.as_string() {
                            if let Some(ref mut cb) = cb {
                                cb(TransportMessage::Text(s));
                            }
                        }
                    }
                }) as Box<dyn FnMut(_)>);
                ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
                onmessage.forget();
                self.message_callback = cb;
            }

            // ── onclose ──
            {
                let mut cb = self.event_callback.take();
                let onclose = Closure::wrap(Box::new(move |e: CloseEvent| {
                    let reason = e.reason();
                    if let Some(ref mut cb) = cb {
                        cb(ConnectionEvent::Disconnected { reason });
                    }
                }) as Box<dyn FnMut(_)>);
                ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
                onclose.forget();
                self.event_callback = cb;
            }

            // ── onerror ──
            {
                let mut cb = self.event_callback.take();
                let onerror = Closure::wrap(Box::new(move |e: ErrorEvent| {
                    let msg = e.message();
                    web_sys::console::error_1(&format!("WebSocket error: {}", msg).into());
                    if let Some(ref mut cb) = cb {
                        cb(ConnectionEvent::Error(msg));
                    }
                }) as Box<dyn FnMut(_)>);
                ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
                onerror.forget();
                self.event_callback = cb;
            }

            self.ws = Some(ws);
            Ok(())
        }

        /// Disconnect
        pub fn disconnect(&mut self) {
            if let Some(ws) = &self.ws {
                let _ = ws.close();
            }
            self.ws = None;
            self.state = ConnectionState::Disconnected;
            self.pending_outgoing.clear();
            self.framer.reset();
        }

        /// Send binary data
        pub fn send_binary(&mut self, data: &[u8]) -> Result<(), String> {
            match &self.ws {
                Some(ws) if ws.ready_state() == WebSocket::OPEN => {
                    let array = js_sys::Uint8Array::from(data);
                    ws.send_with_array_buffer(&array.buffer())
                        .map_err(|e| format!("Send failed: {:?}", e))?;
                    self.stats.bytes_sent += data.len() as u64;
                    self.stats.messages_sent += 1;
                    self.stats.frames_sent += 1;
                    self.stats.last_send_time = timestamp_ms();
                    Ok(())
                }
                _ => {
                    self.pending_outgoing.push_back(data.to_vec());
                    Ok(())
                }
            }
        }

        /// Send text data
        pub fn send_text(&mut self, text: &str) -> Result<(), String> {
            match &self.ws {
                Some(ws) if ws.ready_state() == WebSocket::OPEN => {
                    ws.send_with_str(text)
                        .map_err(|e| format!("Send failed: {:?}", e))?;
                    self.stats.bytes_sent += text.len() as u64;
                    self.stats.messages_sent += 1;
                    Ok(())
                }
                _ => {
                    self.pending_outgoing.push_back(text.as_bytes().to_vec());
                    Ok(())
                }
            }
        }

        /// Flush pending messages
        pub fn flush_pending(&mut self) {
            let pending: Vec<Vec<u8>> = self.pending_outgoing.drain(..).collect();
            for data in pending {
                if let Err(_) = self.send_binary(&data) {
                    self.pending_outgoing.push_back(data);
                    break;
                }
            }
        }

        pub fn state(&self) -> &ConnectionState { &self.state }
        pub fn stats(&self) -> &BandwidthStats { &self.stats }
        pub fn is_connected(&self) -> bool {
            self.ws.as_ref().map_or(false, |ws| ws.ready_state() == WebSocket::OPEN)
        }

        pub fn set_state(&mut self, state: ConnectionState) {
            self.state = state;
        }
    }

    fn timestamp_ms() -> u64 {
        js_sys::Date::now() as u64
    }
}

// ═══════════════════════════════════════════════════════════ Connection Manager

/// High-level connection manager with auto-reconnect and heartbeat
pub struct ConnectionManager {
    pub config: TransportConfig,
    pub state: ConnectionState,
    pub stats: BandwidthStats,
    pub rtt_ms: f32,

    // Reconnect state
    reconnect_attempts: u32,
    last_disconnect_time: u64,
    next_reconnect_time: u64,

    // Heartbeat state
    last_heartbeat_sent: u64,
    last_heartbeat_received: u64,
    heartbeat_missed_count: u32,

    // Message queues
    incoming: VecDeque<TransportMessage>,
    outgoing: VecDeque<Vec<u8>>,

    // Callbacks
    on_connected: Option<Box<dyn Fn()>>,
    on_disconnected: Option<Box<dyn Fn(String)>>,
    on_message: Option<Box<dyn Fn(Vec<u8>)>>,
}

impl ConnectionManager {
    pub fn new(config: TransportConfig) -> Self {
        Self {
            config,
            state: ConnectionState::Disconnected,
            stats: BandwidthStats::default(),
            rtt_ms: 0.0,
            reconnect_attempts: 0,
            last_disconnect_time: 0,
            next_reconnect_time: 0,
            last_heartbeat_sent: 0,
            last_heartbeat_received: 0,
            heartbeat_missed_count: 0,
            incoming: VecDeque::new(),
            outgoing: VecDeque::new(),
            on_connected: None,
            on_disconnected: None,
            on_message: None,
        }
    }

    /// Set callbacks
    pub fn set_on_connected(&mut self, f: impl Fn() + 'static) {
        self.on_connected = Some(Box::new(f));
    }
    pub fn set_on_disconnected(&mut self, f: impl Fn(String) + 'static) {
        self.on_disconnected = Some(Box::new(f));
    }
    pub fn set_on_message(&mut self, f: impl Fn(Vec<u8>) + 'static) {
        self.on_message = Some(Box::new(f));
    }

    /// Queue outgoing message
    pub fn send(&mut self, data: Vec<u8>) {
        self.outgoing.push_back(data);
    }

    /// Process incoming framed messages
    pub fn on_raw_data(&mut self, data: &[u8]) {
        self.stats.bytes_received += data.len() as u64;
        self.stats.messages_received += 1;
        self.stats.last_recv_time = timestamp_ms();
        if let Some(cb) = &mut self.on_message {
            cb(data.to_vec());
        }
    }

    /// Mark as connected
    pub fn on_connected(&mut self) {
        self.state = ConnectionState::Connected;
        self.reconnect_attempts = 0;
        self.heartbeat_missed_count = 0;
        if let Some(cb) = &mut self.on_connected {
            cb();
        }
    }

    /// Mark as disconnected
    pub fn on_disconnected(&mut self, reason: &str) {
        self.state = ConnectionState::Disconnected;
        self.last_disconnect_time = timestamp_ms();
        if let Some(cb) = &mut self.on_disconnected {
            cb(reason.to_string());
        }
        self.schedule_reconnect();
    }

    /// Schedule reconnection
    fn schedule_reconnect(&mut self) {
        if self.reconnect_attempts >= self.config.max_reconnect_attempts {
            self.state = ConnectionState::Failed("Max reconnect attempts reached".into());
            return;
        }
        let delay = (self.config.reconnect_base_delay_ms as u64) *
            (1u64 << self.reconnect_attempts.min(6));
        let delay = delay.min(self.config.reconnect_max_delay_ms as u64);
        self.next_reconnect_time = timestamp_ms() + delay;
        self.state = ConnectionState::Reconnecting;
        self.reconnect_attempts += 1;
        self.stats.reconnect_count += 1;
    }

    /// Check if should reconnect now
    pub fn should_reconnect(&self) -> bool {
        self.state == ConnectionState::Reconnecting &&
        timestamp_ms() >= self.next_reconnect_time
    }

    /// Send heartbeat ping
    pub fn send_heartbeat(&mut self) -> Vec<u8> {
        let ts = timestamp_ms();
        self.last_heartbeat_sent = ts;
        ts.to_le_bytes().to_vec()
    }

    /// Process heartbeat pong
    pub fn on_heartbeat_response(&mut self, data: &[u8]) {
        if data.len() >= 8 {
            let sent_ts = u64::from_le_bytes(data[..8].try_into().unwrap_or([0; 8]));
            self.rtt_ms = (timestamp_ms() - sent_ts) as f32;
            self.last_heartbeat_received = timestamp_ms();
            self.heartbeat_missed_count = 0;
        }
    }

    /// Check heartbeat timeout
    pub fn check_heartbeat_timeout(&mut self) -> bool {
        if self.last_heartbeat_sent > 0 &&
           self.last_heartbeat_received < self.last_heartbeat_sent {
            self.heartbeat_missed_count += 1;
            if self.heartbeat_missed_count >= 3 {
                return true; // Timeout — disconnect
            }
        }
        false
    }

    /// Get connection URL with reconnect info
    pub fn connection_info(&self) -> ConnectionInfo {
        ConnectionInfo {
            state: self.state.clone(),
            url: self.config.url.clone(),
            rtt_ms: self.rtt_ms,
            reconnect_attempts: self.reconnect_attempts,
            bytes_sent: self.stats.bytes_sent,
            bytes_received: self.stats.bytes_received,
            messages_sent: self.stats.messages_sent,
            messages_received: self.stats.messages_received,
        }
    }

    /// Drain outgoing messages
    pub fn drain_outgoing(&mut self) -> Vec<Vec<u8>> {
        self.outgoing.drain(..).collect()
    }

    pub fn is_connected(&self) -> bool { self.state == ConnectionState::Connected }
}

#[derive(Clone, Debug)]
pub struct ConnectionInfo {
    pub state: ConnectionState,
    pub url: String,
    pub rtt_ms: f32,
    pub reconnect_attempts: u32,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
}

fn timestamp_ms() -> u64 {
    #[cfg(target_arch = "wasm32")]
    { js_sys::Date::now() as u64 }
    #[cfg(not(target_arch = "wasm32"))]
    { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64 }
}

// ═══════════════════════════════════════════════════════════ Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_framer() {
        let mut framer = MessageFramer::new(1024);

        // Frame a message
        let data = b"Hello, WebSocket!";
        let framed = framer.frame(data);
        assert_eq!(framed.len(), 4 + data.len());
        assert_eq!(u32::from_le_bytes([framed[0], framed[1], framed[2], framed[3]]), data.len() as u32);

        // Feed and extract
        let messages = framer.feed(&framed);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0], data);
    }

    #[test]
    fn test_message_framer_multiple() {
        let mut framer = MessageFramer::new(1024);

        let msg1 = b"first";
        let msg2 = b"second message";
        let msg3 = b"third";

        let mut combined = Vec::new();
        combined.extend_from_slice(&framer.frame(msg1));
        combined.extend_from_slice(&framer.frame(msg2));
        combined.extend_from_slice(&framer.frame(msg3));

        let messages = framer.feed(&combined);
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0], msg1);
        assert_eq!(messages[1], msg2);
        assert_eq!(messages[2], msg3);
    }

    #[test]
    fn test_message_framer_partial() {
        let mut framer = MessageFramer::new(1024);
        let data = b"partial data test";
        let framed = framer.frame(data);

        // Feed first half
        let mid = framed.len() / 2;
        let messages = framer.feed(&framed[..mid]);
        assert_eq!(messages.len(), 0);

        // Feed second half
        let messages = framer.feed(&framed[mid..]);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0], data);
    }

    #[test]
    fn test_message_framer_max_size() {
        let mut framer = MessageFramer::new(16);
        let big_data = vec![0u8; 32];
        let framed = framer.frame(&big_data);

        let messages = framer.feed(&framed);
        assert_eq!(messages.len(), 0); // Exceeds max size, dropped
    }

    #[test]
    fn test_connection_manager() {
        let config = TransportConfig::default();
        let mut mgr = ConnectionManager::new(config);
        assert!(!mgr.is_connected());
        assert_eq!(mgr.state, ConnectionState::Disconnected);

        mgr.on_connected();
        assert!(mgr.is_connected());

        mgr.on_disconnected("test");
        assert!(!mgr.is_connected());
        assert_eq!(mgr.reconnect_attempts, 1);
        assert!(mgr.state == ConnectionState::Reconnecting);
    }

    #[test]
    fn test_reconnect_backoff() {
        let config = TransportConfig {
            reconnect_base_delay_ms: 100,
            reconnect_max_delay_ms: 5000,
            max_reconnect_attempts: 10,
            ..Default::default()
        };
        let mut mgr = ConnectionManager::new(config);

        // First disconnect -> attempt 1, delay 100ms
        mgr.on_disconnected("test");
        assert_eq!(mgr.reconnect_attempts, 1);

        // Second disconnect -> attempt 2, delay 200ms
        mgr.on_disconnected("test");
        assert_eq!(mgr.reconnect_attempts, 2);

        // Keep going until max
        for _ in 0..10 {
            mgr.on_disconnected("test");
        }
        assert_eq!(mgr.state, ConnectionState::Failed("Max reconnect attempts reached".into()));
    }

    #[test]
    fn test_heartbeat_rtt() {
        let config = TransportConfig::default();
        let mut mgr = ConnectionManager::new(config);
        mgr.on_connected();

        // Send heartbeat
        let ping_data = mgr.send_heartbeat();
        assert_eq!(ping_data.len(), 8);

        // Simulate response (same timestamp)
        mgr.on_heartbeat_response(&ping_data);
        assert!(mgr.rtt_ms < 1.0); // Should be nearly 0
    }

    #[test]
    fn test_bandwidth_stats() {
        let config = TransportConfig::default();
        let mut mgr = ConnectionManager::new(config);
        mgr.on_connected();

        mgr.on_raw_data(&[1, 2, 3, 4, 5]);
        assert_eq!(mgr.stats.bytes_received, 5);
        assert_eq!(mgr.stats.messages_received, 1);

        mgr.send(vec![6, 7, 8]);
        assert_eq!(mgr.outgoing.len(), 1);
    }

    #[test]
    fn test_connection_info() {
        let config = TransportConfig {
            url: "ws://example.com:9000".into(),
            ..Default::default()
        };
        let mgr = ConnectionManager::new(config);
        let info = mgr.connection_info();
        assert_eq!(info.url, "ws://example.com:9000");
        assert_eq!(info.state, ConnectionState::Disconnected);
    }

    #[test]
    fn test_transport_config_defaults() {
        let config = TransportConfig::default();
        assert_eq!(config.connect_timeout_ms, 5000);
        assert_eq!(config.heartbeat_interval_ms, 5000);
        assert_eq!(config.max_reconnect_attempts, 10);
        assert_eq!(config.send_buffer_size, 65536);
    }

    #[test]
    fn test_framer_reset() {
        let mut framer = MessageFramer::new(1024);
        let framed = framer.frame(b"test");
        framer.feed(&framed);
        framer.reset();
        assert_eq!(framer.pending_bytes(), 0);
    }
}
