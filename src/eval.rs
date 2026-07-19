//! Evaluator with currying, Num unification, loops, break, and list comprehensions.

use crate::ast::*;
use crate::chat::ChatState;
use crate::error::ChatError;
use crate::types::{Environment, Value, Number};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::thread;
use std::sync::mpsc::{self, Sender, Receiver};
use once_cell::sync::Lazy;
use std::time::Duration;

pub struct Process {
    pub sender: Sender<Value>,
    pub exit_sender: Sender<Value>,
    pub exit_receiver: Receiver<Value>,
    pub thread: Option<thread::JoinHandle<()>>,
}

pub static PROCESS_MANAGER: Lazy<Mutex<BTreeMap<usize, Process>>> = Lazy::new(|| Mutex::new(BTreeMap::new()));
pub static NEXT_PID: Lazy<Mutex<usize>> = Lazy::new(|| Mutex::new(1));

thread_local! {
    static CURRENT_PID: std::cell::RefCell<Option<usize>> = const { std::cell::RefCell::new(None) };
    static CURRENT_RECEIVER: std::cell::RefCell<Option<Receiver<Value>>> = const { std::cell::RefCell::new(None) };
}

pub fn eval_expr(
    expr: &Expr,
    env: &mut Environment,
    state: Arc<Mutex<ChatState>>,
) -> Result<Value, ChatError> {
    match expr {
        Expr::Lit(lit, _) => eval_literal(lit),
        Expr::Var(name, span) => {
            let val = env.get(name).ok_or_else(|| ChatError::with_span(&format!("Undefined variable '{}'", name), 1, *span))?;
            if let Value::BuiltinFunc(_, 0, f) = val {
                f(vec![])
            } else {
                Ok(val)
            }
        }
        Expr::Lambda(params, body, _) => Ok(Value::Closure(params.clone(), *body.clone(), env.clone())),
        Expr::App(func, arg, span) => {
            let f = eval_expr(func, env, Arc::clone(&state))?;
            let a = eval_expr(arg, env, Arc::clone(&state))?;
            apply(f, a, env, state, *span)
        }
        Expr::If(cond, then_expr, else_expr, span) => {
            let c = eval_expr(cond, env, Arc::clone(&state))?;
            match c {
                Value::Bool(true) => eval_expr(then_expr, env, Arc::clone(&state)),
                Value::Bool(false) => eval_expr(else_expr, env, Arc::clone(&state)),
                _ => Err(ChatError::with_span("Condition must be Bool", 1, *span)),
            }
        }
        Expr::Let { name, type_ann, def, body, span } => {
            if env.vars.contains_key(name) {
                return Err(ChatError::with_span(&format!("variable '{}' already defined", name), 1, *span));
            }
            let val = eval_expr(def, env, Arc::clone(&state))?;
            if let Some(ann) = type_ann {
                let actual_type = type_name(&val);
                if *ann != actual_type {
                    return Err(ChatError::with_span(&format!("Type mismatch: expected '{}', got '{}'", ann, actual_type), 1, *span));
                }
                env.type_map.insert(name.clone(), ann.clone());
            }
            env.set(name.clone(), val.clone());
            match body {
                Some(b) => eval_expr(b, env, Arc::clone(&state)),
                None => Ok(val),
            }
        }
        Expr::Assign(name, expr, span) => {
            let val = eval_expr(expr, env, Arc::clone(&state))?;
            if !env.vars.contains_key(name) {
                return Err(ChatError::with_span(&format!("Variable '{}' not defined for assignment", name), 1, *span));
            }
            if let Some(expected_type) = env.type_map.get(name) {
                let actual_type = type_name(&val);
                if expected_type != &actual_type {
                    return Err(ChatError::with_span(&format!("Type mismatch: expected '{}', got '{}'", expected_type, actual_type), 1, *span));
                }
            }
            env.set(name.clone(), val.clone());
            Ok(val)
        }
        Expr::Case(scrut, arms, span) => {
            let val = eval_expr(scrut, env, Arc::clone(&state))?;
            for (pat, expr) in arms {
                if let Some(bindings) = match_pattern(pat, &val) {
                    let mut new_env = env.clone();
                    for (k, v) in bindings {
                        new_env.set(k, v);
                    }
                    return eval_expr(expr, &mut new_env, Arc::clone(&state));
                }
            }
            Err(ChatError::with_span("Non-exhaustive patterns", 1, *span))
        }
        Expr::Try(body, _span) => {
            match eval_expr(body, env, Arc::clone(&state)) {
                Ok(v) => Ok(v),
                Err(e) => Err(e),
            }
        }
        Expr::Catch(body, pat, handler, _span) => {
            match eval_expr(body, env, Arc::clone(&state)) {
                Ok(v) => Ok(v),
                Err(err) => {
                    let err_val = Value::String(err.message.clone());
                    if let Some(bindings) = match_pattern(pat, &err_val) {
                        let mut new_env = env.clone();
                        for (k, v) in bindings {
                            new_env.set(k, v);
                        }
                        eval_expr(handler, &mut new_env, Arc::clone(&state))
                    } else {
                        Err(err)
                    }
                }
            }
        }
        Expr::Throw(msg, span) => {
            let msg_val = eval_expr(msg, env, Arc::clone(&state))?;
            Err(ChatError::with_span(&msg_val.display(), 1, *span))
        }
        Expr::DataDef(_, _, _, _) => Ok(Value::Unit),
        Expr::StructDef(_, _, _) => Ok(Value::Unit),
        Expr::StructNew(_name, fields, _span) => {
            let mut map = BTreeMap::new();
            for (f, e) in fields {
                map.insert(f.clone(), eval_expr(e, env, Arc::clone(&state))?);
            }
            Ok(Value::Record(map))
        }
        Expr::Constructor(_name, args, _span) => {
            let mut vals = Vec::new();
            for arg in args {
                vals.push(eval_expr(arg, env, Arc::clone(&state))?);
            }
            Ok(Value::Custom(_name.clone(), vals))
        }
        Expr::Record(fields, _span) => {
            let mut map = BTreeMap::new();
            for (k, v) in fields {
                map.insert(k.clone(), eval_expr(v, env, Arc::clone(&state))?);
            }
            Ok(Value::Record(map))
        }
        Expr::FieldAccess(expr, field, span) => {
            let val = eval_expr(expr, env, Arc::clone(&state))?;
            match val {
                Value::Record(map) => {
                    map.get(field).cloned().ok_or_else(|| ChatError::with_span(&format!("Field '{}' not found", field), 1, *span))
                }
                Value::ClassInstance { fields, .. } => {
                    fields.get(field).cloned().ok_or_else(|| ChatError::with_span(&format!("Field '{}' not found", field), 1, *span))
                }
                _ => Err(ChatError::with_span("Field access on non-record or non-class", 1, *span)),
            }
        }
        Expr::RecordUpdate(expr, updates, span) => {
            let val = eval_expr(expr, env, Arc::clone(&state))?;
            match val {
                Value::Record(mut map) => {
                    for (k, v) in updates {
                        map.insert(k.clone(), eval_expr(v, env, Arc::clone(&state))?);
                    }
                    Ok(Value::Record(map))
                }
                _ => Err(ChatError::with_span("Record update on non-record", 1, *span)),
            }
        }
        Expr::BinOp(op, l, r, span) => {
            let left = eval_expr(l, env, Arc::clone(&state))?;
            let right = eval_expr(r, env, Arc::clone(&state))?;
            eval_binop(op, left, right, *span)
        }
        Expr::Concat(l, r, span) => {
            let left = eval_expr(l, env, Arc::clone(&state))?;
            let right = eval_expr(r, env, Arc::clone(&state))?;
            match (left, right) {
                (Value::String(a), Value::String(b)) => Ok(Value::String(a + &b)),
                (Value::List(mut a), Value::List(b)) => { a.extend(b); Ok(Value::List(a)) },
                _ => Err(ChatError::with_span("++ works on strings or lists", 1, *span)),
            }
        }
        Expr::FString(parts, _span) => {
            let mut result = String::new();
            for part in parts {
                match part {
                    FStringPart::Literal(s) => result.push_str(&s),
                    FStringPart::Expr(e) => {
                        let val = eval_expr(e, env, Arc::clone(&state))?;
                        result.push_str(&val.display());
                    }
                }
            }
            Ok(Value::String(result))
        }
        Expr::List(elems, _span) => {
            let vals: Result<Vec<_>, _> = elems.iter().map(|e| eval_expr(e, env, Arc::clone(&state))).collect();
            Ok(Value::List(vals?))
        }
        Expr::Range(start, end, span) => {
            let s = eval_expr(start, env, Arc::clone(&state))?;
            let e = eval_expr(end, env, Arc::clone(&state))?;
            match (s, e) {
                (Value::Num(Number::Int(a)), Value::Num(Number::Int(b))) => {
                    let mut list = Vec::new();
                    for i in a..b {
                        list.push(Value::Num(Number::Int(i)));
                    }
                    Ok(Value::List(list))
                }
                _ => Err(ChatError::with_span("Range requires Ints", 1, *span)),
            }
        }
        Expr::Pipe(left, right, span) => {
            let l = eval_expr(left, env, Arc::clone(&state))?;
            apply(eval_expr(right, env, Arc::clone(&state))?, l, env, state, *span)
        }
        Expr::Dollar(func, arg, span) => {
            let f = eval_expr(func, env, Arc::clone(&state))?;
            let a = eval_expr(arg, env, Arc::clone(&state))?;
            apply(f, a, env, state, *span)
        }
        Expr::LogicalAnd(l, r, span) => {
            match eval_expr(l, env, Arc::clone(&state))? {
                Value::Bool(true) => eval_expr(r, env, Arc::clone(&state)),
                Value::Bool(false) => Ok(Value::Bool(false)),
                _ => Err(ChatError::with_span("and requires Bool", 1, *span)),
            }
        }
        Expr::LogicalOr(l, r, span) => {
            match eval_expr(l, env, Arc::clone(&state))? {
                Value::Bool(true) => Ok(Value::Bool(true)),
                Value::Bool(false) => eval_expr(r, env, Arc::clone(&state)),
                _ => Err(ChatError::with_span("or requires Bool", 1, *span)),
            }
        }
        Expr::Not(expr, span) => {
            match eval_expr(expr, env, Arc::clone(&state))? {
                Value::Bool(b) => Ok(Value::Bool(!b)),
                _ => Err(ChatError::with_span("not requires Bool", 1, *span)),
            }
        }
        Expr::Tuple(elems, _span) => {
            let vals: Result<Vec<_>, _> = elems.iter().map(|e| eval_expr(e, env, Arc::clone(&state))).collect();
            Ok(Value::Tuple(vals?))
        }
        Expr::Index(list, idx, span) => {
            let coll = eval_expr(list, env, Arc::clone(&state))?;
            let index = eval_expr(idx, env, Arc::clone(&state))?;
            // Automatic float to int conversion for indexing
            let index_int = match index {
                Value::Num(Number::Int(i)) => i,
                Value::Num(Number::Float(f)) => f as i64,
                _ => return Err(ChatError::with_span("Index must be numeric (Int or Float)", 1, *span)),
            };
            match coll {
                Value::Map(map) => {
                    let key_str = index.display();
                    if let Some(val) = map.get(&key_str) {
                        Ok(val.clone())
                    } else {
                        Ok(Value::Unit)
                    }
                }
                Value::Set(set) => {
                    Ok(Value::Bool(set.contains(&index.display())))
                }
                Value::List(v) => {
                    let i = index_int;
                    if i < 0 || i as usize >= v.len() {
                        Err(ChatError::with_span("Index out of bounds", 1, *span))
                    } else {
                        Ok(v[i as usize].clone())
                    }
                }
                Value::String(s) => {
                    let i = index_int;
                    if i < 0 || i as usize >= s.len() {
                        Err(ChatError::with_span("Index out of bounds", 1, *span))
                    } else {
                        Ok(Value::Char(s.chars().nth(i as usize).unwrap()))
                    }
                }
                Value::ByteString(b) => {
                    let i = index_int;
                    if i < 0 || i as usize >= b.len() {
                        Err(ChatError::with_span("Index out of bounds", 1, *span))
                    } else {
                        Ok(Value::Num(Number::Int(b[i as usize] as i64)))
                    }
                }
                _ => Err(ChatError::with_span("Indexing requires list, string, byte string, map, or set", 1, *span)),
            }
        }
        Expr::For(var, iterable, body, span) => {
            let iter_val = eval_expr(iterable, env, Arc::clone(&state))?;
            match iter_val {
                Value::List(list) => {
                    for item in list {
                        let mut new_env = env.clone();
                        new_env.set(var.clone(), item);
                        eval_expr(body, &mut new_env, Arc::clone(&state))?;
                    }
                    Ok(Value::Unit)
                }
                Value::Set(set) => {
                    for item in set {
                        let mut new_env = env.clone();
                        new_env.set(var.clone(), Value::String(item));
                        eval_expr(body, &mut new_env, Arc::clone(&state))?;
                    }
                    Ok(Value::Unit)
                }
                Value::Map(map) => {
                    for (k, v) in map {
                        let mut new_env = env.clone();
                        new_env.set(var.clone(), Value::Tuple(vec![Value::String(k), v]));
                        eval_expr(body, &mut new_env, Arc::clone(&state))?;
                    }
                    Ok(Value::Unit)
                }
                _ => Err(ChatError::with_span("for requires list, set, or map", 1, *span)),
            }
        }
        Expr::While(cond, body, span) => {
            loop {
                let c = eval_expr(cond, env, Arc::clone(&state))?;
                match c {
                    Value::Bool(true) => {
                        eval_expr(body, env, Arc::clone(&state))?;
                    }
                    Value::Bool(false) => break,
                    _ => return Err(ChatError::with_span("while condition must be Bool", 1, *span)),
                }
            }
            Ok(Value::Unit)
        }
        Expr::Loop(body, _span) => {
            loop {
                match eval_expr(body, env, Arc::clone(&state)) {
                    Ok(Value::Break(Some(val))) => return Ok(*val),
                    Ok(Value::Break(None)) => return Ok(Value::Unit),
                    Ok(_) => continue,
                    Err(e) => return Err(e),
                }
            }
        }
        Expr::Break(opt, _span) => {
            let val = match opt {
                Some(e) => eval_expr(e, env, Arc::clone(&state))?,
                None => Value::Unit,
            };
            Ok(Value::Break(Some(Box::new(val))))
        }
        Expr::Block(exprs, _span) => {
            let mut block_env = env.clone();
            let mut result = Value::Unit;
            for e in exprs {
                result = eval_expr(e, &mut block_env, Arc::clone(&state))?;
            }
            for (k, v) in block_env.vars {
                if !env.vars.contains_key(&k) {
                    env.set(k, v);
                }
            }
            Ok(result)
        }
        Expr::ClassDef { name, extends, fields, methods, span: _ } => {
            let mut class_info = BTreeMap::new();
            if let Some(parent) = extends {
                class_info.insert("extends".to_string(), Value::String(parent.clone()));
            } else {
                class_info.insert("extends".to_string(), Value::Unit);
            }
            let mut method_env = Environment::new();
            for m in methods {
                let closure = Value::Closure(
                    m.params.clone(),
                    *m.body.clone(),
                    env.clone(),
                );
                method_env.set(m.name.clone(), closure);
            }
            let field_names: Vec<Value> = fields.iter().map(|(n, _)| Value::String(n.clone())).collect();
            class_info.insert("fields".to_string(), Value::List(field_names));
            let mut method_map = BTreeMap::new();
            for (k, v) in method_env.vars {
                method_map.insert(k, v);
            }
            class_info.insert("methods".to_string(), Value::Record(method_map));
            env.set(name.clone(), Value::Record(class_info));
            Ok(Value::Unit)
        }
        Expr::New(class_name, args, span) => {
            let class_def = env.get(class_name)
                .ok_or_else(|| ChatError::with_span(&format!("Class '{}' not defined", class_name), 1, *span))?;
            if let Value::Record(info) = class_def {
                let fields = if let Some(Value::List(field_list)) = info.get("fields") {
                    field_list.iter().filter_map(|v| {
                        if let Value::String(s) = v { Some(s.clone()) } else { None }
                    }).collect::<Vec<String>>()
                } else {
                    Vec::new()
                };
                let mut field_values = BTreeMap::new();
                for (i, field) in fields.iter().enumerate() {
                    if i < args.len() {
                        let val = eval_expr(&args[i], env, Arc::clone(&state))?;
                        field_values.insert(field.clone(), val);
                    } else {
                        field_values.insert(field.clone(), Value::Unit);
                    }
                }
                let instance = Value::ClassInstance {
                    class: class_name.clone(),
                    fields: field_values,
                };
                Ok(instance)
            } else {
                Err(ChatError::with_span("Invalid class definition", 1, *span))
            }
        }
        Expr::MethodCall(obj, method, args, span) => {
            let obj_val = eval_expr(obj, env, Arc::clone(&state))?;
            match obj_val {
                Value::ClassInstance { ref class, .. } => {
                    let class_def = env.get(class)
                        .ok_or_else(|| ChatError::with_span(&format!("Class '{}' not found", class), 1, *span))?;
                    let method_val = find_method(&class_def, method, env)?;
                    let mut all_args = vec![obj_val.clone()];
                    for a in args {
                        all_args.push(eval_expr(a, env, Arc::clone(&state))?);
                    }
                    match method_val {
                        Value::Closure(params, body, closure_env) => {
                            let mut new_env = closure_env.clone();
                            if params.len() != all_args.len() {
                                return Err(ChatError::with_span("Wrong number of arguments", 1, *span));
                            }
                            for (p, a) in params.iter().zip(all_args) {
                                new_env.set(p.clone(), a);
                            }
                            eval_expr(&body, &mut new_env, state)
                        }
                        _ => Err(ChatError::with_span("Method is not a function", 1, *span)),
                    }
                }
                _ => Err(ChatError::with_span("Method call on non-class instance", 1, *span)),
            }
        }
        Expr::MapLiteral(entries, _span) => {
            let mut map = BTreeMap::new();
            for (k, v) in entries {
                let key = eval_expr(k, env, Arc::clone(&state))?;
                let value = eval_expr(v, env, Arc::clone(&state))?;
                map.insert(key.display(), value);
            }
            Ok(Value::Map(map))
        }
        Expr::SetLiteral(elems, _span) => {
            let mut set = BTreeSet::new();
            for e in elems {
                let val = eval_expr(e, env, Arc::clone(&state))?;
                set.insert(val.display());
            }
            Ok(Value::Set(set))
        }
        Expr::ListComp { expr, generators, filters, span: _ } => {
            let mut result = Vec::new();
            fn eval_comp(
                expr: &Expr,
                gens: &[(String, Box<Expr>)],
                filters: &[Box<Expr>],
                env: &mut Environment,
                state: Arc<Mutex<ChatState>>,
                acc: &mut Vec<Value>,
            ) -> Result<(), ChatError> {
                if gens.is_empty() {
                    // Check filters
                    for filter in filters {
                        let cond = eval_expr(filter, env, Arc::clone(&state))?;
                        if let Value::Bool(b) = cond {
                            if !b { return Ok(()); }
                        } else {
                            return Err(ChatError::with_span("Filter must be Bool", 1, e_span(filter)));
                        }
                    }
                    let val = eval_expr(expr, env, Arc::clone(&state))?;
                    acc.push(val);
                    return Ok(());
                }
                let (var, iter_expr) = &gens[0];
                let iter_val = eval_expr(iter_expr, env, Arc::clone(&state))?;
                let rest_gens = &gens[1..];
                match iter_val {
                    Value::List(list) => {
                        for item in list {
                            let mut new_env = env.clone();
                            new_env.set(var.clone(), item);
                            eval_comp(expr, rest_gens, filters, &mut new_env, Arc::clone(&state), acc)?;
                        }
                    }
                    Value::Set(set) => {
                        for item in set {
                            let mut new_env = env.clone();
                            new_env.set(var.clone(), Value::String(item));
                            eval_comp(expr, rest_gens, filters, &mut new_env, Arc::clone(&state), acc)?;
                        }
                    }
                    Value::Map(map) => {
                        for (k, v) in map {
                            let mut new_env = env.clone();
                            new_env.set(var.clone(), Value::Tuple(vec![Value::String(k), v]));
                            eval_comp(expr, rest_gens, filters, &mut new_env, Arc::clone(&state), acc)?;
                        }
                    }
                    _ => return Err(ChatError::with_span("Comprehension iterable must be list, set, or map", 1, e_span(iter_expr))),
                }
                Ok(())
            }
            eval_comp(expr, generators, filters, env, Arc::clone(&state), &mut result)?;
            Ok(Value::List(result))
        }
    }
}

