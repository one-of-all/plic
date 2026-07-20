use crate::ast::*;
use crate::lexer::{LexError, Lexer, Token, TokenKind};

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParseError {}

impl From<LexError> for ParseError {
    fn from(e: LexError) -> Self {
        ParseError { message: e.to_string(), span: Span::dummy() }
    }
}

pub type ParseResult<T> = Result<T, ParseError>;

pub fn strip_comments(input: &str) -> String {
    let mut output = String::new();
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '#' {
            if chars.peek() == Some(&'-') {
                chars.next();
                let mut depth = 1;
                while depth > 0 {
                    match chars.next() {
                        Some('#') => {
                            if chars.peek() == Some(&'-') {
                                chars.next();
                                depth += 1;
                            }
                        }
                        Some('-') => {
                            if chars.peek() == Some(&'#') {
                                chars.next();
                                depth -= 1;
                            }
                        }
                        Some(_) => {}
                        None => break,
                    }
                }
                continue;
            } else {
                while let Some(c) = chars.next() {
                    if c == '\n' {
                        output.push(c);
                        break;
                    }
                }
                continue;
            }
        }
        output.push(ch);
    }
    output
}

pub fn parse_expression(input: &str) -> ParseResult<Expr> {
    let stripped = strip_comments(input);
    let mut lexer = Lexer::new(&stripped);
    let tokens = lexer.tokenize()?;
    let mut pos = 0;
    skip_newlines(&tokens, &mut pos);
    let expr = parse_expr(&tokens, &mut pos)?;
    while pos < tokens.len() && (matches!(tokens[pos].kind, TokenKind::NEWLINE) || matches!(tokens[pos].kind, TokenKind::DEDENT)) {
        pos += 1;
    }
    if pos < tokens.len() && tokens[pos].kind != TokenKind::Eof {
        Err(ParseError { message: format!("Unexpected token: {:?}", tokens[pos].kind), span: Span::new(tokens[pos].start, tokens[pos].end) })
    } else {
        Ok(expr)
    }
}

pub fn parse_script(input: &str) -> ParseResult<Expr> {
    let stripped = strip_comments(input);
    let mut lexer = Lexer::new(&stripped);
    let tokens = lexer.tokenize()?;
    let mut pos = 0;
    let mut exprs = Vec::new();
    skip_newlines(&tokens, &mut pos);
    while pos < tokens.len() && tokens[pos].kind != TokenKind::Eof {
        let e = parse_expr(&tokens, &mut pos)?;
        exprs.push(e);
        while pos < tokens.len() && (matches!(tokens[pos].kind, TokenKind::NEWLINE) || matches!(tokens[pos].kind, TokenKind::Semicolon)) {
            pos += 1;
        }
    }
    if exprs.is_empty() {
        Ok(Expr::Lit(Literal::Unit, Span::dummy()))
    } else if exprs.len() == 1 {
        Ok(exprs.remove(0))
    } else {
        let start = exprs.first().map(|e| e_span(e).start).unwrap_or(0);
        let end = exprs.last().map(|e| e_span(e).end).unwrap_or(0);
        Ok(Expr::Block(exprs, Span::new(start, end)))
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
        Expr::Cast(_, _, s) => *s,
        Expr::SuperMethod { span, .. } => *span,
    }
}

fn parse_expr(tokens: &[Token], pos: &mut usize) -> ParseResult<Expr> {
    parse_dollar(tokens, pos)
}

fn parse_dollar(tokens: &[Token], pos: &mut usize) -> ParseResult<Expr> {
    let mut left = parse_assign(tokens, pos)?;
    while *pos < tokens.len() && tokens[*pos].kind == TokenKind::Dollar {
        let start = tokens[*pos].start;
        *pos += 1;
        let right = parse_assign(tokens, pos)?;
        let end = e_span(&right).end;
        left = Expr::Dollar(Box::new(left), Box::new(right), Span::new(start, end));
    }
    Ok(left)
}

fn parse_assign(tokens: &[Token], pos: &mut usize) -> ParseResult<Expr> {
    let left = parse_pipe(tokens, pos)?;
    if *pos < tokens.len() && tokens[*pos].kind == TokenKind::Assign {
        if let Expr::Var(name, _span) = left {
            let start = tokens[*pos].start;
            *pos += 1;
            let right = parse_assign(tokens, pos)?;
            let end = e_span(&right).end;
            Ok(Expr::Assign(name, Box::new(right), Span::new(start, end)))
        } else {
            Err(ParseError { message: "only variables can be assigned to".to_string(), span: Span::new(tokens[*pos].start, tokens[*pos].end) })
        }
    } else {
        Ok(left)
    }
}

fn parse_pipe(tokens: &[Token], pos: &mut usize) -> ParseResult<Expr> {
    let mut left = parse_logical_or(tokens, pos)?;
    while *pos < tokens.len() && tokens[*pos].kind == TokenKind::Pipe {
        let start = tokens[*pos].start;
        *pos += 1;
        let right = parse_logical_or(tokens, pos)?;
        let end = e_span(&right).end;
        left = Expr::Pipe(Box::new(left), Box::new(right), Span::new(start, end));
    }
    Ok(left)
}

fn parse_logical_or(tokens: &[Token], pos: &mut usize) -> ParseResult<Expr> {
    let mut left = parse_logical_and(tokens, pos)?;
    while *pos < tokens.len() && tokens[*pos].kind == TokenKind::Or {
        let start = tokens[*pos].start;
        *pos += 1;
        let right = parse_logical_and(tokens, pos)?;
        let end = e_span(&right).end;
        left = Expr::LogicalOr(Box::new(left), Box::new(right), Span::new(start, end));
    }
    Ok(left)
}

fn parse_logical_and(tokens: &[Token], pos: &mut usize) -> ParseResult<Expr> {
    let mut left = parse_comparison(tokens, pos)?;
    while *pos < tokens.len() && tokens[*pos].kind == TokenKind::And {
        let start = tokens[*pos].start;
        *pos += 1;
        let right = parse_comparison(tokens, pos)?;
        let end = e_span(&right).end;
        left = Expr::LogicalAnd(Box::new(left), Box::new(right), Span::new(start, end));
    }
    Ok(left)
}

