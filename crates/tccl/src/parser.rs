//! Recursive-descent parser for language version 2. Nesting depth is bounded so
//! hostile source code cannot exhaust the stack of the nodes that compile it.
//! Every version 1 source parses to the same tree as in [`crate::v1::parser`].

use crate::ast::*;
use crate::error::{CompileError, Pos};
use crate::v1::lexer::{tokenize, Tok, Token};

/// Maximum nesting of expressions and blocks.
pub const MAX_NESTING: usize = 32;
/// Most operators of one precedence level chained in one expression.
pub const MAX_CHAIN: usize = 64;
/// Deepest expression tree accepted.
pub const MAX_EXPR_DEPTH: usize = 128;

fn expr_depth(e: &Expr) -> usize {
    1 + match e {
        Expr::Int(..) | Expr::Bool(..) | Expr::Text(..) | Expr::Bytes(..) | Expr::Name(..) => 0,
        Expr::List(items, _) | Expr::Call(_, items, _) => items.iter().map(expr_depth).max().unwrap_or(0),
        Expr::Unary(_, x, _) => expr_depth(x),
        Expr::Binary(_, l, r, _) | Expr::Index(l, r, _) => expr_depth(l).max(expr_depth(r)),
        Expr::Method(x, _, args, _) => args.iter().map(expr_depth).max().unwrap_or(0).max(expr_depth(x)),
        Expr::Field(x, _, _) => expr_depth(x),
        Expr::Construct(x, fields, _) => fields.iter().map(|(_, e, _)| expr_depth(e)).max().unwrap_or(0).max(expr_depth(x)),
        Expr::WithValue(x, v, _) => expr_depth(x).max(expr_depth(v)),
    }
}

struct Parser {
    toks: Vec<Token>,
    i: usize,
    depth: usize,
}

type PResult<T> = Result<T, CompileError>;

/// Most `module` sections in one source file.
pub const MAX_UNITS: usize = 16;

/// Parses a source file: a contract optionally followed by `module` sections, or
/// only `module` sections (a library).
pub fn parse_file(src: &str) -> PResult<SourceFile> {
    let toks = tokenize(src)?;
    let mut p = Parser { toks, i: 0, depth: 0 };
    p.file()
}

/// Contextual keyword check (`record`, `only`, `with`, ...).
fn is_word(t: &Tok, word: &str) -> bool {
    matches!(t, Tok::Ident(s) if s == word)
}

fn describe(t: &Tok) -> String {
    match t {
        Tok::Ident(s) => format!("name '{s}'"),
        Tok::Int(v) => format!("number {v}"),
        Tok::Text(_) => "text".into(),
        Tok::Bytes(_) => "bytes literal".into(),
        Tok::Newline => "end of line".into(),
        Tok::Indent => "indentation".into(),
        Tok::Dedent => "end of block".into(),
        Tok::Eof => "end of file".into(),
        other => format!("{other:?}").to_lowercase(),
    }
}

impl Parser {
    fn peek(&self) -> &Tok {
        &self.toks[self.i].tok
    }

    fn pos(&self) -> Pos {
        self.toks[self.i].pos
    }

    fn advance(&mut self) -> Token {
        let t = self.toks[self.i].clone();
        if self.i < self.toks.len() - 1 {
            self.i += 1;
        }
        t
    }

    fn check(&self, t: &Tok) -> bool {
        std::mem::discriminant(self.peek()) == std::mem::discriminant(t)
    }

    fn eat(&mut self, t: &Tok) -> bool {
        if self.check(t) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, t: &Tok, what: &str) -> PResult<Token> {
        if self.check(t) {
            Ok(self.advance())
        } else {
            Err(CompileError::new(self.pos(), format!("expected {what}, found {}", describe(self.peek()))))
        }
    }

    fn ident(&mut self, what: &str) -> PResult<(String, Pos)> {
        let pos = self.pos();
        match self.peek().clone() {
            Tok::Ident(s) => {
                self.advance();
                Ok((s, pos))
            }
            other => Err(CompileError::new(pos, format!("expected {what}, found {}", describe(&other)))),
        }
    }

    fn enter(&mut self) -> PResult<()> {
        self.depth += 1;
        if self.depth > MAX_NESTING {
            return Err(CompileError::new(self.pos(), format!("nesting too deep (max {MAX_NESTING})")));
        }
        Ok(())
    }

