//! Deterministic interpreter for compiled TCCL programs.
//!
//! * Every operation consumes **fuel**; running out aborts the call.
//! * All arithmetic is checked; errors abort the call and the host reverts
//!   every change made by it.
//! * Memory is bounded: values are limited to 64 KiB, in-memory lists to 4 096
//!   items, call depth to 16.
//! * No floating point, no clock, no randomness: the same call on the same
//!   state always gives the same result on every node.
//!
//! Version 2 programs additionally get:
//! * **calls to other contracts** through interfaces: the callee sees the calling
//!   contract as `caller`, `origin` stays the signer; fuel is shared;
//! * **re-entrancy protection**: a contract that is already running in the
//!   transaction cannot be called again (always on, cannot be disabled);
//! * **atomicity**: any failure anywhere aborts the whole transaction, and the
//!   host discards every change made by every contract involved;
//! * a **memory limit** on the values held by running functions;
//! * **enum transitions** checked whenever a value is stored.
//!
//! Version 1 programs run exactly as on The Coin v0.2.0.

use crate::program::BinOp;
use crate::error::VmError;
use crate::ops::{self, MAX_LIST_LEN, MAX_VALUE_BYTES};
use crate::program::*;
use sha2::Digest;

pub const MAX_CALL_DEPTH: usize = 16;
pub const MAX_RING_SIZE: usize = 64;
/// Most contracts on the call stack of one transaction (version 2).
pub const MAX_CONTRACT_DEPTH: usize = 8;
/// Bytes of values that the functions running in a transaction may hold (version 2).
pub const MAX_MEMORY_BYTES: u64 = 16 * 1024 * 1024;

/// Fuel schedule (consensus-critical).
/// Fuel prices. Calibrated with `tests/fuel_bench.rs` and the node's
/// `fuel_storage_bench.rs` so that every operation costs roughly 20 ns of CPU
/// per unit of fuel on a 2 vCPU VPS: a completely full block (50 M fuel) then
/// executes in about one second in the worst case.
pub mod fuel {
    pub const STMT: u64 = 2;
    pub const EXPR: u64 = 1;
    pub const CALL: u64 = 20;
    pub const PER_32_BYTES: u64 = 1;
    pub const STORAGE_READ: u64 = 250;
    pub const STORAGE_WRITE: u64 = 400;
    pub const STORAGE_WRITE_PER_BYTE: u64 = 4;
    pub const HASH: u64 = 60;
    pub const HASH_PER_64_BYTES: u64 = 20;
    pub const ED25519_VERIFY: u64 = 3_500;
    pub const RING_BASE: u64 = 5_000;
    pub const RING_PER_MEMBER: u64 = 10_000;
    pub const SEND: u64 = 300;
    pub const EMIT: u64 = 100;
    pub const DESTROY: u64 = 1_000;
    /// Charged by the host per byte of source code when deploying.
    pub const COMPILE_PER_BYTE: u64 = 5;
    /// Version 2: a call to another contract, plus [`LOAD_BASE`] + 1 per [`LOAD_PER_BYTES`] of its code.
    pub const CALL_CONTRACT: u64 = 700;
    pub const LOAD_BASE: u64 = 100;
    pub const LOAD_PER_BYTES: u64 = 100;
    /// Version 2: `mul_div`, `isqrt`, `pow`.
    pub const MATH: u64 = 30;
    /// Version 2: per heap allocation when a value is copied (text, bytes and list
    /// nodes). Version 1 charges only per 32 bytes, which under-prices copying lists
    /// of many small texts or lists by ~50× (see implant-the-coin-language.md).
    pub const CLONE_NODE: u64 = 4;
    /// Version 2: per heap allocation when a stored value is decoded.
    pub const DECODE_NODE: u64 = 6;
}

/// Facts about a contract that other contracts may inspect (version 2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractInfo {
    /// BLAKE3 of the compiled program.
    pub code_hash: [u8; 32],
    /// Whether an upgrade authority can still replace the code.
    pub upgradeable: bool,
}

/// Execution context of a call.
#[derive(Clone, Debug)]
pub struct CallContext {
    pub caller: [u8; 20],
    /// TCN (in motes) transferred with the call.
    pub value: u64,
    pub height: u64,
    pub self_address: [u8; 20],
}