fn e_span(e: &Expr) -> Span {
    match e {
        Expr::Lit(_, s) => *s,
        Expr::Var(_, s) => *s,
        Expr::Lambda(_, _, s) => *s,
        Expr::App(_, _, s) => *s,
        Expr::If(_, _, _, s) => *s,
        Expr::Let { span, .. } => *span,
        Expr::Assign(_, _, s) => *s,
        Expr::Case(_, _, s) => *s,
        Expr::Try(_, s) => *s,
        Expr::Catch(_, _, _, s) => *s,
        Expr::Throw(_, s) => *s,
        Expr::DataDef(_, _, _, s) => *s,
        Expr::StructDef(_, _, s) => *s,
        Expr::StructNew(_, _, s) => *s,
        Expr::Constructor(_, _, s) => *s,
        Expr::Record(_, s) => *s,
        Expr::FieldAccess(_, _, s) => *s,
        Expr::RecordUpdate(_, _, s) => *s,
        Expr::List(_, s) => *s,
        Expr::Range(_, _, s) => *s,
        Expr::BinOp(_, _, _, s) => *s,
        Expr::Concat(_, _, s) => *s,
        Expr::Pipe(_, _, s) => *s,
        Expr::Dollar(_, _, s) => *s,
        Expr::LogicalAnd(_, _, s) => *s,
        Expr::LogicalOr(_, _, s) => *s,
        Expr::Not(_, s) => *s,
        Expr::Tuple(_, s) => *s,
        Expr::Index(_, _, s) => *s,
        Expr::For(_, _, _, s) => *s,
        Expr::While(_, _, s) => *s,
        Expr::Loop(_, s) => *s,
        Expr::Break(_, s) => *s,
        Expr::Block(_, s) => *s,
        Expr::ClassDef { span, .. } => *span,
        Expr::New(_, _, s) => *s,
        Expr::MethodCall(_, _, _, s) => *s,
        Expr::MapLiteral(_, s) => *s,
        Expr::SetLiteral(_, s) => *s,
        Expr::FString(_, s) => *s,
        Expr::ListComp { span, .. } => *span,
    }
}

