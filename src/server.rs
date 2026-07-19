//! Contact server.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use native_tls::TlsStream;
use std::net::TcpStream;
use std::sync::mpsc::Sender;

pub type ContactsDb = Arc<Mutex<BTreeMap<String, String>>>;

pub fn start_contacts_server(addr: &str, password: Option<String>) -> Result<(ContactsDb, thread::JoinHandle<()>, Sender<()>), Box<dyn std::error::Error>> {
    let db = Arc::new(Mutex::new(BTreeMap::new()));
    let listener = std::net::TcpListener::bind(addr)?;
    listener.set_nonblocking(true)?;
    let acceptor = crate::p2p::get_tls_acceptor();
    let db_clone = db.clone();
    let addr_owned = addr.to_string();
    let password_arc = Arc::new(password);

    let (stop_tx, stop_rx) = std::sync::mpsc::channel();

    let handle = thread::spawn(move || {
        loop {
            if let Ok(()) = stop_rx.try_recv() {
                break;
            }

            match listener.accept() {
                Ok((stream, _)) => {
                    let db = db_clone.clone();
                    let acceptor = acceptor.clone();
                    let password = password_arc.clone();
                    thread::spawn(move || {
                        if let Ok(tls_stream) = acceptor.accept(stream) {
                            handle_connection(tls_stream, db, password);
                        }
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(std::time::Duration::from_millis(10));
                    continue;
                }
                Err(e) => {
                    eprintln!("Contact server accept error: {}", e);
                    break;
                }
            }
        }
    });

    Ok((db, handle, stop_tx))
}

fn handle_connection(stream: TlsStream<TcpStream>, db: ContactsDb, password: Arc<Option<String>>) {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();

    let mut authenticated = password.is_none();
    if let Some(pass) = &*password {
        if let Ok(n) = reader.read_line(&mut line) {
            if n == 0 { return; }
            let trimmed = line.trim();
            if trimmed.starts_with("PASSWORD ") {
                let provided = trimmed[9..].trim();
                if provided == pass {
                    authenticated = true;
                    let _ = writeln!(reader.get_mut(), "OK");
                    let _ = reader.get_mut().flush();
                } else {
                    let _ = writeln!(reader.get_mut(), "ERROR Invalid password");
                    let _ = reader.get_mut().flush();
                    return;
                }
            } else {
                let _ = writeln!(reader.get_mut(), "ERROR Password required");
                let _ = reader.get_mut().flush();
                return;
            }
        } else {
            return;
        }
    }

    line.clear();
    while let Ok(n) = reader.read_line(&mut line) {
        if n == 0 { break; }
        let trimmed = line.trim();
        let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
        match parts[0] {
            "REGISTER" => {
                if authenticated && parts.len() == 2 {
                    let mut split = parts[1].splitn(2, ' ');
                    if let (Some(uid), Some(addr)) = (split.next(), split.next()) {
                        db.lock().unwrap().insert(uid.to_string(), addr.to_string());
                        let _ = writeln!(reader.get_mut(), "OK");
                        let _ = reader.get_mut().flush();
                    }
                }
            }
            "GET" => {
                if authenticated {
                    let contacts = db.lock().unwrap().iter()
                        .map(|(k, v)| format!("{} {}", k, v))
                        .collect::<Vec<_>>()
                        .join("\n");
                    let _ = writeln!(reader.get_mut(), "{}", contacts);
                    let _ = reader.get_mut().flush();
                }
            }
            _ => {}
        }
        line.clear();
    }
}
