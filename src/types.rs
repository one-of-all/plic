use crate::ast::Expr;
use crate::error::ChatError;
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct FetchOptions {
    pub url: String,
    pub method: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FetchResult {
    pub status: i64,
    pub body: String,
    pub headers: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Number {
    Int(i64),
    Float(f64),
}

impl Number {
    pub fn to_f64(&self) -> f64 {
        match self {
            Number::Int(i) => *i as f64,
            Number::Float(f) => *f,
        }
    }
    pub fn to_i64(&self) -> i64 {
        match self {
            Number::Int(i) => *i,
            Number::Float(f) => *f as i64,
        }
    }
}

#[derive(Clone)]
pub enum Value {
    Num(Number),
    Char(char),
    String(String),
    Bool(bool),
    Unit,
    Uid(String),
    ByteString(Vec<u8>),
    List(Vec<Value>),
    Tuple(Vec<Value>),
    Closure(Vec<String>, Expr, Environment),
    BuiltinFunc(String, usize, Arc<dyn Fn(Vec<Value>) -> Result<Value, ChatError> + Send + Sync>),
    Curry {
        name: String,
        arity: usize,
        f: Arc<dyn Fn(Vec<Value>) -> Result<Value, ChatError> + Send + Sync>,
        args: Vec<Value>,
    },
    Custom(String, Vec<Value>),
    Record(BTreeMap<String, Value>),
    Pid(usize),
    DateTime(chrono::DateTime<chrono::Local>),
    Duration(std::time::Duration),
    Json(JsonValue),
    Maybe(Option<Box<Value>>),
    Either(Box<Either>),
    ChatMsg {
        from: String,
        text: String,
        chat: String,
        attachment: Option<Box<Value>>,
    },
    FileInfo {
        name: String,
        size: i64,
        mime: String,
    },
    FileTransfer {
        from: String,
        filename: String,
        data: Vec<u8>,
    },
    FetchOptions(FetchOptions),
    FetchResult(FetchResult),
    Map(BTreeMap<String, Value>),
    Set(BTreeSet<String>),
    ClassInstance {
        class: String,
        fields: BTreeMap<String, Value>,
    },
    Break(Option<Box<Value>>), // internal use only
}

#[derive(Clone, Serialize, Deserialize)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}

#[derive(Clone)]
pub struct Either {
    pub left: Option<Box<Value>>,
    pub right: Option<Box<Value>>,
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}

impl Value {
    pub fn display(&self) -> String {
        match self {
            Value::Num(n) => match n {
                Number::Int(i) => format!("{}", i),
                Number::Float(x) => {
                    let s = format!("{:.10}", x);
                    if s == "-0.0000000000" { "0.0".to_string() }
                    else {
                        let trimmed = s.trim_end_matches('0').trim_end_matches('.');
                        if trimmed.is_empty() { "0".to_string() } else { trimmed.to_string() }
                    }
                }
            }
            Value::Char(c) => c.to_string(),
            Value::String(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
            Value::Unit => "()".to_string(),
            Value::Uid(u) => u.clone(),
            Value::ByteString(b) => format!("#B\"{}\"", hex::encode(b)),
            Value::List(v) => {
                let items: Vec<String> = v.iter().map(|x| x.display()).collect();
                format!("[{}]", items.join(", "))
            }
            Value::Tuple(v) => {
                let items: Vec<String> = v.iter().map(|x| x.display()).collect();
                format!("({})", items.join(", "))
            }
            Value::Closure(_, _, _) => "<closure>".to_string(),
            Value::BuiltinFunc(name, _, _) => format!("<builtin {}>", name),
            Value::Curry { name, .. } => format!("<curry {}>", name),
            Value::Custom(name, args) => {
                let args_str: Vec<String> = args.iter().map(|x| x.display()).collect();
                if args.is_empty() {
                    name.clone()
                } else {
                    format!("{} {}", name, args_str.join(" "))
                }
            }
            Value::Record(map) => {
                let fields: Vec<String> = map.iter()
                    .map(|(k, v)| format!("{} = {}", k, v.display()))
                    .collect();
                format!("{{ {} }}", fields.join(", "))
            }
            Value::Pid(pid) => format!("<pid {}>", pid),
            Value::DateTime(dt) => dt.to_rfc3339(),
            Value::Duration(d) => format!("{}s", d.as_secs_f64()),
            Value::Json(j) => serde_json::to_string_pretty(j).unwrap_or_default(),
            Value::Maybe(m) => match m {
                Some(v) => format!("Just {}", v.display()),
                None => "Nothing".to_string(),
            },
            Value::Either(e) => {
                if let Some(left) = &e.left {
                    format!("Left {}", left.display())
                } else if let Some(right) = &e.right {
                    format!("Right {}", right.display())
                } else {
                    "Either".to_string()
                }
            }
            Value::ChatMsg { from, text, chat, attachment } => {
                let attach = if let Some(a) = attachment {
                    format!(" with {}", a.display())
                } else { "".to_string() };
                format!("[Message from {} in {}: \"{}\"{}]", from, chat, text, attach)
            }
            Value::FileInfo { name, size, mime } => {
                format!("[FileInfo {} ({} bytes, {})]", name, size, mime)
            }
            Value::FileTransfer { from, filename, .. } => {
                format!("[FileTransfer from {}: {}]", from, filename)
            }
            Value::FetchOptions(opts) => {
                format!("FetchOptions {{ url: {}, method: {}, headers: {:?}, body: {:?} }}",
                        opts.url, opts.method, opts.headers, opts.body)
            }
            Value::FetchResult(res) => {
                format!("FetchResult {{ status: {}, body: {}, headers: {:?} }}",
                        res.status, res.body, res.headers)
            }
            Value::Map(map) => {
                let entries: Vec<String> = map.iter()
                    .map(|(k, v)| format!("{}: {}", k, v.display()))
                    .collect();
                format!("%{{{}}}", entries.join(", "))
            }
            Value::Set(set) => {
                let elems: Vec<String> = set.iter().cloned().collect();
                format!("%[{}]", elems.join(", "))
            }
            Value::ClassInstance { class, fields } => {
                let flds: Vec<String> = fields.iter()
                    .map(|(k, v)| format!("{} = {}", k, v.display()))
                    .collect();
                format!("{} {{ {} }}", class, flds.join(", "))
            }
            Value::Break(opt) => {
                if let Some(v) = opt {
                    format!("break {}", v.display())
                } else {
                    "break".to_string()
                }
            }
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}

#[derive(Debug, Clone)]
pub struct Environment {
    pub vars: BTreeMap<String, Value>,
    pub type_map: BTreeMap<String, String>,
}

impl Environment {
    pub fn new() -> Self {
        Environment { vars: BTreeMap::new(), type_map: BTreeMap::new() }
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        self.vars.get(name).cloned()
    }

    pub fn set(&mut self, name: String, val: Value) {
        self.vars.insert(name, val);
    }

    pub fn extend(&self, other: BTreeMap<String, Value>) -> Self {
        let mut new = self.clone();
        new.vars.extend(other);
        new
    }
}