fn parse_comparison(tokens: &[Token], pos: &mut usize) -> ParseResult<Expr> {
    let mut left = parse_concat(tokens, pos)?;
    while *pos < tokens.len() {
        let op = match tokens[*pos].kind {
            TokenKind::Eq => BinOp::Eq,
            TokenKind::Neq => BinOp::Neq,
            TokenKind::Lt => BinOp::Lt,
            TokenKind::Le => BinOp::Le,
            TokenKind::Gt => BinOp::Gt,
            TokenKind::Ge => BinOp::Ge,
            TokenKind::In => {
                let start = tokens[*pos].start;
                *pos += 1;
                let right = parse_concat(tokens, pos)?;
                let end = e_span(&right).end;
                left = Expr::BinOp(BinOp::In, Box::new(left), Box::new(right), Span::new(start, end));
                continue;
            }
            _ => break,
        };
        let start = tokens[*pos].start;
        *pos += 1;
        let right = parse_concat(tokens, pos)?;
        let end = e_span(&right).end;
        left = Expr::BinOp(op, Box::new(left), Box::new(right), Span::new(start, end));
    }
    Ok(left)
}

fn parse_concat(tokens: &[Token], pos: &mut usize) -> ParseResult<Expr> {
    let mut left = parse_cons(tokens, pos)?;
    while *pos < tokens.len() && tokens[*pos].kind == TokenKind::Concat {
        let start = tokens[*pos].start;
        *pos += 1;
        let right = parse_cons(tokens, pos)?;
        let end = e_span(&right).end;
        left = Expr::Concat(Box::new(left), Box::new(right), Span::new(start, end));
    }
    Ok(left)
}

fn parse_cons(tokens: &[Token], pos: &mut usize) -> ParseResult<Expr> {
    let mut left = parse_addsub(tokens, pos)?;
    while *pos < tokens.len() && tokens[*pos].kind == TokenKind::Cons {
        let start = tokens[*pos].start;
        *pos += 1;
        let right = parse_cons(tokens, pos)?;
        let end = e_span(&right).end;
        left = Expr::BinOp(BinOp::Cons, Box::new(left), Box::new(right), Span::new(start, end));
    }
    Ok(left)
}

fn parse_addsub(tokens: &[Token], pos: &mut usize) -> ParseResult<Expr> {
    let mut left = parse_muldiv(tokens, pos)?;
    while *pos < tokens.len() {
        let op = match tokens[*pos].kind {
            TokenKind::Plus => BinOp::Add,
            TokenKind::Minus => BinOp::Sub,
            _ => break,
        };
        let start = tokens[*pos].start;
        *pos += 1;
        let right = parse_muldiv(tokens, pos)?;
        let end = e_span(&right).end;
        left = Expr::BinOp(op, Box::new(left), Box::new(right), Span::new(start, end));
    }
    Ok(left)
}

fn parse_muldiv(tokens: &[Token], pos: &mut usize) -> ParseResult<Expr> {
    let mut left = parse_unary(tokens, pos)?;
    while *pos < tokens.len() {
        let op = match tokens[*pos].kind {
            TokenKind::Star => BinOp::Mul,
            TokenKind::Slash => BinOp::Div,
            TokenKind::Percent => BinOp::Mod,
            _ => break,
        };
        let start = tokens[*pos].start;
        *pos += 1;
        let right = parse_unary(tokens, pos)?;
        let end = e_span(&right).end;
        left = Expr::BinOp(op, Box::new(left), Box::new(right), Span::new(start, end));
    }
    Ok(left)
}

fn parse_unary(tokens: &[Token], pos: &mut usize) -> ParseResult<Expr> {
    if *pos < tokens.len() && tokens[*pos].kind == TokenKind::Not {
        let start = tokens[*pos].start;
        *pos += 1;
        let expr = parse_atom(tokens, pos)?;
        let end = e_span(&expr).end;
        Ok(Expr::Not(Box::new(expr), Span::new(start, end)))
    } else if *pos < tokens.len() && tokens[*pos].kind == TokenKind::Minus {
        let start = tokens[*pos].start;
        *pos += 1;
        let expr = parse_atom(tokens, pos)?;
        let end = e_span(&expr).end;
        Ok(Expr::BinOp(BinOp::Sub, Box::new(Expr::Lit(Literal::Int(0), Span::new(start, start))), Box::new(expr), Span::new(start, end)))
    } else {
        parse_atom(tokens, pos)
    }
}

fn skip_newlines(tokens: &[Token], pos: &mut usize) {
    while *pos < tokens.len() && tokens[*pos].kind == TokenKind::NEWLINE {
        *pos += 1;
    }
}