fn eval_literal(lit: &Literal) -> Result<Value, ChatError> {
    Ok(match lit {
        Literal::Int(i) => Value::Num(Number::Int(*i)),
        Literal::Float(f) => Value::Num(Number::Float(*f)),
        Literal::Char(c) => Value::Char(*c),
        Literal::String(s) => Value::String(s.clone()),
        Literal::Bool(b) => Value::Bool(*b),
        Literal::Unit => Value::Unit,
        Literal::Uid(u) => Value::Uid(u.clone()),
        Literal::ByteString(b) => Value::ByteString(b.clone()),
        Literal::Duration(dur) => Value::Duration(*dur),
    })
}

fn apply(
    func: Value,
    arg: Value,
    _env: &mut Environment,
    state: Arc<Mutex<ChatState>>,
    span: Span,
) -> Result<Value, ChatError> {
    match func {
        Value::Closure(params, body, closure_env) => {
            if params.is_empty() {
                return Err(ChatError::with_span("Cannot apply nullary function", 1, span));
            }
            let mut new_env = closure_env.clone();
            new_env.set(params[0].clone(), arg);
            if params.len() == 1 {
                eval_expr(&body, &mut new_env, state)
            } else {
                let remaining = params[1..].to_vec();
                Ok(Value::Closure(remaining, body, new_env))
            }
        }
        Value::BuiltinFunc(name, arity, f) => {
            if arity == 0 {
                f(vec![])
            } else if arity == 1 {
                f(vec![arg])
            } else {
                Ok(Value::Curry {
                    name,
                    arity,
                    f,
                    args: vec![arg],
                })
            }
        }
        Value::Curry { name, arity, f, args } => {
            let mut new_args = args;
            new_args.push(arg);
            if new_args.len() == arity {
                f(new_args)
            } else {
                Ok(Value::Curry {
                    name,
                    arity,
                    f,
                    args: new_args,
                })
            }
        }
        _ => Err(ChatError::with_span("Not a function", 1, span)),
    }
}

