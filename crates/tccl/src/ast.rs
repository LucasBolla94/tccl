//! Syntax tree of language version 2 (before type checking).
//!
//! Version 2 is a superset of version 1: every valid version 1 source parses to
//! the same tree. New constructs use *contextual* keywords (`record`, `enum`,
//! `interface`, `role`, `use`, `module`, `upgrade`, `only`, `grant`, `revoke`,
//! `with`), so version 1 contracts that use these words as names still compile.

use crate::error::Pos;

#[derive(Clone, Debug, PartialEq)]
pub enum TypeExpr {
    /// `int`, `Order`, or a qualified name `token.Order`.
    Named(String, Pos),
    List(Box<TypeExpr>, Pos),
    Map(Box<TypeExpr>, Box<TypeExpr>, Pos),
}

impl TypeExpr {
    pub fn pos(&self) -> Pos {
        match self {
            TypeExpr::Named(_, p) | TypeExpr::List(_, p) | TypeExpr::Map(_, _, p) => *p,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitKind {
    Contract,
    Module,
}

/// A contract or a module. A source file holds one contract optionally followed by
/// `module` sections, or only modules (a library file).
#[derive(Clone, Debug)]
pub struct Unit {
    pub kind: UnitKind,
    pub name: String,
    pub pos: Pos,
    pub items: Vec<Item>,
}

#[derive(Clone, Debug)]
pub struct SourceFile {
    pub units: Vec<Unit>,
}

#[derive(Clone, Debug)]
pub struct Variant {
    pub name: String,
    /// Allowed next variants (`Open -> Funded, Cancelled`).
    pub next: Vec<(String, Pos)>,
    pub pos: Pos,
}

#[derive(Clone, Debug)]
pub struct InterfaceFn {
    pub kind: FuncKind,
    pub name: String,
    pub params: Vec<Param>,
    pub ret: Option<TypeExpr>,
    pub payable: bool,
    pub pos: Pos,
}

#[derive(Clone, Debug)]
pub enum Item {
    Const { name: String, ty: TypeExpr, value: Expr, pos: Pos },
    State { name: String, ty: TypeExpr, init: Option<Expr>, pos: Pos },
    Event { name: String, fields: Vec<Param>, pos: Pos },
    Func(FuncDecl),
    Record { name: String, fields: Vec<Param>, pos: Pos },
    /// `transitions` is true when at least one variant lists allowed next variants.
    Enum { name: String, variants: Vec<Variant>, transitions: bool, pos: Pos },
    Interface { name: String, functions: Vec<InterfaceFn>, pos: Pos },
    Role { name: String, pos: Pos },
    Use { path: String, alias: Option<String>, pos: Pos },
}

#[derive(Clone, Debug)]
pub struct Param {
    pub name: String,
    pub ty: TypeExpr,
    pub pos: Pos,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FuncKind {
    Init,
    Action,
    View,
    Fn,
    Upgrade,
}

#[derive(Clone, Debug)]
pub struct FuncDecl {
    pub kind: FuncKind,
    pub name: String,
    pub params: Vec<Param>,
    pub ret: Option<TypeExpr>,
    pub payable: bool,
    /// `only admin, owner` — who may call this action.
    pub only: Vec<(String, Pos)>,
    pub body: Vec<Stmt>,
    pub pos: Pos,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssignOp {
    Set,
    Add,
    Sub,
    Mul,
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Let { name: String, ty: TypeExpr, value: Expr, pos: Pos },
    Assign { target: Expr, op: AssignOp, value: Expr, pos: Pos },
    If { branches: Vec<(Expr, Vec<Stmt>)>, els: Option<Vec<Stmt>>, pos: Pos },
    While { cond: Expr, body: Vec<Stmt>, pos: Pos },
    ForRange { var: String, start: Expr, end: Expr, body: Vec<Stmt>, pos: Pos },
    ForEach { var: String, iter: Expr, body: Vec<Stmt>, pos: Pos },
    Break(Pos),
    Continue(Pos),
    Return(Option<Expr>, Pos),
    Require(Expr, Option<Expr>, Pos),
    Send(Expr, Expr, Pos),
    Emit(String, Vec<Expr>, Pos),
    Destroy(Expr, Pos),
    Pass(Pos),
    Expr(Expr, Pos),
    Grant { role: String, who: Expr, pos: Pos },
    Revoke { role: String, who: Expr, pos: Pos },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Not,
}

pub use crate::program::BinOp;

#[derive(Clone, Debug)]
pub enum Expr {
    Int(i128, Pos),
    Bool(bool, Pos),
    Text(String, Pos),
    Bytes(Vec<u8>, Pos),
    Name(String, Pos),
    List(Vec<Expr>, Pos),
    Unary(UnOp, Box<Expr>, Pos),
    Binary(BinOp, Box<Expr>, Box<Expr>, Pos),
    Call(String, Vec<Expr>, Pos),
    Method(Box<Expr>, String, Vec<Expr>, Pos),
    Index(Box<Expr>, Box<Expr>, Pos),
    /// `order.amount`, `Phase.Open`, `token.total_supply`
    Field(Box<Expr>, String, Pos),
    /// `Order(id: 1, amount: 5)` — a record built with named fields.
    Construct(Box<Expr>, Vec<(String, Expr, Pos)>, Pos),
    /// `Token(addr).buy(1) with value 5 * TCN`
    WithValue(Box<Expr>, Box<Expr>, Pos),
}

impl Expr {
    pub fn pos(&self) -> Pos {
        match self {
            Expr::Int(_, p)
            | Expr::Bool(_, p)
            | Expr::Text(_, p)
            | Expr::Bytes(_, p)
            | Expr::Name(_, p)
            | Expr::List(_, p)
            | Expr::Unary(_, _, p)
            | Expr::Binary(_, _, _, p)
            | Expr::Call(_, _, p)
            | Expr::Method(_, _, _, p)
            | Expr::Index(_, _, p)
            | Expr::Field(_, _, p)
            | Expr::Construct(_, _, p)
            | Expr::WithValue(_, _, p) => *p,
        }
    }

    /// `a.b.c` as a dotted path, if the expression is only names and fields.
    pub fn dotted(&self) -> Option<String> {
        match self {
            Expr::Name(n, _) => Some(n.clone()),
            Expr::Field(base, f, _) => base.dotted().map(|b| format!("{b}.{f}")),
            _ => None,
        }
    }
}
