//! Chat state and logic with control messages.

use crate::error::ChatError;
use crate::types::Value;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::mpsc::Sender;
use std::thread::JoinHandle;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub uid: String,
    pub addr: Option<String>,
    pub online: bool,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub from: String,
    pub text: String,
    pub chat: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum P2pControl {
    NewChat { name: String, members: Vec<String>, from: String },
    AddMember { chat: String, uid: String },
    RemoveMember { chat: String, uid: String },
    DeleteChat { name: String },
    ChatMessage { chat: String, from: String, text: String, timestamp: i64 },
}

pub struct ChatState {
    pub users: BTreeMap<String, User>,
    pub chats: BTreeMap<String, Vec<String>>,
    pub messages: BTreeMap<String, Vec<Message>>,
    pub inbox: BTreeMap<String, Vec<Message>>,
    pub current_user: Option<String>,
    pub p2p_port: u16,
    pub contact_server_addr: Option<String>,
    pub contacts: BTreeMap<String, String>,
    pub downloads: Vec<Value>,
    pub server_handle: Option<JoinHandle<()>>,
    pub server_stop: Option<Sender<()>>,
    pub external_ip: Option<String>,
}

impl ChatState {
    pub fn new() -> Self {
        ChatState {
            users: BTreeMap::new(),
            chats: BTreeMap::new(),
            messages: BTreeMap::new(),
            inbox: BTreeMap::new(),
            current_user: None,
            p2p_port: 0,
            contact_server_addr: None,
            contacts: BTreeMap::new(),
            downloads: Vec::new(),
            server_handle: None,
            server_stop: None,
            external_ip: None,
        }
    }

    pub fn login(&mut self, uid: String) -> Result<(), ChatError> {
        if !uid.starts_with('@') {
            return Err(ChatError::new("Invalid UID (must start with @)", 1));
        }
        self.users.entry(uid.clone()).or_insert(User {
            uid: uid.clone(),
            addr: None,
            online: true,
        });
        self.current_user = Some(uid);
        Ok(())
    }

    pub fn logout(&mut self) -> Result<(), ChatError> {
        if let Some(uid) = &self.current_user {
            if let Some(user) = self.users.get_mut(uid) {
                user.online = false;
            }
            self.current_user = None;
            Ok(())
        } else {
            Err(ChatError::new("Not logged in", 1))
        }
    }

    pub fn delete_user(&mut self, uid: &str) -> Result<(), ChatError> {
        if self.current_user.as_deref() == Some(uid) {
            self.logout()?;
        }
        if self.users.remove(uid).is_some() {
            for members in self.chats.values_mut() {
                members.retain(|u| u != uid);
            }
            self.contacts.remove(uid);
            self.inbox.remove(uid);
            Ok(())
        } else {
            Err(ChatError::new("User not found", 1))
        }
    }

    pub fn new_chat(&mut self, name: String, members: Vec<String>) -> Result<(), ChatError> {
        if self.chats.contains_key(&name) {
            return Err(ChatError::new("Chat already exists", 1));
        }
        self.chats.insert(name.clone(), members);
        self.messages.entry(name).or_default();
        Ok(())
    }

    pub fn add_member(&mut self, uid: String, chat: &str) -> Result<(), ChatError> {
        if let Some(members) = self.chats.get_mut(chat) {
            if !members.contains(&uid) {
                members.push(uid);
            }
            Ok(())
        } else {
            Err(ChatError::new("Chat not found", 1))
        }
    }

    pub fn remove_member(&mut self, uid: &str, chat: &str) -> Result<(), ChatError> {
        if let Some(members) = self.chats.get_mut(chat) {
            members.retain(|u| u != uid);
            Ok(())
        } else {
            Err(ChatError::new("Chat not found", 1))
        }
    }

    pub fn delete_chat(&mut self, name: &str) -> Result<(), ChatError> {
        if self.chats.remove(name).is_some() {
            self.messages.remove(name);
            Ok(())
        } else {
            Err(ChatError::new("Chat not found", 1))
        }
    }

    pub fn list_chats(&self) -> Vec<String> {
        self.chats.keys().cloned().collect()
    }

    pub fn members(&self, chat: &str) -> Result<Vec<String>, ChatError> {
        self.chats.get(chat).cloned().ok_or_else(|| ChatError::new("Chat not found", 1))
    }

