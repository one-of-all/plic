use plic::chat::ChatState;
use plic::eval::eval_expr;
use plic::types::Environment;
use plic::parser::{parse_expression, parse_script};
use rustyline::Editor;
use rustyline::error::ReadlineError;
use rustyline::completion::Completer;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::{Validator, ValidationContext, ValidationResult};
use rustyline::Helper;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::fs;
use std::env;
use std::collections::BTreeSet;
use std::borrow::Cow;
use codespan::Files;
use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use codespan_reporting::term as term_reporting;

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

struct PlicHelper {
    env: Arc<Mutex<Environment>>,
    keywords: Vec<String>,
    builtins: Vec<String>,
    types: Vec<String>,
}

impl Completer for PlicHelper {
    type Candidate = String;

    fn complete(&self, line: &str, pos: usize, _ctx: &rustyline::Context<'_>) -> rustyline::Result<(usize, Vec<String>)> {
        let mut candidates = BTreeSet::new();
        for kw in &self.keywords {
            candidates.insert(kw.clone());
        }
        for b in &self.builtins {
            candidates.insert(b.clone());
        }
        for t in &self.types {
            candidates.insert(t.clone());
        }
        let env_guard = self.env.lock().unwrap();
        for (name, _) in env_guard.vars.iter() {
            candidates.insert(name.clone());
        }
        drop(env_guard);

        let prefix = &line[..pos];
        let last_word = prefix.split_whitespace().last().unwrap_or("");
        let matches: Vec<String> = candidates.into_iter()
            .filter(|s| s.starts_with(last_word) && s != last_word)
            .collect();
        Ok((pos - last_word.len(), matches))
    }
}

impl Highlighter for PlicHelper {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        let mut result = String::new();
        let mut chars = line.chars().peekable();
        let mut in_string = false;
        let mut in_fstring = false;
        let mut fstring_brace_depth = 0;
        let mut escape = false;
        let mut multiline_comment_depth = 0;
        let mut in_comment = false;

        let keywords = [
            "let", "if", "then", "else", "case", "of", "lambda",
            "data", "struct", "try", "catch", "error", "for", "while",
            "true", "false", "in", "and", "or", "not", "class", "extends", "new",
            "loop", "break", "load", "super"
        ];
        let types = [
            "Num", "Char", "String", "Bool", "Unit", "Uid",
            "ByteString", "List", "Tuple", "Record", "Pid", "DateTime",
            "Duration", "Json", "Maybe", "Either", "ChatMsg", "FileInfo",
            "FileTransfer", "FetchOptions", "FetchResult", "Map", "Set",
            "ClassInstance"
        ];

        while let Some(ch) = chars.next() {
            if ch == '#' && chars.peek() == Some(&'-') {
                chars.next();
                multiline_comment_depth += 1;
                if in_comment {
                    result.push_str("\x1b[3;90m#-");
                } else {
                    in_comment = true;
                    result.push_str("\x1b[3;90m#-");
                }
                continue;
            }
            if ch == '-' && chars.peek() == Some(&'#') {
                chars.next();
                if multiline_comment_depth > 0 {
                    multiline_comment_depth -= 1;
                    result.push_str("-#\x1b[0m");
                    if multiline_comment_depth == 0 {
                        in_comment = false;
                    } else {
                        result.push_str("\x1b[3;90m");
                    }
                } else {
                    result.push_str("-#");
                }
                continue;
            }
            if in_comment {
                result.push(ch);
                continue;
            }

            if ch == '#' && chars.peek() != Some(&'B') && chars.peek() != Some(&'-') {
                result.push_str("\x1b[3;90m#");
                while let Some(c) = chars.next() {
                    result.push(c);
                }
                result.push_str("\x1b[0m");
                break;
            }

            if ch == 'f' && chars.peek() == Some(&'"') {
                in_fstring = true;
                result.push_str("\x1b[32m");
                result.push(ch);
                continue;
            }

            if ch == '"' && !in_fstring {
                if !in_string {
                    in_string = true;
                    result.push_str("\x1b[32m");
                    result.push(ch);
                } else {
                    if escape {
                        result.push(ch);
                    } else {
                        result.push_str("\"\x1b[0m");
                        in_string = false;
                    }
                    escape = false;
                }
                continue;
            }

            if in_string {
                if ch == '\\' && !escape {
                    escape = true;
                    result.push(ch);
                } else if ch == '"' && escape {
                    escape = false;
                    result.push(ch);
                } else {
                    if escape { escape = false; }
                    result.push(ch);
                }
                continue;
            }

            if in_fstring {
                if ch == '"' && fstring_brace_depth == 0 {
                    result.push_str("\"\x1b[0m");
                    in_fstring = false;
                    continue;
                }
                if ch == '{' {
                    if let Some(&next) = chars.peek() {
                        if next == '{' {
                            chars.next();
                            result.push_str("{{");
                            continue;
                        }
                    }
                    fstring_brace_depth += 1;
                    result.push_str("\x1b[0m");
                    result.push(ch);
                    let mut expr_chars = String::new();
                    let mut depth = 1;
                    while let Some(c) = chars.next() {
                        if c == '{' {
                            depth += 1;
                            expr_chars.push(c);
                        } else if c == '}' {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            } else {
                                expr_chars.push(c);
                            }
                        } else {
                            expr_chars.push(c);
                        }
                    }
                    result.push_str(&expr_chars);
                    result.push_str("\x1b[32m");
                    result.push('}');
                    continue;
                }
                if ch == '}' {
                    if let Some(&next) = chars.peek() {
                        if next == '}' {
                            chars.next();
                            result.push_str("}}");
                            continue;
                        }
                    }
                    fstring_brace_depth -= 1;
                    result.push_str("\x1b[32m");
                    result.push(ch);
                    continue;
                }
                result.push_str("\x1b[32m");
                result.push(ch);
                continue;
            }