fn parse_atom(tokens: &[Token], pos: &mut usize) -> ParseResult<Expr> {
    if *pos >= tokens.len() {
        return Err(ParseError { message: "incomplete input".to_string(), span: Span::dummy() });
    }
    let start = tokens[*pos].start;
    match &tokens[*pos].kind {
        TokenKind::Literal(lit) => {
            *pos += 1;
            let end = tokens[*pos - 1].end;
            let expr = Expr::Lit(lit.clone(), Span::new(start, end));
            parse_postfix(expr, tokens, pos)
        }
        TokenKind::FString(content) => {
            *pos += 1;
            let end = tokens[*pos - 1].end;
            let parts = parse_fstring(content)?;
            Ok(Expr::FString(parts, Span::new(start, end)))
        }
        TokenKind::Ident(name) => {
            *pos += 1;
            let end = tokens[*pos - 1].end;
            let mut expr = Expr::Var(name.clone(), Span::new(start, end));
            if *pos < tokens.len() && tokens[*pos].kind == TokenKind::LParen {
                let temp_pos = *pos + 1;
                if is_struct_constructor(tokens, temp_pos) {
                    *pos += 1;
                    let mut fields = Vec::new();
                    while *pos < tokens.len() && tokens[*pos].kind != TokenKind::RParen {
                        let field_name = match &tokens[*pos].kind {
                            TokenKind::Ident(n) => n.clone(),
                            _ => return Err(ParseError { message: "expected field name".to_string(), span: Span::new(tokens[*pos].start, tokens[*pos].end) }),
                        };
                        let _field_start = tokens[*pos].start;
                        *pos += 1;
                        expect_kind(tokens, pos, TokenKind::Assign)?;
                        let value = parse_expr(tokens, pos)?;
                        let _value_end = e_span(&value).end;
                        fields.push((field_name, value));
                        if *pos < tokens.len() && tokens[*pos].kind == TokenKind::Comma {
                            *pos += 1;
                        }
                    }
                    let _end = expect_kind(tokens, pos, TokenKind::RParen)?;
                    expr = Expr::StructNew(name.clone(), fields, Span::new(start, _end));
                } else {
                    *pos += 1;
                    let mut args = Vec::new();
                    while *pos < tokens.len() && tokens[*pos].kind != TokenKind::RParen {
                        let arg = parse_expr(tokens, pos)?;
                        args.push(arg);
                        if *pos < tokens.len() && tokens[*pos].kind == TokenKind::Comma {
                            *pos += 1;
                        }
                    }
                    let _end = expect_kind(tokens, pos, TokenKind::RParen)?;
                    for arg in args {
                        let end = e_span(&arg).end;
                        expr = Expr::App(Box::new(expr), Box::new(arg), Span::new(start, end));
                    }
                }
            } else {
                expr = parse_application(expr, tokens, pos)?;
            }
            parse_postfix(expr, tokens, pos)
        }
        TokenKind::Super => {
            *pos += 1;
            let _end = tokens[*pos - 1].end;
            if *pos < tokens.len() && tokens[*pos].kind == TokenKind::Dot {
                *pos += 1;
                if let TokenKind::Ident(method) = &tokens[*pos].kind {
                    let method = method.clone();
                    *pos += 1;
                    expect_kind(tokens, pos, TokenKind::LParen)?;
                    let mut args = Vec::new();
                    while *pos < tokens.len() && tokens[*pos].kind != TokenKind::RParen {
                        args.push(parse_expr(tokens, pos)?);
                        if *pos < tokens.len() && tokens[*pos].kind == TokenKind::Comma {
                            *pos += 1;
                        }
                    }
                    let end2 = expect_kind(tokens, pos, TokenKind::RParen)?;
                    Ok(Expr::SuperMethod { method, args, span: Span::new(start, end2) })
                } else {
                    Err(ParseError { message: "expected method name after super.".to_string(), span: Span::new(tokens[*pos].start, tokens[*pos].end) })
                }
            } else {
                Err(ParseError { message: "expected .method after super".to_string(), span: Span::new(tokens[*pos].start, tokens[*pos].end) })
            }
        }
        TokenKind::If => {
            *pos += 1;
            let cond = parse_expr(tokens, pos)?;
            skip_newlines(tokens, pos);
            expect_kind(tokens, pos, TokenKind::Then)?;
            let then_expr = parse_expr_or_block(tokens, pos)?;
            skip_newlines(tokens, pos);
            expect_kind(tokens, pos, TokenKind::Else)?;
            let else_expr = parse_expr_or_block(tokens, pos)?;
            let end = e_span(&else_expr).end;
            Ok(Expr::If(Box::new(cond), Box::new(then_expr), Box::new(else_expr), Span::new(start, end)))
        }
        TokenKind::Let => {
            *pos += 1;
            let name = match &tokens[*pos].kind {
                TokenKind::Ident(n) => n.clone(),
                _ => return Err(ParseError { message: "expected identifier".to_string(), span: Span::new(tokens[*pos].start, tokens[*pos].end) }),
            };
            let _name_start = tokens[*pos].start;
            *pos += 1;

            let mut type_ann = None;
            if *pos < tokens.len() && tokens[*pos].kind == TokenKind::DoubleColon {
                *pos += 1;
                let type_str = parse_type(tokens, pos)?;
                type_ann = Some(type_str);
            }

            let mut params = Vec::new();
            while *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Ident(_)) {
                if let TokenKind::Ident(p) = &tokens[*pos].kind {
                    params.push(p.clone());
                    *pos += 1;
                } else {
                    break;
                }
            }

            expect_kind(tokens, pos, TokenKind::Assign)?;
            let body = parse_expr_or_block(tokens, pos)?;
            let end = e_span(&body).end;
            if params.is_empty() {
                Ok(Expr::Let {
                    name,
                    type_ann,
                    def: Box::new(body),
                    body: None,
                    span: Span::new(start, end),
                })
            } else {
                let lambda = params.into_iter().rfold(body, |acc, p| {
                    Expr::Lambda(vec![p], Box::new(acc), Span::new(start, end))
                });
                Ok(Expr::Let {
                    name,
                    type_ann,
                    def: Box::new(lambda),
                    body: None,
                    span: Span::new(start, end),
                })
            }
        }
        TokenKind::Case => {
            *pos += 1;
            let scrut = parse_expr(tokens, pos)?;
            skip_newlines(tokens, pos);
            expect_kind(tokens, pos, TokenKind::Of)?;
            skip_newlines(tokens, pos);
            let mut arms = Vec::new();
            if *pos < tokens.len() && tokens[*pos].kind == TokenKind::INDENT {
                *pos += 1;
                while *pos < tokens.len() && tokens[*pos].kind != TokenKind::DEDENT {
                    let pat = parse_pattern(tokens, pos)?;
                    expect_kind(tokens, pos, TokenKind::Arrow)?;
                    let arm_body = parse_expr(tokens, pos)?;
                    arms.push((pat, Box::new(arm_body)));
                    if *pos < tokens.len() && (tokens[*pos].kind == TokenKind::Semicolon || tokens[*pos].kind == TokenKind::NEWLINE) {
                        *pos += 1;
                    }
                }
                if *pos < tokens.len() && tokens[*pos].kind == TokenKind::DEDENT {
                    *pos += 1;
                }
                let end = if !arms.is_empty() { e_span(&arms.last().unwrap().1).end } else { tokens[*pos - 1].end };
                Ok(Expr::Case(Box::new(scrut), arms, Span::new(start, end)))
            } else {
                while *pos < tokens.len() && tokens[*pos].kind != TokenKind::Eof && tokens[*pos].kind != TokenKind::NEWLINE {
                    let pat = parse_pattern(tokens, pos)?;
                    expect_kind(tokens, pos, TokenKind::Arrow)?;
                    let arm_body = parse_expr(tokens, pos)?;
                    arms.push((pat, Box::new(arm_body)));
                    if *pos < tokens.len() && tokens[*pos].kind == TokenKind::Semicolon {
                        *pos += 1;
                    } else {
                        break;
                    }
                }
                let end = if !arms.is_empty() { e_span(&arms.last().unwrap().1).end } else { tokens[*pos - 1].end };
                Ok(Expr::Case(Box::new(scrut), arms, Span::new(start, end)))
            }
        }
        TokenKind::Try => {
            *pos += 1;
            let try_expr = parse_expr(tokens, pos)?;
            if *pos < tokens.len() && tokens[*pos].kind == TokenKind::Catch {
                *pos += 1;
                let pat = parse_pattern(tokens, pos)?;
                skip_newlines(tokens, pos);
                expect_kind(tokens, pos, TokenKind::Arrow)?;
                let handler = parse_expr(tokens, pos)?;
                let end = e_span(&handler).end;
                Ok(Expr::Catch(Box::new(try_expr), pat, Box::new(handler), Span::new(start, end)))
            } else {
                let end = e_span(&try_expr).end;
                Ok(Expr::Try(Box::new(try_expr), Span::new(start, end)))
            }
        }
        TokenKind::Error => {
            *pos += 1;
            let msg = parse_expr(tokens, pos)?;
            let end = e_span(&msg).end;
            Ok(Expr::Throw(Box::new(msg), Span::new(start, end)))
        }
        TokenKind::LBracket => {
            *pos += 1;
            if *pos < tokens.len() && tokens[*pos].kind == TokenKind::RBracket {
                *pos += 1;
                let end = tokens[*pos - 1].end;
                return Ok(Expr::List(vec![], Span::new(start, end)));
            }
            let first = parse_expr(tokens, pos)?;
            if *pos < tokens.len() && tokens[*pos].kind == TokenKind::For {
                let expr = first;
                let generators = Vec::new();
                let filters = Vec::new();
                let (gens, filters) = parse_list_comp_generators(tokens, pos, generators, filters)?;
                let end = expect_kind(tokens, pos, TokenKind::RBracket)?;
                Ok(Expr::ListComp {
                    expr: Box::new(expr),
                    generators: gens,
                    filters,
                    span: Span::new(start, end),
                })
            } else if *pos < tokens.len() && tokens[*pos].kind == TokenKind::DotDot {
                *pos += 1;
                let end_expr = parse_expr(tokens, pos)?;
                let end = expect_kind(tokens, pos, TokenKind::RBracket)?;
                Ok(Expr::Range(Box::new(first), Box::new(end_expr), Span::new(start, end)))
            } else {
                let mut elems = vec![first];
                while *pos < tokens.len() && tokens[*pos].kind == TokenKind::Comma {
                    *pos += 1;
                    elems.push(parse_expr(tokens, pos)?);
                }
                let end = expect_kind(tokens, pos, TokenKind::RBracket)?;
                Ok(Expr::List(elems, Span::new(start, end)))
            }
        }
        TokenKind::LParen => {
            *pos += 1;
            if *pos < tokens.len() && tokens[*pos].kind == TokenKind::RParen {
                *pos += 1;
                let end = tokens[*pos - 1].end;
                return Ok(Expr::Lit(Literal::Unit, Span::new(start, end)));
            }
            let expr = parse_expr(tokens, pos)?;
            if *pos < tokens.len() && tokens[*pos].kind == TokenKind::Comma {
                let mut elems = vec![expr];
                while *pos < tokens.len() && tokens[*pos].kind == TokenKind::Comma {
                    *pos += 1;
                    elems.push(parse_expr(tokens, pos)?);
                }
                let end = expect_kind(tokens, pos, TokenKind::RParen)?;
                Ok(Expr::Tuple(elems, Span::new(start, end)))
            } else {
                let _end = expect_kind(tokens, pos, TokenKind::RParen)?;
                Ok(expr)
            }
        }
        TokenKind::Lambda => {
            *pos += 1;
            let mut params = Vec::new();
            while *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Ident(_)) {
                if let TokenKind::Ident(p) = &tokens[*pos].kind {
                    params.push(p.clone());
                    *pos += 1;
                } else {
                    break;
                }
            }
            expect_kind(tokens, pos, TokenKind::Arrow)?;
            let body = parse_expr_or_block(tokens, pos)?;
            let end = e_span(&body).end;
            Ok(Expr::Lambda(params, Box::new(body), Span::new(start, end)))
        }
        TokenKind::Struct => {
            *pos += 1;
            let name = match &tokens[*pos].kind {
                TokenKind::Ident(n) => n.clone(),
                _ => return Err(ParseError { message: "expected struct name".to_string(), span: Span::new(tokens[*pos].start, tokens[*pos].end) }),
            };
            *pos += 1;
            expect_kind(tokens, pos, TokenKind::Assign)?;
            expect_kind(tokens, pos, TokenKind::LParen)?;
            let mut fields = Vec::new();
            while *pos < tokens.len() && tokens[*pos].kind != TokenKind::RParen {
                let field_name = match &tokens[*pos].kind {
                    TokenKind::Ident(n) => n.clone(),
                    _ => return Err(ParseError { message: "expected field name".to_string(), span: Span::new(tokens[*pos].start, tokens[*pos].end) }),
                };
                *pos += 1;
                expect_kind(tokens, pos, TokenKind::Assign)?;
                let field_type = match &tokens[*pos].kind {
                    TokenKind::Ident(t) => t.clone(),
                    _ => return Err(ParseError { message: "expected type name".to_string(), span: Span::new(tokens[*pos].start, tokens[*pos].end) }),
                };
                *pos += 1;
                fields.push((field_name, field_type));
                if *pos < tokens.len() && tokens[*pos].kind == TokenKind::Comma {
                    *pos += 1;
                } else {
                    break;
                }
            }
            let _end = expect_kind(tokens, pos, TokenKind::RParen)?;
            Ok(Expr::StructDef(name, fields, Span::new(start, _end)))
        }
        TokenKind::Data => {
            *pos += 1;
            let name = match &tokens[*pos].kind {
                TokenKind::Ident(n) => n.clone(),
                _ => return Err(ParseError { message: "expected data type name".to_string(), span: Span::new(tokens[*pos].start, tokens[*pos].end) }),
            };
            *pos += 1;
            let mut params = Vec::new();
            while *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Ident(_)) {
                if let TokenKind::Ident(p) = &tokens[*pos].kind {
                    params.push(p.clone());
                    *pos += 1;
                } else { break; }
            }
            expect_kind(tokens, pos, TokenKind::Assign)?;
            let mut constructors = Vec::new();
            loop {
                if *pos < tokens.len() && tokens[*pos].kind == TokenKind::RParen {
                    *pos += 1;
                    break;
                }
                let ctor_name = match &tokens[*pos].kind {
                    TokenKind::Ident(n) => n.clone(),
                    _ => return Err(ParseError { message: "expected constructor name".to_string(), span: Span::new(tokens[*pos].start, tokens[*pos].end) }),
                };
                let ctor_start = tokens[*pos].start;
                *pos += 1;
                let mut fields = Vec::new();
                while *pos < tokens.len() && matches!(tokens[*pos].kind, TokenKind::Ident(_)) {
                    if let TokenKind::Ident(t) = &tokens[*pos].kind {
                        fields.push(t.clone());
                        *pos += 1;
                    } else { break; }
                }
                let ctor_end = tokens[*pos - 1].end;
                constructors.push(ConstructorDef { name: ctor_name, fields, span: Span::new(ctor_start, ctor_end) });
                if *pos < tokens.len() && tokens[*pos].kind == TokenKind::Pipe {
                    *pos += 1;
                } else {
                    break;
                }
            }
            let _end = tokens[*pos - 1].end;
            Ok(Expr::DataDef(name, params, constructors, Span::new(start, _end)))
        }
        TokenKind::For => {
            *pos += 1;
            let var = match &tokens[*pos].kind {
                TokenKind::Ident(n) => n.clone(),
                _ => return Err(ParseError { message: "expected loop variable".to_string(), span: Span::new(tokens[*pos].start, tokens[*pos].end) }),
            };
            *pos += 1;
            expect_kind(tokens, pos, TokenKind::In)?;
            let iterable = parse_expr(tokens, pos)?;
            skip_newlines(tokens, pos);
            expect_kind(tokens, pos, TokenKind::Colon)?;
            let body = parse_expr_or_block(tokens, pos)?;
            let end = e_span(&body).end;
            Ok(Expr::For(var, Box::new(iterable), Box::new(body), Span::new(start, end)))
        }
        TokenKind::While => {
            *pos += 1;
            let cond = parse_expr(tokens, pos)?;
            skip_newlines(tokens, pos);
            expect_kind(tokens, pos, TokenKind::Colon)?;
            let body = parse_expr_or_block(tokens, pos)?;
            let end = e_span(&body).end;
            Ok(Expr::While(Box::new(cond), Box::new(body), Span::new(start, end)))
        }
        TokenKind::Loop => {
            *pos += 1;
            expect_kind(tokens, pos, TokenKind::Colon)?;
            let body = parse_expr_or_block(tokens, pos)?;
            let end = e_span(&body).end;
            Ok(Expr::Loop(Box::new(body), Span::new(start, end)))
        }
        TokenKind::Break => {
            *pos += 1;
            let opt = if *pos < tokens.len() && !matches!(tokens[*pos].kind, TokenKind::RBrace | TokenKind::NEWLINE | TokenKind::Semicolon | TokenKind::Eof) {
                Some(Box::new(parse_expr(tokens, pos)?))
            } else {
                None
            };
            let end = if let Some(e) = &opt { e_span(e).end } else { tokens[*pos - 1].end };
            Ok(Expr::Break(opt, Span::new(start, end)))
        }
        TokenKind::Class => {
            *pos += 1;
            let name = match &tokens[*pos].kind {
                TokenKind::Ident(n) => n.clone(),
                _ => return Err(ParseError { message: "expected class name".to_string(), span: Span::new(tokens[*pos].start, tokens[*pos].end) }),
            };
            *pos += 1;
            let mut extends = None;
            if *pos < tokens.len() && tokens[*pos].kind == TokenKind::Extends {
                *pos += 1;
                if let TokenKind::Ident(parent) = &tokens[*pos].kind {
                    extends = Some(parent.clone());
                    *pos += 1;
                } else {
                    return Err(ParseError { message: "expected parent class name".to_string(), span: Span::new(tokens[*pos].start, tokens[*pos].end) });
                }
            }
            expect_kind(tokens, pos, TokenKind::Assign)?;
            expect_kind(tokens, pos, TokenKind::LParen)?;
            let mut fields = Vec::new();
            let mut methods = Vec::new();
            while *pos < tokens.len() && tokens[*pos].kind != TokenKind::RParen {
                match &tokens[*pos].kind {
                    TokenKind::Ident(name) => {
                        let method_name = name.clone();
                        let start = tokens[*pos].start;
                        *pos += 1;
                        if *pos < tokens.len() && tokens[*pos].kind == TokenKind::Assign {
                            *pos += 1;
                            let field_type = if let TokenKind::Ident(t) = &tokens[*pos].kind {
                                t.clone()
                            } else {
                                return Err(ParseError { message: "expected type name".to_string(), span: Span::new(tokens[*pos].start, tokens[*pos].end) });
                            };
                            *pos += 1;
                            fields.push((method_name, Some(field_type)));
                            if *pos < tokens.len() && tokens[*pos].kind == TokenKind::Comma {
                                *pos += 1;
                            }
                        } else if *pos < tokens.len() && tokens[*pos].kind == TokenKind::LParen {
                            *pos += 1;
                            let mut params = Vec::new();
                            while *pos < tokens.len() && tokens[*pos].kind != TokenKind::RParen {
                                if let TokenKind::Ident(p) = &tokens[*pos].kind {
                                    params.push(p.clone());
                                    *pos += 1;
                                    if *pos < tokens.len() && tokens[*pos].kind == TokenKind::Comma {
                                        *pos += 1;
                                    }
                                } else {
                                    return Err(ParseError { message: "expected parameter name".to_string(), span: Span::new(tokens[*pos].start, tokens[*pos].end) });
                                }
                            }
                            expect_kind(tokens, pos, TokenKind::RParen)?;
                            expect_kind(tokens, pos, TokenKind::Assign)?;
                            let body = parse_expr(tokens, pos)?;
                            let end = e_span(&body).end;
                            methods.push(MethodDef {
                                name: method_name,
                                params,
                                body: Box::new(body),
                                span: Span::new(start, end),
                            });
                            if *pos < tokens.len() && tokens[*pos].kind == TokenKind::Semicolon {
                                *pos += 1;
                            }
                        } else {
                            return Err(ParseError { message: "expected '=' or '(' after field/method name".to_string(), span: Span::new(tokens[*pos].start, tokens[*pos].end) });
                        }
                    }
                    _ => return Err(ParseError { message: format!("unexpected token {:?}", tokens[*pos].kind), span: Span::new(tokens[*pos].start, tokens[*pos].end) }),
                }
            }
            let _end = expect_kind(tokens, pos, TokenKind::RParen)?;
            Ok(Expr::ClassDef {
                name,
                extends,
                fields,
                methods,
                span: Span::new(start, _end),
            })
        }
        TokenKind::New => {
            *pos += 1;
            let class_name = match &tokens[*pos].kind {
                TokenKind::Ident(n) => n.clone(),
                _ => return Err(ParseError { message: "expected class name".to_string(), span: Span::new(tokens[*pos].start, tokens[*pos].end) }),
            };
            *pos += 1;
            expect_kind(tokens, pos, TokenKind::LParen)?;
            let mut args = Vec::new();
            while *pos < tokens.len() && tokens[*pos].kind != TokenKind::RParen {
                args.push(parse_expr(tokens, pos)?);
                if *pos < tokens.len() && tokens[*pos].kind == TokenKind::Comma {
                    *pos += 1;
                }
            }
            let _end = expect_kind(tokens, pos, TokenKind::RParen)?;
            Ok(Expr::New(class_name, args, Span::new(start, _end)))
        }
        TokenKind::Percent => {
            *pos += 1;
            match &tokens[*pos].kind {
                TokenKind::LParen => {
                    *pos += 1;
                    let mut entries = Vec::new();
                    while *pos < tokens.len() && tokens[*pos].kind != TokenKind::RParen {
                        let key = parse_expr(tokens, pos)?;
                        expect_kind(tokens, pos, TokenKind::FatArrow)?;
                        let value = parse_expr(tokens, pos)?;
                        entries.push((key, value));
                        if *pos < tokens.len() && tokens[*pos].kind == TokenKind::Comma {
                            *pos += 1;
                        }
                    }
                    let _end = expect_kind(tokens, pos, TokenKind::RParen)?;
                    Ok(Expr::MapLiteral(entries, Span::new(start, _end)))
                }
                TokenKind::LBracket => {
                    *pos += 1;
                    let mut elems = Vec::new();
                    while *pos < tokens.len() && tokens[*pos].kind != TokenKind::RBracket {
                        elems.push(parse_expr(tokens, pos)?);
                        if *pos < tokens.len() && tokens[*pos].kind == TokenKind::Comma {
                            *pos += 1;
                        }
                    }
                    let end = expect_kind(tokens, pos, TokenKind::RBracket)?;
                    Ok(Expr::SetLiteral(elems, Span::new(start, end)))
                }
                _ => Err(ParseError { message: "expected '(' or '[' after '%'".to_string(), span: Span::new(tokens[*pos].start, tokens[*pos].end) }),
            }
        }
        _ => Err(ParseError { message: format!("unexpected token {:?}", tokens[*pos].kind), span: Span::new(tokens[*pos].start, tokens[*pos].end) }),
    }
}