    pub fn open_chat(&mut self, chat: String) -> Result<(), ChatError> {
        if self.chats.contains_key(&chat) {
            Ok(())
        } else {
            Err(ChatError::new("Chat not found", 1))
        }
    }

    pub fn send_message(&mut self, target: &str, text: &str) -> Result<(), ChatError> {
        let sender = self.current_user.clone().ok_or(ChatError::new("Not logged in", 1))?;
        let msg = Message {
            from: sender.clone(),
            text: text.to_string(),
            chat: String::new(),
            timestamp: chrono::Utc::now().timestamp(),
        };
        if target == "@everyone" {
            let all_recipients: Vec<String> = self.chats.values()
                .flat_map(|members| members.iter().cloned())
                .collect();
            for uid in all_recipients {
                self.deliver_to_user(uid, msg.clone());
            }
        } else {
            self.deliver_to_user(target.to_string(), msg);
        }
        Ok(())
    }

    pub fn send_to_chat(&mut self, chat: &str, text: &str) -> Result<(), ChatError> {
        let sender = self.current_user.clone().ok_or(ChatError::new("Not logged in", 1))?;
        let members = self.chats.get(chat)
            .ok_or(ChatError::new("Chat not found", 1))?
            .clone();
        let msg = Message {
            from: sender.clone(),
            text: text.to_string(),
            chat: chat.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        };
        self.messages.entry(chat.to_string()).or_default().push(msg.clone());
        for uid in members {
            if uid != sender {
                self.deliver_to_user(uid, msg.clone());
            }
        }
        Ok(())
    }

    pub fn deliver_to_user(&mut self, uid: String, msg: Message) {
        self.inbox.entry(uid).or_default().push(msg);
    }

    pub fn get_inbox(&self, user: &str) -> Vec<Value> {
        self.inbox.get(user).cloned().unwrap_or_default().into_iter().map(|m| Value::ChatMsg {
            from: m.from,
            text: m.text,
            chat: m.chat,
            attachment: None,
        }).collect()
    }

    pub fn get_history(&self, chat: &str) -> Vec<Value> {
        self.messages.get(chat).cloned().unwrap_or_default().into_iter().map(|m| Value::ChatMsg {
            from: m.from,
            text: m.text,
            chat: m.chat,
            attachment: None,
        }).collect()
    }

    pub fn save_file(&mut self, index: usize, path: &str) -> Result<(), ChatError> {
        if index >= self.downloads.len() {
            return Err(ChatError::new("Index out of bounds", 1));
        }
        let val = &self.downloads[index];
        if let Value::FileTransfer { data, .. } = val {
            std::fs::write(path, data).map_err(|e| ChatError::new(&e.to_string(), 1))?;
            Ok(())
        } else {
            Err(ChatError::new("Not a file transfer", 1))
        }
    }

    pub fn add_contact(&mut self, uid: String, addr: String) {
        self.contacts.insert(uid, addr);
    }

    pub fn remove_contact(&mut self, uid: &str) {
        self.contacts.remove(uid);
    }

    pub fn get_contact(&self, uid: &str) -> Option<String> {
        self.contacts.get(uid).cloned()
    }

    pub fn handle_control(&mut self, control: &P2pControl) -> Result<(), ChatError> {
        match control {
            P2pControl::NewChat { name, members, from: _ } => {
                if !self.chats.contains_key(name) {
                    self.chats.insert(name.clone(), members.clone());
                    self.messages.entry(name.clone()).or_default();
                }
            }
            P2pControl::AddMember { chat, uid } => {
                if let Some(m) = self.chats.get_mut(chat) {
                    if !m.contains(uid) {
                        m.push(uid.clone());
                    }
                }
            }
            P2pControl::RemoveMember { chat, uid } => {
                if let Some(m) = self.chats.get_mut(chat) {
                    m.retain(|u| u != uid);
                }
            }
            P2pControl::DeleteChat { name } => {
                self.chats.remove(name);
                self.messages.remove(name);
            }
            P2pControl::ChatMessage { chat, from, text, timestamp } => {
                let msg = Message {
                    from: from.clone(),
                    text: text.clone(),
                    chat: chat.clone(),
                    timestamp: *timestamp,
                };
                self.messages.entry(chat.clone()).or_default().push(msg.clone());
                // Also deliver to inbox if mentioned
                if let Some(current) = &self.current_user {
                    if text.contains(&format!("@{}", current)) {
                        self.deliver_to_user(current.clone(), msg);
                    }
                }
            }
        }
        Ok(())
    }
}