            if ch.is_alphabetic() || ch == '_' {
                let mut word = String::new();
                word.push(ch);
                while let Some(&next) = chars.peek() {
                    if next.is_alphanumeric() || next == '_' {
                        word.push(next);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if keywords.contains(&word.as_str()) {
                    result.push_str(&format!("\x1b[34m{}\x1b[0m", word));
                } else if types.contains(&word.as_str()) {
                    result.push_str(&format!("\x1b[35m{}\x1b[0m", word));
                } else {
                    result.push_str(&format!("\x1b[37m{}\x1b[0m", word));
                }
            } else if ch.is_digit(10) {
                result.push_str(&format!("\x1b[33m{}\x1b[0m", ch));
                while let Some(&next) = chars.peek() {
                    if next.is_digit(10) || next == '.' {
                        result.push_str(&format!("\x1b[33m{}\x1b[0m", next));
                        chars.next();
                    } else {
                        break;
                    }
                }
            } else {
                result.push(ch);
            }
        }
        if in_string || in_fstring {
            result.push_str("\x1b[0m");
        }
        Cow::Owned(result)
    }

    fn highlight_char(&self, _line: &str, _pos: usize) -> bool {
        true
    }
}

impl Hinter for PlicHelper {
    type Hint = String;
    fn hint(&self, _line: &str, _pos: usize, _ctx: &rustyline::Context<'_>) -> Option<String> { None }
}

impl Validator for PlicHelper {
    fn validate(&self, _ctx: &mut ValidationContext<'_>) -> rustyline::Result<ValidationResult> {
        Ok(ValidationResult::Valid(None))
    }
}

impl Helper for PlicHelper {}

fn main() {
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("\x1b[31mFatal error\x1b[0m: {}", panic_info);
        std::process::exit(1);
    }));

    let args: Vec<String> = env::args().collect();
    let state = Arc::new(Mutex::new(ChatState::new()));
    let mut env = Environment::new();
    let env_arc = Arc::new(Mutex::new(env.clone()));

    plic::builtins::populate(&mut env, Arc::clone(&state), Arc::clone(&env_arc));

    {
        let mut guard = env_arc.lock().unwrap();
        *guard = env;
    }

    let mut p2p_port = 19000;
    for i in 0..args.len() {
        if args[i] == "--p2p-port" && i + 1 < args.len() {
            if let Ok(port) = args[i+1].parse() {
                p2p_port = port;
            }
        }
    }
    {
        let mut state_guard = state.lock().unwrap();
        state_guard.p2p_port = p2p_port;
    }

    ctrlc::set_handler(|| {
        INTERRUPTED.store(true, Ordering::SeqCst);
        plic::eval::set_interrupted();
    }).expect("Error setting Ctrl-C handler");

    if args.len() > 1 && !args[1].starts_with("--") {
        let filename = &args[1];
        match fs::read_to_string(filename) {
            Ok(content) => {
                match parse_script(&content) {
                    Ok(expr) => {
                        let mut env_guard = env_arc.lock().unwrap();
                        if let Err(err) = eval_expr(&expr, &mut env_guard, Arc::clone(&state)) {
                            let mut files = Files::new();
                            let file_id = files.add(filename, content.clone());
                            let diagnostic = Diagnostic::error()
                                .with_message(err.message)
                                .with_labels(vec![
                                    Label::primary(file_id, err.span.unwrap_or(plic::ast::Span::dummy()).start..err.span.unwrap_or(plic::ast::Span::dummy()).end)
                                ]);
                            let writer = StandardStream::stderr(ColorChoice::Always);
                            let config = term_reporting::Config::default();
                            let _ = term_reporting::emit(&mut writer.lock(), &config, &files, &diagnostic);
                        }
                    }
                    Err(e) => eprintln!("\x1b[31merror\x1b[0m: Parse error: {}", e),
                }
            }
            Err(e) => eprintln!("\x1b[31merror\x1b[0m: Failed to read file {}: {}", filename, e),
        }
        return;
    }

    let helper = PlicHelper {
        env: Arc::clone(&env_arc),
        keywords: vec![
            "let".into(), "if".into(), "then".into(), "else".into(),
            "case".into(), "of".into(), "lambda".into(),
            "data".into(), "struct".into(), "try".into(), "catch".into(),
            "error".into(), "for".into(), "while".into(), "in".into(),
            "and".into(), "or".into(), "not".into(),
            "class".into(), "extends".into(), "new".into(),
            "loop".into(), "break".into(), "load".into(), "super".into(),
        ],
        builtins: vec![
            "sqrt".into(), "sin".into(), "cos".into(), "tan".into(),
            "asin".into(), "acos".into(), "atan".into(),
            "show".into(), "parseInt".into(), "parseFloat".into(),
            "chr".into(), "ord".into(),
            "null".into(), "length".into(), "map".into(), "filter".into(),
            "foldl".into(), "foldr".into(), "take".into(), "drop".into(),
            "reverse".into(), "all".into(), "any".into(), "find".into(),
            "sort".into(), "sortBy".into(), "sum".into(),
            "concat".into(), "flatten".into(), "zip".into(), "zipWith".into(),
            "unzip".into(), "indexOf".into(), "lastIndexOf".into(),
            "split".into(), "join".into(), "startsWith".into(), "endsWith".into(),
            "trim".into(), "replace".into(), "substring".into(),
            "parseJson".into(), "encodeJson".into(), "lookup".into(),
            "formatTime".into(), "parseTime".into(), "addDuration".into(),
            "diffDuration".into(),
            "packBytes".into(), "unpackBytes".into(),
            "putStrLn".into(), "getLine".into(), "getArgs".into(),
            "readFile".into(), "readBinaryFile".into(), "writeFile".into(),
            "appendFile".into(), "writeBinaryFile".into(), "fileExists".into(),
            "fileSize".into(),
            "fetch".into(), "fetchOpts".into(),
            "login".into(), "newChat".into(), "addMember".into(), "removeMember".into(),
            "open".into(), "send".into(), "sendFile".into(),
            "sendChat".into(), "sendFileToChat".into(),
            "inbox".into(), "history".into(), "downloads".into(), "saveFile".into(),
            "serverStart".into(), "serverStop".into(),
            "connect".into(),
            "now".into(),
            "spawn".into(), "procSelf".into(), "procSend".into(),
            "procRecv".into(), "procWait".into(), "procExit".into(),
            "sleep".into(), "after".into(),
            "Nothing".into(), "Just".into(), "maybe".into(),
            "mapGet".into(), "mapSet".into(), "mapRemove".into(),
            "mapKeys".into(), "mapValues".into(), "mapEntries".into(),
            "mapContains".into(), "mapSize".into(), "mapFilter".into(), "mapMerge".into(),
            "setAdd".into(), "setRemove".into(),
            "setContains".into(), "setUnion".into(), "setIntersection".into(),
            "setDifference".into(), "setSize".into(), "setFilter".into(), "setMap".into(),
            "listToSet".into(), "mapToList".into(),
            "sha256".into(), "sha256String".into(),
            "kyberKeyPair".into(), "kyberEncapsulate".into(), "kyberDecapsulate".into(),
            "listDir".into(), "createDir".into(), "removeDir".into(),
            "fileMove".into(), "filePermissions".into(), "setFilePermissions".into(),
            "typeof".into(), "getPublicIP".into(), "setExternalIP".into(),
            "exit".into(), "load".into(),
            "logout".into(), "deleteUser".into(), "deleteChat".into(),
            "listChats".into(), "members".into(),
            "addContact".into(), "removeContact".into(),
            "p2pPort".into(),
        ],
        types: vec![
            "Num".into(), "Char".into(), "String".into(),
            "Bool".into(), "Unit".into(), "Uid".into(),
            "ByteString".into(), "List".into(), "Tuple".into(),
            "Record".into(), "Pid".into(), "DateTime".into(),
            "Duration".into(), "Json".into(), "Maybe".into(),
            "Either".into(), "ChatMsg".into(), "FileInfo".into(),
            "FileTransfer".into(), "FetchOptions".into(), "FetchResult".into(),
            "Map".into(), "Set".into(), "ClassInstance".into(),
        ],
    };

    let mut rl = Editor::new().unwrap();
    rl.set_helper(Some(helper));
    let _ = rl.load_history(".plic_history");

    let mut buffer = String::new();
    let mut in_multiline = false;
    let mut first_indent = 0;

    loop {
        let prompt = if buffer.is_empty() {
            ">>> "
        } else {
            "... "
        };
        let line = rl.readline(prompt);
        match line {
            Ok(line) => {
                rl.add_history_entry(line.as_str());
                let trimmed = line.trim();
                let indent = line.chars().take_while(|c| *c == ' ').count();

                if !in_multiline {
                    if trimmed.is_empty() {
                        continue;
                    }

                    if trimmed == "{" || trimmed == "}" {
                        eprintln!("\x1b[31merror\x1b[0m: Unexpected token '{}'", trimmed);
                        continue;
                    }

                    // If line ends with a single colon (not ::), enter multiline mode with colon removed
                    if trimmed.ends_with(':') && !line.contains("::") {
                        let mut modified = line.trim_end().to_string();
                        while modified.ends_with(':') || modified.ends_with(' ') {
                            modified.pop();
                        }
                        buffer = modified;
                        in_multiline = true;
                        first_indent = indent;
                        continue;
                    }

                    match parse_expression(&line) {
                        Ok(expr) => {
                            let mut env_guard = env_arc.lock().unwrap();
                            match eval_expr(&expr, &mut env_guard, Arc::clone(&state)) {
                                Ok(_) => {},
                                Err(err) => eprintln!("\x1b[31merror\x1b[0m: {}", err),
                            }
                            continue;
                        }
                        Err(_) => {
                            buffer = line;
                            in_multiline = true;
                            first_indent = indent;
                            continue;
                        }
                    }
                } else {
                    if trimmed.is_empty() {
                        if !buffer.is_empty() {
                            let full_block = buffer.clone();
                            match parse_script(&full_block) {
                                Ok(expr) => {
                                    let mut env_guard = env_arc.lock().unwrap();
                                    match eval_expr(&expr, &mut env_guard, Arc::clone(&state)) {
                                        Ok(_) => {},
                                        Err(err) => eprintln!("\x1b[31merror\x1b[0m: {}", err),
                                    }
                                }
                                Err(e) => eprintln!("\x1b[31merror\x1b[0m: Parse error: {}", e),
                            }
                            buffer.clear();
                            in_multiline = false;
                            first_indent = 0;
                        }
                        continue;
                    }

                    if indent > first_indent {
                        buffer.push('\n');
                        buffer.push_str(&line);
                        continue;
                    } else {
                        let full_block = buffer.clone();
                        let block_ok = match parse_script(&full_block) {
                            Ok(expr) => {
                                let mut env_guard = env_arc.lock().unwrap();
                                match eval_expr(&expr, &mut env_guard, Arc::clone(&state)) {
                                    Ok(_) => true,
                                    Err(err) => {
                                        eprintln!("\x1b[31merror\x1b[0m: {}", err);
                                        false
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("\x1b[31merror\x1b[0m: Parse error: {}", e);
                                false
                            }
                        };
                        buffer.clear();
                        in_multiline = false;
                        first_indent = 0;

                        if !block_ok {
                            continue;
                        }

                        if !trimmed.is_empty() {
                            match parse_expression(&line) {
                                Ok(expr) => {
                                    let mut env_guard = env_arc.lock().unwrap();
                                    match eval_expr(&expr, &mut env_guard, Arc::clone(&state)) {
                                        Ok(_) => {},
                                        Err(err) => eprintln!("\x1b[31merror\x1b[0m: {}", err),
                                    }
                                }
                                Err(_) => {
                                    buffer = line;
                                    in_multiline = true;
                                    first_indent = indent;
                                }
                            }
                        }
                        continue;
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                if in_multiline {
                    println!("Aborted multi-line block.");
                    buffer.clear();
                    in_multiline = false;
                    first_indent = 0;
                }
                continue;
            }
            Err(_) => break,
        }
    }
    let _ = rl.save_history(".plic_history");
}