fn is_struct_constructor(tokens: &[Token], mut pos: usize) -> bool {
    let mut depth = 1;
    while pos < tokens.len() && depth > 0 {
        match tokens[pos].kind {
            TokenKind::LParen => depth += 1,
            TokenKind::RParen => depth -= 1,
            TokenKind::Assign => {
                if depth == 1 {
                    return true;
                }
            }
            _ => {}
        }
        pos += 1;
    }
    false
}

fn parse_postfix(mut expr: Expr, tokens: &[Token], pos: &mut usize) -> ParseResult<Expr> {
    loop {
        if *pos < tokens.len() && tokens[*pos].kind == TokenKind::LBracket {
            let start = tokens[*pos].start;
            *pos += 1;
            let index = parse_expr(tokens, pos)?;
            let end = expect_kind(tokens, pos, TokenKind::RBracket)?;
            expr = Expr::Index(Box::new(expr), Box::new(index), Span::new(start, end));
        } else if *pos < tokens.len() && tokens[*pos].kind == TokenKind::DoubleColon {
            let start = tokens[*pos].start;
            *pos += 1;
            let type_name = match &tokens[*pos].kind {
                TokenKind::Ident(t) => t.clone(),
                _ => return Err(ParseError { message: "expected type name after ::".to_string(), span: Span::new(tokens[*pos].start, tokens[*pos].end) }),
            };
            *pos += 1;
            let end = tokens[*pos - 1].end;
            expr = Expr::Cast(Box::new(expr), type_name, Span::new(start, end));
            continue;
        } else if *pos < tokens.len() && tokens[*pos].kind == TokenKind::LBrace {
            let start = tokens[*pos].start;
            *pos += 1;
            let mut updates = Vec::new();
            while *pos < tokens.len() && tokens[*pos].kind != TokenKind::RBrace {
                let field = match &tokens[*pos].kind {
                    TokenKind::Ident(n) => n.clone(),
                    _ => return Err(ParseError { message: "expected field name".to_string(), span: Span::new(tokens[*pos].start, tokens[*pos].end) }),
                };
                *pos += 1;
                expect_kind(tokens, pos, TokenKind::Assign)?;
                let val = parse_expr(tokens, pos)?;
                updates.push((field, val));
                if *pos < tokens.len() && tokens[*pos].kind == TokenKind::Comma {
                    *pos += 1;
                }
            }
            let end = expect_kind(tokens, pos, TokenKind::RBrace)?;
            expr = Expr::RecordUpdate(Box::new(expr), updates, Span::new(start, end));
        } else if *pos < tokens.len() && tokens[*pos].kind == TokenKind::Dot {
            let start = tokens[*pos].start;
            *pos += 1;
            if let TokenKind::Ident(field) = &tokens[*pos].kind {
                let field = field.clone();
                *pos += 1;
                if *pos < tokens.len() && tokens[*pos].kind == TokenKind::LParen {
                    *pos += 1;
                    let mut args = Vec::new();
                    while *pos < tokens.len() && tokens[*pos].kind != TokenKind::RParen {
                        args.push(parse_expr(tokens, pos)?);
                        if *pos < tokens.len() && tokens[*pos].kind == TokenKind::Comma {
                            *pos += 1;
                        }
                    }
                    let end = expect_kind(tokens, pos, TokenKind::RParen)?;
                    expr = Expr::MethodCall(Box::new(expr), field, args, Span::new(start, end));
                } else {
                    let end = tokens[*pos - 1].end;
                    expr = Expr::FieldAccess(Box::new(expr), field, Span::new(start, end));
                }
            } else {
                return Err(ParseError { message: "expected field or method name".to_string(), span: Span::new(tokens[*pos].start, tokens[*pos].end) });
            }
        } else {
            break;
        }
    }
    Ok(expr)
}

