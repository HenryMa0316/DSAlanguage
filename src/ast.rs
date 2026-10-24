#[derive(Debug, Clone)]
pub struct Program {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub type_name: String,
    pub name: String,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    VarDecl(VarDecl),
    Assign(Assign),
    If(IfStmt),
    While(WhileStmt),
    For(ForStmt),
    Def(DefStmt),
    Return(ReturnStmt),
    Break(usize),
    Continue(usize),
    ExprStmt(ExprStmt),
}

#[derive(Debug, Clone)]
pub struct VarDecl {
    pub type_name: String,
    pub name: String,
    pub init: Expr,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct Assign {
    pub target: Expr,
    pub op: AssignOp,
    pub value: Expr,
    pub line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AssignOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
}

#[derive(Debug, Clone)]
pub struct IfStmt {
    pub branches: Vec<(Expr, Block)>,
    pub else_block: Option<Block>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct WhileStmt {
    pub cond: Expr,
    pub body: Block,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct ForStmt {
    pub vars: Vec<String>,
    pub iter: Expr,
    pub body: Block,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct DefStmt {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: String,
    pub body: Block,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct ReturnStmt {
    pub value: Expr,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct ExprStmt {
    pub expr: Expr,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    None,
    Var(String),
    BinOp(BinOpExpr),
    UnOp(UnOpExpr),
    Call(CallExpr),
    Index(IndexExpr),
    MethodCall(MethodCallExpr),
    ListLit(Vec<Expr>, usize),
    DictLit(Vec<(Expr, Expr)>, usize),
}

#[derive(Debug, Clone)]
pub struct BinOpExpr {
    pub op: BinOp,
    pub left: Box<Expr>,
    pub right: Box<Expr>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct UnOpExpr {
    pub op: UnOp,
    pub operand: Box<Expr>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct CallExpr {
    pub callee: Box<Expr>,
    pub args: Vec<Expr>,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct IndexExpr {
    pub object: Box<Expr>,
    pub index: Box<Expr>,
    pub line: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    FloorDiv,
    Mod,
    Pow,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub struct MethodCallExpr {
    pub object: Box<Expr>,
    pub method: String,
    pub args: Vec<Expr>,
    pub line: usize,
}

impl Expr {
    pub fn line(&self) -> usize {
        match self {
            Expr::Int(_) | Expr::Float(_) | Expr::Str(_) | Expr::Bool(_) | Expr::None | Expr::Var(_) => 0,
            Expr::BinOp(b) => b.line,
            Expr::UnOp(u) => u.line,
            Expr::Call(c) => c.line,
            Expr::Index(i) => i.line,
            Expr::ListLit(_, l) => *l,
            Expr::DictLit(_, l) => *l,
            Expr::MethodCall(m) => m.line,
        }
    }
}