fn eval_binop(op: &BinOp, left: Value, right: Value, span: Span) -> Result<Value, ChatError> {
    match op {
        BinOp::Add => {
            if let (Value::String(_), _) = (&left, &right) {
                return Err(ChatError::with_span("Use '++' to concatenate strings, not '+'", 1, span));
            }
            if let (_, Value::String(_)) = (&left, &right) {
                return Err(ChatError::with_span("Use '++' to concatenate strings, not '+'", 1, span));
            }
            num_op(left, right, |a,b| a+b, |a,b| a+b, span)
        }
        BinOp::Sub => num_op(left, right, |a,b| a-b, |a,b| a-b, span),
        BinOp::Mul => num_op(left, right, |a,b| a*b, |a,b| a*b, span),
        BinOp::Div => match (left, right) {
            (Value::Num(Number::Int(a)), Value::Num(Number::Int(b))) => {
                if b == 0 { Err(ChatError::with_span("Division by zero", 1, span)) }
                else { Ok(Value::Num(Number::Int(a / b))) }
            }
            (Value::Num(Number::Float(a)), Value::Num(Number::Float(b))) => {
                if b == 0.0 { Err(ChatError::with_span("Division by zero", 1, span)) }
                else { Ok(Value::Num(Number::Float(a / b))) }
            }
            (Value::Num(Number::Int(a)), Value::Num(Number::Float(b))) => {
                if b == 0.0 { Err(ChatError::with_span("Division by zero", 1, span)) }
                else { Ok(Value::Num(Number::Float(a as f64 / b))) }
            }
            (Value::Num(Number::Float(a)), Value::Num(Number::Int(b))) => {
                if b == 0 { Err(ChatError::with_span("Division by zero", 1, span)) }
                else { Ok(Value::Num(Number::Float(a / b as f64))) }
            }
            _ => Err(ChatError::with_span("Division requires Num", 1, span)),
        },
        BinOp::Mod => match (left, right) {
            (Value::Num(Number::Int(a)), Value::Num(Number::Int(b))) => {
                if b == 0 { Err(ChatError::with_span("Modulo by zero", 1, span)) }
                else { Ok(Value::Num(Number::Int(a % b))) }
            }
            _ => Err(ChatError::with_span("Modulo requires Int", 1, span)),
        },
        BinOp::Eq => Ok(Value::Bool(left.display() == right.display())),
        BinOp::Neq => Ok(Value::Bool(left.display() != right.display())),
        BinOp::Lt => cmp_op(left, right, |a,b| a < b, span),
        BinOp::Le => cmp_op(left, right, |a,b| a <= b, span),
        BinOp::Gt => cmp_op(left, right, |a,b| a > b, span),
        BinOp::Ge => cmp_op(left, right, |a,b| a >= b, span),
        BinOp::Cons => match (left, right) {
            (v, Value::List(mut list)) => { list.insert(0, v); Ok(Value::List(list)) }
            _ => Err(ChatError::with_span("(:) requires element and list", 1, span)),
        },
        BinOp::In => {
            match right {
                Value::List(list) => {
                    Ok(Value::Bool(list.iter().any(|x| x.display() == left.display())))
                }
                Value::String(s) => {
                    if let Value::Char(c) = left {
                        Ok(Value::Bool(s.contains(c)))
                    } else {
                        Err(ChatError::with_span("in for string requires Char", 1, span))
                    }
                }
                Value::ByteString(b) => {
                    if let Value::Num(Number::Int(i)) = left {
                        Ok(Value::Bool(b.contains(&(i as u8))))
                    } else {
                        Err(ChatError::with_span("in for ByteString requires Int", 1, span))
                    }
                }
                Value::Set(set) => {
                    Ok(Value::Bool(set.contains(&left.display())))
                }
                Value::Map(map) => {
                    Ok(Value::Bool(map.contains_key(&left.display())))
                }
                _ => Err(ChatError::with_span("in requires list, string, byte string, set, or map", 1, span)),
            }
        }
        BinOp::NotIn => {
            match eval_binop(&BinOp::In, left, right, span) {
                Ok(Value::Bool(b)) => Ok(Value::Bool(!b)),
                Err(e) => Err(e),
                _ => Err(ChatError::with_span("not in failed", 1, span)),
            }
        }
    }
}