fn parse_application(mut func: Expr, tokens: &[Token], pos: &mut usize) -> ParseResult<Expr> {
    loop {
        if *pos < tokens.len() && is_argument_start(&tokens[*pos].kind) {
            let arg = parse_expr(tokens, pos)?;
            let end = e_span(&arg).end;
            let start = e_span(&func).start;
            func = Expr::App(Box::new(func), Box::new(arg), Span::new(start, end));
        } else {
            break;
        }
    }
    Ok(func)
}

fn is_argument_start(kind: &TokenKind) -> bool {
    matches!(kind,
        TokenKind::Literal(_) | TokenKind::Ident(_) | TokenKind::Super |
        TokenKind::LParen | TokenKind::LBracket |
        TokenKind::Lambda | TokenKind::If | TokenKind::Let | TokenKind::Case |
        TokenKind::Try | TokenKind::Error | TokenKind::For | TokenKind::While | TokenKind::Not |
        TokenKind::Minus | TokenKind::Percent | TokenKind::Class | TokenKind::New | TokenKind::Struct | TokenKind::Data |
        TokenKind::FString(_) | TokenKind::Loop | TokenKind::Break
    )
}

fn parse_expr_or_block(tokens: &[Token], pos: &mut usize) -> ParseResult<Expr> {
    skip_newlines(tokens, pos);
    if *pos < tokens.len() && tokens[*pos].kind == TokenKind::INDENT {
        let start = tokens[*pos].start;
        *pos += 1;
        let mut exprs = Vec::new();
        while *pos < tokens.len() && tokens[*pos].kind != TokenKind::DEDENT {
            let e = parse_expr(tokens, pos)?;
            exprs.push(e);
            if *pos < tokens.len() && (tokens[*pos].kind == TokenKind::NEWLINE || tokens[*pos].kind == TokenKind::Semicolon) {
                *pos += 1;
            }
        }
        let end = if *pos < tokens.len() && tokens[*pos].kind == TokenKind::DEDENT {
            let dedent_end = tokens[*pos].end;
            *pos += 1;
            dedent_end
        } else {
            tokens[*pos - 1].end
        };
        if exprs.len() == 1 {
            Ok(exprs.remove(0))
        } else {
            Ok(Expr::Block(exprs, Span::new(start, end)))
        }
    } else {
        parse_expr(tokens, pos)
    }
}

