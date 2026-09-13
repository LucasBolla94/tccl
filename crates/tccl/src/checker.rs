//! Type checker and compiler for language version 2: syntax tree → [`Program`].
//!
//! Version 2 is a strict superset of version 1 (see [`crate::v1`], frozen):
//! every valid version 1 contract compiles to the same functions, states and
//! events, and runs with the same results and fuel. On top of version 1 it adds:
//!
//! * `record` — typed records with named fields;
//! * `enum` — named situations, optionally with **allowed transitions** checked
//!   whenever a value is stored;
//! * `role`, `grant`, `revoke` and `only` — explicit permissions;
//! * `interface` and calls to other contracts (with `with value`), with the
//!   caller identity, re-entrancy protection and atomicity enforced by the VM;
//! * `use` — modules (compile-time libraries, e.g. `use std.token`);
//! * `upgrade()` — code that runs once when an upgrade installs this program;
//! * built-ins `mul_div`, `isqrt`, `pow`, `code_hash`, `is_contract`, `is_final`
//!   and the context value `origin`.
//!
//! The rules of version 1 still hold: every name has a declared type, no implicit
//! conversions, views cannot change anything, only payable functions receive TCN,
//! maps live only in state.

use crate::ast::{self, AssignOp, BinOp, FuncKind, Item, TypeExpr, UnOp, UnitKind};
use crate::error::{CompileError, Pos};
use crate::ops;
use crate::program::*;
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_LOCALS: usize = 1_024;
pub const MAX_FUNCTIONS: usize = 256;
pub const MAX_STATE_VARS: usize = 256;
pub const MAX_RECORDS: usize = 128;
pub const MAX_FIELDS: usize = 64;
pub const MAX_ENUMS: usize = 128;
pub const MAX_VARIANTS: usize = 256;
pub const MAX_INTERFACES: usize = 64;
pub const MAX_MODULES: usize = 16;
pub const TCN: i128 = 100_000_000;

/// Compiler options.
#[derive(Clone, Debug)]
pub struct CompileOptions {
    /// Accepted address prefixes for `address("...")` literals.
    pub address_prefixes: Vec<String>,
    /// Language version to compile (1 = frozen version 1 compiler, 2 = current).
    pub version: u16,
    /// State variable names of the program being upgraded, in their stored order.
    /// Variables that still exist keep their storage slots; new ones are appended.
    pub state_order: Vec<String>,
}

impl Default for CompileOptions {
    fn default() -> Self {
        CompileOptions { address_prefixes: vec!["tc".into(), "tct".into(), "tcr".into()], version: LANGUAGE_VERSION, state_order: Vec::new() }
    }
}

impl CompileOptions {
    /// Options for one network (`tc`, `tct` or `tcr`) and language version.
    pub fn network(prefix: &str, version: u16) -> Self {
        CompileOptions { address_prefixes: vec![prefix.to_string()], version, state_order: Vec::new() }
    }
}

type CResult<T> = Result<T, CompileError>;

fn err<T>(pos: Pos, msg: impl Into<String>) -> CResult<T> {
    Err(CompileError::new(pos, msg))
}

/// Compiles a source file with the language version selected in `opts`.
pub fn compile(src: &str, opts: &CompileOptions) -> CResult<Program> {
    match opts.version {
        1 => crate::v1::checker::compile(src, &crate::v1::checker::CompileOptions { address_prefixes: opts.address_prefixes.clone() }),
        2 => compile_v2(src, opts),
        v => err(Pos::default(), format!("unsupported language version {v} (this compiler supports {MIN_LANGUAGE_VERSION} to {LANGUAGE_VERSION})")),
    }
}

const RESERVED: &[&str] = &[
    "caller",
    "value",
    "balance",
    "height",
    "self",
    "TCN",
    "int",
    "bool",
    "text",
    "bytes",
    "address",
    "list",
    "map",
    "len",
    "sha256",
    "blake3",
    "to_bytes",
    "to_text",
    "to_int",
    "min",
    "max",
    "abs",
    "slice",
    "verify_ed25519",
    "ring_verify",
    "address_of",
    "zero_address",
    "range",
];

/// Version 2 built-ins. They are not reserved (a version 1 contract may already use
/// these names): a declaration with the same name takes precedence.
const V2_BUILTINS: &[&str] = &["mul_div", "isqrt", "pow", "code_hash", "is_contract", "is_final"];

/// Edit distance for "did you mean" suggestions.
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            cur.push((prev[j] + usize::from(ca != *cb)).min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}

fn suggest<'a>(name: &str, candidates: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let limit = if name.len() <= 3 { 1 } else { 2 };
    candidates
        .into_iter()
        .filter(|c| *c != name)
        .map(|c| (distance(name, c), c))
        .filter(|(d, _)| *d <= limit)
        .min()
        .map(|(_, c)| format!("did you mean '{c}'?"))
}

struct Sig {
    kind: FuncKind,
    params: Vec<Type>,
    ret: Type,
    unit: Option<String>,
    /// Name in the compiled program (plain for entry points and contract helpers,
    /// `module.name` for module helpers).
    program_name: String,
    only: Vec<AccessRule>,
    only_names: Vec<String>,
}

struct FnCtx {
    kind: FuncKind,
    ret: Type,
    scopes: Vec<BTreeMap<String, (u16, Type)>>,
    next_slot: usize,
    loop_depth: usize,
    mutates: bool,
    calls: BTreeSet<u16>,
    unit: Option<String>,
    fx: Effects,
}

struct Checker<'o> {
    opts: &'o CompileOptions,
    consts: BTreeMap<String, (Type, Value)>,
    states: Vec<StateVar>,
    state_index: BTreeMap<String, u16>,
    state_unit: Vec<Option<String>>,
    events: Vec<EventDef>,
    event_index: BTreeMap<String, u16>,
    sigs: Vec<Sig>,
    fn_index: BTreeMap<String, u16>,
    entry_names: BTreeSet<String>,
    records: Vec<RecordDef>,
    record_index: BTreeMap<String, u16>,
    enums: Vec<EnumDef>,
    enum_index: BTreeMap<String, u16>,
    interfaces: Vec<InterfaceDef>,
    iface_index: BTreeMap<String, u16>,
    roles: Vec<RoleDef>,
    role_index: BTreeMap<String, u16>,
    aliases: BTreeMap<String, String>,
    modules: BTreeSet<String>,
    role_events: Option<(u16, u16)>,
}

fn qualify(unit: &Option<String>, name: &str) -> String {
    match unit {
        Some(m) => format!("{m}.{name}"),
        None => name.to_string(),
    }
}

/// The lines of `src` that belong to the unit starting at `start` (1-based) and
/// ending before `end`.
fn unit_text(src: &str, start: u32, end: Option<u32>) -> String {
    src.split('\n')
        .enumerate()
        .filter(|(i, _)| (*i as u32 + 1) >= start && end.is_none_or(|e| (*i as u32 + 1) < e))
        .map(|(_, l)| l)
        .collect::<Vec<_>>()
        .join("\n")
}