    fn leave(&mut self) {
        self.depth -= 1;
    }

    fn skip_newlines(&mut self) {
        while self.eat(&Tok::Newline) {}
    }

    fn peek_at(&self, n: usize) -> &Tok {
        &self.toks[(self.i + n).min(self.toks.len() - 1)].tok
    }

    fn word(&self, w: &str) -> bool {
        is_word(self.peek(), w)
    }

    fn file(&mut self) -> PResult<SourceFile> {
        self.skip_newlines();
        let mut units = Vec::new();
        loop {
            self.skip_newlines();
            if self.check(&Tok::Eof) {
                break;
            }
            let pos = self.pos();
            let kind = if self.check(&Tok::Contract) {
                if !units.is_empty() {
                    return Err(CompileError::new(pos, "only one contract per file; it must come before any module"));
                }
                UnitKind::Contract
            } else if self.word("module") {
                UnitKind::Module
            } else if units.is_empty() {
                return Err(CompileError::new(pos, format!("expected 'contract <Name>' as the first line, found {}", describe(self.peek()))));
            } else {
                return Err(CompileError::new(pos, format!("expected a declaration, found {}", describe(self.peek()))));
            };
            if units.len() >= MAX_UNITS {
                return Err(CompileError::new(pos, format!("too many modules in one file (max {MAX_UNITS})")));
            }
            self.advance();
            let (name, _) = self.ident(if kind == UnitKind::Contract { "contract name" } else { "module name" })?;
            self.expect(&Tok::Newline, if kind == UnitKind::Contract { "end of line after contract name" } else { "end of line after module name" })?;
            let items = self.items()?;
            units.push(Unit { kind, name, pos, items });
        }
        if units.is_empty() {
            return Err(CompileError::new(self.pos(), "expected 'contract <Name>' as the first line, found end of file"));
        }
        Ok(SourceFile { units })
    }

    fn items(&mut self) -> PResult<Vec<Item>> {
        let mut items = Vec::new();
        loop {
            self.skip_newlines();
            match self.peek() {
                Tok::Eof => break,
                Tok::Ident(w) if w == "module" => break,
                Tok::Const => items.push(self.const_item()?),
                Tok::State => items.push(self.state_item()?),
                Tok::Event => items.push(self.event_item()?),
                Tok::Init | Tok::Action | Tok::View | Tok::Fn => items.push(Item::Func(self.func()?)),
                Tok::Ident(w) if w == "record" => items.push(self.record_item()?),
                Tok::Ident(w) if w == "enum" => items.push(self.enum_item()?),
                Tok::Ident(w) if w == "interface" => items.push(self.interface_item()?),
                Tok::Ident(w) if w == "role" => items.push(self.role_item()?),
                Tok::Ident(w) if w == "use" => items.push(self.use_item()?),
                Tok::Ident(w) if w == "upgrade" && matches!(self.peek_at(1), Tok::LParen) => items.push(Item::Func(self.func()?)),
                Tok::Contract => return Err(CompileError::new(self.pos(), "only one contract per file; it must come before any module")),
                Tok::Indent => return Err(CompileError::new(self.pos(), "unexpected indentation at top level")),
                other => {
                    return Err(CompileError::new(
                        self.pos(),
                        format!(
                            "expected const, state, event, init, action, view, fn, record, enum, interface, role, use or upgrade, found {}",
                            describe(other)
                        ),
                    ))
                }
            }
        }
        Ok(items)
    }

    fn record_item(&mut self) -> PResult<Item> {
        let pos = self.advance().pos;
        let (name, _) = self.ident("record name")?;
        self.expect(&Tok::Colon, "':' after the record name")?;
        self.expect(&Tok::Newline, "end of line after ':'")?;
        self.skip_newlines();
        self.expect(&Tok::Indent, "indented fields ('name: type', one per line)")?;
        let mut fields = Vec::new();
        loop {
            self.skip_newlines();
            if self.eat(&Tok::Dedent) || self.check(&Tok::Eof) {
                break;
            }
            let (fname, fpos) = self.ident("field name")?;
            self.expect(&Tok::Colon, "':' and the field type")?;
            let ty = self.type_expr()?;
            self.end_stmt()?;
            fields.push(Param { name: fname, ty, pos: fpos });
        }
        if fields.is_empty() {
            return Err(CompileError::new(pos, "a record needs at least one field"));
        }
        Ok(Item::Record { name, fields, pos })
    }