fn parse_pattern(tokens: &[Token], pos: &mut usize) -> Result<Pattern, ParseError> {
    if *pos >= tokens.len() {
        return Err(ParseError { message: "incomplete input".to_string(), span: Span::dummy() });
    }
    let start = tokens[*pos].start;
    match &tokens[*pos].kind {
        TokenKind::Ident(name) => {
            *pos += 1;
            let end = tokens[*pos - 1].end;
            if name == "_" {
                Ok(Pattern::Wildcard(Span::new(start, end)))
            } else {
                if *pos < tokens.len() && tokens[*pos].kind == TokenKind::LParen {
                    *pos += 1;
                    let mut sub_pats = Vec::new();
                    while *pos < tokens.len() && tokens[*pos].kind != TokenKind::RParen {
                        let pat = parse_pattern(tokens, pos)?;
                        sub_pats.push(pat);
                        if *pos < tokens.len() && tokens[*pos].kind == TokenKind::Comma {
                            *pos += 1;
                        }
                    }
                    let end = expect_kind(tokens, pos, TokenKind::RParen)?;
                    Ok(Pattern::Constructor(name.clone(), sub_pats, Span::new(start, end)))
                } else {
                    Ok(Pattern::Var(name.clone(), Span::new(start, end)))
                }
            }
        }
        TokenKind::Literal(lit) => {
            *pos += 1;
            let end = tokens[*pos - 1].end;
            Ok(Pattern::Literal(lit.clone(), Span::new(start, end)))
        }
        TokenKind::LBracket => {
            *pos += 1;
            if *pos < tokens.len() && tokens[*pos].kind == TokenKind::RBracket {
                *pos += 1;
                let end = tokens[*pos - 1].end;
                return Ok(Pattern::List(vec![], Span::new(start, end)));
            }
            let mut pats = Vec::new();
            while *pos < tokens.len() && tokens[*pos].kind != TokenKind::RBracket {
                pats.push(parse_pattern(tokens, pos)?);
                if *pos < tokens.len() && tokens[*pos].kind == TokenKind::Comma {
                    *pos += 1;
                }
            }
            let end = expect_kind(tokens, pos, TokenKind::RBracket)?;
            Ok(Pattern::List(pats, Span::new(start, end)))
        }
        TokenKind::LParen => {
            *pos += 1;
            if *pos < tokens.len() && tokens[*pos].kind == TokenKind::RParen {
                *pos += 1;
                let end = tokens[*pos - 1].end;
                return Ok(Pattern::Literal(Literal::Unit, Span::new(start, end)));
            }
            let pat = parse_pattern(tokens, pos)?;
            if *pos < tokens.len() && tokens[*pos].kind == TokenKind::Comma {
                let mut pats = vec![pat];
                while *pos < tokens.len() && tokens[*pos].kind == TokenKind::Comma {
                    *pos += 1;
                    pats.push(parse_pattern(tokens, pos)?);
                }
                let end = expect_kind(tokens, pos, TokenKind::RParen)?;
                Ok(Pattern::Tuple(pats, Span::new(start, end)))
            } else {
                let _end = expect_kind(tokens, pos, TokenKind::RParen)?;
                Ok(pat)
            }
        }
        TokenKind::LBrace => {
            *pos += 1;
            let mut fields = Vec::new();
            while *pos < tokens.len() && tokens[*pos].kind != TokenKind::RBrace {
                let field_name = match &tokens[*pos].kind {
                    TokenKind::Ident(n) => n.clone(),
                    _ => return Err(ParseError { message: "expected field name".to_string(), span: Span::new(tokens[*pos].start, tokens[*pos].end) }),
                };
                *pos += 1;
                expect_kind(tokens, pos, TokenKind::Assign)?;
                let pat = parse_pattern(tokens, pos)?;
                fields.push((field_name, pat));
                if *pos < tokens.len() && tokens[*pos].kind == TokenKind::Comma {
                    *pos += 1;
                }
            }
            let end = expect_kind(tokens, pos, TokenKind::RBrace)?;
            Ok(Pattern::Record(fields, Span::new(start, end)))
        }
        _ => Err(ParseError { message: format!("unexpected token {:?}", tokens[*pos].kind), span: Span::new(tokens[*pos].start, tokens[*pos].end) }),
    }
}