fn compile_v2(src: &str, opts: &CompileOptions) -> CResult<Program> {
    let file = crate::parser::parse_file(src)?;
    let starts: Vec<u32> = file.units.iter().map(|u| u.pos.line).collect();
    let mut units = file.units.into_iter().enumerate();
    let Some((_, contract)) = units.next() else { return err(Pos::default(), "empty source") };
    if contract.kind != UnitKind::Contract {
        return Err(CompileError::new(contract.pos, "this file only contains modules; a deployable file starts with 'contract <Name>'")
            .with_help("check a library with `tccl check --library <file>`"));
    }
    let mut inline: BTreeMap<String, (ast::Unit, [u8; 32])> = BTreeMap::new();
    for (i, u) in units {
        let text = unit_text(src, starts[i], starts.get(i + 1).copied());
        if inline.contains_key(&u.name) {
            return err(u.pos, format!("module '{}' is declared twice", u.name));
        }
        inline.insert(u.name.clone(), (u, *blake3::hash(text.as_bytes()).as_bytes()));
    }

    let mut c = Checker {
        opts,
        consts: BTreeMap::new(),
        states: Vec::new(),
        state_index: BTreeMap::new(),
        state_unit: Vec::new(),
        events: Vec::new(),
        event_index: BTreeMap::new(),
        sigs: Vec::new(),
        fn_index: BTreeMap::new(),
        entry_names: BTreeSet::new(),
        records: Vec::new(),
        record_index: BTreeMap::new(),
        enums: Vec::new(),
        enum_index: BTreeMap::new(),
        interfaces: Vec::new(),
        iface_index: BTreeMap::new(),
        roles: Vec::new(),
        role_index: BTreeMap::new(),
        aliases: BTreeMap::new(),
        modules: BTreeSet::new(),
        role_events: None,
    };
    c.consts.insert("TCN".into(), (Type::Int, Value::Int(TCN)));

    // Resolve `use` declarations: standard library or modules in this file.
    let mut module_units: Vec<(ast::Unit, ModuleRef)> = Vec::new();
    for item in &contract.items {
        let Item::Use { path, alias, pos } = item else { continue };
        if module_units.len() >= MAX_MODULES {
            return err(*pos, format!("too many modules (max {MAX_MODULES})"));
        }
        let (unit, hash) = if let Some(rest) = path.strip_prefix("std.") {
            let Some(text) = crate::modules::std_source(rest) else {
                let names: Vec<&str> = crate::modules::STD_MODULES.iter().map(|(n, _)| *n).collect();
                return Err(CompileError::new(*pos, format!("unknown standard module 'std.{rest}'"))
                    .with_help(format!("available: {}", names.iter().map(|n| format!("std.{n}")).collect::<Vec<_>>().join(", "))));
            };
            let parsed = crate::parser::parse_file(text).map_err(|e| CompileError::new(*pos, format!("std.{rest}: {e}")))?;
            let unit = parsed.units.into_iter().next().expect("std module has one unit");
            (unit, *blake3::hash(text.as_bytes()).as_bytes())
        } else {
            if path.contains('.') {
                return err(*pos, format!("unknown module '{path}' (standard modules start with 'std.')"));
            }
            match inline.remove(path) {
                Some(found) => found,
                None => {
                    return Err(CompileError::new(*pos, format!("module '{path}' not found in this file"))
                        .with_help(format!("add a section 'module {path}' at the end of the file, or run `tccl bundle` to include {path}.tccl")))
                }
            }
        };
        let name = unit.name.clone();
        if c.modules.contains(&name) {
            return err(*pos, format!("module '{name}' is used twice"));
        }
        let alias = alias.clone().unwrap_or_else(|| name.clone());
        if RESERVED.contains(&alias.as_str()) || c.aliases.contains_key(&alias) {
            return err(*pos, format!("module alias '{alias}' is reserved or already used"));
        }
        c.aliases.insert(alias, name.clone());
        c.modules.insert(name.clone());
        module_units.push((unit, ModuleRef { name: path.clone(), source_hash: hash }));
    }
    if let Some((name, (u, _))) = inline.iter().next() {
        return Err(CompileError::new(u.pos, format!("module '{name}' is declared but never used")).with_help(format!("add 'use {name}' to the contract")));
    }
    for (u, _) in &module_units {
        for item in &u.items {
            match item {
                Item::Use { pos, .. } => return err(*pos, "modules cannot use other modules; add the 'use' to the contract"),
                Item::Func(f) if f.kind == FuncKind::Init || f.kind == FuncKind::Upgrade => {
                    return Err(CompileError::new(f.pos, "modules cannot declare init() or upgrade()")
                        .with_help("export a 'fn setup(...)' from the module and call it from the contract's init()"))
                }
                _ => {}
            }
        }
    }

    let mut all: Vec<(Option<String>, &ast::Unit)> = vec![(None, &contract)];
    for (u, _) in &module_units {
        all.push((Some(u.name.clone()), u));
    }
    c.declare_types(&all)?;
    let decls = c.declare_items(&all)?;
    c.order_states()?;
    if !decls.iter().any(|(_, f)| matches!(f.kind, FuncKind::Action | FuncKind::View | FuncKind::Init)) {
        return err(contract.pos, "a contract needs at least one init, action or view");
    }

    // Pass 2: bodies.
    let mut functions = Vec::new();
    let mut call_graph = Vec::new();
    let mut effects = Effects::default();
    let mut direct_fx = Vec::new();
    for (unit, f) in &decls {
        let (func, calls, fx) = c.function(unit, f)?;
        functions.push(func);
        call_graph.push(calls);
        direct_fx.push(fx);
    }
    loop {
        let mut changed = false;
        for i in 0..functions.len() {
            if !functions[i].mutates && call_graph[i].iter().any(|c| functions[*c as usize].mutates) {
                functions[i].mutates = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    for (f, (_, decl)) in functions.iter().zip(&decls) {
        if f.kind == FnKind::View && f.mutates {
            return Err(CompileError::new(
                decl.pos,
                format!("view '{}' changes state, sends TCN or emits events (directly or through a fn it calls)", f.name),
            )
            .with_help("make it an 'action', or move the change into an action"));
        }
    }
    for (f, fx) in functions.iter().zip(&direct_fx) {
        effects.changes_state |= f.mutates;
        effects.receives_tcn |= f.payable;
        effects.sends_tcn |= fx.sends_tcn;
        effects.emits_events |= fx.emits_events;
        effects.calls_contracts |= fx.calls_contracts;
        effects.can_destroy |= fx.can_destroy;
        effects.has_upgrade_hook |= f.kind == FnKind::Upgrade;
    }
    let access = c
        .sigs
        .iter()
        .enumerate()
        .filter(|(_, s)| !s.only.is_empty())
        .map(|(i, s)| FunctionAccess { function: i as u16, rules: s.only.clone() })
        .collect();
    let mut program = Program::empty(2, &contract.name);
    program.states = c.states;
    program.events = c.events;
    program.functions = functions;
    program.records = c.records;
    program.enums = c.enums;
    program.interfaces = c.interfaces;
    program.roles = c.roles;
    program.access = access;
    program.modules = module_units.into_iter().map(|(_, m)| m).collect();
    program.effects = effects;
    Ok(program)
}

impl<'o> Checker<'o> {
    fn name_taken(&self, q: &str) -> bool {
        self.consts.contains_key(q)
            || self.state_index.contains_key(q)
            || self.event_index.contains_key(q)
            || self.fn_index.contains_key(q)
            || self.record_index.contains_key(q)
            || self.enum_index.contains_key(q)
            || self.iface_index.contains_key(q)
            || self.role_index.contains_key(q)
    }

    fn check_new_name(&self, unit: &Option<String>, name: &str, pos: Pos) -> CResult<()> {
        if RESERVED.contains(&name) {
            return err(pos, format!("'{name}' is a reserved name"));
        }
        if name.contains('.') {
            return err(pos, format!("invalid name '{name}'"));
        }
        if self.name_taken(&qualify(unit, name)) {
            return err(pos, format!("'{name}' is already declared"));
        }
        if unit.is_none() && self.aliases.contains_key(name) {
            return err(pos, format!("'{name}' is the name of a module used by this contract"));
        }
        Ok(())
    }

    /// Qualified name of `path` seen from `unit`: `x` → `unit.x`; from the contract,
    /// `alias.x` → `module.x`.
    fn resolve_path(&self, unit: &Option<String>, path: &str) -> Option<String> {
        match path.split_once('.') {
            None => Some(qualify(unit, path)),
            Some((alias, rest)) if unit.is_none() && !rest.contains('.') => self.aliases.get(alias).map(|m| format!("{m}.{rest}")),
            _ => None,
        }
    }

    fn type_names(&self, unit: &Option<String>) -> Vec<String> {
        let prefix = unit.as_ref().map(|m| format!("{m}.")).unwrap_or_default();
        let mut names: Vec<String> = ["int", "bool", "text", "bytes", "address"].iter().map(|s| s.to_string()).collect();
        for k in self.record_index.keys().chain(self.enum_index.keys()).chain(self.iface_index.keys()) {
            if let Some(rest) = k.strip_prefix(&prefix) {
                if unit.is_some() || !rest.contains('.') {
                    names.push(rest.to_string());
                }
            }
        }
        names
    }

    fn resolve_type(&self, unit: &Option<String>, t: &TypeExpr, allow_map: bool) -> CResult<Type> {
        Ok(match t {
            TypeExpr::Named(n, pos) => match n.as_str() {
                "int" => Type::Int,
                "bool" => Type::Bool,
                "text" => Type::Text,
                "bytes" => Type::Bytes,
                "address" => Type::Address,
                other => {
                    let q = self.resolve_path(unit, other);
                    if let Some(q) = q {
                        if let Some(i) = self.record_index.get(&q) {
                            return Ok(Type::Record(*i));
                        }
                        if let Some(i) = self.enum_index.get(&q) {
                            return Ok(Type::Enum(*i));
                        }
                        if let Some(i) = self.iface_index.get(&q) {
                            return Ok(Type::Interface(*i));
                        }
                    }
                    let names = self.type_names(unit);
                    let mut e = CompileError::new(
                        *pos,
                        format!("unknown type '{other}' (types: int, bool, text, bytes, address, list[T], map[K, V], records, enums, interfaces)"),
                    );
                    if let Some(h) = suggest(other, names.iter().map(String::as_str)) {
                        e = e.with_help(h);
                    }
                    return Err(e);
                }
            },
            TypeExpr::List(inner, _) => Type::List(Box::new(self.resolve_type(unit, inner, false)?)),
            TypeExpr::Map(k, v, pos) => {
                if !allow_map {
                    return err(*pos, "maps can only be used as state variables");
                }
                let kt = self.resolve_type(unit, k, false)?;
                if !kt.is_key() && !matches!(kt, Type::Enum(_)) {
                    return err(*pos, format!("map keys must be int, bool, text, bytes, address or an enum, not {}", self.tname(&kt)));
                }
                Type::Map(Box::new(kt), Box::new(self.resolve_type(unit, v, false)?))
            }
        })
    }

    fn tname(&self, t: &Type) -> String {
        match t {
            Type::Record(i) => self.records[*i as usize].name.clone(),
            Type::Enum(i) => self.enums[*i as usize].name.clone(),
            Type::Interface(i) => self.interfaces[*i as usize].name.clone(),
            Type::List(inner) => format!("list[{}]", self.tname(inner)),
            Type::Map(k, v) => format!("map[{}, {}]", self.tname(k), self.tname(v)),
            other => other.to_string(),
        }
    }

    // ---- pass 1a: records, enums, interfaces ----

    fn declare_types(&mut self, units: &[(Option<String>, &ast::Unit)]) -> CResult<()> {
        let display = |unit: &Option<String>, n: &str| match unit {
            Some(m) => format!("{m}.{n}"),
            None => n.to_string(),
        };
        for (unit, u) in units {
            for item in &u.items {
                match item {
                    Item::Record { name, pos, .. } => {
                        self.check_new_name(unit, name, *pos)?;
                        if self.records.len() >= MAX_RECORDS {
                            return err(*pos, "too many records");
                        }
                        self.record_index.insert(qualify(unit, name), self.records.len() as u16);
                        self.records.push(RecordDef { name: display(unit, name), fields: Vec::new() });
                    }
                    Item::Enum { name, variants, pos, .. } => {
                        self.check_new_name(unit, name, *pos)?;
                        if self.enums.len() >= MAX_ENUMS {
                            return err(*pos, "too many enums");
                        }
                        if variants.len() > MAX_VARIANTS {
                            return err(*pos, format!("too many variants (max {MAX_VARIANTS})"));
                        }
                        self.enum_index.insert(qualify(unit, name), self.enums.len() as u16);
                        self.enums.push(EnumDef { name: display(unit, name), variants: Vec::new(), transitions: None });
                    }
                    Item::Interface { name, pos, .. } => {
                        self.check_new_name(unit, name, *pos)?;
                        if self.interfaces.len() >= MAX_INTERFACES {
                            return err(*pos, "too many interfaces");
                        }
                        self.iface_index.insert(qualify(unit, name), self.interfaces.len() as u16);
                        self.interfaces.push(InterfaceDef { name: display(unit, name), functions: Vec::new() });
                    }
                    _ => {}
                }
            }
        }
        for (unit, u) in units {
            for item in &u.items {
                match item {
                    Item::Record { name, fields, pos } => {
                        if fields.len() > MAX_FIELDS {
                            return err(*pos, format!("too many fields (max {MAX_FIELDS})"));
                        }
                        let mut seen = BTreeSet::new();
                        let mut fs = Vec::new();
                        for f in fields {
                            if !seen.insert(f.name.clone()) {
                                return err(f.pos, format!("duplicate field '{}'", f.name));
                            }
                            fs.push((f.name.clone(), self.resolve_type(unit, &f.ty, false)?));
                        }
                        let idx = self.record_index[&qualify(unit, name)];
                        self.records[idx as usize].fields = fs;
                    }
                    Item::Enum { name, variants, transitions, .. } => {
                        let idx = self.enum_index[&qualify(unit, name)];
                        let mut names: Vec<String> = Vec::new();
                        for v in variants {
                            if names.contains(&v.name) {
                                return err(v.pos, format!("duplicate variant '{}'", v.name));
                            }
                            names.push(v.name.clone());
                        }
                        let table = if *transitions {
                            let mut next = Vec::new();
                            for v in variants {
                                let mut row = Vec::new();
                                for (n, p) in &v.next {
                                    let Some(i) = names.iter().position(|x| x == n) else {
                                        let mut e = CompileError::new(*p, format!("'{n}' is not a variant of {name}"));
                                        if let Some(h) = suggest(n, names.iter().map(String::as_str)) {
                                            e = e.with_help(h);
                                        }
                                        return Err(e);
                                    };
                                    if n == &v.name {
                                        return err(*p, format!("'{n} -> {n}' is not needed: staying in the same variant is always allowed"));
                                    }
                                    if row.contains(&(i as u16)) {
                                        return err(*p, format!("'{n}' is listed twice"));
                                    }
                                    row.push(i as u16);
                                }
                                next.push(row);
                            }
                            Some(next)
                        } else {
                            None
                        };
                        self.enums[idx as usize].variants = names;
                        self.enums[idx as usize].transitions = table;
                    }
                    Item::Interface { name, functions, .. } => {
                        let idx = self.iface_index[&qualify(unit, name)];
                        let mut fs: Vec<InterfaceFnDef> = Vec::new();
                        for f in functions {
                            if fs.iter().any(|x| x.name == f.name) {
                                return err(f.pos, format!("'{}' is declared twice in interface {name}", f.name));
                            }
                            let mut params = Vec::new();
                            for p in &f.params {
                                let t = self.resolve_type(unit, &p.ty, false)?;
                                if !t.is_portable() {
                                    return Err(CompileError::new(p.pos, format!("interface parameters must be int, bool, text, bytes, address or lists of them, not {}", self.tname(&t)))
                                        .with_help("records, enums and interfaces are local to a contract; pass their fields instead"));
                                }
                                params.push((p.name.clone(), t));
                            }
                            let ret = match &f.ret {
                                Some(t) => {
                                    let t = self.resolve_type(unit, t, false)?;
                                    if !t.is_portable() {
                                        return err(f.ret.as_ref().unwrap().pos(), "interface return types must be int, bool, text, bytes, address or lists of them");
                                    }
                                    t
                                }
                                None => Type::Unit,
                            };
                            if f.kind == FuncKind::View && ret == Type::Unit {
                                return err(f.pos, "a view must declare a return type ('-> type')");
                            }
                            if f.payable && f.kind != FuncKind::Action {
                                return err(f.pos, "only actions can be payable");
                            }
                            fs.push(InterfaceFnDef {
                                name: f.name.clone(),
                                kind: if f.kind == FuncKind::View { FnKind::View } else { FnKind::Action },
                                payable: f.payable,
                                params,
                                ret,
                            });
                        }
                        self.interfaces[idx as usize].functions = fs;
                    }
                    _ => {}
                }
            }
        }
        // Records cannot contain themselves (their default value would be infinite).
        for start in 0..self.records.len() {
            let mut stack = vec![(start, 0usize)];
            while let Some((r, depth)) = stack.pop() {
                if depth > MAX_VALUE_DEPTH {
                    return err(Pos::default(), format!("record {} is nested too deeply", self.records[start].name));
                }
                for (_, t) in &self.records[r].fields {
                    let mut t = t;
                    while let Type::List(inner) = t {
                        t = inner;
                    }
                    if let Type::Record(inner) = t {
                        if *inner as usize == start {
                            return err(Pos::default(), format!("record {} contains itself", self.records[start].name));
                        }
                        stack.push((*inner as usize, depth + 1));
                    }
                }
            }
        }
        Ok(())
    }

    // ---- pass 1b: constants, state, roles, events, function signatures ----

    fn declare_items<'a>(&mut self, units: &[(Option<String>, &'a ast::Unit)]) -> CResult<Vec<(Option<String>, &'a ast::FuncDecl)>> {
        let mut decls = Vec::new();
        for (unit, u) in units {
            for item in &u.items {
                match item {
                    Item::Const { name, ty, value, pos } => {
                        self.check_new_name(unit, name, *pos)?;
                        let t = self.resolve_type(unit, ty, false)?;
                        if !(t.is_scalar() || matches!(t, Type::Enum(_))) {
                            return err(*pos, "constants must be int, bool, text, bytes, address or an enum");
                        }
                        let v = self.const_value(unit, value, &t)?;
                        self.consts.insert(qualify(unit, name), (t, v));
                    }
                    Item::State { name, ty, init, pos } => {
                        self.check_new_name(unit, name, *pos)?;
                        let t = self.resolve_type(unit, ty, true)?;
                        let init_v = match init {
                            Some(e) => {
                                if !(t.is_scalar() || matches!(t, Type::Enum(_))) {
                                    return err(*pos, "only int, bool, text, bytes, address and enum state variables can have an initial value");
                                }
                                Some(self.const_value(unit, e, &t)?)
                            }
                            None => None,
                        };
                        self.add_state(unit, name, t, init_v, *pos)?;
                    }
                    Item::Role { name, pos } => {
                        self.check_new_name(unit, name, *pos)?;
                        let var = self.add_state(unit, name, Type::Map(Box::new(Type::Address), Box::new(Type::Bool)), None, *pos)?;
                        // The state entry is registered under the role name; keep it out of the
                        // state namespace so it can only be used through grant/revoke/only/has.
                        let q = qualify(unit, name);
                        self.state_index.remove(&q);
                        self.role_index.insert(q, self.roles.len() as u16);
                        self.roles.push(RoleDef { name: qualify(unit, name), state: var });
                    }
                    Item::Event { name, fields, pos } => {
                        self.check_new_name(unit, name, *pos)?;
                        if self.events.iter().any(|e| &e.name == name) {
                            return err(*pos, format!("an event named '{name}' is already declared by the contract or a module"));
                        }
                        let mut fs = Vec::new();
                        for p in fields {
                            fs.push((p.name.clone(), self.resolve_type(unit, &p.ty, false)?));
                        }
                        self.event_index.insert(qualify(unit, name), self.events.len() as u16);
                        self.events.push(EventDef { name: name.clone(), fields: fs });
                    }
                    Item::Func(f) => {
                        self.declare_function(unit, f)?;
                        decls.push((unit.clone(), f));
                    }
                    Item::Record { .. } | Item::Enum { .. } | Item::Interface { .. } | Item::Use { .. } => {}
                }
            }
        }
        if !self.roles.is_empty() {
            for ev in ["RoleGranted", "RoleRevoked"] {
                if self.events.iter().any(|e| e.name == ev) {
                    return err(Pos::default(), format!("'{ev}' is declared automatically for contracts with roles; rename your event"));
                }
            }
            let fields = vec![("role".to_string(), Type::Text), ("account".to_string(), Type::Address), ("by".to_string(), Type::Address)];
            let g = self.events.len() as u16;
            self.events.push(EventDef { name: "RoleGranted".into(), fields: fields.clone() });
            self.events.push(EventDef { name: "RoleRevoked".into(), fields });
            self.role_events = Some((g, g + 1));
        }
        // `only` rules need every state variable and role declared.
        for i in 0..self.sigs.len() {
            let names = std::mem::take(&mut self.sigs[i].only_names);
            let unit = self.sigs[i].unit.clone();
            let mut rules = Vec::new();
            for n in &names {
                let q = qualify(&unit, n);
                if let Some(r) = self.role_index.get(&q) {
                    rules.push(AccessRule::Role(self.roles[*r as usize].state));
                } else if let Some(v) = self.state_index.get(&q) {
                    if self.states[*v as usize].ty != Type::Address {
                        return err(Pos::default(), format!("'only {n}': {n} must be a role or an address state variable"));
                    }
                    rules.push(AccessRule::Address(*v));
                } else {
                    let cands: Vec<String> = self.roles.iter().map(|r| r.name.clone()).collect();
                    let mut e = CompileError::new(Pos::default(), format!("'only {n}': unknown role or address state variable '{n}'"));
                    if let Some(h) = suggest(n, cands.iter().map(String::as_str)) {
                        e = e.with_help(h);
                    } else {
                        e = e.with_help(format!("declare it with 'role {n}' or 'state {n}: address'"));
                    }
                    return Err(e);
                }
            }
            self.sigs[i].only_names = names;
            self.sigs[i].only = rules;
        }
        Ok(decls)
    }

    fn add_state(&mut self, unit: &Option<String>, name: &str, t: Type, init: Option<Value>, pos: Pos) -> CResult<u16> {
        if self.states.len() >= MAX_STATE_VARS {
            return err(pos, "too many state variables");
        }
        let idx = self.states.len() as u16;
        self.state_index.insert(qualify(unit, name), idx);
        self.states.push(StateVar { name: qualify(unit, name), ty: t, init });
        self.state_unit.push(unit.clone());
        Ok(idx)
    }

    /// Upgrades keep the storage slot of every state variable that still exists.
    fn order_states(&mut self) -> CResult<()> {
        if self.opts.state_order.is_empty() {
            return Ok(());
        }
        let n = self.states.len();
        let mut order: Vec<usize> = Vec::with_capacity(n);
        for old in &self.opts.state_order {
            match self.states.iter().position(|s| &s.name == old) {
                Some(i) => order.push(i),
                None => {
                    return Err(CompileError::new(Pos::default(), format!("state variable '{old}' of the deployed version was removed"))
                        .with_help("upgrades cannot remove state variables; keep it (you may stop using it)"))
                }
            }
        }
        for i in 0..n {
            if !order.contains(&i) {
                order.push(i);
            }
        }
        let mut remap = vec![0u16; n];
        for (new_i, old_i) in order.iter().enumerate() {
            remap[*old_i] = new_i as u16;
        }
        self.states = order.iter().map(|i| self.states[*i].clone()).collect();
        self.state_unit = order.iter().map(|i| self.state_unit[*i].clone()).collect();
        for v in self.state_index.values_mut() {
            *v = remap[*v as usize];
        }
        for r in self.roles.iter_mut() {
            r.state = remap[r.state as usize];
        }
        for s in self.sigs.iter_mut() {
            for rule in s.only.iter_mut() {
                match rule {
                    AccessRule::Role(v) | AccessRule::Address(v) => *v = remap[*v as usize],
                }
            }
        }
        Ok(())
    }

    fn declare_function(&mut self, unit: &Option<String>, f: &ast::FuncDecl) -> CResult<()> {
        let exported = matches!(f.kind, FuncKind::Action | FuncKind::View);
        match f.kind {
            FuncKind::Init | FuncKind::Upgrade => {
                if self.fn_index.contains_key(&f.name) {
                    return err(f.pos, format!("only one {}() is allowed", f.name));
                }
            }
            _ => self.check_new_name(unit, &f.name, f.pos)?,
        }
        if exported && unit.is_some() && (self.entry_names.contains(&f.name) || self.name_taken(&f.name)) {
            return err(f.pos, format!("module {} exports '{}', which the contract or another module already declares", unit.as_ref().unwrap(), f.name));
        }
        if exported && unit.is_none() && self.entry_names.contains(&f.name) {
            return err(f.pos, format!("'{}' is already declared", f.name));
        }
        if self.sigs.len() >= MAX_FUNCTIONS {
            return err(f.pos, "too many functions");
        }
        if f.payable && !matches!(f.kind, FuncKind::Action | FuncKind::Init) {
            return err(f.pos, "only action and init can be payable");
        }
        if !f.only.is_empty() && !matches!(f.kind, FuncKind::Action | FuncKind::Fn | FuncKind::Upgrade) {
            return Err(CompileError::new(f.only[0].1, "'only' can be used on actions, upgrade() and fn helpers")
                .with_help("views are free queries without a verified caller; init() runs before any role exists"));
        }
        let mut params = Vec::new();
        let mut seen = BTreeSet::new();
        for p in &f.params {
            if RESERVED.contains(&p.name.as_str()) || self.consts.contains_key(&qualify(unit, &p.name)) || self.state_index.contains_key(&qualify(unit, &p.name)) {
                return err(p.pos, format!("parameter name '{}' is reserved or already declared", p.name));
            }
            if !seen.insert(p.name.clone()) {
                return err(p.pos, format!("duplicate parameter '{}'", p.name));
            }
            params.push(self.resolve_type(unit, &p.ty, false)?);
        }
        let ret = match &f.ret {
            Some(t) => self.resolve_type(unit, t, false)?,
            None => Type::Unit,
        };
        if f.kind == FuncKind::View && ret == Type::Unit {
            return Err(CompileError::new(f.pos, "a view must declare a return type ('-> type')").with_help(format!("view {}(...) -> int:", f.name)));
        }
        if matches!(f.kind, FuncKind::Init | FuncKind::Upgrade) && ret != Type::Unit {
            return err(f.pos, format!("{} cannot return a value", f.name));
        }
        if f.kind == FuncKind::Upgrade && f.payable {
            return err(f.pos, "upgrade() cannot be payable");
        }
        let program_name = if unit.is_some() && f.kind == FuncKind::Fn { qualify(unit, &f.name) } else { f.name.clone() };
        if exported || matches!(f.kind, FuncKind::Init | FuncKind::Upgrade) {
            self.entry_names.insert(f.name.clone());
        }
        let key = if matches!(f.kind, FuncKind::Init | FuncKind::Upgrade) { f.name.clone() } else { qualify(unit, &f.name) };
        self.fn_index.insert(key, self.sigs.len() as u16);
        self.sigs.push(Sig {
            kind: f.kind,
            params,
            ret,
            unit: unit.clone(),
            program_name,
            only: Vec::new(),
            only_names: f.only.iter().map(|(n, _)| n.clone()).collect(),
        });
        Ok(())
    }

    // ---- constants ----

    fn const_value(&self, unit: &Option<String>, e: &ast::Expr, expected: &Type) -> CResult<Value> {
        let (v, t) = self.const_eval(unit, e)?;
        if &t != expected {
            return err(e.pos(), format!("expected a {} value, found {}", self.tname(expected), self.tname(&t)));
        }
        Ok(v)
    }

    fn const_eval(&self, unit: &Option<String>, e: &ast::Expr) -> CResult<(Value, Type)> {
        let pos = e.pos();
        match e {
            ast::Expr::Int(v, _) => Ok((Value::Int(*v), Type::Int)),
            ast::Expr::Bool(b, _) => Ok((Value::Bool(*b), Type::Bool)),
            ast::Expr::Text(s, _) => Ok((Value::Text(s.clone()), Type::Text)),
            ast::Expr::Bytes(b, _) => Ok((Value::Bytes(b.clone()), Type::Bytes)),
            ast::Expr::Name(n, _) => match self.consts.get(&qualify(unit, n)).or_else(|| self.consts.get(n.as_str()).filter(|_| n == "TCN")) {
                Some((t, v)) => Ok((v.clone(), t.clone())),
                None => err(pos, format!("'{n}' is not a constant (constant values can only use literals and other constants)")),
            },
            ast::Expr::Field(..) => {
                if let Some(v) = self.enum_literal(unit, e) {
                    return Ok(v);
                }
                if let Some(q) = e.dotted().and_then(|d| self.resolve_path(unit, &d)) {
                    if let Some((t, v)) = self.consts.get(&q) {
                        return Ok((v.clone(), t.clone()));
                    }
                }
                err(pos, "constant values can only use literals, other constants, arithmetic, enum variants and address(\"...\")")
            }
            ast::Expr::Call(name, args, _) if name == "address" => self.address_literal(args, pos).map(|v| (v, Type::Address)),
            ast::Expr::Unary(UnOp::Neg, inner, _) => {
                let (v, t) = self.const_eval(unit, inner)?;
                match v {
                    Value::Int(i) if t == Type::Int => Ok((Value::Int(i.checked_neg().ok_or_else(|| CompileError::new(pos, "overflow"))?), t)),
                    _ => err(pos, "'-' needs an int"),
                }
            }
            ast::Expr::Unary(UnOp::Not, inner, _) => match self.const_eval(unit, inner)? {
                (Value::Bool(b), t) => Ok((Value::Bool(!b), t)),
                _ => err(pos, "'not' needs a bool"),
            },
            ast::Expr::Binary(op, l, r, _) => {
                let (lv, lt) = self.const_eval(unit, l)?;
                let (rv, rt) = self.const_eval(unit, r)?;
                let t = self.binary_type(*op, &lt, &rt, pos)?;
                let v = ops::binary(*op, &lv, &rv).map_err(|e| CompileError::new(pos, format!("constant expression: {e}")))?;
                Ok((v, t))
            }
            _ => err(pos, "constant values can only use literals, other constants, arithmetic, enum variants and address(\"...\")"),
        }
    }

    /// `Phase.Open` or `module.Phase.Open`.
    fn enum_literal(&self, unit: &Option<String>, e: &ast::Expr) -> Option<(Value, Type)> {
        let ast::Expr::Field(base, variant, _) = e else { return None };
        let q = self.resolve_path(unit, &base.dotted()?)?;
        let idx = *self.enum_index.get(&q)?;
        let i = self.enums[idx as usize].variants.iter().position(|v| v == variant)?;
        Some((Value::Int(i as i128), Type::Enum(idx)))
    }

    fn address_literal(&self, args: &[ast::Expr], pos: Pos) -> CResult<Value> {
        let [ast::Expr::Text(s, _)] = args else {
            return err(pos, "address(...) takes one text literal, e.g. address(\"tc1...\")");
        };
        let checked = bech32::primitives::decode::CheckedHrpstring::new::<bech32::Bech32m>(s)
            .map_err(|_| CompileError::new(pos, "invalid address literal"))?;
        let hrp = checked.hrp().to_lowercase();
        if !self.opts.address_prefixes.contains(&hrp) {
            return err(pos, format!("address prefix '{hrp}' is not valid on this network"));
        }
        let bytes: Vec<u8> = checked.byte_iter().collect();
        let arr: [u8; 20] = bytes.try_into().map_err(|_| CompileError::new(pos, "invalid address length"))?;
        Ok(Value::Address(arr))
    }

    fn binary_type(&self, op: BinOp, l: &Type, r: &Type, pos: Pos) -> CResult<Type> {
        match op {
            BinOp::Add => match (l, r) {
                (Type::Int, Type::Int) => Ok(Type::Int),
                (Type::Text, Type::Text) => Ok(Type::Text),
                (Type::Bytes, Type::Bytes) => Ok(Type::Bytes),
                _ => err(pos, format!("cannot add {} and {}", self.tname(l), self.tname(r))),
            },
            BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => {
                if *l == Type::Int && *r == Type::Int {
                    Ok(Type::Int)
                } else {
                    err(pos, format!("arithmetic needs int operands, found {} and {}", self.tname(l), self.tname(r)))
                }
            }
            BinOp::Eq | BinOp::Ne => {
                let addressish = |t: &Type| matches!(t, Type::Address | Type::Interface(_));
                if addressish(l) && addressish(r) {
                    return Ok(Type::Bool);
                }
                if l != r {
                    return err(pos, format!("cannot compare {} with {}", self.tname(l), self.tname(r)));
                }
                if !l.is_scalar() && !matches!(l, Type::Enum(_)) {
                    return err(pos, format!("cannot compare values of type {}", self.tname(l)));
                }
                Ok(Type::Bool)
            }
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                if *l == Type::Int && *r == Type::Int {
                    Ok(Type::Bool)
                } else {
                    err(pos, format!("ordering comparisons need int operands, found {} and {}", self.tname(l), self.tname(r)))
                }
            }
            BinOp::And | BinOp::Or => {
                if *l == Type::Bool && *r == Type::Bool {
                    Ok(Type::Bool)
                } else {
                    err(pos, format!("'and'/'or' need bool operands, found {} and {}", self.tname(l), self.tname(r)))
                }
            }
        }
    }

    // ---- pass 2: function bodies ----

    fn function(&self, unit: &Option<String>, f: &ast::FuncDecl) -> CResult<(Function, BTreeSet<u16>, Effects)> {
        let key = if matches!(f.kind, FuncKind::Init | FuncKind::Upgrade) { f.name.clone() } else { qualify(unit, &f.name) };
        let idx = self.fn_index[&key];
        let sig = &self.sigs[idx as usize];
        let mut ctx = FnCtx {
            kind: f.kind,
            ret: sig.ret.clone(),
            scopes: vec![BTreeMap::new()],
            next_slot: 0,
            loop_depth: 0,
            mutates: false,
            calls: BTreeSet::new(),
            unit: unit.clone(),
            fx: Effects::default(),
        };
        for (p, t) in f.params.iter().zip(&sig.params) {
            self.declare_local(&mut ctx, &p.name, t.clone(), p.pos)?;
        }
        let mut body = Vec::new();
        if !sig.only.is_empty() {
            body.push(self.access_check(&sig.only, &sig.only_names, &f.name));
        }
        body.extend(self.block(&mut ctx, &f.body)?);
        if sig.ret != Type::Unit && !always_returns(&body) {
            return Err(CompileError::new(f.pos, format!("function '{}' must return a {} on every path", f.name, self.tname(&sig.ret)))
                .with_help("end the function with 'return <value>', or give every if/elif/else branch a return"));
        }
        let kind = match f.kind {
            FuncKind::Init => FnKind::Init,
            FuncKind::Action => FnKind::Action,
            FuncKind::View => FnKind::View,
            FuncKind::Fn => FnKind::Internal,
            FuncKind::Upgrade => FnKind::Upgrade,
        };
        let params = f.params.iter().zip(&sig.params).map(|(p, t)| (p.name.clone(), t.clone())).collect();
        Ok((
            Function {
                name: sig.program_name.clone(),
                kind,
                payable: f.payable,
                params,
                ret: sig.ret.clone(),
                locals: ctx.next_slot as u16,
                mutates: ctx.mutates,
                body,
            },
            ctx.calls,
            ctx.fx,
        ))
    }

    fn access_check(&self, rules: &[AccessRule], names: &[String], function: &str) -> Stmt {
        let caller = || Expr::Ctx(Ctx::Caller);
        let mut cond: Option<Expr> = None;
        for r in rules {
            let c = match r {
                AccessRule::Role(var) => Expr::MapHas { var: *var, key: Box::new(caller()) },
                AccessRule::Address(var) => Expr::Binary { op: BinOp::Eq, left: Box::new(Expr::State(*var)), right: Box::new(caller()) },
            };
            cond = Some(match cond {
                None => c,
                Some(prev) => Expr::Binary { op: BinOp::Or, left: Box::new(prev), right: Box::new(c) },
            });
        }
        let who = names.join(" or ");
        Stmt::Require { cond: cond.expect("at least one rule"), message: Expr::Const(Value::Text(format!("only {who} can call '{function}'"))) }
    }

    fn declare_local(&self, ctx: &mut FnCtx, name: &str, t: Type, pos: Pos) -> CResult<u16> {
        if RESERVED.contains(&name) {
            return err(pos, format!("'{name}' is a reserved name"));
        }
        let q = qualify(&ctx.unit, name);
        if self.name_taken(&q) || (ctx.unit.is_none() && self.aliases.contains_key(name)) {
            return err(pos, format!("'{name}' is already declared at contract level"));
        }
        if ctx.scopes.iter().any(|s| s.contains_key(name)) {
            return err(pos, format!("variable '{name}' is already declared"));
        }
        if ctx.next_slot >= MAX_LOCALS {
            return err(pos, "too many local variables");
        }
        let slot = ctx.next_slot as u16;
        ctx.next_slot += 1;
        ctx.scopes.last_mut().expect("scope").insert(name.to_string(), (slot, t));
        Ok(slot)
    }

    fn temp_slot(&self, ctx: &mut FnCtx, pos: Pos) -> CResult<u16> {
        if ctx.next_slot >= MAX_LOCALS {
            return err(pos, "too many local variables");
        }
        ctx.next_slot += 1;
        Ok((ctx.next_slot - 1) as u16)
    }

    fn lookup_local(&self, ctx: &FnCtx, name: &str) -> Option<(u16, Type)> {
        ctx.scopes.iter().rev().find_map(|s| s.get(name).cloned())
    }

    /// State variable visible from this function (`x`, or `alias.x` from the contract).
    fn lookup_state(&self, ctx: &FnCtx, path: &str) -> Option<u16> {
        if !path.contains('.') && self.lookup_local(ctx, path).is_some() {
            return None;
        }
        self.resolve_path(&ctx.unit, path).and_then(|q| self.state_index.get(&q).copied())
    }

    fn block(&self, ctx: &mut FnCtx, stmts: &[ast::Stmt]) -> CResult<Vec<Stmt>> {
        ctx.scopes.push(BTreeMap::new());
        let mut out = Vec::with_capacity(stmts.len());
        for s in stmts {
            out.push(self.stmt(ctx, s)?);
        }
        ctx.scopes.pop();
        Ok(out)
    }

    fn require_mutable(&self, ctx: &mut FnCtx, pos: Pos, what: &str) -> CResult<()> {
        if ctx.kind == FuncKind::View {
            return Err(CompileError::new(pos, format!("a view cannot {what}")).with_help("views only read; use an 'action' to change things"));
        }
        ctx.mutates = true;
        Ok(())
    }

    /// Writing module state is only allowed from the module's own functions.
    fn check_state_owner(&self, ctx: &FnCtx, var: u16, pos: Pos) -> CResult<()> {
        let owner = &self.state_unit[var as usize];
        if owner.is_some() && owner != &ctx.unit {
            let m = owner.as_ref().unwrap();
            return Err(CompileError::new(pos, format!("state of module {m} can only be changed by the functions of module {m}"))
                .with_help(format!("call a function of the module, e.g. {m}.<function>(...)")));
        }
        Ok(())
    }

    fn expect_type(&self, ctx: &mut FnCtx, e: &ast::Expr, t: &Type) -> CResult<Expr> {
        if let (ast::Expr::List(items, _), Type::List(_)) = (e, t) {
            if items.is_empty() {
                return Ok(Expr::List(Vec::new()));
            }
        }
        let (x, xt) = self.expr(ctx, e)?;
        if &xt != t && !(matches!(t, Type::Address) && matches!(xt, Type::Interface(_))) {
            let mut error = CompileError::new(e.pos(), format!("expected {}, found {}", self.tname(t), self.tname(&xt)));
            if *t == Type::Text && xt == Type::Int {
                error = error.with_help("convert with to_text(...)");
            } else if *t == Type::Bytes && xt.is_scalar() {
                error = error.with_help("convert with to_bytes(...)");
            }
            return Err(error);
        }
        Ok(x)
    }

    fn type_has_transitions(&self, t: &Type) -> bool {
        self.transitions_depth(t, 0)
    }

    fn transitions_depth(&self, t: &Type, depth: usize) -> bool {
        match t {
            Type::Enum(e) => self.enums[*e as usize].transitions.is_some(),
            Type::Record(r) if depth < MAX_VALUE_DEPTH => self.records[*r as usize].fields.iter().any(|(_, ft)| self.transitions_depth(ft, depth + 1)),
            _ => false,
        }
    }

    fn stmt(&self, ctx: &mut FnCtx, s: &ast::Stmt) -> CResult<Stmt> {
        match s {
            ast::Stmt::Let { name, ty, value, pos } => {
                let t = self.resolve_type(&ctx.unit, ty, false)?;
                let v = self.expect_type(ctx, value, &t)?;
                let slot = self.declare_local(ctx, name, t, *pos)?;
                Ok(Stmt::SetLocal { slot, value: v })
            }
            ast::Stmt::Assign { target, op, value, pos } => self.assign(ctx, target, *op, value, *pos),
            ast::Stmt::If { branches, els, .. } => {
                let mut else_part = match els {
                    Some(b) => self.block(ctx, b)?,
                    None => Vec::new(),
                };
                for (cond, body) in branches.iter().rev() {
                    let c = self.expect_type(ctx, cond, &Type::Bool)?;
                    let then = self.block(ctx, body)?;
                    else_part = vec![Stmt::If { cond: c, then, els: else_part }];
                }
                Ok(else_part.pop().expect("at least one branch"))
            }
            ast::Stmt::While { cond, body, .. } => {
                let c = self.expect_type(ctx, cond, &Type::Bool)?;
                ctx.loop_depth += 1;
                let b = self.block(ctx, body)?;
                ctx.loop_depth -= 1;
                Ok(Stmt::While { cond: c, body: b })
            }
            ast::Stmt::ForRange { var, start, end, body, pos } => {
                let s = self.expect_type(ctx, start, &Type::Int)?;
                let e = self.expect_type(ctx, end, &Type::Int)?;
                ctx.scopes.push(BTreeMap::new());
                let slot = self.declare_local(ctx, var, Type::Int, *pos)?;
                ctx.loop_depth += 1;
                let b = self.block(ctx, body)?;
                ctx.loop_depth -= 1;
                ctx.scopes.pop();
                Ok(Stmt::ForRange { slot, start: s, end: e, body: b })
            }
            ast::Stmt::ForEach { var, iter, body, pos } => {
                if let Some(path) = iter.dotted() {
                    if let Some(var_idx) = self.lookup_state(ctx, &path) {
                        let Type::List(inner) = self.states[var_idx as usize].ty.clone() else {
                            return err(*pos, format!("cannot iterate over state variable '{path}' (only lists)"));
                        };
                        ctx.scopes.push(BTreeMap::new());
                        let slot = self.declare_local(ctx, var, *inner, *pos)?;
                        ctx.loop_depth += 1;
                        let b = self.block(ctx, body)?;
                        ctx.loop_depth -= 1;
                        ctx.scopes.pop();
                        return Ok(Stmt::ForEachState { slot, var: var_idx, body: b });
                    }
                }
                let (list, lt) = self.expr(ctx, iter)?;
                let Type::List(inner) = lt else {
                    return err(*pos, format!("'for ... in' needs a list or range(start, end), found {}", self.tname(&lt)));
                };
                ctx.scopes.push(BTreeMap::new());
                let slot = self.declare_local(ctx, var, *inner, *pos)?;
                ctx.loop_depth += 1;
                let b = self.block(ctx, body)?;
                ctx.loop_depth -= 1;
                ctx.scopes.pop();
                Ok(Stmt::ForEachLocal { slot, list, body: b })
            }
            ast::Stmt::Break(pos) | ast::Stmt::Continue(pos) => {
                if ctx.loop_depth == 0 {
                    return err(*pos, "break/continue outside of a loop");
                }
                Ok(if matches!(s, ast::Stmt::Break(_)) { Stmt::Break } else { Stmt::Continue })
            }
            ast::Stmt::Return(v, pos) => match (v, &ctx.ret) {
                (None, Type::Unit) => Ok(Stmt::Return(None)),
                (None, t) => err(*pos, format!("this function must return a {}", self.tname(t))),
                (Some(_), Type::Unit) => err(*pos, "this function does not return a value (declare '-> type')"),
                (Some(e), t) => {
                    let t = t.clone();
                    Ok(Stmt::Return(Some(self.expect_type(ctx, e, &t)?)))
                }
            },
            ast::Stmt::Require(cond, msg, pos) => {
                let c = self.expect_type(ctx, cond, &Type::Bool)?;
                let m = match msg {
                    Some(m) => self.expect_type(ctx, m, &Type::Text)?,
                    None => Expr::Const(Value::Text(format!("requirement at line {} failed", pos.line))),
                };
                Ok(Stmt::Require { cond: c, message: m })
            }
            ast::Stmt::Send(to, amount, pos) => {
                self.require_mutable(ctx, *pos, "send TCN")?;
                ctx.fx.sends_tcn = true;
                let t = self.expect_type(ctx, to, &Type::Address)?;
                let a = self.expect_type(ctx, amount, &Type::Int)?;
                Ok(Stmt::Send { to: t, amount: a })
            }
            ast::Stmt::Emit(name, args, pos) => {
                self.require_mutable(ctx, *pos, "emit events")?;
                ctx.fx.emits_events = true;
                let Some(&idx) = self.event_index.get(&qualify(&ctx.unit, name)) else {
                    let cands: Vec<&str> = self.events.iter().map(|e| e.name.as_str()).collect();
                    let mut e = CompileError::new(*pos, format!("unknown event '{name}'"));
                    e = match suggest(name, cands) {
                        Some(h) => e.with_help(h),
                        None => e.with_help(format!("declare it: event {name}(...)")),
                    };
                    return Err(e);
                };
                let fields = self.events[idx as usize].fields.clone();
                if fields.len() != args.len() {
                    return err(*pos, format!("event '{name}' has {} fields, {} given", fields.len(), args.len()));
                }
                let mut out = Vec::new();
                for (a, (_, t)) in args.iter().zip(fields.iter()) {
                    out.push(self.expect_type(ctx, a, t)?);
                }
                Ok(Stmt::Emit { event: idx, args: out })
            }
            ast::Stmt::Destroy(to, pos) => {
                if ctx.kind != FuncKind::Action {
                    return err(*pos, "destroy() can only be used inside an action");
                }
                self.require_mutable(ctx, *pos, "destroy the contract")?;
                ctx.fx.can_destroy = true;
                let t = self.expect_type(ctx, to, &Type::Address)?;
                Ok(Stmt::Destroy { to: t })
            }
            ast::Stmt::Pass(_) => Ok(Stmt::Eval(Expr::Const(Value::Unit))),
            ast::Stmt::Grant { role, who, pos } | ast::Stmt::Revoke { role, who, pos } => {
                let grant = matches!(s, ast::Stmt::Grant { .. });
                let Some(&r) = self.role_index.get(&qualify(&ctx.unit, role)) else {
                    let cands: Vec<String> = self.roles.iter().map(|r| r.name.clone()).collect();
                    let mut e = CompileError::new(*pos, format!("unknown role '{role}'"));
                    e = match suggest(role, cands.iter().map(String::as_str)) {
                        Some(h) => e.with_help(h),
                        None => e.with_help(format!("declare it: role {role}")),
                    };
                    return Err(e);
                };
                self.require_mutable(ctx, *pos, if grant { "grant roles" } else { "revoke roles" })?;
                ctx.fx.emits_events = true;
                let var = self.roles[r as usize].state;
                let w = self.expect_type(ctx, who, &Type::Address)?;
                let tmp = self.temp_slot(ctx, *pos)?;
                let (ge, re) = self.role_events.expect("role events declared");
                let change = if grant {
                    Stmt::SetMap { var, key: Expr::Local(tmp), value: Expr::Const(Value::Bool(true)) }
                } else {
                    Stmt::RemoveMap { var, key: Expr::Local(tmp) }
                };
                Ok(Stmt::If {
                    cond: Expr::Const(Value::Bool(true)),
                    then: vec![
                        Stmt::SetLocal { slot: tmp, value: w },
                        change,
                        Stmt::Emit {
                            event: if grant { ge } else { re },
                            args: vec![Expr::Const(Value::Text(role.clone())), Expr::Local(tmp), Expr::Ctx(Ctx::Caller)],
                        },
                    ],
                    els: vec![],
                })
            }
            ast::Stmt::Expr(e, pos) => {
                if let ast::Expr::Method(base, m, args, mpos) = e {
                    if let Some(st) = self.method_stmt(ctx, base, m, args, *mpos)? {
                        return Ok(st);
                    }
                }
                let (x, t) = self.expr(ctx, e)?;
                let allowed = matches!(x, Expr::Call { .. } | Expr::StateListPop { .. } | Expr::CallContract { .. });
                if !allowed {
                    return err(*pos, format!("this expression ({}) does nothing as a statement", self.tname(&t)));
                }
                Ok(Stmt::Eval(x))
            }
        }
    }

    fn method_stmt(&self, ctx: &mut FnCtx, base: &ast::Expr, m: &str, args: &[ast::Expr], pos: Pos) -> CResult<Option<Stmt>> {
        let Some(n) = base.dotted() else { return Ok(None) };
        match m {
            "push" => {
                let [arg] = args else { return err(pos, "push takes one argument") };
                if !n.contains('.') {
                    if let Some((slot, Type::List(inner))) = self.lookup_local(ctx, &n) {
                        let v = self.expect_type(ctx, arg, &inner)?;
                        return Ok(Some(Stmt::PushLocalList { slot, value: v }));
                    }
                }
                if let Some(var) = self.lookup_state(ctx, &n) {
                    if let Type::List(inner) = self.states[var as usize].ty.clone() {
                        self.require_mutable(ctx, pos, "change state")?;
                        self.check_state_owner(ctx, var, pos)?;
                        let mut v = self.expect_type(ctx, arg, &inner)?;
                        if self.type_has_transitions(&inner) {
                            v = Expr::Transition { ty: (*inner).clone(), from: Box::new(Expr::Const(self.default_value(&inner))), to: Box::new(v) };
                        }
                        return Ok(Some(Stmt::PushStateList { var, value: v }));
                    }
                }
                if self.lookup_local(ctx, &n).is_none() && self.lookup_state(ctx, &n).is_none() && !n.contains('.') {
                    return Ok(None);
                }
                err(pos, format!("'{n}' is not a list"))
            }
            "remove" => {
                let [arg] = args else { return err(pos, "remove takes one argument (the key)") };
                if let Some(var) = self.lookup_state(ctx, &n) {
                    if let Type::Map(k, _) = self.states[var as usize].ty.clone() {
                        self.require_mutable(ctx, pos, "change state")?;
                        self.check_state_owner(ctx, var, pos)?;
                        let key = self.expect_type(ctx, arg, &k)?;
                        return Ok(Some(Stmt::RemoveMap { var, key }));
                    }
                }
                if self.role_index.contains_key(&qualify(&ctx.unit, &n)) {
                    return Err(CompileError::new(pos, format!("'{n}' is a role")).with_help(format!("use 'revoke {n} from <address>'")));
                }
                err(pos, format!("'{n}' is not a map"))
            }
            "pop" => {
                if !args.is_empty() {
                    return err(pos, "pop takes no arguments");
                }
                if let Some(var) = self.lookup_state(ctx, &n) {
                    if matches!(self.states[var as usize].ty, Type::List(_)) {
                        self.require_mutable(ctx, pos, "change state")?;
                        self.check_state_owner(ctx, var, pos)?;
                        return Ok(Some(Stmt::PopStateList { var }));
                    }
                }
                if self.aliases.contains_key(&n) && ctx.unit.is_none() {
                    return Ok(None);
                }
                err(pos, "pop() is only available on state lists")
            }
            _ => Ok(None),
        }
    }

    fn default_value(&self, t: &Type) -> Value {
        match t {
            Type::Record(r) => Value::List(self.records[*r as usize].fields.iter().map(|(_, ft)| self.default_value(ft)).collect()),
            other => Value::default_for(other),
        }
    }

    /// Resolves an assignment target into its storage root and record-field path.
    fn lvalue(&self, ctx: &mut FnCtx, target: &ast::Expr, pos: Pos) -> CResult<(Root, Vec<(u16, Type)>, Type)> {
        match target {
            ast::Expr::Field(base, field, fpos) => {
                if let Some(path) = target.dotted() {
                    if path.contains('.') && self.lookup_local(ctx, path.split('.').next().unwrap()).is_none() {
                        if let Some(var) = self.lookup_state(ctx, &path) {
                            return self.state_root(ctx, var, &path, pos);
                        }
                        if let Some(q) = self.resolve_path(&ctx.unit, &path) {
                            if self.consts.contains_key(&q) {
                                return err(*fpos, format!("'{path}' is a constant and cannot be changed"));
                            }
                        }
                    }
                }
                let (root, mut path, t) = self.lvalue(ctx, base, pos)?;
                let Type::Record(r) = t else {
                    return err(*fpos, format!("{} has no fields", self.tname(&t)));
                };
                let def = &self.records[r as usize];
                let Some(i) = def.fields.iter().position(|(n, _)| n == field) else {
                    let mut e = CompileError::new(*fpos, format!("record {} has no field '{field}'", def.name));
                    if let Some(h) = suggest(field, def.fields.iter().map(|(n, _)| n.as_str())) {
                        e = e.with_help(h);
                    }
                    return Err(e);
                };
                let ft = def.fields[i].1.clone();
                path.push((i as u16, t));
                Ok((root, path, ft))
            }
            ast::Expr::Name(n, npos) => {
                if let Some((slot, t)) = self.lookup_local(ctx, n) {
                    return Ok((Root::Local(slot), Vec::new(), t));
                }
                if let Some(var) = self.lookup_state(ctx, n) {
                    return self.state_root(ctx, var, n, pos);
                }
                if self.consts.contains_key(&qualify(&ctx.unit, n)) {
                    return err(*npos, format!("'{n}' is a constant and cannot be changed"));
                }
                if self.role_index.contains_key(&qualify(&ctx.unit, n)) {
                    return Err(CompileError::new(*npos, format!("'{n}' is a role")).with_help(format!("use 'grant {n} to <address>' or 'revoke {n} from <address>'")));
                }
                let mut e = CompileError::new(*npos, format!("unknown variable '{n}'"));
                if let Some(h) = self.name_suggestion(ctx, n) {
                    e = e.with_help(h);
                }
                Err(e)
            }
            ast::Expr::Index(base, index, ipos) => {
                if let Some(path) = base.dotted() {
                    let is_local = self.lookup_local(ctx, path.split('.').next().unwrap()).is_some() && !path.contains('.');
                    if let (true, Some((slot, Type::List(inner)))) = (is_local, self.lookup_local(ctx, &path)) {
                        let i = self.expect_type(ctx, index, &Type::Int)?;
                        return Ok((Root::LocalItem(slot, i), Vec::new(), *inner));
                    }
                    if !is_local {
                        if let Some(var) = self.lookup_state(ctx, &path) {
                            match self.states[var as usize].ty.clone() {
                                Type::Map(k, v) => {
                                    self.require_mutable(ctx, pos, "change state")?;
                                    self.check_state_owner(ctx, var, pos)?;
                                    let key = self.expect_type(ctx, index, &k)?;
                                    return Ok((Root::MapItem(var, key), Vec::new(), *v));
                                }
                                Type::List(inner) => {
                                    self.require_mutable(ctx, pos, "change state")?;
                                    self.check_state_owner(ctx, var, pos)?;
                                    let i = self.expect_type(ctx, index, &Type::Int)?;
                                    return Ok((Root::StateItem(var, i), Vec::new(), *inner));
                                }
                                _ => {}
                            }
                        }
                    }
                }
                err(*ipos, "only items of lists and maps can be assigned with [ ]")
            }
            other => err(other.pos(), "cannot assign to this expression"),
        }
    }

    fn state_root(&self, ctx: &mut FnCtx, var: u16, name: &str, pos: Pos) -> CResult<(Root, Vec<(u16, Type)>, Type)> {
        let t = self.states[var as usize].ty.clone();
        if matches!(t, Type::Map(..) | Type::List(_)) {
            return err(pos, format!("cannot assign a whole {} ('{name}'); change its items instead", self.tname(&t)));
        }
        self.require_mutable(ctx, pos, "change state")?;
        self.check_state_owner(ctx, var, pos)?;
        Ok((Root::State(var), Vec::new(), t))
    }

    fn assign(&self, ctx: &mut FnCtx, target: &ast::Expr, op: AssignOp, value: &ast::Expr, pos: Pos) -> CResult<Stmt> {
        let (root, path, t) = self.lvalue(ctx, target, pos)?;
        if op != AssignOp::Set && t != Type::Int && !(op == AssignOp::Add && matches!(t, Type::Text | Type::Bytes)) {
            return err(pos, format!("compound assignment is not defined for {}", self.tname(&t)));
        }
        let rhs = self.expect_type(ctx, value, &t)?;
        // Version 1 shapes (no record path) compile exactly like version 1.
        let needs_temp = |key: &Expr| !(op == AssignOp::Set && path.is_empty()) && !matches!(key, Expr::Const(_) | Expr::Local(_));
        let bop = match op {
            AssignOp::Set => None,
            AssignOp::Add => Some(BinOp::Add),
            AssignOp::Sub => Some(BinOp::Sub),
            AssignOp::Mul => Some(BinOp::Mul),
        };
        // Builds `new root value` from the current root value.
        let build = |current: Expr| -> Expr {
            fn rebuild(current: Expr, path: &[(u16, Type)], rhs: Expr, bop: Option<BinOp>) -> Expr {
                match path.split_first() {
                    None => match bop {
                        None => rhs,
                        Some(op) => Expr::Binary { op, left: Box::new(current), right: Box::new(rhs) },
                    },
                    Some(((i, _), rest)) => {
                        let inner = Expr::Index { base: Box::new(current.clone()), index: Box::new(Expr::Const(Value::Int(*i as i128))) };
                        Expr::Replace { base: Box::new(current), index: *i, value: Box::new(rebuild(inner, rest, rhs, bop)) }
                    }
                }
            }
            rebuild(current, &path, rhs.clone(), bop)
        };
        let guard = |c: &Checker, ty: &Type, current: Expr, new: Expr| -> Expr {
            if c.type_has_transitions(ty) {
                Expr::Transition { ty: ty.clone(), from: Box::new(current), to: Box::new(new) }
            } else {
                new
            }
        };
        match root {
            Root::Local(slot) => Ok(Stmt::SetLocal { slot, value: build(Expr::Local(slot)) }),
            Root::State(var) => {
                let ty = self.states[var as usize].ty.clone();
                let value = guard(self, &ty, Expr::State(var), build(Expr::State(var)));
                Ok(Stmt::SetState { var, value })
            }
            Root::LocalItem(slot, index) => {
                let make = |i: Expr| {
                    let current = Expr::Index { base: Box::new(Expr::Local(slot)), index: Box::new(i.clone()) };
                    Stmt::SetLocalListItem { slot, index: i, value: build(current) }
                };
                self.with_temp(ctx, index, needs_temp, pos, make)
            }
            Root::MapItem(var, key) => {
                let Type::Map(_, vt) = self.states[var as usize].ty.clone() else { unreachable!() };
                let make = |k: Expr| {
                    let current = Expr::MapGet { var, key: Box::new(k.clone()) };
                    let value = guard(self, &vt, current.clone(), build(current));
                    Stmt::SetMap { var, key: k, value }
                };
                self.with_temp(ctx, key, needs_temp, pos, make)
            }
            Root::StateItem(var, index) => {
                let Type::List(it) = self.states[var as usize].ty.clone() else { unreachable!() };
                let make = |i: Expr| {
                    let current = Expr::StateListGet { var, index: Box::new(i.clone()) };
                    let value = guard(self, &it, current.clone(), build(current));
                    Stmt::SetStateListItem { var, index: i, value }
                };
                self.with_temp(ctx, index, needs_temp, pos, make)
            }
        }
    }

    /// Evaluates a key/index once when it is read and written (it may call a function).
    fn with_temp(&self, ctx: &mut FnCtx, key: Expr, needs: impl Fn(&Expr) -> bool, pos: Pos, make: impl FnOnce(Expr) -> Stmt) -> CResult<Stmt> {
        if !needs(&key) {
            return Ok(make(key));
        }
        let tmp = self.temp_slot(ctx, pos)?;
        Ok(Stmt::If { cond: Expr::Const(Value::Bool(true)), then: vec![Stmt::SetLocal { slot: tmp, value: key }, make(Expr::Local(tmp))], els: vec![] })
    }

    fn name_suggestion(&self, ctx: &FnCtx, n: &str) -> Option<String> {
        let mut cands: Vec<String> = ctx.scopes.iter().flat_map(|s| s.keys().cloned()).collect();
        let prefix = ctx.unit.as_ref().map(|m| format!("{m}."));
        for k in self.state_index.keys().chain(self.consts.keys()).chain(self.fn_index.keys()) {
            match &prefix {
                Some(p) => {
                    if let Some(rest) = k.strip_prefix(p.as_str()) {
                        cands.push(rest.to_string());
                    }
                }
                None => {
                    if !k.contains('.') {
                        cands.push(k.clone());
                    }
                }
            }
        }
        cands.extend(["caller", "value", "balance", "height", "self", "origin"].iter().map(|s| s.to_string()));
        suggest(n, cands.iter().map(String::as_str))
    }

    fn expr(&self, ctx: &mut FnCtx, e: &ast::Expr) -> CResult<(Expr, Type)> {
        let pos = e.pos();
        match e {
            ast::Expr::Int(v, _) => Ok((Expr::Const(Value::Int(*v)), Type::Int)),
            ast::Expr::Bool(b, _) => Ok((Expr::Const(Value::Bool(*b)), Type::Bool)),
            ast::Expr::Text(s, _) => {
                if s.len() > ops::MAX_VALUE_BYTES {
                    return err(pos, "text literal too long");
                }
                Ok((Expr::Const(Value::Text(s.clone())), Type::Text))
            }
            ast::Expr::Bytes(b, _) => Ok((Expr::Const(Value::Bytes(b.clone())), Type::Bytes)),
            ast::Expr::Name(n, _) => {
                if let Some((slot, t)) = self.lookup_local(ctx, n) {
                    return Ok((Expr::Local(slot), t));
                }
                if let Some((t, v)) = self.consts.get(&qualify(&ctx.unit, n)).or_else(|| if n == "TCN" { self.consts.get("TCN") } else { None }) {
                    return Ok((Expr::Const(v.clone()), t.clone()));
                }
                if let Some(var) = self.lookup_state(ctx, n) {
                    return self.state_read(var, n, pos);
                }
                if self.role_index.contains_key(&qualify(&ctx.unit, n)) {
                    return Err(CompileError::new(pos, format!("'{n}' is a role, not a value")).with_help(format!("check membership with {n}.has(address)")));
                }
                let ctxv = match n.as_str() {
                    "caller" => (Ctx::Caller, Type::Address),
                    "value" => (Ctx::Value, Type::Int),
                    "balance" => (Ctx::Balance, Type::Int),
                    "height" => (Ctx::Height, Type::Int),
                    "self" => (Ctx::SelfAddress, Type::Address),
                    "origin" => (Ctx::Origin, Type::Address),
                    _ => {
                        let mut error = CompileError::new(pos, format!("unknown name '{n}'"));
                        if let Some(h) = self.name_suggestion(ctx, n) {
                            error = error.with_help(h);
                        }
                        return Err(error);
                    }
                };
                if ctxv.0 == Ctx::Value && ctx.kind == FuncKind::View {
                    return err(pos, "'value' is not available in a view");
                }
                Ok((Expr::Ctx(ctxv.0), ctxv.1))
            }
            ast::Expr::List(items, _) => {
                if items.is_empty() {
                    return err(pos, "an empty list needs a known type, e.g. 'let xs: list[int] = []'");
                }
                let (first, t) = self.expr(ctx, &items[0])?;
                let mut out = vec![first];
                for it in &items[1..] {
                    out.push(self.expect_type(ctx, it, &t)?);
                }
                if out.len() > ops::MAX_LIST_LEN {
                    return err(pos, "list literal too long");
                }
                Ok((Expr::List(out), Type::List(Box::new(t))))
            }
            ast::Expr::Unary(UnOp::Neg, inner, _) => Ok((Expr::Neg(Box::new(self.expect_type(ctx, inner, &Type::Int)?)), Type::Int)),
            ast::Expr::Unary(UnOp::Not, inner, _) => Ok((Expr::Not(Box::new(self.expect_type(ctx, inner, &Type::Bool)?)), Type::Bool)),
            ast::Expr::Binary(op, l, r, _) => {
                let (lx, lt) = self.expr(ctx, l)?;
                let (rx, rt) = self.expr(ctx, r)?;
                let t = self.binary_type(*op, &lt, &rt, pos)?;
                Ok((Expr::Binary { op: *op, left: Box::new(lx), right: Box::new(rx) }, t))
            }
            ast::Expr::Index(base, index, _) => {
                if let Some(path) = base.dotted() {
                    let local = !path.contains('.') && self.lookup_local(ctx, &path).is_some();
                    if !local {
                        if let Some(var) = self.lookup_state(ctx, &path) {
                            match self.states[var as usize].ty.clone() {
                                Type::Map(k, v) => {
                                    let key = self.expect_type(ctx, index, &k)?;
                                    return Ok((Expr::MapGet { var, key: Box::new(key) }, *v));
                                }
                                Type::List(inner) => {
                                    let i = self.expect_type(ctx, index, &Type::Int)?;
                                    return Ok((Expr::StateListGet { var, index: Box::new(i) }, *inner));
                                }
                                t => return err(pos, format!("cannot index a {}", self.tname(&t))),
                            }
                        }
                    }
                }
                let (b, bt) = self.expr(ctx, base)?;
                let i = self.expect_type(ctx, index, &Type::Int)?;
                match bt {
                    Type::List(inner) => Ok((Expr::Index { base: Box::new(b), index: Box::new(i) }, *inner)),
                    Type::Bytes => Ok((Expr::Index { base: Box::new(b), index: Box::new(i) }, Type::Int)),
                    t => err(pos, format!("cannot index a {}", self.tname(&t))),
                }
            }
            ast::Expr::Field(base, field, fpos) => {
                if let Some((v, t)) = self.enum_literal(&ctx.unit, e) {
                    return Ok((Expr::Const(v), t));
                }
                if let Some(path) = e.dotted() {
                    let head = path.split('.').next().unwrap();
                    if self.lookup_local(ctx, head).is_none() && ctx.unit.is_none() && self.aliases.contains_key(head) {
                        let q = self.resolve_path(&ctx.unit, &path);
                        if let Some(q) = q {
                            if let Some((t, v)) = self.consts.get(&q) {
                                return Ok((Expr::Const(v.clone()), t.clone()));
                            }
                            if let Some(var) = self.state_index.get(&q) {
                                return self.state_read(*var, &path, pos);
                            }
                        }
                        return err(*fpos, format!("module {} has no constant or state '{field}'", self.aliases[head]));
                    }
                    if let Some(q) = base.dotted().and_then(|b| self.resolve_path(&ctx.unit, &b)) {
                        if let Some(idx) = self.enum_index.get(&q) {
                            let def = &self.enums[*idx as usize];
                            let mut e = CompileError::new(*fpos, format!("{} has no variant '{field}'", def.name));
                            if let Some(h) = suggest(field, def.variants.iter().map(String::as_str)) {
                                e = e.with_help(h);
                            }
                            return Err(e);
                        }
                    }
                }
                let (b, bt) = self.expr(ctx, base)?;
                match bt {
                    Type::Record(r) => {
                        let def = &self.records[r as usize];
                        let Some(i) = def.fields.iter().position(|(n, _)| n == field) else {
                            let mut e = CompileError::new(*fpos, format!("record {} has no field '{field}'", def.name));
                            if let Some(h) = suggest(field, def.fields.iter().map(|(n, _)| n.as_str())) {
                                e = e.with_help(h);
                            }
                            return Err(e);
                        };
                        Ok((Expr::Index { base: Box::new(b), index: Box::new(Expr::Const(Value::Int(i as i128))) }, def.fields[i].1.clone()))
                    }
                    t => Err(CompileError::new(*fpos, format!("{} has no field '{field}'", self.tname(&t)))
                        .with_help(if matches!(t, Type::Map(..)) { "maps are read with m[key]".to_string() } else { "only records have fields".to_string() })),
                }
            }
            ast::Expr::Construct(base, fields, cpos) => {
                let Some(path) = base.dotted() else { return err(*cpos, "expected a record name before '('") };
                let Some(idx) = self.resolve_path(&ctx.unit, &path).and_then(|q| self.record_index.get(&q).copied()) else {
                    let names = self.type_names(&ctx.unit);
                    let mut e = CompileError::new(*cpos, format!("unknown record '{path}'"));
                    if let Some(h) = suggest(&path, names.iter().map(String::as_str)) {
                        e = e.with_help(h);
                    }
                    return Err(e);
                };
                let def = self.records[idx as usize].clone();
                let mut values: Vec<Option<Expr>> = vec![None; def.fields.len()];
                for (name, value, fpos) in fields {
                    let Some(i) = def.fields.iter().position(|(n, _)| n == name) else {
                        let mut e = CompileError::new(*fpos, format!("record {} has no field '{name}'", def.name));
                        if let Some(h) = suggest(name, def.fields.iter().map(|(n, _)| n.as_str())) {
                            e = e.with_help(h);
                        }
                        return Err(e);
                    };
                    if values[i].is_some() {
                        return err(*fpos, format!("field '{name}' is given twice"));
                    }
                    values[i] = Some(self.expect_type(ctx, value, &def.fields[i].1)?);
                }
                let missing: Vec<&str> = def.fields.iter().zip(&values).filter(|(_, v)| v.is_none()).map(|((n, _), _)| n.as_str()).collect();
                if !missing.is_empty() {
                    return Err(CompileError::new(*cpos, format!("record {} needs every field; missing: {}", def.name, missing.join(", ")))
                        .with_help(format!("{}({})", def.name, def.fields.iter().map(|(n, _)| format!("{n}: ...")).collect::<Vec<_>>().join(", "))));
                }
                Ok((Expr::List(values.into_iter().map(|v| v.expect("present")).collect()), Type::Record(idx)))
            }
            ast::Expr::Method(base, m, args, mpos) => self.method(ctx, base, m, args, *mpos),
            ast::Expr::WithValue(call, v, wpos) => {
                let (x, t) = self.expr(ctx, call)?;
                let Expr::CallContract { target, function, kind, params, ret, args, value: None } = x else {
                    return err(*wpos, "'with value' can only follow a call to another contract's action");
                };
                if kind != FnKind::Action {
                    return err(*wpos, "views cannot receive TCN; 'with value' needs an action");
                }
                let iface_payable = self.interfaces.iter().flat_map(|i| &i.functions).any(|f| f.name == function && f.payable);
                if !iface_payable {
                    return Err(CompileError::new(*wpos, format!("'{function}' is not payable in its interface")).with_help("declare it 'payable' in the interface"));
                }
                self.require_mutable(ctx, *wpos, "send TCN")?;
                ctx.fx.sends_tcn = true;
                let amount = self.expect_type(ctx, v, &Type::Int)?;
                Ok((Expr::CallContract { target, function, kind, params, ret, args, value: Some(Box::new(amount)) }, t))
            }
            ast::Expr::Call(name, args, _) => self.call(ctx, name, args, pos),
        }
    }

    fn state_read(&self, var: u16, n: &str, pos: Pos) -> CResult<(Expr, Type)> {
        let t = self.states[var as usize].ty.clone();
        match t {
            Type::Map(_, _) => err(pos, format!("'{n}' is a map; use {n}[key] or {n}.has(key)")),
            Type::List(_) => err(pos, format!("'{n}' is a state list; use {n}[i], len({n}) or 'for x in {n}'")),
            _ => Ok((Expr::State(var), t)),
        }
    }

    fn method(&self, ctx: &mut FnCtx, base: &ast::Expr, m: &str, args: &[ast::Expr], pos: Pos) -> CResult<(Expr, Type)> {
        if let Some(path) = base.dotted() {
            let head = path.split('.').next().unwrap().to_string();
            let is_local = self.lookup_local(ctx, &head).is_some();
            // module.function(...)
            if !is_local && !path.contains('.') && ctx.unit.is_none() && self.aliases.contains_key(&path) {
                let module = &self.aliases[&path];
                return self.call_function(ctx, &format!("{module}.{m}"), &format!("{path}.{m}"), args, pos);
            }
            // role.has(address)
            if !is_local && !path.contains('.') {
                if let Some(r) = self.role_index.get(&qualify(&ctx.unit, &path)) {
                    let var = self.roles[*r as usize].state;
                    return match (m, args) {
                        ("has", [who]) => {
                            let w = self.expect_type(ctx, who, &Type::Address)?;
                            Ok((Expr::MapHas { var, key: Box::new(w) }, Type::Bool))
                        }
                        _ => Err(CompileError::new(pos, format!("role '{path}' has no method '{m}'")).with_help(format!("{path}.has(address), grant {path} to ..., revoke {path} from ..."))),
                    };
                }
            }
            if !is_local || path.contains('.') {
                if let Some(var) = self.lookup_state(ctx, &path) {
                    let t = self.states[var as usize].ty.clone();
                    match (m, &t, args) {
                        ("has", Type::Map(k, _), [key]) => {
                            let kx = self.expect_type(ctx, key, k)?;
                            return Ok((Expr::MapHas { var, key: Box::new(kx) }, Type::Bool));
                        }
                        ("len", Type::List(_), []) => return Ok((Expr::StateListLen { var }, Type::Int)),
                        ("pop", Type::List(inner), []) => {
                            self.require_mutable(ctx, pos, "change state")?;
                            self.check_state_owner(ctx, var, pos)?;
                            return Ok((Expr::StateListPop { var }, (**inner).clone()));
                        }
                        _ => {}
                    }
                    if !matches!(t, Type::Interface(_)) {
                        return err(pos, format!("'{path}' ({}) has no method '{m}' with these arguments", self.tname(&t)));
                    }
                }
            }
        }
        let (b, bt) = self.expr(ctx, base)?;
        if let Type::Interface(i) = bt {
            let def = &self.interfaces[i as usize];
            let Some(f) = def.functions.iter().find(|f| f.name == m) else {
                let mut e = CompileError::new(pos, format!("interface {} has no function '{m}'", def.name));
                if let Some(h) = suggest(m, def.functions.iter().map(|f| f.name.as_str())) {
                    e = e.with_help(h);
                }
                return Err(e);
            };
            let f = f.clone();
            if f.params.len() != args.len() {
                return err(pos, format!("{m}() takes {} argument(s), {} given", f.params.len(), args.len()));
            }
            if f.kind == FnKind::Action {
                if ctx.kind == FuncKind::View {
                    return Err(CompileError::new(pos, format!("a view cannot call the action '{m}' of another contract")).with_help("views can only call views of other contracts"));
                }
                ctx.mutates = true;
            }
            ctx.fx.calls_contracts = true;
            let mut out = Vec::new();
            for (a, (_, t)) in args.iter().zip(&f.params) {
                out.push(self.expect_type(ctx, a, t)?);
            }
            return Ok((
                Expr::CallContract {
                    target: Box::new(b),
                    function: f.name.clone(),
                    kind: f.kind,
                    params: f.params.iter().map(|(_, t)| t.clone()).collect(),
                    ret: f.ret.clone(),
                    args: out,
                    value: None,
                },
                f.ret.clone(),
            ));
        }
        match (m, &bt, args.is_empty()) {
            ("len", Type::List(_) | Type::Text | Type::Bytes, true) => Ok((Expr::Builtin { f: Builtin::Len, args: vec![b] }, Type::Int)),
            _ => err(pos, format!("{} has no method '{m}' usable here", self.tname(&bt))),
        }
    }

    fn call(&self, ctx: &mut FnCtx, name: &str, args: &[ast::Expr], pos: Pos) -> CResult<(Expr, Type)> {
        let n = args.len();
        let arity = |want: usize| -> CResult<()> {
            if n != want {
                return err(pos, format!("{name}() takes {want} argument(s), {n} given"));
            }
            Ok(())
        };
        let b = |f: Builtin, a: Vec<Expr>| Expr::Builtin { f, args: a };
        match name {
            "address" => return Ok((Expr::Const(self.address_literal(args, pos)?), Type::Address)),
            "len" => {
                arity(1)?;
                if let Some(path) = args[0].dotted() {
                    if let Some(var) = self.lookup_state(ctx, &path) {
                        if matches!(self.states[var as usize].ty, Type::List(_)) {
                            return Ok((Expr::StateListLen { var }, Type::Int));
                        }
                    }
                }
                let (x, t) = self.expr(ctx, &args[0])?;
                if !matches!(t, Type::List(_) | Type::Text | Type::Bytes) {
                    return err(pos, format!("len() needs a list, text or bytes, found {}", self.tname(&t)));
                }
                return Ok((b(Builtin::Len, vec![x]), Type::Int));
            }
            "sha256" | "blake3" => {
                arity(1)?;
                let (x, t) = self.expr(ctx, &args[0])?;
                let x = match t {
                    Type::Bytes => x,
                    Type::Text => b(Builtin::BytesOfText, vec![x]),
                    other => return err(pos, format!("{name}() needs bytes or text, found {}", self.tname(&other))),
                };
                return Ok((b(if name == "sha256" { Builtin::Sha256 } else { Builtin::Blake3 }, vec![x]), Type::Bytes));
            }
            "to_bytes" => {
                arity(1)?;
                let (x, t) = self.expr(ctx, &args[0])?;
                let f = match t {
                    Type::Int => Builtin::BytesOfInt,
                    Type::Address | Type::Interface(_) => Builtin::BytesOfAddress,
                    Type::Text => Builtin::BytesOfText,
                    Type::Bool => Builtin::BytesOfBool,
                    Type::Bytes => return Ok((x, Type::Bytes)),
                    other => return err(pos, format!("to_bytes() does not accept {}", self.tname(&other))),
                };
                return Ok((b(f, vec![x]), Type::Bytes));
            }
            "to_text" => {
                arity(1)?;
                let (x, t) = self.expr(ctx, &args[0])?;
                return match t {
                    Type::Int => Ok((b(Builtin::TextOfInt, vec![x]), Type::Text)),
                    Type::Enum(e) => Ok((b(Builtin::TextOfEnum, vec![Expr::Const(Value::Int(e as i128)), x]), Type::Text)),
                    other => err(pos, format!("expected int, found {}", self.tname(&other))),
                };
            }
            "to_int" => {
                arity(1)?;
                let x = self.expect_type(ctx, &args[0], &Type::Bytes)?;
                return Ok((b(Builtin::IntOfBytes, vec![x]), Type::Int));
            }
            "min" | "max" => {
                arity(2)?;
                let a = self.expect_type(ctx, &args[0], &Type::Int)?;
                let c = self.expect_type(ctx, &args[1], &Type::Int)?;
                return Ok((b(if name == "min" { Builtin::Min } else { Builtin::Max }, vec![a, c]), Type::Int));
            }
            "abs" => {
                arity(1)?;
                let a = self.expect_type(ctx, &args[0], &Type::Int)?;
                return Ok((b(Builtin::Abs, vec![a]), Type::Int));
            }
            "slice" => {
                arity(3)?;
                let x = self.expect_type(ctx, &args[0], &Type::Bytes)?;
                let s = self.expect_type(ctx, &args[1], &Type::Int)?;
                let e = self.expect_type(ctx, &args[2], &Type::Int)?;
                return Ok((b(Builtin::Slice, vec![x, s, e]), Type::Bytes));
            }
            "verify_ed25519" => {
                arity(3)?;
                let mut a = Vec::new();
                for x in args {
                    a.push(self.expect_type(ctx, x, &Type::Bytes)?);
                }
                return Ok((b(Builtin::VerifyEd25519, a), Type::Bool));
            }
            "ring_verify" => {
                arity(4)?;
                let ring = self.expect_type(ctx, &args[0], &Type::List(Box::new(Type::Bytes)))?;
                let mut a = vec![ring];
                for x in &args[1..] {
                    a.push(self.expect_type(ctx, x, &Type::Bytes)?);
                }
                return Ok((b(Builtin::RingVerify, a), Type::Bool));
            }
            "address_of" => {
                arity(1)?;
                let x = self.expect_type(ctx, &args[0], &Type::Bytes)?;
                return Ok((b(Builtin::AddressOfKey, vec![x]), Type::Address));
            }
            "zero_address" => {
                arity(0)?;
                return Ok((b(Builtin::ZeroAddress, vec![]), Type::Address));
            }
            "range" => return err(pos, "range() can only be used in 'for i in range(start, end)'"),
            _ => {}
        }
        let q = qualify(&ctx.unit, name);
        if self.fn_index.contains_key(&q) {
            return self.call_function(ctx, &q, name, args, pos);
        }
        // Interface cast: Token(addr)
        if let Some(i) = self.iface_index.get(&q) {
            arity(1)?;
            let a = self.expect_type(ctx, &args[0], &Type::Address)?;
            return Ok((a, Type::Interface(*i)));
        }
        if self.record_index.contains_key(&q) {
            let def = &self.records[self.record_index[&q] as usize];
            return Err(CompileError::new(pos, format!("records are built with named fields: {name}(field: value, ...)"))
                .with_help(format!("{name}({})", def.fields.iter().map(|(n, _)| format!("{n}: ...")).collect::<Vec<_>>().join(", "))));
        }
        let v2 = |f: Builtin, types: &[Type], ret: Type, ctx: &mut FnCtx| -> CResult<(Expr, Type)> {
            if n != types.len() {
                return err(pos, format!("{name}() takes {} argument(s), {n} given", types.len()));
            }
            let mut a = Vec::new();
            for (x, t) in args.iter().zip(types) {
                a.push(self.expect_type(ctx, x, t)?);
            }
            Ok((Expr::Builtin { f, args: a }, ret))
        };
        match name {
            "mul_div" => return v2(Builtin::MulDiv, &[Type::Int, Type::Int, Type::Int], Type::Int, ctx),
            "isqrt" => return v2(Builtin::Isqrt, &[Type::Int], Type::Int, ctx),
            "pow" => return v2(Builtin::Pow, &[Type::Int, Type::Int], Type::Int, ctx),
            "code_hash" => return v2(Builtin::CodeHash, &[Type::Address], Type::Bytes, ctx),
            "is_contract" => return v2(Builtin::IsContract, &[Type::Address], Type::Bool, ctx),
            "is_final" => return v2(Builtin::IsFinal, &[Type::Address], Type::Bool, ctx),
            _ => {}
        }
        let mut cands: Vec<String> = self
            .fn_index
            .keys()
            .filter_map(|k| match &ctx.unit {
                Some(m) => k.strip_prefix(&format!("{m}.")).map(str::to_string),
                None => (!k.contains('.')).then(|| k.clone()),
            })
            .collect();
        cands.extend(RESERVED.iter().chain(V2_BUILTINS).map(|s| s.to_string()));
        let mut e = CompileError::new(pos, format!("unknown function '{name}'"));
        if let Some(h) = suggest(name, cands.iter().map(String::as_str)) {
            e = e.with_help(h);
        }
        Err(e)
    }

    fn call_function(&self, ctx: &mut FnCtx, q: &str, shown: &str, args: &[ast::Expr], pos: Pos) -> CResult<(Expr, Type)> {
        let Some(&idx) = self.fn_index.get(q) else {
            let mut e = CompileError::new(pos, format!("unknown function '{shown}'"));
            if let Some((module, f)) = q.split_once('.') {
                let cands: Vec<&str> = self.fn_index.keys().filter_map(|k| k.strip_prefix(&format!("{module}."))).collect();
                if let Some(h) = suggest(f, cands) {
                    e = e.with_help(h);
                }
            }
            return Err(e);
        };
        let sig = &self.sigs[idx as usize];
        if sig.kind != FuncKind::Fn {
            return Err(CompileError::new(pos, format!("'{shown}' is an entry point; only 'fn' helpers can be called from code"))
                .with_help("move the shared logic into a 'fn' and call it from both places"));
        }
        let n = args.len();
        if sig.params.len() != n {
            return err(pos, format!("{shown}() takes {} argument(s), {n} given", sig.params.len()));
        }
        let mut out = Vec::new();
        for (a, t) in args.iter().zip(sig.params.iter()) {
            out.push(self.expect_type(ctx, a, t)?);
        }
        ctx.calls.insert(idx);
        Ok((Expr::Call { func: idx, args: out }, sig.ret.clone()))
    }
}