    fn variant(&mut self) -> PResult<Variant> {
        let (name, pos) = self.ident("variant name")?;
        let mut next = Vec::new();
        if self.eat(&Tok::Arrow) {
            loop {
                next.push(self.ident("next variant name")?);
                if !self.eat(&Tok::Comma) {
                    break;
                }
            }
        }
        Ok(Variant { name, next, pos })
    }

    fn enum_item(&mut self) -> PResult<Item> {
        let pos = self.advance().pos;
        let (name, _) = self.ident("enum name")?;
        self.expect(&Tok::Colon, "':' after the enum name")?;
        let mut variants = Vec::new();
        if self.check(&Tok::Newline) {
            // Block form: one variant per line, optionally `A -> B, C`.
            self.advance();
            self.skip_newlines();
            self.expect(&Tok::Indent, "indented variants, one per line")?;
            loop {
                self.skip_newlines();
                if self.eat(&Tok::Dedent) || self.check(&Tok::Eof) {
                    break;
                }
                variants.push(self.variant()?);
                self.end_stmt()?;
            }
        } else {
            // Inline form: `enum Color: Red, Green, Blue`
            loop {
                let (vname, vpos) = self.ident("variant name")?;
                variants.push(Variant { name: vname, next: Vec::new(), pos: vpos });
                if !self.eat(&Tok::Comma) {
                    break;
                }
            }
            self.expect(&Tok::Newline, "end of line")?;
        }
        let transitions = variants.iter().any(|v| !v.next.is_empty());
        if variants.is_empty() {
            return Err(CompileError::new(pos, "an enum needs at least one variant"));
        }
        Ok(Item::Enum { name, variants, transitions, pos })
    }

    fn interface_item(&mut self) -> PResult<Item> {
        let pos = self.advance().pos;
        let (name, _) = self.ident("interface name")?;
        self.expect(&Tok::Colon, "':' after the interface name")?;
        self.expect(&Tok::Newline, "end of line after ':'")?;
        self.skip_newlines();
        self.expect(&Tok::Indent, "indented 'action' or 'view' signatures")?;
        let mut functions = Vec::new();
        loop {
            self.skip_newlines();
            if self.eat(&Tok::Dedent) || self.check(&Tok::Eof) {
                break;
            }
            let fpos = self.pos();
            let kind = match self.peek() {
                Tok::Action => FuncKind::Action,
                Tok::View => FuncKind::View,
                other => {
                    return Err(CompileError::new(fpos, format!("an interface lists 'action' or 'view' signatures, found {}", describe(other))))
                }
            };
            self.advance();
            let (fname, _) = self.ident("function name")?;
            let params = self.params()?;
            let ret = if self.eat(&Tok::Arrow) { Some(self.type_expr()?) } else { None };
            let payable = self.eat(&Tok::Payable);
            if self.check(&Tok::Colon) {
                return Err(CompileError::new(self.pos(), "interface functions have no body; remove the ':'"));
            }
            self.end_stmt()?;
            functions.push(InterfaceFn { kind, name: fname, params, ret, payable, pos: fpos });
        }
        if functions.is_empty() {
            return Err(CompileError::new(pos, "an interface needs at least one function"));
        }
        Ok(Item::Interface { name, functions, pos })
    }

    fn role_item(&mut self) -> PResult<Item> {
        let pos = self.advance().pos;
        let (name, _) = self.ident("role name")?;
        self.expect(&Tok::Newline, "end of line")?;
        Ok(Item::Role { name, pos })
    }

    fn use_item(&mut self) -> PResult<Item> {
        let pos = self.advance().pos;
        let (mut path, _) = self.ident("module name, e.g. 'use std.token'")?;
        while self.eat(&Tok::Dot) {
            let (part, _) = self.ident("module name")?;
            path = format!("{path}.{part}");
        }
        let alias = if self.word("as") {
            self.advance();
            Some(self.ident("module alias")?.0)
        } else {
            None
        };
        self.expect(&Tok::Newline, "end of line")?;
        Ok(Item::Use { path, alias, pos })
    }

    fn type_expr(&mut self) -> PResult<TypeExpr> {
        self.enter()?;
        let t = self.type_expr_inner();
        self.leave();
        t
    }