fn parse_type(tokens: &[Token], pos: &mut usize) -> Result<String, ParseError> {
    let mut type_str = String::new();
    while *pos < tokens.len() {
        match &tokens[*pos].kind {
            TokenKind::Ident(name) => {
                if !type_str.is_empty() { type_str.push(' '); }
                type_str.push_str(name);
                *pos += 1;
            }
            TokenKind::Arrow => {
                type_str.push_str(" -> ");
                *pos += 1;
            }
            _ => break,
        }
    }
    if type_str.is_empty() {
        Err(ParseError { message: "expected type".to_string(), span: Span::new(tokens[*pos].start, tokens[*pos].end) })
    } else {
        Ok(type_str)
    }
}

fn expect_kind(tokens: &[Token], pos: &mut usize, expected: TokenKind) -> Result<usize, ParseError> {
    if *pos < tokens.len() && tokens[*pos].kind == expected {
        let end = tokens[*pos].end;
        *pos += 1;
        Ok(end)
    } else {
        Err(ParseError { message: format!("expected {:?}", expected), span: Span::new(tokens[*pos].start, tokens[*pos].end) })
    }
}

fn parse_fstring(raw: &str) -> Result<Vec<FStringPart>, ParseError> {
    let mut parts = Vec::new();
    let chars: Vec<char> = raw.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '{' {
            if i + 1 < chars.len() && chars[i + 1] == '{' {
                parts.push(FStringPart::Literal("{".to_string()));
                i += 2;
                continue;
            }
            i += 1;
            let start = i;
            let mut depth = 1;
            let mut in_string = false;
            while i < chars.len() && depth > 0 {
                let c = chars[i];
                if c == '"' && (i == 0 || chars[i-1] != '\\') {
                    in_string = !in_string;
                }
                if !in_string {
                    if c == '{' {
                        depth += 1;
                    } else if c == '}' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                }
                i += 1;
            }
            if depth != 0 {
                return Err(ParseError { message: "unclosed expression in f-string".to_string(), span: Span::dummy() });
            }
            let expr_str: String = chars[start..i].iter().collect();
            let expr = parse_expression(&expr_str)?;
            parts.push(FStringPart::Expr(Box::new(expr)));
            i += 1;
        } else if ch == '}' && i + 1 < chars.len() && chars[i + 1] == '}' {
            parts.push(FStringPart::Literal("}".to_string()));
            i += 2;
        } else {
            let start = i;
            while i < chars.len() {
                let c = chars[i];
                if c == '{' || c == '}' {
                    break;
                }
                i += 1;
            }
            let lit: String = chars[start..i].iter().collect();
            let processed = process_escapes(&lit)?;
            parts.push(FStringPart::Literal(processed));
        }
    }
    Ok(parts)
}