fn num_op(l: Value, r: Value, ifn: fn(i64, i64) -> i64, ffn: fn(f64, f64) -> f64, span: Span) -> Result<Value, ChatError> {
    match (l, r) {
        (Value::Num(Number::Int(a)), Value::Num(Number::Int(b))) => Ok(Value::Num(Number::Int(ifn(a, b)))),
        (Value::Num(Number::Float(a)), Value::Num(Number::Float(b))) => Ok(Value::Num(Number::Float(ffn(a, b)))),
        (Value::Num(Number::Int(a)), Value::Num(Number::Float(b))) => Ok(Value::Num(Number::Float(ffn(a as f64, b)))),
        (Value::Num(Number::Float(a)), Value::Num(Number::Int(b))) => Ok(Value::Num(Number::Float(ffn(a, b as f64)))),
        _ => Err(ChatError::with_span("Arithmetic requires Num", 1, span)),
    }
}

fn cmp_op(l: Value, r: Value, f: fn(f64, f64) -> bool, _span: Span) -> Result<Value, ChatError> {
    let lf = to_f64(&l)?;
    let rf = to_f64(&r)?;
    Ok(Value::Bool(f(lf, rf)))
}

fn to_f64(v: &Value) -> Result<f64, ChatError> {
    match v {
        Value::Num(Number::Int(i)) => Ok(*i as f64),
        Value::Num(Number::Float(x)) => Ok(*x),
        _ => Err(ChatError::new("Comparison requires numeric value", 1)),
    }
}