    fn type_expr_inner(&mut self) -> PResult<TypeExpr> {
        let (name, pos) = self.ident("type")?;
        match name.as_str() {
            "list" => {
                self.expect(&Tok::LBracket, "'[' after list")?;
                let inner = self.type_expr()?;
                self.expect(&Tok::RBracket, "']'")?;
                Ok(TypeExpr::List(Box::new(inner), pos))
            }
            "map" => {
                self.expect(&Tok::LBracket, "'[' after map")?;
                let k = self.type_expr()?;
                self.expect(&Tok::Comma, "',' between map key and value types")?;
                let v = self.type_expr()?;
                self.expect(&Tok::RBracket, "']'")?;
                Ok(TypeExpr::Map(Box::new(k), Box::new(v), pos))
            }
            _ => {
                let mut name = name;
                if self.eat(&Tok::Dot) {
                    let (inner, _) = self.ident("type name after '.'")?;
                    name = format!("{name}.{inner}");
                }
                Ok(TypeExpr::Named(name, pos))
            }
        }
    }

    fn const_item(&mut self) -> PResult<Item> {
        let pos = self.advance().pos;
        let (name, _) = self.ident("constant name")?;
        self.expect(&Tok::Colon, "':' and a type (constants must declare their type)")?;
        let ty = self.type_expr()?;
        self.expect(&Tok::Assign, "'=' and a value")?;
        let value = self.expr()?;
        self.expect(&Tok::Newline, "end of line")?;
        Ok(Item::Const { name, ty, value, pos })
    }

    fn state_item(&mut self) -> PResult<Item> {
        let pos = self.advance().pos;
        let (name, _) = self.ident("state variable name")?;
        self.expect(&Tok::Colon, "':' and a type (state variables must declare their type)")?;
        let ty = self.type_expr()?;
        let init = if self.eat(&Tok::Assign) { Some(self.expr()?) } else { None };
        self.expect(&Tok::Newline, "end of line")?;
        Ok(Item::State { name, ty, init, pos })
    }

    fn params(&mut self) -> PResult<Vec<Param>> {
        self.expect(&Tok::LParen, "'('")?;
        let mut params = Vec::new();
        if !self.check(&Tok::RParen) {
            loop {
                let (name, pos) = self.ident("parameter name")?;
                self.expect(&Tok::Colon, "':' and the parameter type")?;
                let ty = self.type_expr()?;
                params.push(Param { name, ty, pos });
                if !self.eat(&Tok::Comma) {
                    break;
                }
            }
        }
        self.expect(&Tok::RParen, "')'")?;
        Ok(params)
    }

    fn event_item(&mut self) -> PResult<Item> {
        let pos = self.advance().pos;
        let (name, _) = self.ident("event name")?;
        let fields = self.params()?;
        self.expect(&Tok::Newline, "end of line")?;
        Ok(Item::Event { name, fields, pos })
    }

    fn func(&mut self) -> PResult<FuncDecl> {
        let t = self.advance();
        let pos = t.pos;
        let kind = match t.tok {
            Tok::Init => FuncKind::Init,
            Tok::Action => FuncKind::Action,
            Tok::View => FuncKind::View,
            Tok::Ident(_) => FuncKind::Upgrade,
            _ => FuncKind::Fn,
        };
        let name = match kind {
            FuncKind::Init => "init".to_string(),
            FuncKind::Upgrade => "upgrade".to_string(),
            _ => self.ident("function name")?.0,
        };
        let params = self.params()?;
        // Version 2 accepts `payable` before or after the return type.
        let mut payable = self.eat(&Tok::Payable);
        let ret = if self.eat(&Tok::Arrow) { Some(self.type_expr()?) } else { None };
        payable |= self.eat(&Tok::Payable);
        let mut only = Vec::new();
        if self.word("only") {
            self.advance();
            loop {
                let (who, wpos) = self.ident("a role or an address state variable after 'only'")?;
                only.push((who, wpos));
                if !self.eat(&Tok::Comma) {
                    break;
                }
            }
        }
        self.expect(&Tok::Colon, "':' at the end of the function header")?;
        let body = self.block()?;
        Ok(FuncDecl { kind, name, params, ret, payable, only, body, pos })
    }

