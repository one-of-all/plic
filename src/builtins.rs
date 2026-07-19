use crate::chat::{ChatState, P2pControl};
use crate::error::ChatError;
use crate::eval::{
    self, spawn_process, proc_self, proc_send, proc_recv, proc_wait, proc_exit, sleep, after, type_name,
};
use crate::p2p::{self, P2pMessage};
use crate::server;
use crate::types::{Environment, Value, JsonValue, FetchResult, Number};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::{DateTime, Local};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use serde_json::Value as SerdeValue;
use sha2::{Sha256, Digest};
use std::fs;
use pqcrypto_kyber::kyber768::{keypair, encapsulate, decapsulate};
use pqcrypto_traits::kem::{PublicKey, SecretKey, Ciphertext, SharedSecret};
use reqwest::blocking::get;

macro_rules! builtin {
    ($name:expr, $arity:expr, $func:expr) => {
        Value::BuiltinFunc($name.to_string(), $arity, std::sync::Arc::new($func))
    };
}

pub fn populate(env: &mut Environment, state: Arc<Mutex<ChatState>>, global_env: Arc<Mutex<Environment>>) {
    env.set(
        "sqrt".to_string(),
        builtin!("sqrt", 1, |args| {
            if let [Value::Num(n)] = &args[..] {
                let x = n.to_f64();
                Ok(Value::Num(Number::Float(x.sqrt())))
            } else {
                Err(ChatError::new("sqrt expects a Num argument", 1))
            }
        }),
    );

    env.set(
        "sin".to_string(),
        builtin!("sin", 1, |args| {
            if let [Value::Num(n)] = &args[..] {
                let x = n.to_f64();
                Ok(Value::Num(Number::Float(x.sin())))
            } else {
                Err(ChatError::new("sin expects a Num argument", 1))
            }
        }),
    );

    env.set(
        "cos".to_string(),
        builtin!("cos", 1, |args| {
            if let [Value::Num(n)] = &args[..] {
                let x = n.to_f64();
                Ok(Value::Num(Number::Float(x.cos())))
            } else {
                Err(ChatError::new("cos expects a Num argument", 1))
            }
        }),
    );

    env.set(
        "tan".to_string(),
        builtin!("tan", 1, |args| {
            if let [Value::Num(n)] = &args[..] {
                let x = n.to_f64();
                Ok(Value::Num(Number::Float(x.tan())))
            } else {
                Err(ChatError::new("tan expects a Num argument", 1))
            }
        }),
    );

    env.set(
        "asin".to_string(),
        builtin!("asin", 1, |args| {
            if let [Value::Num(n)] = &args[..] {
                let x = n.to_f64();
                Ok(Value::Num(Number::Float(x.asin())))
            } else {
                Err(ChatError::new("asin expects a Num argument", 1))
            }
        }),
    );

    env.set(
        "acos".to_string(),
        builtin!("acos", 1, |args| {
            if let [Value::Num(n)] = &args[..] {
                let x = n.to_f64();
                Ok(Value::Num(Number::Float(x.acos())))
            } else {
                Err(ChatError::new("acos expects a Num argument", 1))
            }
        }),
    );

    env.set(
        "atan".to_string(),
        builtin!("atan", 1, |args| {
            if let [Value::Num(n)] = &args[..] {
                let x = n.to_f64();
                Ok(Value::Num(Number::Float(x.atan())))
            } else {
                Err(ChatError::new("atan expects a Num argument", 1))
            }
        }),
    );

    env.set(
        "toFloat".to_string(),
        builtin!("toFloat", 1, |args| {
            if let [Value::Num(n)] = &args[..] {
                Ok(Value::Num(Number::Float(n.to_f64())))
            } else {
                Err(ChatError::new("toFloat expects a Num argument", 1))
            }
        }),
    );

    env.set(
        "toInt".to_string(),
        builtin!("toInt", 1, |args| {
            if let [Value::Num(n)] = &args[..] {
                Ok(Value::Num(Number::Int(n.to_i64())))
            } else {
                Err(ChatError::new("toInt expects a Num argument", 1))
            }
        }),
    );

    env.set(
        "show".to_string(),
        builtin!("show", 1, |args| {
            if args.len() == 1 {
                println!("{}", args[0].display());
                Ok(Value::Unit)
            } else {
                Err(ChatError::new("show expects exactly one argument", 1))
            }
        }),
    );

    env.set(
        "parseInt".to_string(),
        builtin!("parseInt", 1, |args| {
            if let [Value::String(s)] = &args[..] {
                s.parse::<i64>()
                    .map(|i| Value::Num(Number::Int(i)))
                    .map_err(|_| ChatError::new("parseInt: invalid integer string", 1))
            } else {
                Err(ChatError::new("parseInt expects a String argument", 1))
            }
        }),
    );

    env.set(
        "parseFloat".to_string(),
        builtin!("parseFloat", 1, |args| {
            if let [Value::String(s)] = &args[..] {
                s.parse::<f64>()
                    .map(|f| Value::Num(Number::Float(f)))
                    .map_err(|_| ChatError::new("parseFloat: invalid float string", 1))
            } else {
                Err(ChatError::new("parseFloat expects a String argument", 1))
            }
        }),
    );

    env.set(
        "chr".to_string(),
        builtin!("chr", 1, |args| {
            if let [Value::Num(Number::Int(i))] = &args[..] {
                if let Some(c) = std::char::from_u32(*i as u32) {
                    Ok(Value::Char(c))
                } else {
                    Err(ChatError::new("chr: invalid Unicode code point", 1))
                }
            } else {
                Err(ChatError::new("chr expects an Int (Num) argument", 1))
            }
        }),
    );

    env.set(
        "ord".to_string(),
        builtin!("ord", 1, |args| {
            if let [Value::Char(c)] = &args[..] {
                Ok(Value::Num(Number::Int(*c as i64)))
            } else {
                Err(ChatError::new("ord expects a Char argument", 1))
            }
        }),
    );

    env.set(
        "typeof".to_string(),
        builtin!("typeof", 1, |args| {
            if args.len() == 1 {
                Ok(Value::String(type_name(&args[0])))
            } else {
                Err(ChatError::new("typeof expects exactly one argument", 1))
            }
        }),
    );

    env.set(
        "null".to_string(),
        builtin!("null", 1, |args| {
            if let [Value::List(v)] = &args[..] {
                Ok(Value::Bool(v.is_empty()))
            } else {
                Err(ChatError::new("null expects a List argument", 1))
            }
        }),
    );

    env.set(
        "length".to_string(),
        builtin!("length", 1, |args| {
            if let [v] = &args[..] {
                match v {
                    Value::List(l) => Ok(Value::Num(Number::Int(l.len() as i64))),
                    Value::String(s) => Ok(Value::Num(Number::Int(s.len() as i64))),
                    Value::ByteString(b) => Ok(Value::Num(Number::Int(b.len() as i64))),
                    Value::Map(m) => Ok(Value::Num(Number::Int(m.len() as i64))),
                    Value::Set(s) => Ok(Value::Num(Number::Int(s.len() as i64))),
                    Value::Tuple(t) => Ok(Value::Num(Number::Int(t.len() as i64))),
                    _ => Err(ChatError::new(
                        "length expects a List, String, ByteString, Map, Set, or Tuple",
                        1,
                    )),
                }
            } else {
                Err(ChatError::new("length expects exactly one argument", 1))
            }
        }),
    );

    let state_map = state.clone();
    env.set(
        "map".to_string(),
        builtin!("map", 2, move |args| {
            if let [Value::Closure(params, body, env), Value::List(list)] = &args[..] {
                let mut result = Vec::new();
                for item in list {
                    let mut new_env = env.clone();
                    new_env.set(params[0].clone(), item.clone());
                    let val = eval::eval_expr(&body, &mut new_env, Arc::clone(&state_map))?;
                    result.push(val);
                }
                Ok(Value::List(result))
            } else {
                Err(ChatError::new("map expects a function and a List", 1))
            }
        }),
    );

    let state_filter = state.clone();
    env.set(
        "filter".to_string(),
        builtin!("filter", 2, move |args| {
            if let [Value::Closure(params, body, env), Value::List(list)] = &args[..] {
                let mut result = Vec::new();
                for item in list {
                    let mut new_env = env.clone();
                    new_env.set(params[0].clone(), item.clone());
                    let val = eval::eval_expr(&body, &mut new_env, Arc::clone(&state_filter))?;
                    if let Value::Bool(b) = val {
                        if b {
                            result.push(item.clone());
                        }
                    } else {
                        return Err(ChatError::new(
                            "filter predicate must return a Bool value",
                            1,
                        ));
                    }
                }
                Ok(Value::List(result))
            } else {
                Err(ChatError::new("filter expects a function and a List", 1))
            }
        }),
    );

    let state_foldl = state.clone();
    env.set(
        "foldl".to_string(),
        builtin!("foldl", 3, move |args| {
            if let [Value::Closure(params, body, env), Value::List(list), acc] = &args[..] {
                let mut acc_val = acc.clone();
                for item in list {
                    let mut new_env = env.clone();
                    new_env.set(params[0].clone(), acc_val);
                    new_env.set(params[1].clone(), item.clone());
                    acc_val = eval::eval_expr(&body, &mut new_env, Arc::clone(&state_foldl))?;
                }
                Ok(acc_val)
            } else {
                Err(ChatError::new(
                    "foldl expects a function, a List, and an initial accumulator",
                    1,
                ))
            }
        }),
    );

    let state_foldr = state.clone();
    env.set(
        "foldr".to_string(),
        builtin!("foldr", 3, move |args| {
            if let [Value::Closure(params, body, env), Value::List(list), acc] = &args[..] {
                let mut acc_val = acc.clone();
                for item in list.iter().rev() {
                    let mut new_env = env.clone();
                    new_env.set(params[0].clone(), item.clone());
                    new_env.set(params[1].clone(), acc_val);
                    acc_val = eval::eval_expr(&body, &mut new_env, Arc::clone(&state_foldr))?;
                }
                Ok(acc_val)
            } else {
                Err(ChatError::new(
                    "foldr expects a function, a List, and an initial accumulator",
                    1,
                ))
            }
        }),
    );

    env.set(
        "take".to_string(),
        builtin!("take", 2, |args| {
            if let [Value::Num(Number::Int(n)), v] = &args[..] {
                let n = *n as usize;
                match v {
                    Value::List(l) => {
                        let taken: Vec<Value> = l.iter().take(n).cloned().collect();
                        Ok(Value::List(taken))
                    }
                    Value::String(s) => {
                        let taken: String = s.chars().take(n).collect();
                        Ok(Value::String(taken))
                    }
                    Value::ByteString(b) => {
                        let taken: Vec<u8> = b.iter().take(n).cloned().collect();
                        Ok(Value::ByteString(taken))
                    }
                    _ => Err(ChatError::new(
                        "take expects a List, String, or ByteString as second argument",
                        1,
                    )),
                }
            } else {
                Err(ChatError::new(
                    "take expects an Int (Num) and a collection",
                    1,
                ))
            }
        }),
    );

    env.set(
        "drop".to_string(),
        builtin!("drop", 2, |args| {
            if let [Value::Num(Number::Int(n)), v] = &args[..] {
                let n = *n as usize;
                match v {
                    Value::List(l) => {
                        if n >= l.len() {
                            Ok(Value::List(vec![]))
                        } else {
                            Ok(Value::List(l[n..].to_vec()))
                        }
                    }
                    Value::String(s) => {
                        let dropped: String = s.chars().skip(n).collect();
                        Ok(Value::String(dropped))
                    }
                    Value::ByteString(b) => {
                        if n >= b.len() {
                            Ok(Value::ByteString(vec![]))
                        } else {
                            Ok(Value::ByteString(b[n..].to_vec()))
                        }
                    }
                    _ => Err(ChatError::new(
                        "drop expects a List, String, or ByteString as second argument",
                        1,
                    )),
                }
            } else {
                Err(ChatError::new(
                    "drop expects an Int (Num) and a collection",
                    1,
                ))
            }
        }),
    );

    env.set(
        "reverse".to_string(),
        builtin!("reverse", 1, |args| {
            if let [Value::List(l)] = &args[..] {
                let mut rev = l.clone();
                rev.reverse();
                Ok(Value::List(rev))
            } else {
                Err(ChatError::new("reverse expects a List argument", 1))
            }
        }),
    );

    let state_all = state.clone();
    env.set(
        "all".to_string(),
        builtin!("all", 2, move |args| {
            if let [Value::Closure(params, body, env), Value::List(list)] = &args[..] {
                for item in list {
                    let mut new_env = env.clone();
                    new_env.set(params[0].clone(), item.clone());
                    let val = eval::eval_expr(&body, &mut new_env, Arc::clone(&state_all))?;
                    if let Value::Bool(b) = val {
                        if !b {
                            return Ok(Value::Bool(false));
                        }
                    } else {
                        return Err(ChatError::new(
                            "all predicate must return a Bool value",
                            1,
                        ));
                    }
                }
                Ok(Value::Bool(true))
            } else {
                Err(ChatError::new("all expects a function and a List", 1))
            }
        }),
    );

    let state_any = state.clone();
    env.set(
        "any".to_string(),
        builtin!("any", 2, move |args| {
            if let [Value::Closure(params, body, env), Value::List(list)] = &args[..] {
                for item in list {
                    let mut new_env = env.clone();
                    new_env.set(params[0].clone(), item.clone());
                    let val = eval::eval_expr(&body, &mut new_env, Arc::clone(&state_any))?;
                    if let Value::Bool(b) = val {
                        if b {
                            return Ok(Value::Bool(true));
                        }
                    } else {
                        return Err(ChatError::new(
                            "any predicate must return a Bool value",
                            1,
                        ));
                    }
                }
                Ok(Value::Bool(false))
            } else {
                Err(ChatError::new("any expects a function and a List", 1))
            }
        }),
    );

    let state_find = state.clone();
    env.set(
        "find".to_string(),
        builtin!("find", 2, move |args| {
            if let [Value::Closure(params, body, env), Value::List(list)] = &args[..] {
                for item in list {
                    let mut new_env = env.clone();
                    new_env.set(params[0].clone(), item.clone());
                    let val = eval::eval_expr(&body, &mut new_env, Arc::clone(&state_find))?;
                    if let Value::Bool(b) = val {
                        if b {
                            return Ok(Value::Maybe(Some(Box::new(item.clone()))));
                        }
                    } else {
                        return Err(ChatError::new(
                            "find predicate must return a Bool value",
                            1,
                        ));
                    }
                }
                Ok(Value::Maybe(None))
            } else {
                Err(ChatError::new("find expects a function and a List", 1))
            }
        }),
    );

    env.set(
        "sort".to_string(),
        builtin!("sort", 1, |args| {
            if let [Value::List(list)] = &args[..] {
                let mut sorted = list.clone();
                sorted.sort_by(|a, b| a.display().cmp(&b.display()));
                Ok(Value::List(sorted))
            } else {
                Err(ChatError::new("sort expects a List argument", 1))
            }
        }),
    );

    let state_sortby = state.clone();
    env.set(
        "sortBy".to_string(),
        builtin!("sortBy", 2, move |args| {
            if let [Value::Closure(params, body, closure_env), Value::List(list)] = &args[..] {
                let mut sorted = list.clone();
                sorted.sort_by(|a, b| {
                    let mut env = closure_env.clone();
                    env.set(params[0].clone(), a.clone());
                    env.set(params[1].clone(), b.clone());
                    let result =
                        eval::eval_expr(&body, &mut env, Arc::clone(&state_sortby)).unwrap_or(Value::Num(Number::Int(0)));
                    match result {
                        Value::Num(Number::Int(i)) => i.cmp(&0),
                        Value::Num(Number::Float(f)) => {
                            if f < 0.0 {
                                std::cmp::Ordering::Less
                            } else if f > 0.0 {
                                std::cmp::Ordering::Greater
                            } else {
                                std::cmp::Ordering::Equal
                            }
                        }
                        _ => std::cmp::Ordering::Equal,
                    }
                });
                Ok(Value::List(sorted))
            } else {
                Err(ChatError::new(
                    "sortBy expects a function (a -> a -> Num) and a List",
                    1,
                ))
            }
        }),
    );

    env.set(
        "sum".to_string(),
        builtin!("sum", 1, |args| {
            if let [Value::List(list)] = &args[..] {
                let mut total = 0.0;
                for item in list {
                    match item {
                        Value::Num(Number::Int(i)) => total += *i as f64,
                        Value::Num(Number::Float(f)) => total += f,
                        _ => return Err(ChatError::new("sum expects a list of numbers", 1)),
                    }
                }
                Ok(Value::Num(Number::Float(total)))
            } else {
                Err(ChatError::new("sum expects a List argument", 1))
            }
        }),
    );

    env.set(
        "concat".to_string(),
        builtin!("concat", 1, |args| {
            if let [Value::List(lists)] = &args[..] {
                let mut result = Vec::new();
                for item in lists {
                    match item {
                        Value::List(inner) => result.extend(inner.clone()),
                        _ => {
                            return Err(ChatError::new(
                                "concat expects a list of lists (each element must be a List)",
                                1,
                            ))
                        }
                    }
                }
                Ok(Value::List(result))
            } else {
                Err(ChatError::new("concat expects a List argument", 1))
            }
        }),
    );

    env.set(
        "flatten".to_string(),
        builtin!("flatten", 1, |args| {
            if let [Value::List(lists)] = &args[..] {
                let mut result = Vec::new();
                for item in lists {
                    match item {
                        Value::List(inner) => result.extend(inner.clone()),
                        _ => {
                            return Err(ChatError::new(
                                "flatten expects a list of lists (each element must be a List)",
                                1,
                            ))
                        }
                    }
                }
                Ok(Value::List(result))
            } else {
                Err(ChatError::new("flatten expects a List argument", 1))
            }
        }),
    );

    env.set(
        "zip".to_string(),
        builtin!("zip", 2, |args| {
            if let [Value::List(a), Value::List(b)] = &args[..] {
                let min_len = a.len().min(b.len());
                let mut result = Vec::new();
                for i in 0..min_len {
                    result.push(Value::Tuple(vec![a[i].clone(), b[i].clone()]));
                }
                Ok(Value::List(result))
            } else {
                Err(ChatError::new("zip expects two List arguments", 1))
            }
        }),
    );

    let state_zipwith = state.clone();
    env.set(
        "zipWith".to_string(),
        builtin!("zipWith", 3, move |args| {
            if let [Value::Closure(params, body, closure_env), Value::List(a), Value::List(b)] = &args[..] {
                let min_len = a.len().min(b.len());
                let mut result = Vec::new();
                for i in 0..min_len {
                    let mut env = closure_env.clone();
                    env.set(params[0].clone(), a[i].clone());
                    env.set(params[1].clone(), b[i].clone());
                    let val = eval::eval_expr(&body, &mut env, Arc::clone(&state_zipwith))?;
                    result.push(val);
                }
                Ok(Value::List(result))
            } else {
                Err(ChatError::new(
                    "zipWith expects a function and two List arguments",
                    1,
                ))
            }
        }),
    );

    env.set(
        "unzip".to_string(),
        builtin!("unzip", 1, |args| {
            if let [Value::List(pairs)] = &args[..] {
                let mut left = Vec::new();
                let mut right = Vec::new();
                for pair in pairs {
                    match pair {
                        Value::Tuple(v) if v.len() == 2 => {
                            left.push(v[0].clone());
                            right.push(v[1].clone());
                        }
                        _ => {
                            return Err(ChatError::new(
                                "unzip expects a list of 2‑element tuples",
                                1,
                            ))
                        }
                    }
                }
                Ok(Value::Tuple(vec![Value::List(left), Value::List(right)]))
            } else {
                Err(ChatError::new("unzip expects a List argument", 1))
            }
        }),
    );

    env.set(
        "indexOf".to_string(),
        builtin!("indexOf", 2, |args| {
            if let [Value::List(list), elem] = &args[..] {
                for (i, item) in list.iter().enumerate() {
                    if item.display() == elem.display() {
                        return Ok(Value::Maybe(Some(Box::new(Value::Num(Number::Int(i as i64))))));
                    }
                }
                Ok(Value::Maybe(None))
            } else {
                Err(ChatError::new("indexOf expects a List and an element", 1))
            }
        }),
    );

    env.set(
        "lastIndexOf".to_string(),
        builtin!("lastIndexOf", 2, |args| {
            if let [Value::List(list), elem] = &args[..] {
                for i in (0..list.len()).rev() {
                    if list[i].display() == elem.display() {
                        return Ok(Value::Maybe(Some(Box::new(Value::Num(Number::Int(i as i64))))));
                    }
                }
                Ok(Value::Maybe(None))
            } else {
                Err(ChatError::new("lastIndexOf expects a List and an element", 1))
            }
        }),
    );

    env.set(
        "split".to_string(),
        builtin!("split", 2, |args| {
            if let [Value::String(delim), Value::String(s)] = &args[..] {
                let parts: Vec<String> = s.split(delim).map(|x| x.to_string()).collect();
                let list = parts.into_iter().map(Value::String).collect();
                Ok(Value::List(list))
            } else {
                Err(ChatError::new(
                    "split expects a String delimiter and a String",
                    1,
                ))
            }
        }),
    );

    env.set(
        "join".to_string(),
        builtin!("join", 2, |args| {
            if let [Value::String(delim), Value::List(list)] = &args[..] {
                let strings: Result<Vec<String>, ChatError> = list
                    .iter()
                    .map(|v| {
                        if let Value::String(s) = v {
                            Ok(s.clone())
                        } else {
                            Err(ChatError::new("join expects a list of Strings", 1))
                        }
                    })
                    .collect();
                let joined = strings?.join(delim);
                Ok(Value::String(joined))
            } else {
                Err(ChatError::new("join expects a String and a list of Strings", 1))
            }
        }),
    );

    env.set(
        "startsWith".to_string(),
        builtin!("startsWith", 2, |args| {
            if let [Value::String(prefix), Value::String(s)] = &args[..] {
                Ok(Value::Bool(s.starts_with(prefix)))
            } else {
                Err(ChatError::new("startsWith expects two String arguments", 1))
            }
        }),
    );

    env.set(
        "endsWith".to_string(),
        builtin!("endsWith", 2, |args| {
            if let [Value::String(suffix), Value::String(s)] = &args[..] {
                Ok(Value::Bool(s.ends_with(suffix)))
            } else {
                Err(ChatError::new("endsWith expects two String arguments", 1))
            }
        }),
    );

    env.set(
        "trim".to_string(),
        builtin!("trim", 1, |args| {
            if let [Value::String(s)] = &args[..] {
                Ok(Value::String(s.trim().to_string()))
            } else {
                Err(ChatError::new("trim expects a String argument", 1))
            }
        }),
    );

    env.set(
        "replace".to_string(),
        builtin!("replace", 3, |args| {
            if let [Value::String(from), Value::String(to), Value::String(s)] = &args[..] {
                Ok(Value::String(s.replace(from, to)))
            } else {
                Err(ChatError::new(
                    "replace expects three String arguments: from, to, and source",
                    1,
                ))
            }
        }),
    );

    env.set(
        "substring".to_string(),
        builtin!("substring", 3, |args| {
            if let [Value::Num(Number::Int(start)), Value::Num(Number::Int(len)), Value::String(s)] = &args[..] {
                let start = *start as usize;
                let len = *len as usize;
                if start + len > s.len() {
                    Err(ChatError::new("substring: indices out of bounds", 1))
                } else {
                    Ok(Value::String(s[start..start + len].to_string()))
                }
            } else {
                Err(ChatError::new(
                    "substring expects two Int (Num) arguments and a String",
                    1,
                ))
            }
        }),
    );

    env.set(
        "parseJson".to_string(),
        builtin!("parseJson", 1, |args| {
            if let [Value::String(s)] = &args[..] {
                let v: SerdeValue =
                    serde_json::from_str(s).map_err(|e| ChatError::new(&format!("parseJson: {}", e), 1))?;
                Ok(Value::Json(serde_json_to_chatlang(v)))
            } else {
                Err(ChatError::new("parseJson expects a String argument", 1))
            }
        }),
    );

    env.set(
        "encodeJson".to_string(),
        builtin!("encodeJson", 1, |args| {
            if let [Value::Json(j)] = &args[..] {
                let serde_val = chatlang_json_to_serde(j);
                let json_str = serde_json::to_string(&serde_val)
                    .map_err(|e| ChatError::new(&format!("encodeJson: {}", e), 1))?;
                Ok(Value::String(json_str))
            } else {
                Err(ChatError::new("encodeJson expects a JsonValue argument", 1))
            }
        }),
    );

    env.set(
        "lookup".to_string(),
        builtin!("lookup", 2, |args| {
            if let [Value::String(key), Value::Json(j)] = &args[..] {
                match j {
                    JsonValue::Object(map) => {
                        if let Some(val) = map.get(key) {
                            Ok(Value::Json(val.clone()))
                        } else {
                            Ok(Value::Maybe(None))
                        }
                    }
                    _ => Err(ChatError::new(
                        "lookup expects a String key and a JsonObject",
                        1,
                    )),
                }
            } else {
                Err(ChatError::new(
                    "lookup expects a String and a JsonValue argument",
                    1,
                ))
            }
        }),
    );

    env.set(
        "formatTime".to_string(),
        builtin!("formatTime", 2, |args| {
            if let [Value::String(fmt), Value::DateTime(dt)] = &args[..] {
                let formatted = dt.format(&fmt).to_string();
                Ok(Value::String(formatted))
            } else {
                Err(ChatError::new(
                    "formatTime expects a String format and a DateTime",
                    1,
                ))
            }
        }),
    );

    env.set(
        "parseTime".to_string(),
        builtin!("parseTime", 2, |args| {
            if let [Value::String(fmt), Value::String(s)] = &args[..] {
                let dt = DateTime::parse_from_str(s, &fmt)
                    .map_err(|e| ChatError::new(&format!("parseTime: {}", e), 1))?;
                Ok(Value::DateTime(dt.with_timezone(&Local)))
            } else {
                Err(ChatError::new("parseTime expects two String arguments", 1))
            }
        }),
    );

    env.set(
        "addDuration".to_string(),
        builtin!("addDuration", 2, |args| {
            if let [Value::DateTime(dt), Value::Duration(dur)] = &args[..] {
                Ok(Value::DateTime(*dt + *dur))
            } else {
                Err(ChatError::new(
                    "addDuration expects a DateTime and a Duration",
                    1,
                ))
            }
        }),
    );

    env.set(
        "diffDuration".to_string(),
        builtin!("diffDuration", 2, |args| {
            if let [Value::DateTime(dt1), Value::DateTime(dt2)] = &args[..] {
                let dur = if *dt1 > *dt2 {
                    *dt1 - *dt2
                } else {
                    *dt2 - *dt1
                };
                Ok(Value::Duration(dur.to_std().unwrap_or(Duration::from_secs(0))))
            } else {
                Err(ChatError::new(
                    "diffDuration expects two DateTime arguments",
                    1,
                ))
            }
        }),
    );

    env.set(
        "now".to_string(),
        builtin!("now", 0, |_args| Ok(Value::DateTime(Local::now()))),
    );

    env.set(
        "packBytes".to_string(),
        builtin!("packBytes", 1, |args| {
            if let [Value::List(list)] = &args[..] {
                let mut bytes = Vec::new();
                for item in list {
                    if let Value::Num(Number::Int(i)) = item {
                        if *i < 0 || *i > 255 {
                            return Err(ChatError::new("packBytes: byte value out of range (0‑255)", 1));
                        }
                        bytes.push(*i as u8);
                    } else {
                        return Err(ChatError::new(
                            "packBytes expects a list of Int (Num) values",
                            1,
                        ));
                    }
                }
                Ok(Value::ByteString(bytes))
            } else {
                Err(ChatError::new("packBytes expects a List argument", 1))
            }
        }),
    );

    env.set(
        "unpackBytes".to_string(),
        builtin!("unpackBytes", 1, |args| {
            if let [Value::ByteString(b)] = &args[..] {
                let list = b.iter().map(|&x| Value::Num(Number::Int(x as i64))).collect();
                Ok(Value::List(list))
            } else {
                Err(ChatError::new("unpackBytes expects a ByteString argument", 1))
            }
        }),
    );

    env.set(
        "putStrLn".to_string(),
        builtin!("putStrLn", 1, |args| {
            if let [Value::String(s)] = &args[..] {
                println!("{}", s);
                Ok(Value::Unit)
            } else {
                Err(ChatError::new("putStrLn expects a String argument", 1))
            }
        }),
    );

    env.set(
        "getLine".to_string(),
        builtin!("getLine", 0, |_args| {
            let mut input = String::new();
            std::io::stdin()
                .read_line(&mut input)
                .map_err(|e| ChatError::new(&format!("getLine: {}", e), 1))?;
            Ok(Value::String(input.trim().to_string()))
        }),
    );

    env.set(
        "getArgs".to_string(),
        builtin!("getArgs", 0, |_args| {
            let args: Vec<String> = std::env::args().collect();
            let list = args.into_iter().skip(1).map(Value::String).collect();
            Ok(Value::List(list))
        }),
    );

    env.set(
        "readFile".to_string(),
        builtin!("readFile", 1, |args| {
            if let [Value::String(path)] = &args[..] {
                let content =
                    std::fs::read_to_string(path).map_err(|e| ChatError::new(&format!("readFile: {}", e), 1))?;
                Ok(Value::String(content))
            } else {
                Err(ChatError::new("readFile expects a String path", 1))
            }
        }),
    );

    env.set(
        "readBinaryFile".to_string(),
        builtin!("readBinaryFile", 1, |args| {
            if let [Value::String(path)] = &args[..] {
                let bytes =
                    std::fs::read(path).map_err(|e| ChatError::new(&format!("readBinaryFile: {}", e), 1))?;
                Ok(Value::ByteString(bytes))
            } else {
                Err(ChatError::new("readBinaryFile expects a String path", 1))
            }
        }),
    );

    env.set(
        "writeFile".to_string(),
        builtin!("writeFile", 2, |args| {
            if let [Value::String(path), Value::String(content)] = &args[..] {
                std::fs::write(path, content).map_err(|e| ChatError::new(&format!("writeFile: {}", e), 1))?;
                Ok(Value::Unit)
            } else {
                Err(ChatError::new("writeFile expects a String path and String content", 1))
            }
        }),
    );

    env.set(
        "appendFile".to_string(),
        builtin!("appendFile", 2, |args| {
            if let [Value::String(path), Value::String(content)] = &args[..] {
                use std::io::Write;
                let mut file = std::fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(path)
                    .map_err(|e| ChatError::new(&format!("appendFile: {}", e), 1))?;
                file.write_all(content.as_bytes())
                    .map_err(|e| ChatError::new(&format!("appendFile: {}", e), 1))?;
                Ok(Value::Unit)
            } else {
                Err(ChatError::new("appendFile expects a String path and String content", 1))
            }
        }),
    );

    env.set(
        "writeBinaryFile".to_string(),
        builtin!("writeBinaryFile", 2, |args| {
            if let [Value::String(path), Value::ByteString(bytes)] = &args[..] {
                std::fs::write(path, bytes)
                    .map_err(|e| ChatError::new(&format!("writeBinaryFile: {}", e), 1))?;
                Ok(Value::Unit)
            } else {
                Err(ChatError::new(
                    "writeBinaryFile expects a String path and a ByteString",
                    1,
                ))
            }
        }),
    );

    env.set(
        "fileExists".to_string(),
        builtin!("fileExists", 1, |args| {
            if let [Value::String(path)] = &args[..] {
                Ok(Value::Bool(std::path::Path::new(path).exists()))
            } else {
                Err(ChatError::new("fileExists expects a String path", 1))
            }
        }),
    );

    env.set(
        "fileSize".to_string(),
        builtin!("fileSize", 1, |args| {
            if let [Value::String(path)] = &args[..] {
                let meta =
                    std::fs::metadata(path).map_err(|e| ChatError::new(&format!("fileSize: {}", e), 1))?;
                Ok(Value::Num(Number::Int(meta.len() as i64)))
            } else {
                Err(ChatError::new("fileSize expects a String path", 1))
            }
        }),
    );

    env.set(
        "listDir".to_string(),
        builtin!("listDir", 1, |args| {
            if let [Value::String(path)] = &args[..] {
                let entries = fs::read_dir(path)
                    .map_err(|e| ChatError::new(&format!("listDir: {}", e), 1))?
                    .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().to_string()))
                    .map(Value::String)
                    .collect();
                Ok(Value::List(entries))
            } else {
                Err(ChatError::new("listDir expects a String path", 1))
            }
        }),
    );

    env.set(
        "createDir".to_string(),
        builtin!("createDir", 1, |args| {
            if let [Value::String(path)] = &args[..] {
                fs::create_dir_all(path).map_err(|e| ChatError::new(&format!("createDir: {}", e), 1))?;
                Ok(Value::Unit)
            } else {
                Err(ChatError::new("createDir expects a String path", 1))
            }
        }),
    );

    env.set(
        "removeDir".to_string(),
        builtin!("removeDir", 1, |args| {
            if let [Value::String(path)] = &args[..] {
                fs::remove_dir_all(path).map_err(|e| ChatError::new(&format!("removeDir: {}", e), 1))?;
                Ok(Value::Unit)
            } else {
                Err(ChatError::new("removeDir expects a String path", 1))
            }
        }),
    );

    env.set(
        "fileMove".to_string(),
        builtin!("fileMove", 2, |args| {
            if let [Value::String(from), Value::String(to)] = &args[..] {
                fs::rename(from, to).map_err(|e| ChatError::new(&format!("fileMove: {}", e), 1))?;
                Ok(Value::Unit)
            } else {
                Err(ChatError::new("fileMove expects two String paths", 1))
            }
        }),
    );

    #[cfg(unix)]
    env.set(
        "filePermissions".to_string(),
        builtin!("filePermissions", 1, |args| {
            if let [Value::String(path)] = &args[..] {
                use std::os::unix::fs::PermissionsExt;
                let meta =
                    fs::metadata(path).map_err(|e| ChatError::new(&format!("filePermissions: {}", e), 1))?;
                Ok(Value::Num(Number::Int(meta.permissions().mode() as i64)))
            } else {
                Err(ChatError::new("filePermissions expects a String path", 1))
            }
        }),
    );
    #[cfg(not(unix))]
    env.set(
        "filePermissions".to_string(),
        builtin!("filePermissions", 1, |_args| Ok(Value::Num(Number::Int(0)))),
    );

    #[cfg(unix)]
    env.set(
        "setFilePermissions".to_string(),
        builtin!("setFilePermissions", 2, |args| {
            if let [Value::String(path), Value::Num(Number::Int(mode))] = &args[..] {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(path)
                    .map_err(|e| ChatError::new(&format!("setFilePermissions: {}", e), 1))?
                    .permissions();
                perms.set_mode(*mode as u32);
                fs::set_permissions(path, perms)
                    .map_err(|e| ChatError::new(&format!("setFilePermissions: {}", e), 1))?;
                Ok(Value::Unit)
            } else {
                Err(ChatError::new(
                    "setFilePermissions expects a String path and an Int (Num) mode",
                    1,
                ))
            }
        }),
    );
    #[cfg(not(unix))]
    env.set(
        "setFilePermissions".to_string(),
        builtin!("setFilePermissions", 2, |_args| Ok(Value::Unit)),
    );

    env.set(
        "fetch".to_string(),
        builtin!("fetch", 1, |args| {
            if let [Value::String(url)] = &args[..] {
                let resp = reqwest::blocking::get(url)
                    .map_err(|e| ChatError::new(&format!("fetch: {}", e), 1))?;
                let status = resp.status().as_u16() as i64;
                let headers: Vec<(String, String)> = resp
                    .headers()
                    .iter()
                    .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
                    .collect();
                let body = resp.text().map_err(|e| ChatError::new(&format!("fetch: {}", e), 1))?;
                Ok(Value::FetchResult(FetchResult {
                    status,
                    body,
                    headers,
                }))
            } else {
                Err(ChatError::new("fetch expects a String URL", 1))
            }
        }),
    );

    env.set(
        "fetchOpts".to_string(),
        builtin!("fetchOpts", 1, |args| {
            if let [Value::FetchOptions(opts)] = &args[..] {
                let client = reqwest::blocking::Client::new();
                let mut req = client.request(
                    opts.method.parse().unwrap_or(reqwest::Method::GET),
                    &opts.url,
                );
                for (k, v) in &opts.headers {
                    req = req.header(k, v);
                }
                if let Some(b) = &opts.body {
                    req = req.body(b.clone());
                }
                let resp = req.send().map_err(|e| ChatError::new(&format!("fetchOpts: {}", e), 1))?;
                let status = resp.status().as_u16() as i64;
                let resp_headers: Vec<(String, String)> = resp
                    .headers()
                    .iter()
                    .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
                    .collect();
                let body_str = resp.text().map_err(|e| ChatError::new(&format!("fetchOpts: {}", e), 1))?;
                Ok(Value::FetchResult(FetchResult {
                    status,
                    body: body_str,
                    headers: resp_headers,
                }))
            } else {
                Err(ChatError::new("fetchOpts expects a FetchOptions value", 1))
            }
        }),
    );

    let state_clone = state.clone();
    env.set(
        "login".to_string(),
        builtin!("login", 1, move |args| {
            if let [Value::Uid(uid)] = &args[..] {
                state_clone.lock().unwrap().login(uid.clone())?;
                Ok(Value::Unit)
            } else {
                Err(ChatError::new("login expects a Uid argument", 1))
            }
        }),
    );

    let state_clone = state.clone();
    env.set(
        "logout".to_string(),
        builtin!("logout", 0, move |_args| {
            state_clone.lock().unwrap().logout()?;
            Ok(Value::Unit)
        }),
    );

    let state_clone = state.clone();
    env.set(
        "deleteUser".to_string(),
        builtin!("deleteUser", 1, move |args| {
            if let [Value::Uid(uid)] = &args[..] {
                state_clone.lock().unwrap().delete_user(uid)?;
                Ok(Value::Unit)
            } else {
                Err(ChatError::new("deleteUser expects a Uid argument", 1))
            }
        }),
    );

    let state_clone = state.clone();
    env.set(
        "newChat".to_string(),
        builtin!("newChat", 2, move |args| {
            if let [Value::String(name), Value::List(members)] = &args[..] {
                let uids: Vec<String> = members
                    .iter()
                    .filter_map(|v| {
                        if let Value::Uid(u) = v {
                            Some(u.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                let mut state = state_clone.lock().unwrap();
                state.new_chat(name.clone(), uids.clone())?;
                let sender = state
                    .current_user
                    .clone()
                    .ok_or(ChatError::new("Not logged in", 1))?;
                let control = P2pControl::NewChat {
                    name: name.clone(),
                    members: uids.clone(),
                    from: sender.clone(),
                };
                for uid in uids {
                    if uid != sender {
                        if let Some(addr) = state.get_contact(&uid) {
                            let _ = p2p::send_control(&addr, control.clone(), &sender);
                        }
                    }
                }
                Ok(Value::String(name.clone()))
            } else {
                Err(ChatError::new("newChat expects a String name and a list of Uids", 1))
            }
        }),
    );

    let state_clone = state.clone();
    env.set(
        "addMember".to_string(),
        builtin!("addMember", 2, move |args| {
            if let [Value::Uid(uid), Value::String(chat)] = &args[..] {
                let mut state = state_clone.lock().unwrap();
                state.add_member(uid.clone(), chat)?;
                let sender = state
                    .current_user
                    .clone()
                    .ok_or(ChatError::new("Not logged in", 1))?;
                let control = P2pControl::AddMember {
                    chat: chat.clone(),
                    uid: uid.clone(),
                };
                for member in state.members(chat)? {
                    if member != sender && member != *uid {
                        if let Some(addr) = state.get_contact(&member) {
                            let _ = p2p::send_control(&addr, control.clone(), &sender);
                        }
                    }
                }
                Ok(Value::Unit)
            } else {
                Err(ChatError::new(
                    "addMember expects a Uid and a String (chat name)",
                    1,
                ))
            }
        }),
    );

    let state_clone = state.clone();
    env.set(
        "removeMember".to_string(),
        builtin!("removeMember", 2, move |args| {
            if let [Value::Uid(uid), Value::String(chat)] = &args[..] {
                let mut state = state_clone.lock().unwrap();
                state.remove_member(uid, chat)?;
                let sender = state
                    .current_user
                    .clone()
                    .ok_or(ChatError::new("Not logged in", 1))?;
                let control = P2pControl::RemoveMember {
                    chat: chat.clone(),
                    uid: uid.clone(),
                };
                for member in state.members(chat)? {
                    if member != sender && member != *uid {
                        if let Some(addr) = state.get_contact(&member) {
                            let _ = p2p::send_control(&addr, control.clone(), &sender);
                        }
                    }
                }
                Ok(Value::Unit)
            } else {
                Err(ChatError::new(
                    "removeMember expects a Uid and a String (chat name)",
                    1,
                ))
            }
        }),
    );

    let state_clone = state.clone();
    env.set(
        "deleteChat".to_string(),
        builtin!("deleteChat", 1, move |args| {
            if let [Value::String(chat)] = &args[..] {
                let mut state = state_clone.lock().unwrap();
                let members = state.members(chat)?;
                state.delete_chat(chat)?;
                let sender = state
                    .current_user
                    .clone()
                    .ok_or(ChatError::new("Not logged in", 1))?;
                let control = P2pControl::DeleteChat { name: chat.clone() };
                for uid in members {
                    if uid != sender {
                        if let Some(addr) = state.get_contact(&uid) {
                            let _ = p2p::send_control(&addr, control.clone(), &sender);
                        }
                    }
                }
                Ok(Value::Unit)
            } else {
                Err(ChatError::new("deleteChat expects a String (chat name)", 1))
            }
        }),
    );

    let state_clone = state.clone();
    env.set(
        "open".to_string(),
        builtin!("open", 1, move |args| {
            if let [Value::String(chat)] = &args[..] {
                state_clone.lock().unwrap().open_chat(chat.clone())?;
                Ok(Value::Unit)
            } else {
                Err(ChatError::new("open expects a String (chat name)", 1))
            }
        }),
    );

    let state_clone = state.clone();
    env.set(
        "listChats".to_string(),
        builtin!("listChats", 0, move |_args| {
            let state = state_clone.lock().unwrap();
            let chats = state.list_chats();
            Ok(Value::List(chats.into_iter().map(Value::String).collect()))
        }),
    );

    let state_clone = state.clone();
    env.set(
        "members".to_string(),
        builtin!("members", 1, move |args| {
            if let [Value::String(chat)] = &args[..] {
                let state = state_clone.lock().unwrap();
                let members = state.members(chat)?;
                Ok(Value::List(members.into_iter().map(Value::Uid).collect()))
            } else {
                Err(ChatError::new("members expects a String (chat name)", 1))
            }
        }),
    );

    let state_clone = state.clone();
    env.set(
        "send".to_string(),
        builtin!("send", 2, move |args| {
            if let [Value::Uid(target), Value::String(text)] = &args[..] {
                let (current_user, target_addr) = {
                    let state = state_clone.lock().unwrap();
                    let current = state
                        .current_user
                        .clone()
                        .ok_or(ChatError::new("Not logged in", 1))?;
                    let addr = state
                        .get_contact(target)
                        .ok_or_else(|| ChatError::new(&format!("Target '{}' not in contacts", target), 1))?;
                    (current, addr)
                };
                {
                    let mut state = state_clone.lock().unwrap();
                    state.send_message(target, text)?;
                }
                let msg = P2pMessage {
                    msg_type: "msg".to_string(),
                    from: current_user.clone(),
                    to: target.clone(),
                    text: Some(text.clone()),
                    chat: None,
                    filename: None,
                    data_base64: None,
                    timestamp: Some(chrono::Utc::now().timestamp()),
                    control: None,
                };
                if let Err(e) = p2p::send_message_to(&target_addr, msg) {
                    eprintln!("P2P send error: {}", e);
                    return Err(ChatError::new(&format!("P2P delivery failed: {}", e), 1));
                }
                Ok(Value::Bool(true))
            } else {
                Err(ChatError::new("send expects a Uid and a String (message text)", 1))
            }
        }),
    );

    let state_clone = state.clone();
    env.set(
        "sendFile".to_string(),
        builtin!("sendFile", 2, move |args| {
            if let [Value::Uid(target), Value::String(path)] = &args[..] {
                let bytes = std::fs::read(&path)
                    .map_err(|e| ChatError::new(&format!("sendFile: {}", e), 1))?;
                let b64 = STANDARD.encode(&bytes);
                let filename = std::path::Path::new(&path)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let (current_user, target_addr) = {
                    let state = state_clone.lock().unwrap();
                    let current = state
                        .current_user
                        .clone()
                        .ok_or(ChatError::new("Not logged in", 1))?;
                    let addr = state
                        .get_contact(target)
                        .ok_or_else(|| ChatError::new(&format!("Target '{}' not in contacts", target), 1))?;
                    (current, addr)
                };
                let msg = P2pMessage {
                    msg_type: "file".to_string(),
                    from: current_user,
                    to: target.clone(),
                    text: None,
                    chat: None,
                    filename: Some(filename.clone()),
                    data_base64: Some(b64),
                    timestamp: Some(chrono::Utc::now().timestamp()),
                    control: None,
                };
                if let Err(e) = p2p::send_message_to(&target_addr, msg) {
                    eprintln!("P2P send file error: {}", e);
                    return Err(ChatError::new(&format!("P2P delivery failed: {}", e), 1));
                }
                Ok(Value::Bool(true))
            } else {
                Err(ChatError::new("sendFile expects a Uid and a String (file path)", 1))
            }
        }),
    );

    let state_clone = state.clone();
    env.set(
        "sendChat".to_string(),
        builtin!("sendChat", 2, move |args| {
            if let [Value::String(chat), Value::String(text)] = &args[..] {
                let mut state = state_clone.lock().unwrap();
                state.send_to_chat(chat, text)?;
                let members = state.members(chat)?;
                let sender = state
                    .current_user
                    .clone()
                    .ok_or(ChatError::new("Not logged in", 1))?;
                let timestamp = chrono::Utc::now().timestamp();
                let control = P2pControl::ChatMessage {
                    chat: chat.clone(),
                    from: sender.clone(),
                    text: text.clone(),
                    timestamp,
                };
                for uid in members {
                    if uid != sender {
                        if let Some(addr) = state.get_contact(&uid) {
                            let _ = p2p::send_control(&addr, control.clone(), &sender);
                        }
                        if text.contains(&format!("@{}", uid)) {
                            state.deliver_to_user(
                                uid.clone(),
                                crate::chat::Message {
                                    from: sender.clone(),
                                    text: text.clone(),
                                    chat: chat.clone(),
                                    timestamp,
                                },
                            );
                        }
                    }
                }
                Ok(Value::Bool(true))
            } else {
                Err(ChatError::new(
                    "sendChat expects a String (chat name) and a String (message text)",
                    1,
                ))
            }
        }),
    );

    let state_clone = state.clone();
    env.set(
        "sendFileToChat".to_string(),
        builtin!("sendFileToChat", 2, move |args| {
            if let [Value::String(chat), Value::String(path)] = &args[..] {
                let bytes = std::fs::read(&path)
                    .map_err(|e| ChatError::new(&format!("sendFileToChat: {}", e), 1))?;
                let b64 = STANDARD.encode(&bytes);
                let filename = std::path::Path::new(&path)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let state = state_clone.lock().unwrap();
                let members = state.members(chat)?;
                let sender = state
                    .current_user
                    .clone()
                    .ok_or(ChatError::new("Not logged in", 1))?;
                let msg = P2pMessage {
                    msg_type: "file".to_string(),
                    from: sender.clone(),
                    to: "".to_string(),
                    text: None,
                    chat: Some(chat.clone()),
                    filename: Some(filename.clone()),
                    data_base64: Some(b64),
                    timestamp: Some(chrono::Utc::now().timestamp()),
                    control: None,
                };
                for uid in members {
                    if uid != sender {
                        if let Some(addr) = state.get_contact(&uid) {
                            let _ = p2p::send_message_to(&addr, msg.clone());
                        }
                    }
                }
                Ok(Value::Bool(true))
            } else {
                Err(ChatError::new(
                    "sendFileToChat expects a String (chat name) and a String (file path)",
                    1,
                ))
            }
        }),
    );

    let state_clone = state.clone();
    env.set(
        "inbox".to_string(),
        builtin!("inbox", 0, move |_args| {
            let state = state_clone.lock().unwrap();
            let user = state
                .current_user
                .clone()
                .ok_or(ChatError::new("Not logged in", 1))?;
            let msgs = state.get_inbox(&user);
            Ok(Value::List(msgs))
        }),
    );

    let state_clone = state.clone();
    env.set(
        "history".to_string(),
        builtin!("history", 1, move |args| {
            if let [Value::String(chat)] = &args[..] {
                let msgs = state_clone.lock().unwrap().get_history(chat);
                Ok(Value::List(msgs))
            } else {
                Err(ChatError::new("history expects a String (chat name)", 1))
            }
        }),
    );

    let state_clone = state.clone();
    env.set(
        "downloads".to_string(),
        builtin!("downloads", 0, move |_args| {
            let state = state_clone.lock().unwrap();
            let downloads = state.downloads.clone();
            Ok(Value::List(downloads))
        }),
    );

    let state_clone = state.clone();
    env.set(
        "saveFile".to_string(),
        builtin!("saveFile", 2, move |args| {
            if let [Value::Num(Number::Int(index)), Value::String(path)] = &args[..] {
                let mut state = state_clone.lock().unwrap();
                state.save_file(*index as usize, path)?;
                Ok(Value::Bool(true))
            } else {
                Err(ChatError::new(
                    "saveFile expects an Int (Num) index and a String path",
                    1,
                ))
            }
        }),
    );

    let state_clone = state.clone();
    env.set(
        "serverStart".to_string(),
        builtin!("serverStart", 2, move |args| {
            let (addr, password) = match &args[..] {
                [Value::String(addr), Value::String(pass)] => (addr.clone(), Some(pass.clone())),
                [Value::String(addr)] => (addr.clone(), None),
                _ => {
                    return Err(ChatError::new(
                        "serverStart expects a String address and optional String password",
                        1,
                    ))
                }
            };
            let mut state = state_clone.lock().unwrap();
            if state.server_handle.is_some() {
                return Ok(Value::Unit);
            }
            match server::start_contacts_server(&addr, password) {
                Ok((_db, handle, stop_tx)) => {
                    state.server_handle = Some(handle);
                    state.server_stop = Some(stop_tx);
                    state.contact_server_addr = Some(addr);
                    Ok(Value::Unit)
                }
                Err(e) => Err(ChatError::new(&format!("Failed to start server: {}", e), 1)),
            }
        }),
    );

    let state_clone = state.clone();
    env.set(
        "serverStop".to_string(),
        builtin!("serverStop", 0, move |_args| {
            let mut state = state_clone.lock().unwrap();
            if let Some(stop_tx) = state.server_stop.take() {
                let _ = stop_tx.send(());
                if let Some(handle) = state.server_handle.take() {
                    let _ = handle.join();
                }
                state.contact_server_addr = None;
            }
            Ok(Value::Unit)
        }),
    );

    let state_clone = state.clone();
    env.set(
        "connect".to_string(),
        builtin!("connect", 3, move |args| {
            let (host, uid, password) = match &args[..] {
                [Value::String(host), Value::Uid(uid), Value::String(pass)] => {
                    (host.clone(), uid.clone(), Some(pass.clone()))
                }
                [Value::String(host), Value::Uid(uid)] => (host.clone(), uid.clone(), None),
                _ => {
                    return Err(ChatError::new(
                        "connect expects a String host, a Uid, and optional String password",
                        1,
                    ))
                }
            };
            use std::io::{BufRead, Write};
            let stream = std::net::TcpStream::connect(&host)
                .map_err(|e| ChatError::new(&format!("connect: {}", e), 1))?;
            let connector = p2p::get_tls_connector();
            let mut stream = connector
                .connect("localhost", stream)
                .map_err(|e| ChatError::new(&format!("connect: {}", e), 1))?;
            if let Some(pass) = password {
                writeln!(stream, "PASSWORD {}", pass)
                    .map_err(|e| ChatError::new(&format!("connect: {}", e), 1))?;
                let mut response = String::new();
                let mut reader = std::io::BufReader::new(&mut stream);
                reader
                    .read_line(&mut response)
                    .map_err(|e| ChatError::new(&format!("connect: {}", e), 1))?;
                if !response.trim().starts_with("OK") {
                    return Err(ChatError::new("Authentication failed", 1));
                }
            }
            let p2p_port = state_clone.lock().unwrap().p2p_port;
            let own_addr = {
                let state = state_clone.lock().unwrap();
                if let Some(ip) = &state.external_ip {
                    format!("{}:{}", ip, p2p_port)
                } else {
                    format!("127.0.0.1:{}", p2p_port)
                }
            };
            writeln!(stream, "REGISTER {} {}", uid, own_addr)
                .map_err(|e| ChatError::new(&format!("connect: {}", e), 1))?;
            writeln!(stream, "GET").map_err(|e| ChatError::new(&format!("connect: {}", e), 1))?;
            let mut response = String::new();
            let mut reader = std::io::BufReader::new(stream);
            while let Ok(n) = reader.read_line(&mut response) {
                if n == 0 {
                    break;
                }
            }
            let mut state = state_clone.lock().unwrap();
            for line in response.lines() {
                let parts: Vec<&str> = line.splitn(2, ' ').collect();
                if parts.len() == 2 {
                    state
                        .contacts
                        .insert(parts[0].to_string(), parts[1].to_string());
                }
            }
            let count = state.contacts.len() as i64;
            Ok(Value::Num(Number::Int(count)))
        }),
    );

    let state_clone = state.clone();
    env.set(
        "addContact".to_string(),
        builtin!("addContact", 2, move |args| {
            if let [Value::Uid(uid), Value::String(addr)] = &args[..] {
                state_clone.lock().unwrap().add_contact(uid.clone(), addr.clone());
                Ok(Value::Unit)
            } else {
                Err(ChatError::new("addContact expects a Uid and a String address", 1))
            }
        }),
    );

    let state_clone = state.clone();
    env.set(
        "removeContact".to_string(),
        builtin!("removeContact", 1, move |args| {
            if let [Value::Uid(uid)] = &args[..] {
                state_clone.lock().unwrap().remove_contact(uid);
                Ok(Value::Unit)
            } else {
                Err(ChatError::new("removeContact expects a Uid argument", 1))
            }
        }),
    );

    env.set(
        "getPublicIP".to_string(),
        builtin!("getPublicIP", 0, |_args| {
            match get("https://api.ipify.org") {
                Ok(resp) => {
                    if resp.status().is_success() {
                        if let Ok(ip) = resp.text() {
                            return Ok(Value::Maybe(Some(Box::new(Value::String(
                                ip.trim().to_string(),
                            )))));
                        }
                    }
                    Ok(Value::Maybe(None))
                }
                Err(_) => Ok(Value::Maybe(None)),
            }
        }),
    );

    let state_ip = state.clone();
    env.set(
        "setExternalIP".to_string(),
        builtin!("setExternalIP", 1, move |args| {
            if let [Value::String(ip)] = &args[..] {
                let mut state = state_ip.lock().unwrap();
                state.external_ip = Some(ip.clone());
                Ok(Value::Unit)
            } else {
                Err(ChatError::new("setExternalIP expects a String IP address", 1))
            }
        }),
    );

    let state_spawn = state.clone();
    env.set(
        "spawn".to_string(),
        builtin!("spawn", 1, move |args| {
            if let [closure] = &args[..] {
                spawn_process(closure.clone(), Arc::clone(&state_spawn))
            } else {
                Err(ChatError::new("spawn expects a function", 1))
            }
        }),
    );

    env.set(
        "procSelf".to_string(),
        builtin!("procSelf", 0, |_args| proc_self()),
    );

    env.set(
        "procSend".to_string(),
        builtin!("procSend", 2, |args| {
            if let [Value::Pid(pid), val] = &args[..] {
                proc_send(*pid, val.clone())
            } else {
                Err(ChatError::new("procSend expects a Pid and a value", 1))
            }
        }),
    );

    env.set(
        "procRecv".to_string(),
        builtin!("procRecv", 0, |_args| proc_recv()),
    );

    env.set(
        "procWait".to_string(),
        builtin!("procWait", 1, |args| {
            if let [Value::Pid(pid)] = &args[..] {
                proc_wait(*pid)
            } else {
                Err(ChatError::new("procWait expects a Pid argument", 1))
            }
        }),
    );

    env.set(
        "procExit".to_string(),
        builtin!("procExit", 1, |args| {
            if let [val] = &args[..] {
                proc_exit(val.clone())
            } else {
                Err(ChatError::new("procExit expects a value", 1))
            }
        }),
    );

    env.set(
        "sleep".to_string(),
        builtin!("sleep", 1, |args| {
            if let [Value::Duration(dur)] = &args[..] {
                sleep(*dur)
            } else {
                Err(ChatError::new("sleep expects a Duration argument", 1))
            }
        }),
    );

    let state_after = state.clone();
    env.set(
        "after".to_string(),
        builtin!("after", 2, move |args| {
            if let [Value::Duration(dur), closure] = &args[..] {
                after(*dur, closure.clone(), Arc::clone(&state_after))
            } else {
                Err(ChatError::new(
                    "after expects a Duration and a nullary function",
                    1,
                ))
            }
        }),
    );

    env.set("Nothing".to_string(), Value::Maybe(None));
    env.set(
        "Just".to_string(),
        builtin!("Just", 1, |args| {
            if args.len() == 1 {
                Ok(Value::Maybe(Some(Box::new(args[0].clone()))))
            } else {
                Err(ChatError::new("Just expects exactly one argument", 1))
            }
        }),
    );

    let state_maybe = state.clone();
    env.set(
        "maybe".to_string(),
        builtin!("maybe", 3, move |args| {
            if let [Value::Closure(params, body, env), default, Value::Maybe(maybe_val)] = &args[..] {
                let val = match maybe_val {
                    Some(v) => *v.clone(),
                    None => default.clone(),
                };
                let mut new_env = env.clone();
                new_env.set(params[0].clone(), val);
                eval::eval_expr(&body, &mut new_env, Arc::clone(&state_maybe))
            } else {
                Err(ChatError::new(
                    "maybe expects a function, a default value, and a Maybe",
                    1,
                ))
            }
        }),
    );

    env.set(
        "mapGet".to_string(),
        builtin!("mapGet", 2, |args| {
            if let [Value::Map(map), key] = &args[..] {
                let key_str = key.display();
                Ok(map.get(&key_str).cloned().unwrap_or(Value::Unit))
            } else {
                Err(ChatError::new("mapGet expects a Map and a key", 1))
            }
        }),
    );

    env.set(
        "mapSet".to_string(),
        builtin!("mapSet", 3, |args| {
            if let [Value::Map(map), key, val] = &args[..] {
                let mut new_map = map.clone();
                new_map.insert(key.display(), val.clone());
                Ok(Value::Map(new_map))
            } else {
                Err(ChatError::new("mapSet expects a Map, a key, and a value", 1))
            }
        }),
    );

    env.set(
        "mapRemove".to_string(),
        builtin!("mapRemove", 2, |args| {
            if let [Value::Map(map), key] = &args[..] {
                let mut new_map = map.clone();
                new_map.remove(&key.display());
                Ok(Value::Map(new_map))
            } else {
                Err(ChatError::new("mapRemove expects a Map and a key", 1))
            }
        }),
    );

    env.set(
        "mapKeys".to_string(),
        builtin!("mapKeys", 1, |args| {
            if let [Value::Map(map)] = &args[..] {
                let keys: Vec<Value> = map.keys().map(|s| Value::String(s.clone())).collect();
                Ok(Value::List(keys))
            } else {
                Err(ChatError::new("mapKeys expects a Map argument", 1))
            }
        }),
    );

    env.set(
        "mapValues".to_string(),
        builtin!("mapValues", 1, |args| {
            if let [Value::Map(map)] = &args[..] {
                let values: Vec<Value> = map.values().cloned().collect();
                Ok(Value::List(values))
            } else {
                Err(ChatError::new("mapValues expects a Map argument", 1))
            }
        }),
    );

    env.set(
        "mapEntries".to_string(),
        builtin!("mapEntries", 1, |args| {
            if let [Value::Map(map)] = &args[..] {
                let entries: Vec<Value> = map
                    .iter()
                    .map(|(k, v)| Value::Tuple(vec![Value::String(k.clone()), v.clone()]))
                    .collect();
                Ok(Value::List(entries))
            } else {
                Err(ChatError::new("mapEntries expects a Map argument", 1))
            }
        }),
    );

    env.set(
        "mapContains".to_string(),
        builtin!("mapContains", 2, |args| {
            if let [Value::Map(map), key] = &args[..] {
                Ok(Value::Bool(map.contains_key(&key.display())))
            } else {
                Err(ChatError::new("mapContains expects a Map and a key", 1))
            }
        }),
    );

    env.set(
        "mapSize".to_string(),
        builtin!("mapSize", 1, |args| {
            if let [Value::Map(map)] = &args[..] {
                Ok(Value::Num(Number::Int(map.len() as i64)))
            } else {
                Err(ChatError::new("mapSize expects a Map argument", 1))
            }
        }),
    );

    let state_mf = state.clone();
    env.set(
        "mapFilter".to_string(),
        Value::BuiltinFunc(
            "mapFilter".to_string(),
            2,
            Arc::new(move |args| {
                if let [Value::Closure(params, body, closure_env), Value::Map(map)] = &args[..] {
                    let mut new_map = BTreeMap::new();
                    let state_ref = state_mf.clone();
                    for (k, v) in map {
                        let mut env = closure_env.clone();
                        env.set(params[0].clone(), Value::String(k.clone()));
                        env.set(params[1].clone(), v.clone());
                        let val = eval::eval_expr(&body, &mut env, state_ref.clone())?;
                        if let Value::Bool(b) = val {
                            if b {
                                new_map.insert(k.clone(), v.clone());
                            }
                        } else {
                            return Err(ChatError::new(
                                "mapFilter predicate must return a Bool value",
                                1,
                            ));
                        }
                    }
                    Ok(Value::Map(new_map))
                } else {
                    Err(ChatError::new(
                        "mapFilter expects a function (k -> v -> Bool) and a Map",
                        1,
                    ))
                }
            }),
        ),
    );

    env.set(
        "mapMerge".to_string(),
        builtin!("mapMerge", 2, |args| {
            if let [Value::Map(a), Value::Map(b)] = &args[..] {
                let mut merged = a.clone();
                for (k, v) in b {
                    merged.insert(k.clone(), v.clone());
                }
                Ok(Value::Map(merged))
            } else {
                Err(ChatError::new("mapMerge expects two Map arguments", 1))
            }
        }),
    );

    env.set(
        "setAdd".to_string(),
        builtin!("setAdd", 2, |args| {
            if let [Value::Set(set), elem] = &args[..] {
                let mut new_set = set.clone();
                new_set.insert(elem.display());
                Ok(Value::Set(new_set))
            } else {
                Err(ChatError::new("setAdd expects a Set and an element", 1))
            }
        }),
    );

    env.set(
        "setRemove".to_string(),
        builtin!("setRemove", 2, |args| {
            if let [Value::Set(set), elem] = &args[..] {
                let mut new_set = set.clone();
                new_set.remove(&elem.display());
                Ok(Value::Set(new_set))
            } else {
                Err(ChatError::new("setRemove expects a Set and an element", 1))
            }
        }),
    );

    env.set(
        "setContains".to_string(),
        builtin!("setContains", 2, |args| {
            if let [Value::Set(set), elem] = &args[..] {
                Ok(Value::Bool(set.contains(&elem.display())))
            } else {
                Err(ChatError::new("setContains expects a Set and an element", 1))
            }
        }),
    );

    env.set(
        "setUnion".to_string(),
        builtin!("setUnion", 2, |args| {
            if let [Value::Set(a), Value::Set(b)] = &args[..] {
                let union: BTreeSet<_> = a.union(b).cloned().collect();
                Ok(Value::Set(union))
            } else {
                Err(ChatError::new("setUnion expects two Set arguments", 1))
            }
        }),
    );

    env.set(
        "setIntersection".to_string(),
        builtin!("setIntersection", 2, |args| {
            if let [Value::Set(a), Value::Set(b)] = &args[..] {
                let inter: BTreeSet<_> = a.intersection(b).cloned().collect();
                Ok(Value::Set(inter))
            } else {
                Err(ChatError::new("setIntersection expects two Set arguments", 1))
            }
        }),
    );

    env.set(
        "setDifference".to_string(),
        builtin!("setDifference", 2, |args| {
            if let [Value::Set(a), Value::Set(b)] = &args[..] {
                let diff: BTreeSet<_> = a.difference(b).cloned().collect();
                Ok(Value::Set(diff))
            } else {
                Err(ChatError::new("setDifference expects two Set arguments", 1))
            }
        }),
    );

    env.set(
        "setSize".to_string(),
        builtin!("setSize", 1, |args| {
            if let [Value::Set(set)] = &args[..] {
                Ok(Value::Num(Number::Int(set.len() as i64)))
            } else {
                Err(ChatError::new("setSize expects a Set argument", 1))
            }
        }),
    );

    let state_sf = state.clone();
    env.set(
        "setFilter".to_string(),
        Value::BuiltinFunc(
            "setFilter".to_string(),
            2,
            Arc::new(move |args| {
                if let [Value::Closure(params, body, closure_env), Value::Set(set)] = &args[..] {
                    let mut new_set = BTreeSet::new();
                    let state_ref = state_sf.clone();
                    for item in set {
                        let mut env = closure_env.clone();
                        env.set(params[0].clone(), Value::String(item.clone()));
                        let val = eval::eval_expr(&body, &mut env, state_ref.clone())?;
                        if let Value::Bool(b) = val {
                            if b {
                                new_set.insert(item.clone());
                            }
                        } else {
                            return Err(ChatError::new(
                                "setFilter predicate must return a Bool value",
                                1,
                            ));
                        }
                    }
                    Ok(Value::Set(new_set))
                } else {
                    Err(ChatError::new(
                        "setFilter expects a function (a -> Bool) and a Set",
                        1,
                    ))
                }
            }),
        ),
    );

    let state_sm = state.clone();
    env.set(
        "setMap".to_string(),
        Value::BuiltinFunc(
            "setMap".to_string(),
            2,
            Arc::new(move |args| {
                if let [Value::Closure(params, body, closure_env), Value::Set(set)] = &args[..] {
                    let mut new_set = BTreeSet::new();
                    let state_ref = state_sm.clone();
                    for item in set {
                        let mut env = closure_env.clone();
                        env.set(params[0].clone(), Value::String(item.clone()));
                        let val = eval::eval_expr(&body, &mut env, state_ref.clone())?;
                        new_set.insert(val.display());
                    }
                    Ok(Value::Set(new_set))
                } else {
                    Err(ChatError::new(
                        "setMap expects a function (a -> b) and a Set",
                        1,
                    ))
                }
            }),
        ),
    );

    env.set(
        "listToSet".to_string(),
        builtin!("listToSet", 1, |args| {
            if let [Value::List(list)] = &args[..] {
                let set: BTreeSet<_> = list.iter().map(|v| v.display()).collect();
                Ok(Value::Set(set))
            } else {
                Err(ChatError::new("listToSet expects a List argument", 1))
            }
        }),
    );

    env.set(
        "mapToList".to_string(),
        builtin!("mapToList", 1, |args| {
            if let [Value::Map(map)] = &args[..] {
                let list: Vec<Value> = map
                    .iter()
                    .map(|(k, v)| Value::Tuple(vec![Value::String(k.clone()), v.clone()]))
                    .collect();
                Ok(Value::List(list))
            } else {
                Err(ChatError::new("mapToList expects a Map argument", 1))
            }
        }),
    );

    env.set(
        "sha256".to_string(),
        builtin!("sha256", 1, |args| {
            if let [Value::ByteString(data)] = &args[..] {
                let hash = Sha256::digest(data);
                Ok(Value::ByteString(hash.to_vec()))
            } else {
                Err(ChatError::new("sha256 expects a ByteString argument", 1))
            }
        }),
    );

    env.set(
        "sha256String".to_string(),
        builtin!("sha256String", 1, |args| {
            if let [Value::String(s)] = &args[..] {
                let hash = Sha256::digest(s.as_bytes());
                Ok(Value::String(hex::encode(hash)))
            } else {
                Err(ChatError::new("sha256String expects a String argument", 1))
            }
        }),
    );

    env.set(
        "kyberKeyPair".to_string(),
        builtin!("kyberKeyPair", 0, |_args| {
            let (pk, sk) = keypair();
            Ok(Value::Tuple(vec![
                Value::ByteString(pk.as_bytes().to_vec()),
                Value::ByteString(sk.as_bytes().to_vec()),
            ]))
        }),
    );

    env.set(
        "kyberEncapsulate".to_string(),
        builtin!("kyberEncapsulate", 1, |args| {
            if let [Value::ByteString(pk_bytes)] = &args[..] {
                let pk = pqcrypto_kyber::kyber768::PublicKey::from_bytes(pk_bytes)
                    .map_err(|_| ChatError::new("Invalid Kyber public key", 1))?;
                let (ciphertext, shared_secret) = encapsulate(&pk);
                Ok(Value::Tuple(vec![
                    Value::ByteString(ciphertext.as_bytes().to_vec()),
                    Value::ByteString(shared_secret.as_bytes().to_vec()),
                ]))
            } else {
                Err(ChatError::new(
                    "kyberEncapsulate expects a ByteString (public key)",
                    1,
                ))
            }
        }),
    );

    env.set(
        "kyberDecapsulate".to_string(),
        builtin!("kyberDecapsulate", 2, |args| {
            if let [Value::ByteString(sk_bytes), Value::ByteString(ciphertext_bytes)] = &args[..] {
                let sk = pqcrypto_kyber::kyber768::SecretKey::from_bytes(sk_bytes)
                    .map_err(|_| ChatError::new("Invalid Kyber secret key", 1))?;
                let ciphertext = pqcrypto_kyber::kyber768::Ciphertext::from_bytes(ciphertext_bytes)
                    .map_err(|_| ChatError::new("Invalid Kyber ciphertext", 1))?;
                let shared_secret = decapsulate(&ciphertext, &sk);
                Ok(Value::ByteString(shared_secret.as_bytes().to_vec()))
            } else {
                Err(ChatError::new(
                    "kyberDecapsulate expects SecretKey and Ciphertext as ByteStrings",
                    1,
                ))
            }
        }),
    );

    env.set(
        "exit".to_string(),
        builtin!("exit", 0, |_args| {
            std::process::exit(0);
        }),
    );

    let state_load = state.clone();
    let global_env_load = global_env.clone();
    env.set(
        "load".to_string(),
        builtin!("load", 1, move |args| {
            if let [Value::String(path)] = &args[..] {
                let content = match fs::read_to_string(&path) {
                    Ok(c) => c,
                    Err(e) => return Err(ChatError::new(&format!("Failed to read file: {}", e), 1)),
                };
                let expr = match crate::parser::parse_script(&content) {
                    Ok(e) => e,
                    Err(e) => return Err(ChatError::new(&format!("Parse error: {}", e), 1)),
                };
                let mut env_guard = global_env_load.lock().unwrap();
                match crate::eval::eval_expr(&expr, &mut env_guard, state_load.clone()) {
                    Ok(_) => Ok(Value::Unit),
                    Err(e) => Err(ChatError::new(&format!("Execution error: {}", e), 1)),
                }
            } else {
                Err(ChatError::new("load expects a String (file path)", 1))
            }
        }),
    );

    let global_env_del = global_env.clone();
    env.set(
        "del".to_string(),
        builtin!("del", 1, move |args| {
            if let [Value::String(name)] = &args[..] {
                let mut env_guard = global_env_del.lock().unwrap();
                if env_guard.vars.remove(name).is_some() {
                    env_guard.type_map.remove(name);
                    Ok(Value::Unit)
                } else {
                    Err(ChatError::new(&format!("Variable '{}' not found", name), 1))
                }
            } else {
                Err(ChatError::new("del expects a String (variable name)", 1))
            }
        }),
    );

    let state_for_p2p_port = state.clone();
    env.set(
        "p2pPort".to_string(),
        builtin!("p2pPort", 0, move |_args| {
            let port = state_for_p2p_port.lock().unwrap().p2p_port;
            Ok(Value::Num(Number::Int(port as i64)))
        }),
    );

    {
        let mut state_guard = state.lock().unwrap();
        if state_guard.p2p_port == 0 {
            state_guard.p2p_port = 19000;
            drop(state_guard);
            p2p::start_p2p_listener(19000, state.clone());
        }
    }
}

fn serde_json_to_chatlang(v: SerdeValue) -> JsonValue {
    match v {
        SerdeValue::Null => JsonValue::Null,
        SerdeValue::Bool(b) => JsonValue::Bool(b),
        SerdeValue::Number(n) => JsonValue::Number(n.as_f64().unwrap_or(0.0)),
        SerdeValue::String(s) => JsonValue::String(s),
        SerdeValue::Array(arr) => {
            let items: Vec<JsonValue> = arr.into_iter().map(serde_json_to_chatlang).collect();
            JsonValue::Array(items)
        }
        SerdeValue::Object(map) => {
            let mut obj = BTreeMap::new();
            for (k, v) in map {
                obj.insert(k, serde_json_to_chatlang(v));
            }
            JsonValue::Object(obj)
        }
    }
}

fn chatlang_json_to_serde(j: &JsonValue) -> SerdeValue {
    match j {
        JsonValue::Null => SerdeValue::Null,
        JsonValue::Bool(b) => SerdeValue::Bool(*b),
        JsonValue::Number(n) => {
            SerdeValue::Number(serde_json::Number::from_f64(*n).unwrap_or(serde_json::Number::from(0)))
        }
        JsonValue::String(s) => SerdeValue::String(s.clone()),
        JsonValue::Array(arr) => {
            let items: Vec<SerdeValue> = arr.iter().map(chatlang_json_to_serde).collect();
            SerdeValue::Array(items)
        }
        JsonValue::Object(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                obj.insert(k.clone(), chatlang_json_to_serde(v));
            }
            SerdeValue::Object(obj)
        }
    }
}