/// What the blockchain provides to a running contract.
///
/// Keys are contract-local; the host namespaces them per contract and keeps
/// every write in a revertible overlay.
pub trait Host {
    fn storage_read(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>, VmError>;
    fn storage_write(&mut self, key: &[u8], value: Option<Vec<u8>>) -> Result<(), VmError>;
    /// Contract balance in motes (includes the value of the current call).
    fn balance(&mut self) -> Result<u64, VmError>;
    fn send(&mut self, to: &[u8; 20], amount: u64) -> Result<(), VmError>;
    fn emit(&mut self, event: &str, fields: Vec<(String, Value)>) -> Result<(), VmError>;
    /// Number of storage entries of the contract.
    fn storage_items(&mut self) -> Result<u64, VmError>;
    /// Removes the contract, paying its remaining balance to `to`.
    fn destroy(&mut self, to: &[u8; 20]) -> Result<(), VmError>;

    // ---- version 2 (defaults: calls between contracts are not available) ----

    /// Starts a call from `caller` (the running contract) into `callee`: moves
    /// `value` motes from caller to callee and makes `callee` the current contract
    /// for storage, balance, events and sends. Returns the callee program and the
    /// size of its code, or `None` if there is no contract at `callee` (nothing changed).
    fn enter_contract(&mut self, caller: &[u8; 20], callee: &[u8; 20], value: u64) -> Result<Option<(std::sync::Arc<Program>, usize)>, VmError> {
        let _ = (caller, callee, value);
        Err(VmError::Unsupported("calls between contracts are not enabled on this network".into()))
    }

    /// Ends the call started by the last successful `enter_contract`.
    fn leave_contract(&mut self) -> Result<(), VmError> {
        Ok(())
    }

    /// Code hash and upgradeability of a contract, `None` if `addr` is not a contract.
    fn contract_info(&mut self, addr: &[u8; 20]) -> Result<Option<ContractInfo>, VmError> {
        let _ = addr;
        Ok(None)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Deployment: state initial values, then `init` (if any).
    Deploy,
    /// Transaction calling an `action`.
    Action,
    /// Read-only query of a `view`.
    View,
    /// Version 2: the upgrade transaction runs `upgrade()` (if any) of the new code.
    Upgrade,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Outcome {
    pub result: Result<Value, VmError>,
    pub fuel_used: u64,
}

enum Flow {
    Normal,
    Break,
    Continue,
    Return(Value),
    Halt,
}

// Storage key layout (contract-local).
fn scalar_key(var: u16) -> Vec<u8> {
    let mut k = vec![0u8];
    k.extend_from_slice(&var.to_be_bytes());
    k
}
fn map_key(var: u16, key: &Value) -> Vec<u8> {
    let mut k = vec![1u8];
    k.extend_from_slice(&var.to_be_bytes());
    k.extend_from_slice(&key.key_bytes());
    k
}
fn list_len_key(var: u16) -> Vec<u8> {
    let mut k = vec![2u8];
    k.extend_from_slice(&var.to_be_bytes());
    k
}
fn list_item_key(var: u16, index: u64) -> Vec<u8> {
    let mut k = vec![3u8];
    k.extend_from_slice(&var.to_be_bytes());
    k.extend_from_slice(&index.to_be_bytes());
    k
}

struct Vm<'a, H: Host> {
    program: &'a Program,
    host: &'a mut H,
    ctx: &'a CallContext,
    fuel_left: u64,
    depth: usize,
    read_only: bool,
    // ---- version 2 ----
    origin: [u8; 20],
    /// Contracts running in this transaction (outermost first).
    stack: Vec<[u8; 20]>,
    contract_depth: usize,
    mem_used: u64,
}

/// Runs `function` of `program`.
///
/// In [`Mode::Deploy`] the function name is ignored and `init` is used.
pub fn execute<H: Host>(
    program: &Program,
    mode: Mode,
    function: &str,
    args: Vec<Value>,
    ctx: &CallContext,
    host: &mut H,
    fuel_limit: u64,
) -> Outcome {
    let mut vm = Vm {
        program,
        host,
        ctx,
        fuel_left: fuel_limit,
        depth: 0,
        read_only: mode == Mode::View,
        origin: ctx.caller,
        stack: vec![ctx.self_address],
        contract_depth: 0,
        mem_used: 0,
    };
    let result = vm.run(mode, function, args);
    Outcome { result, fuel_used: fuel_limit - vm.fuel_left }
}

impl<'a, H: Host> Vm<'a, H> {
    fn charge(&mut self, amount: u64) -> Result<(), VmError> {
        match self.fuel_left.checked_sub(amount) {
            Some(left) => {
                self.fuel_left = left;
                Ok(())
            }
            None => {
                self.fuel_left = 0;
                Err(VmError::OutOfFuel)
            }
        }
    }

    fn run(&mut self, mode: Mode, function: &str, args: Vec<Value>) -> Result<Value, VmError> {
        match mode {
            Mode::Deploy => {
                for (i, s) in self.program.states.iter().enumerate() {
                    if let Some(v) = &s.init {
                        self.write_scalar(i as u16, &s.ty, v.clone())?;
                    }
                }
                match self.program.find("init") {
                    Some((idx, f)) => {
                        if self.ctx.value > 0 && !f.payable {
                            return Err(VmError::NotPayable);
                        }
                        self.call_entry(idx, args)
                    }
                    None => {
                        if !args.is_empty() {
                            return Err(VmError::BadArguments("this contract has no init()".into()));
                        }
                        if self.ctx.value > 0 {
                            return Err(VmError::NotPayable);
                        }
                        Ok(Value::Unit)
                    }
                }
            }
            Mode::Upgrade => {
                if self.program.version < 2 {
                    return Err(VmError::Unsupported("upgrades need language version 2".into()));
                }
                if self.ctx.value > 0 {
                    return Err(VmError::NotPayable);
                }
                match self.program.functions.iter().position(|f| f.kind == FnKind::Upgrade) {
                    Some(idx) => self.call_entry(idx as u16, args),
                    None if args.is_empty() => Ok(Value::Unit),
                    None => Err(VmError::BadArguments("this program has no upgrade()".into())),
                }
            }
            Mode::Action | Mode::View => {
                let (idx, f) = self.program.find(function).ok_or_else(|| VmError::UnknownFunction(function.to_string()))?;
                let expected = if mode == Mode::Action { FnKind::Action } else { FnKind::View };
                if f.kind != expected {
                    return Err(VmError::NotCallable(function.to_string()));
                }
                if self.ctx.value > 0 && !f.payable {
                    return Err(VmError::NotPayable);
                }
                self.call_entry(idx, args)
            }
        }
    }

    fn call_entry(&mut self, idx: u16, args: Vec<Value>) -> Result<Value, VmError> {
        let f = &self.program.functions[idx as usize];
        if args.len() != f.params.len() {
            return Err(VmError::BadArguments(format!("{} expects {} argument(s), got {}", f.name, f.params.len(), args.len())));
        }
        for (i, (a, (name, t))) in args.iter().zip(f.params.iter()).enumerate() {
            if !self.program.value_has_type(a, t) {
                return Err(VmError::BadArguments(format!("argument {} ({name}) must be {}", i + 1, self.program.type_name(t))));
            }
            check_size(a)?;
        }
        self.call(idx, args)
    }

    fn v2(&self) -> bool {
        self.program.version >= 2
    }

    /// Fuel to copy a value: per 32 bytes, plus per heap allocation in version 2.
    fn charge_copy(&mut self, v: &Value) -> Result<(), VmError> {
        let mut cost = v.size() as u64 / 32 * fuel::PER_32_BYTES;
        if self.v2() {
            cost += heap_nodes(v) * fuel::CLONE_NODE;
        }
        self.charge(cost)
    }

    /// Version 2 memory accounting: `delta` bytes more (or fewer) held by locals.
    fn memory(&mut self, add: u64, remove: u64) -> Result<(), VmError> {
        if !self.v2() {
            return Ok(());
        }
        self.mem_used = self.mem_used.saturating_sub(remove).saturating_add(add);
        if self.mem_used > MAX_MEMORY_BYTES {
            return Err(VmError::MemoryLimit(MAX_MEMORY_BYTES));
        }
        Ok(())
    }

    fn call(&mut self, idx: u16, args: Vec<Value>) -> Result<Value, VmError> {
        self.charge(fuel::CALL)?;
        if self.depth >= MAX_CALL_DEPTH {
            return Err(VmError::CallDepth);
        }
        self.depth += 1;
        let program = self.program;
        let f = &program.functions[idx as usize];
        let mut locals = vec![Value::Unit; f.locals as usize];
        let arg_bytes: u64 = args.iter().map(|a| a.size() as u64).sum();
        self.memory(arg_bytes + f.locals as u64, 0)?;
        for (i, a) in args.into_iter().enumerate() {
            locals[i] = a;
        }
        let flow = self.block(&f.body, &mut locals)?;
        if self.v2() {
            let held: u64 = locals.iter().map(|v| v.size() as u64).sum::<u64>() + f.locals as u64;
            self.mem_used = self.mem_used.saturating_sub(held);
        }
        self.depth -= 1;
        Ok(match flow {
            Flow::Return(v) => v,
            _ => Value::Unit,
        })
    }

    fn block(&mut self, stmts: &[Stmt], locals: &mut Vec<Value>) -> Result<Flow, VmError> {
        for s in stmts {
            match self.stmt(s, locals)? {
                Flow::Normal => {}
                other => return Ok(other),
            }
        }
        Ok(Flow::Normal)
    }

    fn mutation(&self) -> Result<(), VmError> {
        if self.read_only {
            Err(VmError::ReadOnly)
        } else {
            Ok(())
        }
    }

    fn stmt(&mut self, s: &Stmt, locals: &mut Vec<Value>) -> Result<Flow, VmError> {
        self.charge(fuel::STMT)?;
        match s {
            Stmt::SetLocal { slot, value } => {
                let v = self.eval(value, locals)?;
                self.memory(v.size() as u64, locals[*slot as usize].size() as u64)?;
                locals[*slot as usize] = v;
            }
            Stmt::SetState { var, value } => {
                self.mutation()?;
                let v = self.eval(value, locals)?;
                let ty = self.program.states[*var as usize].ty.clone();
                self.write_scalar(*var, &ty, v)?;
            }
            Stmt::SetMap { var, key, value } => {
                self.mutation()?;
                let k = self.eval(key, locals)?;
                let v = self.eval(value, locals)?;
                let Type::Map(_, vt) = &self.program.states[*var as usize].ty else {
                    return Err(VmError::Type("not a map".into()));
                };
                let default = self.program.default_value(vt);
                let key = map_key(*var, &k);
                if v == default {
                    self.write(&key, None)?;
                } else {
                    self.write(&key, Some(borsh::to_vec(&v).expect("serializable")))?;
                }
            }
            Stmt::RemoveMap { var, key } => {
                self.mutation()?;
                let k = self.eval(key, locals)?;
                self.write(&map_key(*var, &k), None)?;
            }
            Stmt::SetStateListItem { var, index, value } => {
                self.mutation()?;
                let i = self.eval_int(index, locals)?;
                let v = self.eval(value, locals)?;
                let len = self.list_len(*var)?;
                let idx = check_index(i, len)?;
                self.write(&list_item_key(*var, idx), Some(borsh::to_vec(&v).expect("serializable")))?;
            }
            Stmt::PushStateList { var, value } => {
                self.mutation()?;
                let v = self.eval(value, locals)?;
                let len = self.list_len(*var)?;
                self.write(&list_item_key(*var, len), Some(borsh::to_vec(&v).expect("serializable")))?;
                self.write(&list_len_key(*var), Some((len + 1).to_be_bytes().to_vec()))?;
            }
            Stmt::PopStateList { var } => {
                self.mutation()?;
                self.pop_state_list(*var)?;
            }
            Stmt::SetLocalListItem { slot, index, value } => {
                let i = self.eval_int(index, locals)?;
                let v = self.eval(value, locals)?;
                let Value::List(items) = &mut locals[*slot as usize] else {
                    return Err(VmError::Type("not a list".into()));
                };
                let idx = check_index(i, items.len() as u64)? as usize;
                let (add, remove) = (v.size() as u64, items[idx].size() as u64);
                items[idx] = v;
                self.memory(add, remove)?;
            }
            Stmt::PushLocalList { slot, value } => {
                let v = self.eval(value, locals)?;
                let Value::List(items) = &mut locals[*slot as usize] else {
                    return Err(VmError::Type("not a list".into()));
                };
                if items.len() >= MAX_LIST_LEN {
                    return Err(VmError::TooLarge);
                }
                self.charge(fuel::EXPR + v.size() as u64 / 32)?;
                let add = v.size() as u64;
                items.push(v);
                self.memory(add, 0)?;
            }
            Stmt::If { cond, then, els } => {
                return if self.eval_bool(cond, locals)? { self.block(then, locals) } else { self.block(els, locals) };
            }
            Stmt::While { cond, body } => {
                while self.eval_bool(cond, locals)? {
                    match self.block(body, locals)? {
                        Flow::Break => break,
                        Flow::Normal | Flow::Continue => {}
                        other => return Ok(other),
                    }
                }
            }
            Stmt::ForRange { slot, start, end, body } => {
                let s = self.eval_int(start, locals)?;
                let e = self.eval_int(end, locals)?;
                let mut i = s;
                while i < e {
                    self.charge(fuel::EXPR)?;
                    locals[*slot as usize] = Value::Int(i);
                    match self.block(body, locals)? {
                        Flow::Break => break,
                        Flow::Normal | Flow::Continue => {}
                        other => return Ok(other),
                    }
                    i += 1;
                }
            }
            Stmt::ForEachLocal { slot, list, body } => {
                let Value::List(items) = self.eval(list, locals)? else {
                    return Err(VmError::Type("not a list".into()));
                };
                for item in items {
                    self.charge(fuel::EXPR)?;
                    locals[*slot as usize] = item;
                    match self.block(body, locals)? {
                        Flow::Break => break,
                        Flow::Normal | Flow::Continue => {}
                        other => return Ok(other),
                    }
                }
            }
            Stmt::ForEachState { slot, var, body } => {
                let len = self.list_len(*var)?;
                for i in 0..len {
                    let item = self.list_get(*var, i)?;
                    locals[*slot as usize] = item;
                    match self.block(body, locals)? {
                        Flow::Break => break,
                        Flow::Normal | Flow::Continue => {}
                        other => return Ok(other),
                    }
                }
            }
            Stmt::Break => return Ok(Flow::Break),
            Stmt::Continue => return Ok(Flow::Continue),
            Stmt::Return(v) => {
                let value = match v {
                    Some(e) => self.eval(e, locals)?,
                    None => Value::Unit,
                };
                return Ok(Flow::Return(value));
            }
            Stmt::Require { cond, message } => {
                if !self.eval_bool(cond, locals)? {
                    let msg = match self.eval(message, locals)? {
                        Value::Text(t) => t,
                        _ => "requirement failed".into(),
                    };
                    return Err(VmError::Require(msg));
                }
            }
            Stmt::Send { to, amount } => {
                self.mutation()?;
                self.charge(fuel::SEND)?;
                let Value::Address(addr) = self.eval(to, locals)? else {
                    return Err(VmError::Type("not an address".into()));
                };
                let amount = self.eval_int(amount, locals)?;
                if amount <= 0 || amount > u64::MAX as i128 {
                    return Err(VmError::BadAmount);
                }
                self.host.send(&addr, amount as u64)?;
            }
            Stmt::Emit { event, args } => {
                self.mutation()?;
                let def = &self.program.events[*event as usize];
                let mut fields = Vec::with_capacity(args.len());
                let mut size = 0usize;
                for (a, (name, _)) in args.iter().zip(def.fields.iter()) {
                    let v = self.eval(a, locals)?;
                    size += v.size();
                    fields.push((name.clone(), v));
                }
                self.charge(fuel::EMIT + size as u64)?;
                self.host.emit(&def.name, fields)?;
            }
            Stmt::Destroy { to } => {
                self.mutation()?;
                if self.contract_depth > 0 {
                    return Err(VmError::DestroyInNestedCall);
                }
                self.charge(fuel::DESTROY)?;
                let Value::Address(addr) = self.eval(to, locals)? else {
                    return Err(VmError::Type("not an address".into()));
                };
                // Scalar state variables are cleared automatically; maps and lists
                // cannot be enumerated, so the contract must empty them first.
                let program = self.program;
                for (i, sv) in program.states.iter().enumerate() {
                    if sv.ty.is_scalar() {
                        self.write(&scalar_key(i as u16), None)?;
                    }
                }
                let items = self.host.storage_items()?;
                if items > 0 {
                    return Err(VmError::StorageNotEmpty(items));
                }
                self.host.destroy(&addr)?;
                return Ok(Flow::Halt);
            }
            Stmt::Eval(e) => {
                self.eval(e, locals)?;
            }
        }
        Ok(Flow::Normal)
    }

    fn read(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>, VmError> {
        self.charge(fuel::STORAGE_READ)?;
        self.host.storage_read(key)
    }

    fn write(&mut self, key: &[u8], value: Option<Vec<u8>>) -> Result<(), VmError> {
        self.mutation()?;
        let bytes = key.len() + value.as_ref().map_or(0, Vec::len);
        self.charge(fuel::STORAGE_WRITE + fuel::STORAGE_WRITE_PER_BYTE * bytes as u64)?;
        self.host.storage_write(key, value)
    }

    fn decode_value(bytes: &[u8]) -> Result<Value, VmError> {
        borsh::from_slice::<Value>(bytes).map_err(|e| VmError::Host(format!("corrupt storage value: {e}")))
    }

    /// Decodes a stored value; version 2 also pays for the allocations of large values.
    fn load_value(&mut self, bytes: &[u8]) -> Result<Value, VmError> {
        let v = Self::decode_value(bytes)?;
        if self.v2() {
            self.charge(bytes.len() as u64 / 32 * fuel::PER_32_BYTES + heap_nodes(&v) * fuel::DECODE_NODE)?;
        }
        Ok(v)
    }

    fn write_scalar(&mut self, var: u16, ty: &Type, v: Value) -> Result<(), VmError> {
        if v == self.program.default_value(ty) {
            self.write(&scalar_key(var), None)
        } else {
            self.write(&scalar_key(var), Some(borsh::to_vec(&v).expect("serializable")))
        }
    }

    fn list_len(&mut self, var: u16) -> Result<u64, VmError> {
        Ok(match self.read(&list_len_key(var))? {
            Some(b) => u64::from_be_bytes(b.try_into().map_err(|_| VmError::Host("corrupt list length".into()))?),
            None => 0,
        })
    }

    fn list_get(&mut self, var: u16, index: u64) -> Result<Value, VmError> {
        match self.read(&list_item_key(var, index))? {
            Some(b) => self.load_value(&b),
            None => Err(VmError::Host("missing list item".into())),
        }
    }

    fn pop_state_list(&mut self, var: u16) -> Result<Value, VmError> {
        let len = self.list_len(var)?;
        if len == 0 {
            return Err(VmError::IndexOutOfBounds { index: -1, len: 0 });
        }
        let v = self.list_get(var, len - 1)?;
        self.write(&list_item_key(var, len - 1), None)?;
        self.write(&list_len_key(var), if len == 1 { None } else { Some((len - 1).to_be_bytes().to_vec()) })?;
        Ok(v)
    }

    fn eval_int(&mut self, e: &Expr, locals: &[Value]) -> Result<i128, VmError> {
        match self.eval(e, locals)? {
            Value::Int(i) => Ok(i),
            _ => Err(VmError::Type("expected int".into())),
        }
    }

    fn eval_bool(&mut self, e: &Expr, locals: &[Value]) -> Result<bool, VmError> {
        match self.eval(e, locals)? {
            Value::Bool(b) => Ok(b),
            _ => Err(VmError::Type("expected bool".into())),
        }
    }

    fn eval_bytes(&mut self, e: &Expr, locals: &[Value]) -> Result<Vec<u8>, VmError> {
        match self.eval(e, locals)? {
            Value::Bytes(b) => Ok(b),
            _ => Err(VmError::Type("expected bytes".into())),
        }
    }

    fn eval(&mut self, e: &Expr, locals: &[Value]) -> Result<Value, VmError> {
        self.charge(fuel::EXPR)?;
        Ok(match e {
            Expr::Const(v) => {
                self.charge_copy(v)?;
                v.clone()
            }
            Expr::Local(slot) => {
                self.charge_copy(&locals[*slot as usize])?;
                locals[*slot as usize].clone()
            }
            // Version 2: read an item of a local list/record without copying the whole value.
            Expr::Index { base, index } if self.v2() && matches!(**base, Expr::Local(_)) => {
                let Expr::Local(slot) = **base else { unreachable!() };
                self.charge(fuel::EXPR)?;
                let i = self.eval_int(index, locals)?;
                match &locals[slot as usize] {
                    Value::List(items) => {
                        let idx = check_index(i, items.len() as u64)? as usize;
                        self.charge_copy(&items[idx])?;
                        items[idx].clone()
                    }
                    Value::Bytes(bytes) => {
                        let idx = check_index(i, bytes.len() as u64)? as usize;
                        Value::Int(bytes[idx] as i128)
                    }
                    _ => return Err(VmError::Type("cannot index".into())),
                }
            }
            Expr::State(var) => {
                let program = self.program;
                let ty = &program.states[*var as usize].ty;
                match self.read(&scalar_key(*var))? {
                    Some(b) => self.load_value(&b)?,
                    None => program.default_value(ty),
                }
            }
            Expr::MapGet { var, key } => {
                let k = self.eval(key, locals)?;
                let Type::Map(_, vt) = &self.program.states[*var as usize].ty else {
                    return Err(VmError::Type("not a map".into()));
                };
                let default = self.program.default_value(vt);
                match self.read(&map_key(*var, &k))? {
                    Some(b) => self.load_value(&b)?,
                    None => default,
                }
            }
            Expr::MapHas { var, key } => {
                let k = self.eval(key, locals)?;
                Value::Bool(self.read(&map_key(*var, &k))?.is_some())
            }
            Expr::StateListGet { var, index } => {
                let i = self.eval_int(index, locals)?;
                let len = self.list_len(*var)?;
                let idx = check_index(i, len)?;
                self.list_get(*var, idx)?
            }
            Expr::StateListLen { var } => Value::Int(self.list_len(*var)? as i128),
            Expr::StateListPop { var } => {
                self.mutation()?;
                self.pop_state_list(*var)?
            }
            Expr::List(items) => {
                if items.len() > MAX_LIST_LEN {
                    return Err(VmError::TooLarge);
                }
                let mut out = Vec::with_capacity(items.len());
                for it in items {
                    out.push(self.eval(it, locals)?);
                }
                let v = Value::List(out);
                check_size(&v)?;
                v
            }
            Expr::Index { base, index } => {
                let b = self.eval(base, locals)?;
                let i = self.eval_int(index, locals)?;
                match b {
                    Value::List(items) => {
                        let idx = check_index(i, items.len() as u64)? as usize;
                        items.into_iter().nth(idx).expect("checked index")
                    }
                    Value::Bytes(bytes) => {
                        let idx = check_index(i, bytes.len() as u64)? as usize;
                        Value::Int(bytes[idx] as i128)
                    }
                    _ => return Err(VmError::Type("cannot index".into())),
                }
            }
            Expr::Neg(inner) => Value::Int(self.eval_int(inner, locals)?.checked_neg().ok_or(VmError::Overflow)?),
            Expr::Not(inner) => Value::Bool(!self.eval_bool(inner, locals)?),
            Expr::Binary { op, left, right } => match op {
                BinOp::And => Value::Bool(self.eval_bool(left, locals)? && self.eval_bool(right, locals)?),
                BinOp::Or => Value::Bool(self.eval_bool(left, locals)? || self.eval_bool(right, locals)?),
                _ => {
                    let l = self.eval(left, locals)?;
                    let r = self.eval(right, locals)?;
                    let size = (l.size() + r.size()) as u64;
                    self.charge(size / 32 * fuel::PER_32_BYTES)?;
                    ops::binary(*op, &l, &r)?
                }
            },
            Expr::Call { func, args } => {
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(self.eval(a, locals)?);
                }
                self.call(*func, vals)?
            }
            Expr::Ctx(c) => match c {
                Ctx::Caller => Value::Address(self.ctx.caller),
                Ctx::Value => Value::Int(self.ctx.value as i128),
                Ctx::Balance => Value::Int(self.host.balance()? as i128),
                Ctx::Height => Value::Int(self.ctx.height as i128),
                Ctx::SelfAddress => Value::Address(self.ctx.self_address),
                Ctx::Origin => Value::Address(self.origin),
            },
            Expr::Builtin { f, args } => self.builtin(*f, args, locals)?,
            Expr::Replace { base, index, value } => {
                let Value::List(mut items) = self.eval(base, locals)? else {
                    return Err(VmError::Type("not a record".into()));
                };
                let v = self.eval(value, locals)?;
                let slot = items.get_mut(*index as usize).ok_or_else(|| VmError::Type("bad field".into()))?;
                *slot = v;
                let out = Value::List(items);
                check_size(&out)?;
                out
            }
            Expr::Transition { ty, from, to } => {
                let old = self.eval(from, locals)?;
                let new = self.eval(to, locals)?;
                self.check_transition(ty, &old, &new, 0)?;
                new
            }
            Expr::CallContract { target, function, kind, params, ret, args, value } => {
                let Value::Address(addr) = self.eval(target, locals)? else {
                    return Err(VmError::Type("not an address".into()));
                };
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(self.eval(a, locals)?);
                }
                let amount = match value {
                    Some(v) => {
                        let a = self.eval_int(v, locals)?;
                        if a < 0 || a > u64::MAX as i128 {
                            return Err(VmError::BadAmount);
                        }
                        a as u64
                    }
                    None => 0,
                };
                self.call_contract(addr, function, *kind, params, ret, vals, amount)?
            }
        })
    }

    fn check_transition(&mut self, ty: &Type, old: &Value, new: &Value, depth: usize) -> Result<(), VmError> {
        if depth > MAX_VALUE_DEPTH {
            return Err(VmError::TooLarge);
        }
        self.charge(fuel::EXPR)?;
        let program = self.program;
        match (ty, old, new) {
            (Type::Enum(e), Value::Int(a), Value::Int(b)) => {
                let def = program.enums.get(*e as usize).ok_or_else(|| VmError::Type("bad enum".into()))?;
                if a == b {
                    return Ok(());
                }
                if let Some(table) = &def.transitions {
                    let allowed = table.get(*a as usize).is_some_and(|next| next.iter().any(|n| *n as i128 == *b));
                    if !allowed {
                        let name = |i: i128| def.variants.get(i as usize).cloned().unwrap_or_else(|| i.to_string());
                        return Err(VmError::TransitionNotAllowed { enum_name: def.name.clone(), from: name(*a), to: name(*b) });
                    }
                }
                Ok(())
            }
            (Type::Record(r), Value::List(xs), Value::List(ys)) => {
                let def = program.records.get(*r as usize).ok_or_else(|| VmError::Type("bad record".into()))?;
                for ((_, ft), (x, y)) in def.fields.iter().zip(xs.iter().zip(ys.iter())) {
                    self.check_transition(ft, x, y, depth + 1)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn call_contract(&mut self, target: [u8; 20], function: &str, kind: FnKind, params: &[Type], ret: &Type, args: Vec<Value>, value: u64) -> Result<Value, VmError> {
        if self.read_only && (kind == FnKind::Action || value > 0) {
            return Err(VmError::ReadOnly);
        }
        let arg_bytes: u64 = args.iter().map(|a| a.size() as u64).sum();
        self.charge(fuel::CALL_CONTRACT + arg_bytes / 32 * fuel::PER_32_BYTES)?;
        if self.contract_depth + 1 >= MAX_CONTRACT_DEPTH {
            return Err(VmError::ContractDepth);
        }
        if self.depth >= MAX_CALL_DEPTH {
            return Err(VmError::CallDepth);
        }
        if self.stack.contains(&target) {
            return Err(VmError::Reentrancy(hex::encode(target)));
        }
        let Some((program, code_len)) = self.host.enter_contract(&self.ctx.self_address, &target, value)? else {
            return Err(VmError::NoContract(hex::encode(target)));
        };
        let result = self.run_callee(&program, code_len, target, function, kind, params, ret, args, value);
        let left = self.host.leave_contract();
        let v = result?;
        left?;
        Ok(v)
    }

    #[allow(clippy::too_many_arguments)]
    fn run_callee(
        &mut self,
        program: &Program,
        code_len: usize,
        target: [u8; 20],
        function: &str,
        kind: FnKind,
        params: &[Type],
        ret: &Type,
        args: Vec<Value>,
        value: u64,
    ) -> Result<Value, VmError> {
        self.charge(fuel::LOAD_BASE + code_len as u64 / fuel::LOAD_PER_BYTES)?;
        let (idx, f) = program.find(function).ok_or_else(|| VmError::UnknownFunction(function.to_string()))?;
        if f.kind != kind {
            return Err(VmError::NotCallable(function.to_string()));
        }
        let portable = |t: &Type| match t {
            Type::Interface(_) => Type::Address,
            other => other.clone(),
        };
        let callee_params: Vec<Type> = f.params.iter().map(|(_, t)| portable(t)).collect();
        if callee_params != params || portable(&f.ret) != *ret {
            let shown = |ps: &[Type], r: &Type| format!("({}) -> {}", ps.iter().map(|t| program.type_name(t)).collect::<Vec<_>>().join(", "), program.type_name(r));
            return Err(VmError::InterfaceMismatch(format!(
                "'{function}' is {} {} in the called contract but {} in the interface",
                if f.kind == FnKind::View { "view" } else { "action" },
                shown(&callee_params, &portable(&f.ret)),
                shown(params, ret)
            )));
        }
        if value > 0 && !f.payable {
            return Err(VmError::NotPayable);
        }
        let ctx = CallContext { caller: self.ctx.self_address, value, height: self.ctx.height, self_address: target };
        let mut stack = std::mem::take(&mut self.stack);
        stack.push(target);
        let mut child = Vm {
            program,
            host: &mut *self.host,
            ctx: &ctx,
            fuel_left: self.fuel_left,
            depth: self.depth + 1,
            read_only: self.read_only || kind == FnKind::View,
            origin: self.origin,
            stack,
            contract_depth: self.contract_depth + 1,
            mem_used: self.mem_used,
        };
        let r = child.call_entry(idx, args);
        self.fuel_left = child.fuel_left;
        self.mem_used = child.mem_used;
        let mut stack = std::mem::take(&mut child.stack);
        stack.pop();
        self.stack = stack;
        r
    }

    fn builtin(&mut self, f: Builtin, args: &[Expr], locals: &[Value]) -> Result<Value, VmError> {
        Ok(match f {
            Builtin::Len if self.v2() && matches!(args[0], Expr::Local(_)) => {
                let Expr::Local(slot) = args[0] else { unreachable!() };
                self.charge(fuel::EXPR)?;
                match &locals[slot as usize] {
                    Value::List(l) => Value::Int(l.len() as i128),
                    Value::Text(t) => Value::Int(t.len() as i128),
                    Value::Bytes(b) => Value::Int(b.len() as i128),
                    _ => return Err(VmError::Type("len".into())),
                }
            }
            Builtin::Len => match self.eval(&args[0], locals)? {
                Value::List(l) => Value::Int(l.len() as i128),
                Value::Text(t) => Value::Int(t.len() as i128),
                Value::Bytes(b) => Value::Int(b.len() as i128),
                _ => return Err(VmError::Type("len".into())),
            },
            Builtin::Sha256 | Builtin::Blake3 => {
                let data = self.eval_bytes(&args[0], locals)?;
                self.charge(fuel::HASH + fuel::HASH_PER_64_BYTES * (data.len() as u64 / 64))?;
                if f == Builtin::Sha256 {
                    Value::Bytes(sha2::Sha256::digest(&data).to_vec())
                } else {
                    Value::Bytes(blake3::hash(&data).as_bytes().to_vec())
                }
            }
            Builtin::BytesOfInt => Value::Bytes(self.eval_int(&args[0], locals)?.to_be_bytes().to_vec()),
            Builtin::BytesOfAddress => match self.eval(&args[0], locals)? {
                Value::Address(a) => Value::Bytes(a.to_vec()),
                _ => return Err(VmError::Type("address".into())),
            },
            Builtin::BytesOfText => match self.eval(&args[0], locals)? {
                Value::Text(t) => Value::Bytes(t.into_bytes()),
                _ => return Err(VmError::Type("text".into())),
            },
            Builtin::BytesOfBool => Value::Bytes(vec![self.eval_bool(&args[0], locals)? as u8]),
            Builtin::TextOfInt => Value::Text(self.eval_int(&args[0], locals)?.to_string()),
            Builtin::IntOfBytes => {
                let b = self.eval_bytes(&args[0], locals)?;
                if b.len() > 15 {
                    return Err(VmError::BadArguments("to_int() accepts at most 15 bytes".into()));
                }
                let mut acc: i128 = 0;
                for byte in b {
                    acc = (acc << 8) | byte as i128;
                }
                Value::Int(acc)
            }
            Builtin::Min | Builtin::Max => {
                let a = self.eval_int(&args[0], locals)?;
                let b = self.eval_int(&args[1], locals)?;
                Value::Int(if f == Builtin::Min { a.min(b) } else { a.max(b) })
            }
            Builtin::Abs => Value::Int(self.eval_int(&args[0], locals)?.checked_abs().ok_or(VmError::Overflow)?),
            Builtin::Slice => {
                let b = self.eval_bytes(&args[0], locals)?;
                let s = self.eval_int(&args[1], locals)?;
                let e = self.eval_int(&args[2], locals)?;
                if s < 0 || e < s || e > b.len() as i128 {
                    return Err(VmError::IndexOutOfBounds { index: e, len: b.len() as u64 });
                }
                Value::Bytes(b[s as usize..e as usize].to_vec())
            }
            Builtin::VerifyEd25519 => {
                let pk = self.eval_bytes(&args[0], locals)?;
                let msg = self.eval_bytes(&args[1], locals)?;
                let sig = self.eval_bytes(&args[2], locals)?;
                self.charge(fuel::ED25519_VERIFY + msg.len() as u64 / 64)?;
                Value::Bool(verify_ed25519(&pk, &msg, &sig))
            }
            Builtin::RingVerify => {
                let Value::List(ring) = self.eval(&args[0], locals)? else {
                    return Err(VmError::Type("ring".into()));
                };
                let msg = self.eval_bytes(&args[1], locals)?;
                let sig = self.eval_bytes(&args[2], locals)?;
                let key_image = self.eval_bytes(&args[3], locals)?;
                if ring.len() > MAX_RING_SIZE {
                    return Err(VmError::BadArguments(format!("ring larger than {MAX_RING_SIZE} members")));
                }
                self.charge(fuel::RING_BASE + fuel::RING_PER_MEMBER * ring.len() as u64)?;
                let mut keys = Vec::with_capacity(ring.len());
                for k in ring {
                    match k {
                        Value::Bytes(b) if b.len() == 32 => keys.push(<[u8; 32]>::try_from(b.as_slice()).expect("32 bytes")),
                        _ => return Ok(Value::Bool(false)),
                    }
                }
                let ki: [u8; 32] = match key_image.as_slice().try_into() {
                    Ok(k) => k,
                    Err(_) => return Ok(Value::Bool(false)),
                };
                Value::Bool(crate::ring::verify(&msg, &keys, &sig, &ki))
            }
            Builtin::AddressOfKey => {
                let pk = self.eval_bytes(&args[0], locals)?;
                if pk.len() != 32 {
                    return Err(VmError::BadArguments("address_of() needs a 32-byte public key".into()));
                }
                // Same derivation as The Coin addresses: tagged BLAKE3("address").
                let mut h = blake3::Hasher::new();
                h.update(b"TheCoin:address\x00");
                h.update(&pk);
                let mut a = [0u8; 20];
                a.copy_from_slice(&h.finalize().as_bytes()[..20]);
                Value::Address(a)
            }
            Builtin::ZeroAddress => Value::Address([0u8; 20]),
            Builtin::MulDiv => {
                let a = self.eval_int(&args[0], locals)?;
                let b = self.eval_int(&args[1], locals)?;
                let c = self.eval_int(&args[2], locals)?;
                self.charge(fuel::MATH)?;
                Value::Int(ops::mul_div(a, b, c)?)
            }
            Builtin::Isqrt => {
                let x = self.eval_int(&args[0], locals)?;
                self.charge(fuel::MATH)?;
                Value::Int(ops::isqrt(x)?)
            }
            Builtin::Pow => {
                let base = self.eval_int(&args[0], locals)?;
                let exp = self.eval_int(&args[1], locals)?;
                self.charge(fuel::MATH)?;
                Value::Int(ops::pow(base, exp)?)
            }
            Builtin::CodeHash | Builtin::IsContract | Builtin::IsFinal => {
                let Value::Address(addr) = self.eval(&args[0], locals)? else {
                    return Err(VmError::Type("address".into()));
                };
                self.charge(fuel::STORAGE_READ)?;
                let info = self.host.contract_info(&addr)?;
                match f {
                    Builtin::CodeHash => Value::Bytes(info.map(|i| i.code_hash.to_vec()).unwrap_or_default()),
                    Builtin::IsContract => Value::Bool(info.is_some()),
                    _ => Value::Bool(info.is_some_and(|i| !i.upgradeable)),
                }
            }
            Builtin::TextOfEnum => {
                let e = self.eval_int(&args[0], locals)?;
                let v = self.eval_int(&args[1], locals)?;
                let program = self.program;
                let def = program.enums.get(e as usize).ok_or_else(|| VmError::Type("bad enum".into()))?;
                Value::Text(def.variants.get(v as usize).cloned().ok_or_else(|| VmError::Type("bad variant".into()))?)
            }
        })
    }
}

fn verify_ed25519(pk: &[u8], msg: &[u8], sig: &[u8]) -> bool {
    let (Ok(pk), Ok(sig)) = (<[u8; 32]>::try_from(pk), <[u8; 64]>::try_from(sig)) else {
        return false;
    };
    let Ok(vk) = ed25519_dalek::VerifyingKey::from_bytes(&pk) else { return false };
    if vk.is_weak() {
        return false;
    }
    vk.verify_strict(msg, &ed25519_dalek::Signature::from_bytes(&sig)).is_ok()
}

/// Heap allocations needed to copy a value.
fn heap_nodes(v: &Value) -> u64 {
    match v {
        Value::Text(t) => u64::from(!t.is_empty()),
        Value::Bytes(b) => u64::from(!b.is_empty()),
        Value::List(items) => u64::from(!items.is_empty()) + items.iter().map(heap_nodes).sum::<u64>(),
        _ => 0,
    }
}

fn check_index(i: i128, len: u64) -> Result<u64, VmError> {
    if i < 0 || i >= len as i128 {
        return Err(VmError::IndexOutOfBounds { index: i, len });
    }
    Ok(i as u64)
}

fn check_size(v: &Value) -> Result<(), VmError> {
    if v.size() > MAX_VALUE_BYTES {
        return Err(VmError::TooLarge);
    }
    if let Value::List(items) = v {
        if items.len() > MAX_LIST_LEN {
            return Err(VmError::TooLarge);
        }
    }
    Ok(())
}