    fn block(&mut self) -> PResult<Vec<Stmt>> {
        self.expect(&Tok::Newline, "end of line after ':'")?;
        self.skip_newlines();
        self.expect(&Tok::Indent, "an indented block")?;
        self.enter()?;
        let mut stmts = Vec::new();
        loop {
            self.skip_newlines();
            if self.eat(&Tok::Dedent) || self.check(&Tok::Eof) {
                break;
            }
            stmts.push(self.stmt()?);
        }
        self.leave();
        if stmts.is_empty() {
            return Err(CompileError::new(self.pos(), "empty block (use 'pass')"));
        }
        Ok(stmts)
    }

    fn end_stmt(&mut self) -> PResult<()> {
        if self.check(&Tok::Dedent) || self.check(&Tok::Eof) {
            return Ok(());
        }
        self.expect(&Tok::Newline, "end of line")?;
        Ok(())
    }

    fn stmt(&mut self) -> PResult<Stmt> {
        let pos = self.pos();
        let s = match self.peek().clone() {
            Tok::Let => {
                self.advance();
                let (name, _) = self.ident("variable name")?;
                self.expect(&Tok::Colon, "':' and a type (variables must declare their type)")?;
                let ty = self.type_expr()?;
                self.expect(&Tok::Assign, "'=' and an initial value")?;
                let value = self.expr()?;
                self.end_stmt()?;
                Stmt::Let { name, ty, value, pos }
            }
            Tok::If => {
                self.advance();
                let mut branches = Vec::new();
                let cond = self.expr()?;
                self.expect(&Tok::Colon, "':'")?;
                branches.push((cond, self.block()?));
                let mut els = None;
                loop {
                    self.skip_newlines();
                    if self.eat(&Tok::Elif) {
                        let c = self.expr()?;
                        self.expect(&Tok::Colon, "':'")?;
                        branches.push((c, self.block()?));
                    } else if self.eat(&Tok::Else) {
                        self.expect(&Tok::Colon, "':'")?;
                        els = Some(self.block()?);
                        break;
                    } else {
                        break;
                    }
                }
                return Ok(Stmt::If { branches, els, pos });
            }
            Tok::While => {
                self.advance();
                let cond = self.expr()?;
                self.expect(&Tok::Colon, "':'")?;
                let body = self.block()?;
                return Ok(Stmt::While { cond, body, pos });
            }
            Tok::For => {
                self.advance();
                let (var, _) = self.ident("loop variable")?;
                self.expect(&Tok::In, "'in'")?;
                let iter = self.expr()?;
                self.expect(&Tok::Colon, "':'")?;
                let body = self.block()?;
                return Ok(match iter {
                    Expr::Call(name, mut args, _) if name == "range" => {
                        if args.len() != 2 {
                            return Err(CompileError::new(pos, "range needs two arguments: range(start, end)"));
                        }
                        let end = args.pop().expect("2 args");
                        let start = args.pop().expect("2 args");
                        Stmt::ForRange { var, start, end, body, pos }
                    }
                    other => Stmt::ForEach { var, iter: other, body, pos },
                });
            }
            Tok::Break => {
                self.advance();
                self.end_stmt()?;
                Stmt::Break(pos)
            }
            Tok::Continue => {
                self.advance();
                self.end_stmt()?;
                Stmt::Continue(pos)
            }
            Tok::Pass => {
                self.advance();
                self.end_stmt()?;
                Stmt::Pass(pos)
            }
            Tok::Return => {
                self.advance();
                let value = if self.check(&Tok::Newline) || self.check(&Tok::Dedent) { None } else { Some(self.expr()?) };
                self.end_stmt()?;
                Stmt::Return(value, pos)
            }
            Tok::Require => {
                self.advance();
                let cond = self.expr()?;
                let msg = if self.eat(&Tok::Comma) { Some(self.expr()?) } else { None };
                self.end_stmt()?;
                Stmt::Require(cond, msg, pos)
            }
            Tok::Send => {
                self.advance();
                self.expect(&Tok::LParen, "'(' after send")?;
                let to = self.expr()?;
                self.expect(&Tok::Comma, "',' between address and amount")?;
                let amount = self.expr()?;
                self.expect(&Tok::RParen, "')'")?;
                self.end_stmt()?;
                Stmt::Send(to, amount, pos)
            }
            Tok::Emit => {
                self.advance();
                let (name, _) = self.ident("event name")?;
                let args = self.call_args()?;
                self.end_stmt()?;
                Stmt::Emit(name, args, pos)
            }
            Tok::Destroy => {
                self.advance();
                self.expect(&Tok::LParen, "'(' after destroy")?;
                let to = self.expr()?;
                self.expect(&Tok::RParen, "')'")?;
                self.end_stmt()?;
                Stmt::Destroy(to, pos)
            }
            Tok::Ident(w)
                if (w == "grant" || w == "revoke")
                    && matches!(self.peek_at(1), Tok::Ident(_))
                    && is_word(self.peek_at(2), if w == "grant" { "to" } else { "from" }) =>
            {
                self.advance();
                let (role, _) = self.ident("role name")?;
                self.advance();
                let who = self.expr()?;
                self.end_stmt()?;
                if w == "grant" {
                    Stmt::Grant { role, who, pos }
                } else {
                    Stmt::Revoke { role, who, pos }
                }
            }
            _ => {
                let target = self.expr()?;
                let op = match self.peek() {
                    Tok::Assign => Some(AssignOp::Set),
                    Tok::PlusAssign => Some(AssignOp::Add),
                    Tok::MinusAssign => Some(AssignOp::Sub),
                    Tok::StarAssign => Some(AssignOp::Mul),
                    _ => None,
                };
                if let Some(op) = op {
                    self.advance();
                    let value = self.expr()?;
                    self.end_stmt()?;
                    Stmt::Assign { target, op, value, pos }
                } else {
                    self.end_stmt()?;
                    Stmt::Expr(target, pos)
                }
            }
        };
        Ok(s)
    }