pub fn match_pattern(pat: &Pattern, val: &Value) -> Option<BTreeMap<String, Value>> {
    match pat {
        Pattern::Wildcard(_) => Some(BTreeMap::new()),
        Pattern::Var(name, _) => {
            let mut map = BTreeMap::new();
            map.insert(name.clone(), val.clone());
            Some(map)
        }
        Pattern::Literal(lit, _) => {
            if let Ok(v) = eval_literal(lit) {
                if v.display() == val.display() {
                    Some(BTreeMap::new())
                } else { None }
            } else { None }
        }
        Pattern::Constructor(name, pats, _) => {
            match val {
                Value::Custom(cname, args) if cname == name => {
                    if pats.len() == args.len() {
                        let mut map = BTreeMap::new();
                        for (p, a) in pats.iter().zip(args) {
                            if let Some(sub) = match_pattern(p, a) {
                                map.extend(sub);
                            } else { return None; }
                        }
                        Some(map)
                    } else { None }
                }
                _ => None,
            }
        }
        Pattern::List(pats, _) => {
            match val {
                Value::List(vals) if pats.len() == vals.len() => {
                    let mut map = BTreeMap::new();
                    for (p, v) in pats.iter().zip(vals) {
                        if let Some(sub) = match_pattern(p, v) {
                            map.extend(sub);
                        } else { return None; }
                    }
                    Some(map)
                }
                _ => None,
            }
        }
        Pattern::Tuple(pats, _) => {
            match val {
                Value::Tuple(vals) if pats.len() == vals.len() => {
                    let mut map = BTreeMap::new();
                    for (p, v) in pats.iter().zip(vals) {
                        if let Some(sub) = match_pattern(p, v) {
                            map.extend(sub);
                        } else { return None; }
                    }
                    Some(map)
                }
                _ => None,
            }
        }
        Pattern::Record(field_pats, _) => {
            match val {
                Value::Record(map) => {
                    let mut bindings = BTreeMap::new();
                    for (field, pat) in field_pats {
                        if let Some(v) = map.get(field) {
                            if let Some(sub) = match_pattern(pat, v) {
                                bindings.extend(sub);
                            } else { return None; }
                        } else { return None; }
                    }
                    Some(bindings)
                }
                Value::ClassInstance { fields, .. } => {
                    let mut bindings = BTreeMap::new();
                    for (field, pat) in field_pats {
                        if let Some(v) = fields.get(field) {
                            if let Some(sub) = match_pattern(pat, v) {
                                bindings.extend(sub);
                            } else { return None; }
                        } else { return None; }
                    }
                    Some(bindings)
                }
                _ => None,
            }
        }
    }
}

