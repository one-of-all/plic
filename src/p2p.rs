//! P2P messaging with control messages.

use crate::chat::{ChatState, P2pControl};
use crate::types::Value;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use native_tls::{TlsConnector, TlsAcceptor, Identity};
use std::fs;
use std::path::Path;
use once_cell::sync::Lazy;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct P2pMessage {
    pub msg_type: String,
    pub from: String,
    pub to: String,
    pub text: Option<String>,
    pub filename: Option<String>,
    pub data_base64: Option<String>,
    pub chat: Option<String>,
    pub timestamp: Option<i64>,
    pub control: Option<P2pControl>,
}

static TLS_CONNECTOR: Lazy<TlsConnector> = Lazy::new(|| {
    let identity = load_or_generate_identity().expect("Failed to load or generate TLS identity");
    TlsConnector::builder()
        .identity(identity)
        .danger_accept_invalid_certs(true)
        .build()
        .expect("Failed to build TLS connector")
});

static TLS_ACCEPTOR: Lazy<TlsAcceptor> = Lazy::new(|| {
    let identity = load_or_generate_identity().expect("Failed to load or generate TLS identity");
    TlsAcceptor::new(identity).expect("Failed to build TLS acceptor")
});

fn load_or_generate_identity() -> Result<Identity, Box<dyn std::error::Error>> {
    let cert_path = ".plic_cert.pem";
    let key_path = ".plic_key.pem";
    if !Path::new(cert_path).exists() || !Path::new(key_path).exists() {
        let cert = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
        let cert_pem = cert.serialize_pem()?;
        let key_pem = cert.serialize_private_key_pem();
        fs::write(cert_path, &cert_pem)?;
        fs::write(key_path, &key_pem)?;
    }
    let cert = fs::read(cert_path)?;
    let key = fs::read(key_path)?;
    Identity::from_pkcs8(&cert, &key).map_err(|e| e.into())
}

pub fn get_tls_connector() -> TlsConnector {
    TLS_CONNECTOR.clone()
}

pub fn get_tls_acceptor() -> TlsAcceptor {
    TLS_ACCEPTOR.clone()
}

pub fn start_p2p_listener(port: u16, state: Arc<Mutex<ChatState>>) {
    let listener = match std::net::TcpListener::bind(format!("0.0.0.0:{}", port)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind P2P port {}: {}", port, e);
            return;
        }
    };
    let acceptor = get_tls_acceptor();
    thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(stream) = stream {
                let acceptor = acceptor.clone();
                let state = Arc::clone(&state);
                thread::spawn(move || {
                    if let Ok(tls_stream) = acceptor.accept(stream) {
                        handle_p2p_connection(tls_stream, state);
                    }
                });
            }
        }
    });
}

fn handle_p2p_connection(stream: native_tls::TlsStream<TcpStream>, state: Arc<Mutex<ChatState>>) {
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        if let Ok(line) = line {
            if let Ok(msg) = serde_json::from_str::<P2pMessage>(&line) {
                let mut state = state.lock().unwrap();
                match msg.msg_type.as_str() {
                    "msg" => {
                        let message = crate::chat::Message {
                            from: msg.from,
                            text: msg.text.unwrap_or_default(),
                            chat: msg.chat.clone().unwrap_or_default(),
                            timestamp: msg.timestamp.unwrap_or(0),
                        };
                        if let Some(chat) = msg.chat {
                            state.messages.entry(chat).or_default().push(message.clone());
                        }
                        if !msg.to.is_empty() {
                            state.deliver_to_user(msg.to, message);
                        }
                    }
                    "file" => {
                        use base64::Engine;
                        let data = base64::engine::general_purpose::STANDARD
                            .decode(msg.data_base64.as_deref().unwrap_or(""))
                            .unwrap_or_default();
                        state.downloads.push(Value::FileTransfer {
                            from: msg.from,
                            filename: msg.filename.unwrap_or_default(),
                            data,
                        });
                    }
                    "control" => {
                        if let Some(control) = msg.control {
                            if let Err(e) = state.handle_control(&control) {
                                eprintln!("Control error: {}", e);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

pub fn send_message_to(addr: &str, msg: P2pMessage) -> Result<(), Box<dyn std::error::Error>> {
    let connector = get_tls_connector();
    let stream = TcpStream::connect(addr)?;
    let mut tls_stream = connector.connect("localhost", stream)?;
    let json = serde_json::to_string(&msg)?;
    writeln!(tls_stream, "{}", json)?;
    Ok(())
}

pub fn send_control(addr: &str, control: P2pControl, from: &str) -> Result<(), Box<dyn std::error::Error>> {
    let msg = P2pMessage {
        msg_type: "control".to_string(),
        from: from.to_string(),
        to: String::new(),
        text: None,
        filename: None,
        data_base64: None,
        chat: None,
        timestamp: None,
        control: Some(control),
    };
    send_message_to(addr, msg)
}