    fn call_args(&mut self) -> PResult<Vec<Expr>> {
        self.expect(&Tok::LParen, "'('")?;
        let mut args = Vec::new();
        if !self.check(&Tok::RParen) {
            loop {
                args.push(self.expr()?);
                if !self.eat(&Tok::Comma) {
                    break;
                }
            }
        }
        self.expect(&Tok::RParen, "')'")?;
        Ok(args)
    }

    /// `(name: ...` — named arguments build a record.
    fn named_args_ahead(&self) -> bool {
        matches!(self.peek(), Tok::LParen) && matches!(self.peek_at(1), Tok::Ident(_)) && matches!(self.peek_at(2), Tok::Colon)
    }

    fn named_args(&mut self) -> PResult<Vec<(String, Expr, Pos)>> {
        self.expect(&Tok::LParen, "'('")?;
        let mut out = Vec::new();
        loop {
            let (name, pos) = self.ident("field name")?;
            self.expect(&Tok::Colon, "':' after the field name")?;
            let value = self.expr()?;
            out.push((name, value, pos));
            if !self.eat(&Tok::Comma) || self.check(&Tok::RParen) {
                break;
            }
        }
        self.expect(&Tok::RParen, "')'")?;
        Ok(out)
    }

    fn expr(&mut self) -> PResult<Expr> {
        self.enter()?;
        let r = self.or_expr();
        self.leave();
        let e = r?;
        // Operator chains build left-deep trees without recursing here, but the
        // checker and the VM walk trees recursively: bound the depth.
        if expr_depth(&e) > MAX_EXPR_DEPTH {
            return Err(CompileError::new(
                e.pos(),
                format!("expression too deeply nested (max depth {MAX_EXPR_DEPTH}); split it with 'let'"),
            ));
        }
        Ok(e)
    }

    /// Counts one more operator in a chain like `a + b + c`.
    fn chain_step(&mut self, count: &mut usize) -> PResult<()> {
        *count += 1;
        if *count > MAX_CHAIN {
            return Err(CompileError::new(
                self.pos(),
                format!("too many operators in one expression (max {MAX_CHAIN}); split it with 'let'"),
            ));
        }
        Ok(())
    }

    fn or_expr(&mut self) -> PResult<Expr> {
        let mut l = self.and_expr()?;
        let mut count = 0;
        while self.check(&Tok::Or) {
            self.chain_step(&mut count)?;
            let pos = self.advance().pos;
            let r = self.and_expr()?;
            l = Expr::Binary(BinOp::Or, Box::new(l), Box::new(r), pos);
        }
        Ok(l)
    }

