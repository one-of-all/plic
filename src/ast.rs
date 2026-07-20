#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
    pub fn dummy() -> Self {
        Self { start: 0, end: 0 }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),      // will become Num(f64) after parsing
    Float(f64),    // also Num
    Char(char),
    String(String),
    Bool(bool),
    Unit,
    Uid(String),
    ByteString(Vec<u8>),
    Duration(std::time::Duration),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Wildcard(Span),
    Var(String, Span),
    Literal(Literal, Span),
    Constructor(String, Vec<Pattern>, Span),
    List(Vec<Pattern>, Span),
    Tuple(Vec<Pattern>, Span),
    Record(Vec<(String, Pattern)>, Span),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: Box<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FStringPart {
    Literal(String),
    Expr(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Lit(Literal, Span),
    Var(String, Span),
    Lambda(Vec<String>, Box<Expr>, Span),
    App(Box<Expr>, Box<Expr>, Span),
    If(Box<Expr>, Box<Expr>, Box<Expr>, Span),
    Let {
        name: String,
        type_ann: Option<String>,
        def: Box<Expr>,
        body: Option<Box<Expr>>,
        span: Span,
    },
    Assign(String, Box<Expr>, Span),
    Case(Box<Expr>, Vec<(Pattern, Box<Expr>)>, Span),
    Try(Box<Expr>, Span),
    Catch(Box<Expr>, Pattern, Box<Expr>, Span),
    Throw(Box<Expr>, Span),
    DataDef(String, Vec<String>, Vec<ConstructorDef>, Span),
    StructDef(String, Vec<(String, String)>, Span),
    StructNew(String, Vec<(String, Expr)>, Span),
    Constructor(String, Vec<Expr>, Span),
    Record(Vec<(String, Expr)>, Span),
    FieldAccess(Box<Expr>, String, Span),
    RecordUpdate(Box<Expr>, Vec<(String, Expr)>, Span),
    List(Vec<Expr>, Span),
    Range(Box<Expr>, Box<Expr>, Span),
    BinOp(BinOp, Box<Expr>, Box<Expr>, Span),
    Concat(Box<Expr>, Box<Expr>, Span),
    Pipe(Box<Expr>, Box<Expr>, Span),
    Dollar(Box<Expr>, Box<Expr>, Span),
    LogicalAnd(Box<Expr>, Box<Expr>, Span),
    LogicalOr(Box<Expr>, Box<Expr>, Span),
    Not(Box<Expr>, Span),
    Tuple(Vec<Expr>, Span),
    Index(Box<Expr>, Box<Expr>, Span),
    For(String, Box<Expr>, Box<Expr>, Span),
    While(Box<Expr>, Box<Expr>, Span),
    Loop(Box<Expr>, Span),
    Break(Option<Box<Expr>>, Span),
    Block(Vec<Expr>, Span),
    ClassDef {
        name: String,
        extends: Option<String>,
        fields: Vec<(String, Option<String>)>,
        methods: Vec<MethodDef>,
        span: Span,
    },
    New(String, Vec<Expr>, Span),
    MethodCall(Box<Expr>, String, Vec<Expr>, Span),
    MapLiteral(Vec<(Expr, Expr)>, Span),
    SetLiteral(Vec<Expr>, Span),
    FString(Vec<FStringPart>, Span),
    ListComp {
        expr: Box<Expr>,
        generators: Vec<(String, Box<Expr>)>,
        filters: Vec<Box<Expr>>,
        span: Span,
    },
    // New: type cast expression
    Cast(Box<Expr>, String, Span),
    // New: super method call
    SuperMethod { method: String, args: Vec<Expr>, span: Span },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstructorDef {
    pub name: String,
    pub fields: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Cons,
    Eq,
    Neq,
    Lt,
    Le,
    Gt,
    Ge,
    In,
    NotIn,
}