fn process_escapes(s: &str) -> Result<String, ParseError> {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(esc) = chars.next() {
                match esc {
                    'n' => result.push('\n'),
                    't' => result.push('\t'),
                    'r' => result.push('\r'),
                    '\\' => result.push('\\'),
                    '"' => result.push('"'),
                    '\'' => result.push('\''),
                    _ => return Err(ParseError { message: format!("invalid escape sequence: \\{}", esc), span: Span::dummy() }),
                }
            } else {
                return Err(ParseError { message: "unfinished escape sequence".to_string(), span: Span::dummy() });
            }
        } else {
            result.push(ch);
        }
    }
    Ok(result)
}

fn parse_list_comp_generators(tokens: &[Token], pos: &mut usize, mut gens: Vec<(String, Box<Expr>)>, mut filters: Vec<Box<Expr>>) -> ParseResult<(Vec<(String, Box<Expr>)>, Vec<Box<Expr>>)> {
    while *pos < tokens.len() && tokens[*pos].kind != TokenKind::RBracket {
        match tokens[*pos].kind {
            TokenKind::For => {
                *pos += 1;
                let var = match &tokens[*pos].kind {
                    TokenKind::Ident(n) => n.clone(),
                    _ => return Err(ParseError { message: "expected loop variable".to_string(), span: Span::new(tokens[*pos].start, tokens[*pos].end) }),
                };
                *pos += 1;
                expect_kind(tokens, pos, TokenKind::In)?;
                let iter = parse_expr(tokens, pos)?;
                gens.push((var, Box::new(iter)));
            }
            TokenKind::If => {
                *pos += 1;
                let cond = parse_expr(tokens, pos)?;
                filters.push(Box::new(cond));
            }
            _ => break,
        }
    }
    Ok((gens, filters))
}