    fn and_expr(&mut self) -> PResult<Expr> {
        let mut l = self.not_expr()?;
        let mut count = 0;
        while self.check(&Tok::And) {
            self.chain_step(&mut count)?;
            let pos = self.advance().pos;
            let r = self.not_expr()?;
            l = Expr::Binary(BinOp::And, Box::new(l), Box::new(r), pos);
        }
        Ok(l)
    }

    fn not_expr(&mut self) -> PResult<Expr> {
        if self.check(&Tok::Not) {
            let pos = self.advance().pos;
            self.enter()?;
            let e = self.not_expr();
            self.leave();
            return Ok(Expr::Unary(UnOp::Not, Box::new(e?), pos));
        }
        self.cmp_expr()
    }

    fn cmp_expr(&mut self) -> PResult<Expr> {
        let l = self.add_expr()?;
        let op = match self.peek() {
            Tok::Eq => BinOp::Eq,
            Tok::Ne => BinOp::Ne,
            Tok::Lt => BinOp::Lt,
            Tok::Le => BinOp::Le,
            Tok::Gt => BinOp::Gt,
            Tok::Ge => BinOp::Ge,
            _ => return Ok(l),
        };
        let pos = self.advance().pos;
        let r = self.add_expr()?;
        if matches!(self.peek(), Tok::Eq | Tok::Ne | Tok::Lt | Tok::Le | Tok::Gt | Tok::Ge) {
            return Err(CompileError::new(self.pos(), "chained comparisons are not allowed; use 'and'"));
        }
        Ok(Expr::Binary(op, Box::new(l), Box::new(r), pos))
    }

    fn add_expr(&mut self) -> PResult<Expr> {
        let mut l = self.mul_expr()?;
        let mut count = 0;
        loop {
            let op = match self.peek() {
                Tok::Plus => BinOp::Add,
                Tok::Minus => BinOp::Sub,
                _ => return Ok(l),
            };
            self.chain_step(&mut count)?;
            let pos = self.advance().pos;
            let r = self.mul_expr()?;
            l = Expr::Binary(op, Box::new(l), Box::new(r), pos);
        }
    }

    fn mul_expr(&mut self) -> PResult<Expr> {
        let mut l = self.unary_expr()?;
        let mut count = 0;
        loop {
            let op = match self.peek() {
                Tok::Star => BinOp::Mul,
                Tok::Slash => BinOp::Div,
                Tok::Percent => BinOp::Rem,
                _ => return Ok(l),
            };
            self.chain_step(&mut count)?;
            let pos = self.advance().pos;
            let r = self.unary_expr()?;
            l = Expr::Binary(op, Box::new(l), Box::new(r), pos);
        }
    }

    fn unary_expr(&mut self) -> PResult<Expr> {
        if self.check(&Tok::Minus) {
            let pos = self.advance().pos;
            self.enter()?;
            let e = self.unary_expr();
            self.leave();
            let e = e?;
            if let Expr::Int(v, _) = e {
                return Ok(Expr::Int(-v, pos));
            }
            return Ok(Expr::Unary(UnOp::Neg, Box::new(e), pos));
        }
        self.postfix_expr()
    }

    fn postfix_expr(&mut self) -> PResult<Expr> {
        let mut e = self.primary()?;
        let mut chain = 0;
        loop {
            chain += 1;
            if chain > MAX_NESTING {
                return Err(CompileError::new(self.pos(), "expression chain too long"));
            }
            if self.check(&Tok::LBracket) {
                let pos = self.advance().pos;
                let idx = self.expr()?;
                self.expect(&Tok::RBracket, "']'")?;
                e = Expr::Index(Box::new(e), Box::new(idx), pos);
            } else if self.check(&Tok::Dot) {
                let pos = self.advance().pos;
                let (name, _) = self.ident("field or method name")?;
                if self.check(&Tok::LParen) {
                    if self.named_args_ahead() {
                        let fields = self.named_args()?;
                        e = Expr::Construct(Box::new(Expr::Field(Box::new(e), name, pos)), fields, pos);
                    } else {
                        let args = self.call_args()?;
                        e = Expr::Method(Box::new(e), name, args, pos);
                    }
                } else {
                    e = Expr::Field(Box::new(e), name, pos);
                }
            } else if self.word("with") && is_word(self.peek_at(1), "value") && matches!(e, Expr::Method(..)) {
                let pos = self.advance().pos;
                self.advance();
                let v = self.add_expr()?;
                return Ok(Expr::WithValue(Box::new(e), Box::new(v), pos));
            } else {
                return Ok(e);
            }
        }
    }