pub fn spawn_process(closure: Value, state: Arc<Mutex<ChatState>>) -> Result<Value, ChatError> {
    match closure {
        Value::Closure(params, body, env) if params.is_empty() => {
            let (sender, receiver) = mpsc::channel();
            let (exit_sender, exit_receiver) = mpsc::channel();
            let pid = {
                let mut next = NEXT_PID.lock().unwrap();
                let id = *next;
                *next += 1;
                id
            };

            let exit_sender_clone = exit_sender.clone();
            let handle = thread::spawn(move || {
                CURRENT_PID.with(|cell| {
                    *cell.borrow_mut() = Some(pid);
                });
                CURRENT_RECEIVER.with(|cell| {
                    *cell.borrow_mut() = Some(receiver);
                });

                let mut new_env = env;
                let result = eval_expr(&body, &mut new_env, state);
                let exit_val = match result {
                    Ok(v) => v,
                    Err(e) => Value::String(e.message),
                };
                let _ = exit_sender_clone.send(exit_val);
            });

            let proc = Process {
                sender,
                exit_sender,
                exit_receiver,
                thread: Some(handle),
            };
            PROCESS_MANAGER.lock().unwrap().insert(pid, proc);
            Ok(Value::Pid(pid))
        }
        _ => Err(ChatError::new("spawn expects a nullary function", 1)),
    }
}

pub fn proc_self() -> Result<Value, ChatError> {
    CURRENT_PID.with(|cell| {
        if let Some(pid) = *cell.borrow() {
            Ok(Value::Pid(pid))
        } else {
            Err(ChatError::new("Not inside a process", 1))
        }
    })
}