enum Root {
    Local(u16),
    State(u16),
    LocalItem(u16, Expr),
    MapItem(u16, Expr),
    StateItem(u16, Expr),
}

fn always_returns(stmts: &[Stmt]) -> bool {
    match stmts.last() {
        Some(Stmt::Return(_)) => true,
        Some(Stmt::If { then, els, .. }) => always_returns(then) && always_returns(els),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(src: &str) -> Program {
        compile(src, &CompileOptions::default()).unwrap_or_else(|e| panic!("{e} (help: {:?})\n{src}", e.help))
    }

    fn fails(src: &str, needle: &str) -> CompileError {
        match compile(src, &CompileOptions::default()) {
            Ok(_) => panic!("expected error containing '{needle}'\n{src}"),
            Err(e) => {
                assert!(e.message.contains(needle), "error '{}' does not contain '{needle}'", e.message);
                e
            }
        }
    }

    #[test]
    fn compiles_valid_contract() {
        let p = ok(r#"
contract Bank
const MIN: int = 2 * TCN
state balances: map[address, int]
state owners: list[address]
state count: int = 5
event Deposit(who: address, amount: int)

action deposit() payable:
    require value >= MIN, "too small"
    balances[caller] += value
    owners.push(caller)
    emit Deposit(caller, value)

action withdraw(amount: int):
    require balances[caller] >= amount, "not enough"
    balances[caller] -= amount
    send(caller, amount)

view total() -> int:
    let sum: int = 0
    for who in owners:
        sum += balances[who]
    return sum

view doubled(x: int) -> int:
    return twice(x)

fn twice(x: int) -> int:
    if x > 0:
        return x * 2
    else:
        return 0
"#);
        assert_eq!(p.version, 2);
        assert_eq!(p.functions.len(), 5);
        assert!(p.functions.iter().find(|f| f.name == "deposit").unwrap().mutates);
        assert!(!p.functions.iter().find(|f| f.name == "twice").unwrap().mutates);
        assert_eq!(p.abi().len(), 4);
        assert!(p.effects.sends_tcn && p.effects.receives_tcn && !p.effects.calls_contracts);
    }

    #[test]
    fn rejects_bad_contracts() {
        fails("contract A\nview f() -> int:\n    let x: int = true\n    return x\n", "expected int");
        fails("contract A\nview f() -> int:\n    if true:\n        return 1\n", "every path");
        fails("contract A\nstate n: int\nview f() -> int:\n    n = 2\n    return n\n", "view cannot");
        fails("contract A\nstate n: int\nview f() -> int:\n    return g()\nfn g() -> int:\n    n = 1\n    return n\n", "changes state");
        fails("contract A\nview f() -> int:\n    return y\n", "unknown name");
        fails("contract A\nview f(caller: int) -> int:\n    return 1\n", "reserved");
        fails("contract A\naction f():\n    let m: map[int, int] = 1\n", "maps can only");
        fails("contract A\nconst C: int = 1\naction f():\n    C = 2\n", "constant");
        fails("contract A\naction f():\n    break\n", "outside of a loop");
        fails("contract A\nview f() -> bool:\n    return 1 == \"a\"\n", "cannot compare");
        fails("contract A\naction f():\n    1 + 2\n", "does nothing");
        fails("contract A\naction f():\n    g()\naction g():\n    pass\n", "entry point");
        fails("contract A\nconst X: address = address(\"tc1notvalid\")\naction f():\n    pass\n", "invalid address");
    }

    #[test]
    fn suggestions() {
        let e = fails("contract A\nstate balances: map[address, int]\nview f() -> int:\n    return balnces[caller]\n", "unknown name");
        assert_eq!(e.help.as_deref(), Some("did you mean 'balances'?"));
        let e2 = fails("contract A\nstate total: int\nview f() -> int:\n    return totl\n", "unknown name");
        assert_eq!(e2.help.as_deref(), Some("did you mean 'total'?"));
        let e3 = fails("contract A\nevent Paid(x: int)\naction f():\n    emit Payd(1)\n", "unknown event");
        assert_eq!(e3.help.as_deref(), Some("did you mean 'Paid'?"));
    }

    #[test]
    fn records_enums_roles() {
        let p = ok(r#"
contract Shop
enum Phase:
    Open -> Paid, Cancelled
    Paid -> Shipped
    Shipped
    Cancelled
record Order:
    buyer: address
    amount: int
    phase: Phase
role clerk
state orders: map[int, Order]
state next_id: int

init():
    grant clerk to caller

action order() payable -> int:
    next_id += 1
    orders[next_id] = Order(buyer: caller, amount: value, phase: Phase.Open)
    return next_id

action ship(id: int) only clerk:
    orders[id].phase = Phase.Shipped

view phase_of(id: int) -> text:
    return to_text(orders[id].phase)

view is_clerk(who: address) -> bool:
    return clerk.has(who)
"#);
        assert_eq!(p.records.len(), 1);
        assert_eq!(p.enums[0].transitions.as_ref().unwrap()[0], vec![1, 3]);
        assert_eq!(p.access.len(), 1);
        assert_eq!(p.events.len(), 2, "role events added");
        fails("contract A\nrecord R:\n    x: int\nview f() -> R:\n    return R(y: 1)\n", "no field 'y'");
        fails("contract A\nrecord R:\n    x: int\n    y: int\nview f() -> R:\n    return R(x: 1)\n", "missing: y");
        fails("contract A\nenum E:\n    A -> Z\n    B\naction f():\n    pass\n", "not a variant");
        fails("contract A\nstate owner: address\nview f() -> int only owner:\n    return 1\n", "'only' can be used");
    }

    #[test]
    fn interfaces_and_calls() {
        let p = ok(r#"
contract Payer
interface Token:
    action transfer(to: address, amount: int) -> bool
    view balance_of(who: address) -> int
    action buy() payable

action pay(token: address, to: address, amount: int):
    let t: Token = Token(token)
    require t.transfer(to, amount), "transfer failed"
    t.buy() with value 5

view mine(token: address) -> int:
    return Token(token).balance_of(self)
"#);
        assert!(p.effects.calls_contracts);
        fails(
            "contract A\ninterface T:\n    action f()\nview g(a: address) -> int:\n    T(a).f()\n    return 1\n",
            "a view cannot call the action",
        );
        fails("contract A\ninterface T:\n    action f()\naction g(a: address):\n    T(a).f() with value 1\n", "not payable");
    }

    #[test]
    fn modules_and_std() {
        let p = ok(r#"
contract Coin
use std.token

init():
    token.setup("Cloud", "CLD", 8)
    token.mint(caller, 1000)

view supply() -> int:
    return token.total_supply
"#);
        assert!(p.find("transfer").is_some(), "module actions are exported");
        assert!(p.find("token.mint").is_some());
        assert_eq!(p.modules.len(), 1);
        fails("contract Coin\nuse std.token\naction f():\n    token.total_supply = 5\n", "can only be changed by the functions of module token");
        ok("contract App\nuse mathx\nview f() -> int:\n    return mathx.double(21)\n\nmodule mathx\nfn double(x: int) -> int:\n    return x * 2\n");
        fails("contract App\nview f() -> int:\n    return 1\n\nmodule unused\nfn g() -> int:\n    return 1\n", "never used");
    }

    #[test]
    fn upgrade_keeps_state_slots() {
        let old = ok("contract A\nstate a: int\nstate b: int\naction f():\n    a += 1\n");
        let opts = CompileOptions { state_order: old.states.iter().map(|s| s.name.clone()).collect(), ..Default::default() };
        let new = compile("contract A\nstate c: int\nstate b: int\nstate a: int\naction f():\n    c += 1\n", &opts).unwrap();
        assert_eq!(new.states.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["a", "b", "c"]);
        assert!(compile("contract A\nstate a: int\naction f():\n    a += 1\n", &opts).is_err(), "removing b is rejected");
    }

    #[test]
    fn version_one_sources_are_accepted_by_version_two() {
        for f in std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/examples")).unwrap() {
            let path = f.unwrap().path();
            let src = std::fs::read_to_string(&path).unwrap();
            let v1 = compile(&src, &CompileOptions { version: 1, ..Default::default() });
            let Ok(v1) = v1 else { continue };
            let v2 = compile(&src, &CompileOptions::default()).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert_eq!(v1.states, v2.states, "{}", path.display());
            assert_eq!(v1.events, v2.events, "{}", path.display());
            assert_eq!(v1.functions, v2.functions, "{}", path.display());
        }
    }
}