    fn primary(&mut self) -> PResult<Expr> {
        let pos = self.pos();
        match self.peek().clone() {
            Tok::Int(v) => {
                self.advance();
                Ok(Expr::Int(v, pos))
            }
            Tok::Text(s) => {
                self.advance();
                Ok(Expr::Text(s, pos))
            }
            Tok::Bytes(b) => {
                self.advance();
                Ok(Expr::Bytes(b, pos))
            }
            Tok::True => {
                self.advance();
                Ok(Expr::Bool(true, pos))
            }
            Tok::False => {
                self.advance();
                Ok(Expr::Bool(false, pos))
            }
            Tok::Ident(name) => {
                self.advance();
                if self.check(&Tok::LParen) && self.named_args_ahead() {
                    let fields = self.named_args()?;
                    Ok(Expr::Construct(Box::new(Expr::Name(name, pos)), fields, pos))
                } else if self.check(&Tok::LParen) {
                    let args = self.call_args()?;
                    Ok(Expr::Call(name, args, pos))
                } else {
                    Ok(Expr::Name(name, pos))
                }
            }
            Tok::LParen => {
                self.advance();
                let e = self.expr()?;
                self.expect(&Tok::RParen, "')'")?;
                Ok(e)
            }
            Tok::LBracket => {
                self.advance();
                let mut items = Vec::new();
                if !self.check(&Tok::RBracket) {
                    loop {
                        items.push(self.expr()?);
                        if !self.eat(&Tok::Comma) {
                            break;
                        }
                    }
                }
                self.expect(&Tok::RBracket, "']'")?;
                Ok(Expr::List(items, pos))
            }
            other => Err(CompileError::new(pos, format!("expected an expression, found {}", describe(&other)))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_contract() {
        let src = r#"
contract Bank

const LIMIT: int = 10 * 100_000_000
state owner: address
state balances: map[address, int]
event Deposited(who: address, amount: int)

init():
    owner = caller

action deposit() payable:
    require value > 0, "send TCN"
    balances[caller] += value
    emit Deposited(caller, value)

view balance_of(who: address) -> int:
    return balances[who]

fn double(x: int) -> int:
    if x > 0 and not false:
        return x * 2
    elif x == 0:
        return 0
    else:
        return -x
"#;
        let c = parse_file(src).unwrap();
        assert_eq!(c.units[0].name, "Bank");
        assert_eq!(c.units[0].items.len(), 8);
    }

    #[test]
    fn hostile_nesting_is_rejected_without_overflow() {
        let body = |e: String| format!("contract Deep\n\nview f() -> int:\n    return {e}\n");
        // Long operator chains, deep parentheses, nested types, unary chains.
        let cases = [
            body(format!("1{}", "+1".repeat(20_000))),
            body(format!("{}1{}", "(".repeat(10_000), ")".repeat(10_000))),
            body(format!("{}1", "-".repeat(10_000))),
            format!("contract Deep\nstate x: {}int{}\n", "list[".repeat(7_000), "]".repeat(7_000)),
            body((0..30).fold("1".to_string(), |e, _| format!("({e}{})", "+1".repeat(60)))),
        ];
        for src in cases {
            let src2 = src.clone();
            let result = std::thread::Builder::new().stack_size(1024 * 1024).spawn(move || parse_file(&src2).is_err()).unwrap().join();
            assert_eq!(result.ok(), Some(true), "must fail cleanly: {}", &src[..60.min(src.len())]);
        }
        assert!(parse_file(&body(format!("1{}", "+1".repeat(MAX_CHAIN)))).is_ok());
    }

    #[test]
    fn errors_have_positions() {
        let e = parse_file("contract A\naction f(:\n    pass\n").unwrap_err();
        assert_eq!(e.pos.line, 2);
        assert!(parse_file("contract A\naction f():\n").is_err());
        assert!(parse_file("contract A\naction f():\n    let x = 1\n").unwrap_err().message.contains("type"));
        let deep = format!("contract A\nview f() -> int:\n    return {}1{}\n", "(".repeat(100), ")".repeat(100));
        assert!(parse_file(&deep).unwrap_err().message.contains("nesting"));
        assert!(parse_file("contract A\nview f() -> bool:\n    return 1 < 2 < 3\n").is_err());
    }
}