pub fn proc_send(pid: usize, val: Value) -> Result<Value, ChatError> {
    let mut map = PROCESS_MANAGER.lock().unwrap();
    if let Some(proc) = map.get_mut(&pid) {
        proc.sender.send(val).map_err(|_| ChatError::new("Process receiver dead", 1))?;
        Ok(Value::Unit)
    } else {
        Err(ChatError::new("Process not found", 1))
    }
}

pub fn proc_recv() -> Result<Value, ChatError> {
    CURRENT_RECEIVER.with(|cell| {
        let receiver_opt = cell.borrow_mut().take();
        if let Some(receiver) = receiver_opt {
            let result = receiver.recv().map_err(|_| ChatError::new("Process mailbox closed", 1));
            *cell.borrow_mut() = Some(receiver);
            result
        } else {
            Err(ChatError::new("Not inside a process or no receiver", 1))
        }
    })
}

pub fn proc_wait(pid: usize) -> Result<Value, ChatError> {
    let mut map = PROCESS_MANAGER.lock().unwrap();
    if let Some(proc) = map.remove(&pid) {
        drop(map);
        let exit_val = proc.exit_receiver.recv()
            .map_err(|_| ChatError::new("Process exited without sending value", 1))?;
        if let Some(handle) = proc.thread {
            let _ = handle.join();
        }
        Ok(exit_val)
    } else {
        Err(ChatError::new("Process not found", 1))
    }
}

pub fn proc_exit(val: Value) -> Result<Value, ChatError> {
    let pid = CURRENT_PID.with(|cell| *cell.borrow());
    if let Some(pid) = pid {
        let map = PROCESS_MANAGER.lock().unwrap();
        if let Some(proc) = map.get(&pid) {
            let exit_sender = proc.exit_sender.clone();
            drop(map);
            let _ = exit_sender.send(val);
            return Ok(Value::Unit);
        }
    }
    Err(ChatError::new("Not inside a process", 1))
}

pub fn sleep(dur: Duration) -> Result<Value, ChatError> {
    thread::sleep(dur);
    Ok(Value::Unit)
}

pub fn after(dur: Duration, closure: Value, state: Arc<Mutex<ChatState>>) -> Result<Value, ChatError> {
    match closure {
        Value::Closure(params, body, env) if params.is_empty() => {
            thread::spawn(move || {
                thread::sleep(dur);
                let mut new_env = env;
                let _ = eval_expr(&body, &mut new_env, state);
            });
            Ok(Value::Unit)
        }
        _ => Err(ChatError::new("after expects a nullary function", 1)),
    }
}

pub fn type_name(val: &Value) -> String {
    match val {
        Value::Num(_) => "Num".to_string(),
        Value::Char(_) => "Char".to_string(),
        Value::String(_) => "String".to_string(),
        Value::Bool(_) => "Bool".to_string(),
        Value::Unit => "Unit".to_string(),
        Value::Uid(_) => "Uid".to_string(),
        Value::ByteString(_) => "ByteString".to_string(),
        Value::List(_) => "List".to_string(),
        Value::Tuple(_) => "Tuple".to_string(),
        Value::Closure(_, _, _) => "Closure".to_string(),
        Value::BuiltinFunc(_, _, _) => "BuiltinFunc".to_string(),
        Value::Curry { .. } => "Curry".to_string(),
        Value::Custom(_, _) => "Custom".to_string(),
        Value::Record(_) => "Record".to_string(),
        Value::Pid(_) => "Pid".to_string(),
        Value::DateTime(_) => "DateTime".to_string(),
        Value::Duration(_) => "Duration".to_string(),
        Value::Json(_) => "Json".to_string(),
        Value::Maybe(_) => "Maybe".to_string(),
        Value::Either(_) => "Either".to_string(),
        Value::ChatMsg { .. } => "ChatMsg".to_string(),
        Value::FileInfo { .. } => "FileInfo".to_string(),
        Value::FileTransfer { .. } => "FileTransfer".to_string(),
        Value::FetchOptions(_) => "FetchOptions".to_string(),
        Value::FetchResult(_) => "FetchResult".to_string(),
        Value::Map(_) => "Map".to_string(),
        Value::Set(_) => "Set".to_string(),
        Value::ClassInstance { .. } => "ClassInstance".to_string(),
        Value::Break(_) => "Break".to_string(),
    }
}

fn find_method(class_def: &Value, method: &str, env: &Environment) -> Result<Value, ChatError> {
    if let Value::Record(info) = class_def {
        if let Some(Value::Record(methods)) = info.get("methods") {
            if let Some(m) = methods.get(method) {
                return Ok(m.clone());
            }
        }
        if let Some(Value::String(parent_name)) = info.get("extends") {
            if !parent_name.is_empty() {
                let parent_def = env.get(parent_name)
                    .ok_or_else(|| ChatError::new(&format!("Parent class '{}' not found", parent_name), 1))?;
                return find_method(&parent_def, method, env);
            }
        }
    }
    Err(ChatError::new(&format!("Method '{}' not found in class hierarchy", method), 1))
}
