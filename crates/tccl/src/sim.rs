//! In-memory blockchain simulator for developing and testing contracts locally
//! (used by `tccl run`, `tccl test`, the unit tests and the playground).
//!
//! It runs the **same compiler and VM** as a node and follows the same rules:
//! failed transactions revert every change (in every contract they touched),
//! TCN sent with a call is credited to the contract, `send` moves TCN out of the
//! contract balance, views never change anything. It is a simulation: nothing is
//! signed or published, accounts are fictitious, and fees and storage deposits are
//! estimates computed from The Coin's default parameters (simulated balances do
//! not include them).

use crate::abi::display_typed;
use crate::error::VmError;
use crate::program::{Program, Type, Value};
use crate::vm::{self, CallContext, ContractInfo, Host, Mode};
use crate::{compile, CompileOptions};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

pub const DEFAULT_FUEL: u64 = 5_000_000;
pub const SIM_START_BALANCE: u64 = 1_000_000 * 100_000_000;

/// Default fee parameters of The Coin mainnet/testnet (v0.2.0), used for estimates.
pub mod params {
    pub const BASE_FEE: u64 = 1_000;
    pub const FEE_PER_KB: u64 = 10_000;
    pub const FEE_PER_KFUEL: u64 = 1_000;
    pub const STORAGE_DEPOSIT_PER_KB: u64 = 100_000;
    pub const MAX_TX_FUEL: u64 = 10_000_000;
}

fn default_language() -> u16 {
    crate::program::LANGUAGE_VERSION
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SimContract {
    pub source: String,
    #[serde(skip)]
    pub program: Option<Arc<Program>>,
    /// hex key → hex value
    pub storage: BTreeMap<String, String>,
    #[serde(default = "default_language")]
    pub language: u16,
    /// Storage order of state variables (kept across upgrades).
    #[serde(default)]
    pub state_order: Vec<String>,
    /// Upgrade authority (hex address); `None` = final, the code can never change.
    #[serde(default)]
    pub authority: Option<String>,
    #[serde(default)]
    pub code_version: u32,
    #[serde(default)]
    pub deployer: String,
    /// Refundable storage deposit currently held (estimate with default parameters).
    #[serde(default)]
    pub deposit: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Event {
    pub name: String,
    pub fields: Vec<(String, Value)>,
    /// Contract that emitted the event (hex).
    #[serde(default)]
    pub contract: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Simulator {
    pub height: u64,
    /// hex address → motes
    pub balances: BTreeMap<String, u64>,
    pub contracts: BTreeMap<String, SimContract>,
    pub nonce: u64,
}

/// Estimated transaction cost with the default parameters.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct FeeEstimate {
    /// Fuel the wallet would reserve (`measured × 1.3 + 5 000`).
    pub max_fuel: u64,
    pub tx_bytes: u64,
    /// Minimum fee in motes at normal congestion.
    pub fee: u64,
    /// Storage deposit paid (positive) or refunded (negative) in motes.
    pub deposit_change: i64,
}

#[derive(Debug)]
pub struct CallResult {
    pub result: Result<Value, VmError>,
    pub fuel_used: u64,
    pub events: Vec<Event>,
    pub fee: FeeEstimate,
    /// Type of the returned value (for display).
    pub ret: Type,
}

/// Deploy settings.
#[derive(Clone, Debug)]
pub struct DeployOptions {
    pub language: u16,
    /// Deploy without an upgrade authority (the code can never change).
    pub final_code: bool,
}

impl Default for DeployOptions {
    fn default() -> Self {
        DeployOptions { language: crate::program::LANGUAGE_VERSION, final_code: false }
    }
}

/// Deterministic address for a named test account (`alice`, `bob`, ...).
pub fn account(name: &str) -> [u8; 20] {
    let h = blake3::hash(format!("tccl-sim-account:{name}").as_bytes());
    let mut a = [0u8; 20];
    a.copy_from_slice(&h.as_bytes()[..20]);
    a
}

fn parse_hex20(s: &str) -> [u8; 20] {
    let mut a = [0u8; 20];
    if let Ok(b) = hex::decode(s) {
        if b.len() == 20 {
            a.copy_from_slice(&b);
        }
    }
    a
}

struct SimHost<'s> {
    sim: &'s mut Simulator,
    /// Contract call stack (hex); the last entry is the running contract.
    stack: Vec<String>,
    events: Vec<Event>,
    destroyed: Vec<(String, String)>,
}

impl SimHost<'_> {
    fn current(&self) -> &str {
        self.stack.last().expect("a running contract")
    }

    fn credit(&mut self, to: &str, amount: u64) -> Result<(), VmError> {
        let is_contract = self.sim.contracts.contains_key(to);
        let dest = self.sim.balances.entry(to.to_string()).or_insert(if is_contract { 0 } else { SIM_START_BALANCE });
        *dest = dest.checked_add(amount).ok_or(VmError::Overflow)?;
        Ok(())
    }
}

impl Host for SimHost<'_> {
    fn storage_read(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>, VmError> {
        let c = self.sim.contracts.get(self.current()).ok_or_else(|| VmError::Host("contract missing".into()))?;
        Ok(c.storage.get(&hex::encode(key)).map(|v| hex::decode(v).expect("valid hex")))
    }

    fn storage_write(&mut self, key: &[u8], value: Option<Vec<u8>>) -> Result<(), VmError> {
        let cur = self.current().to_string();
        let c = self.sim.contracts.get_mut(&cur).ok_or_else(|| VmError::Host("contract missing".into()))?;
        match value {
            Some(v) => c.storage.insert(hex::encode(key), hex::encode(v)),
            None => c.storage.remove(&hex::encode(key)),
        };
        Ok(())
    }

    fn balance(&mut self) -> Result<u64, VmError> {
        Ok(*self.sim.balances.get(self.current()).unwrap_or(&0))
    }

    fn send(&mut self, to: &[u8; 20], amount: u64) -> Result<(), VmError> {
        let cur = self.current().to_string();
        let bal = self.sim.balances.entry(cur).or_insert(0);
        if *bal < amount {
            return Err(VmError::InsufficientBalance);
        }
        *bal -= amount;
        self.credit(&hex::encode(to), amount)
    }

    fn emit(&mut self, event: &str, fields: Vec<(String, Value)>) -> Result<(), VmError> {
        if self.events.len() >= 256 {
            return Err(VmError::TooLarge);
        }
        let contract = self.current().to_string();
        self.events.push(Event { name: event.to_string(), fields, contract });
        Ok(())
    }

    fn storage_items(&mut self) -> Result<u64, VmError> {
        Ok(self.sim.contracts.get(self.current()).map_or(0, |c| c.storage.len() as u64))
    }

    fn destroy(&mut self, to: &[u8; 20]) -> Result<(), VmError> {
        let cur = self.current().to_string();
        self.destroyed.push((cur, hex::encode(to)));
        Ok(())
    }

    fn enter_contract(&mut self, caller: &[u8; 20], callee: &[u8; 20], value: u64) -> Result<Option<(Arc<Program>, usize)>, VmError> {
        let callee_hex = hex::encode(callee);
        let Some(program) = self.sim.contracts.get(&callee_hex).and_then(|c| c.program.clone()) else {
            return Ok(None);
        };
        if value > 0 {
            let from = hex::encode(caller);
            let bal = self.sim.balances.entry(from).or_insert(0);
            if *bal < value {
                return Err(VmError::InsufficientBalance);
            }
            *bal -= value;
            self.credit(&callee_hex, value)?;
        }
        let len = program.to_bytes().len();
        self.stack.push(callee_hex);
        Ok(Some((program, len)))
    }

    fn leave_contract(&mut self) -> Result<(), VmError> {
        if self.stack.len() > 1 {
            self.stack.pop();
        }
        Ok(())
    }

    fn contract_info(&mut self, addr: &[u8; 20]) -> Result<Option<ContractInfo>, VmError> {
        Ok(self
            .sim
            .contracts
            .get(&hex::encode(addr))
            .and_then(|c| c.program.as_ref().map(|p| ContractInfo { code_hash: p.code_hash(), upgradeable: c.authority.is_some() })))
    }
}

/// One entry of a contract's state, decoded for display.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StateEntry {
    pub name: String,
    pub ty: String,
    /// Scalars and records: the value. Maps: `key → value` lines. Lists: items. Roles: members.
    pub value: String,
    pub items: Vec<(String, String)>,
}

impl Simulator {
    pub fn new() -> Self {
        Simulator { height: 1, ..Default::default() }
    }

    pub fn load(json: &str) -> Result<Simulator, String> {
        let mut sim: Simulator = serde_json::from_str(json).map_err(|e| e.to_string())?;
        for (addr, c) in sim.contracts.iter_mut() {
            let opts = CompileOptions { version: c.language, state_order: c.state_order.clone(), ..Default::default() };
            c.program = Some(Arc::new(compile(&c.source, &opts).map_err(|e| format!("contract {addr}: {e}"))?));
        }
        Ok(sim)
    }

    pub fn save(&self) -> String {
        serde_json::to_string_pretty(self).expect("serializable")
    }

    /// Balance of an address. Test accounts start with 1 000 000 TCN; contracts with 0.
    pub fn balance_of(&self, addr: &[u8; 20]) -> u64 {
        let key = hex::encode(addr);
        match self.balances.get(&key) {
            Some(b) => *b,
            None if self.contracts.contains_key(&key) => 0,
            None => SIM_START_BALANCE,
        }
    }

    fn ensure_account(&mut self, addr: &[u8; 20]) {
        self.balances.entry(hex::encode(addr)).or_insert(SIM_START_BALANCE);
    }

    /// Bytes of state a contract occupies (compiled code + storage entries).
    pub fn state_bytes(&self, addr_hex: &str) -> u64 {
        self.contracts.get(addr_hex).map_or(0, |c| {
            c.program.as_ref().map_or(0, |p| p.to_bytes().len() as u64) + c.storage.iter().map(|(k, v)| (k.len() / 2 + v.len() / 2) as u64).sum::<u64>()
        })
    }

    fn fee(tx_bytes: u64, fuel_used: u64) -> (u64, u64) {
        let max_fuel = (fuel_used.saturating_mul(13) / 10 + 5_000).min(params::MAX_TX_FUEL);
        let fee = params::BASE_FEE + (tx_bytes * params::FEE_PER_KB).div_ceil(1000) + (max_fuel * params::FEE_PER_KFUEL).div_ceil(1000);
        (max_fuel, fee)
    }

    /// Storage deposit rule of The Coin: growth pays up to the required deposit,
    /// shrinking refunds `deposit × freed ÷ old size` to the payer.
    fn settle_deposit(&mut self, addr_hex: &str, payer: &[u8; 20], old_bytes: u64, is_deploy: bool) -> i64 {
        let new_bytes = self.state_bytes(addr_hex);
        let Some(c) = self.contracts.get_mut(addr_hex) else { return 0 };
        let change: i64 = if is_deploy || new_bytes > old_bytes {
            let required = new_bytes.div_ceil(1000) * params::STORAGE_DEPOSIT_PER_KB;
            let extra = required.saturating_sub(c.deposit);
            c.deposit += extra;
            extra as i64
        } else if new_bytes < old_bytes && old_bytes > 0 && c.deposit > 0 {
            let refund = ((c.deposit as u128 * (old_bytes - new_bytes) as u128) / old_bytes as u128) as u64;
            c.deposit -= refund;
            -(refund as i64)
        } else {
            0
        };
        // Estimates only: simulated balances exclude fees and deposits.
        let _ = payer;
        change
    }

    #[allow(clippy::too_many_arguments)]
    fn run(&mut self, addr_hex: &str, mode: Mode, function: &str, caller: [u8; 20], args: Vec<Value>, value: u64, tx_bytes: u64) -> CallResult {
        let snapshot = self.clone();
        let old_bytes: BTreeMap<String, u64> = self.contracts.keys().map(|k| (k.clone(), if mode == Mode::Deploy && k == addr_hex { 0 } else { self.state_bytes(k) })).collect();
        let program = self.contracts[addr_hex].program.clone().expect("compiled");
        let ret = match mode {
            Mode::Action | Mode::View => program.find(function).map_or(Type::Unit, |(_, f)| f.ret.clone()),
            _ => Type::Unit,
        };
        if mode != Mode::View {
            self.ensure_account(&caller);
        }
        let fail = |sim: &mut Simulator, snapshot: Simulator, e: VmError| {
            let keep = sim.height;
            *sim = snapshot;
            sim.height = keep;
            CallResult { result: Err(e), fuel_used: 0, events: vec![], fee: FeeEstimate::default(), ret: Type::Unit }
        };
        if value > 0 {
            let from = self.balances.entry(hex::encode(caller)).or_insert(0);
            if *from < value {
                return fail(self, snapshot, VmError::Host("caller has insufficient balance".into()));
            }
            *from -= value;
            *self.balances.entry(addr_hex.to_string()).or_insert(0) += value;
        }
        let self_address = parse_hex20(addr_hex);
        let ctx = CallContext { caller, value, height: self.height, self_address };
        let mut host = SimHost { sim: self, stack: vec![addr_hex.to_string()], events: Vec::new(), destroyed: Vec::new() };
        let out = vm::execute(&program, mode, function, args, &ctx, &mut host, DEFAULT_FUEL);
        let events = std::mem::take(&mut host.events);
        let destroyed = std::mem::take(&mut host.destroyed);
        let (max_fuel, fee) = Self::fee(tx_bytes, out.fuel_used);
        let mut estimate = FeeEstimate { max_fuel, tx_bytes, fee, deposit_change: 0 };
        if out.result.is_err() || mode == Mode::View {
            let keep_height = self.height;
            *self = snapshot;
            self.height = keep_height;
            if mode == Mode::View {
                estimate = FeeEstimate::default();
            }
        } else {
            for (contract, to) in destroyed {
                if let Some(c) = self.contracts.remove(&contract) {
                    let _deposit = c.deposit;
                    let bal = self.balances.remove(&contract).unwrap_or(0);
                    let is_contract = self.contracts.contains_key(&to);
                    *self.balances.entry(to).or_insert(if is_contract { 0 } else { SIM_START_BALANCE }) += bal;
                }
            }
            let touched: Vec<(String, u64)> = old_bytes.into_iter().filter(|(k, _)| self.contracts.contains_key(k)).collect();
            for (k, old) in touched {
                let deploy = mode == Mode::Deploy && k == addr_hex;
                if deploy || self.state_bytes(&k) != old {
                    estimate.deposit_change += self.settle_deposit(&k, &caller, old, deploy);
                }
            }
            self.height += 1;
        }
        CallResult { result: out.result, fuel_used: out.fuel_used, events, fee: estimate, ret }
    }

    /// Deploys a contract with the current language version and the deployer as upgrade authority.
    pub fn deploy(&mut self, source: &str, deployer: [u8; 20], args: Vec<Value>, value: u64) -> Result<(String, CallResult), String> {
        self.deploy_with(source, deployer, args, value, &DeployOptions::default())
    }

    pub fn deploy_with(&mut self, source: &str, deployer: [u8; 20], args: Vec<Value>, value: u64, o: &DeployOptions) -> Result<(String, CallResult), String> {
        let opts = CompileOptions { version: o.language, ..Default::default() };
        let program = compile(source, &opts).map_err(|e| e.to_string())?;
        self.nonce += 1;
        let h = blake3::hash(&[&deployer[..], &self.nonce.to_le_bytes()].concat());
        let addr = hex::encode(&h.as_bytes()[..20]);
        let arg_bytes: u64 = args.iter().map(|a| a.size() as u64).sum();
        self.contracts.insert(
            addr.clone(),
            SimContract {
                source: source.to_string(),
                state_order: program.states.iter().map(|s| s.name.clone()).collect(),
                program: Some(Arc::new(program)),
                storage: BTreeMap::new(),
                language: o.language,
                authority: if o.final_code { None } else { Some(hex::encode(deployer)) },
                code_version: 1,
                deployer: hex::encode(deployer),
                deposit: 0,
            },
        );
        let mut r = self.run(&addr, Mode::Deploy, "init", deployer, args, value, 120 + source.len() as u64 + arg_bytes);
        // On-chain deployments also pay for compiling the source.
        r.fuel_used += source.len() as u64 * vm::fuel::COMPILE_PER_BYTE;
        let (max_fuel, fee) = Self::fee(r.fee.tx_bytes.max(120 + source.len() as u64), r.fuel_used);
        r.fee.max_fuel = max_fuel;
        r.fee.fee = fee;
        if r.result.is_err() {
            self.contracts.remove(&addr);
        }
        Ok((addr, r))
    }

    pub fn call(&mut self, addr_hex: &str, caller: [u8; 20], function: &str, args: Vec<Value>, value: u64) -> Result<CallResult, String> {
        if !self.contracts.contains_key(addr_hex) {
            return Err(format!("no contract at {addr_hex}"));
        }
        let arg_bytes: u64 = args.iter().map(|a| a.size() as u64).sum();
        Ok(self.run(addr_hex, Mode::Action, function, caller, args, value, 150 + function.len() as u64 + arg_bytes))
    }

    pub fn view(&mut self, addr_hex: &str, function: &str, args: Vec<Value>) -> Result<CallResult, String> {
        if !self.contracts.contains_key(addr_hex) {
            return Err(format!("no contract at {addr_hex}"));
        }
        Ok(self.run(addr_hex, Mode::View, function, [0u8; 20], args, 0, 0))
    }

    /// Replaces a contract's code (signed by its upgrade authority). Returns the
    /// compatibility report and the result of `upgrade()`.
    pub fn upgrade(&mut self, addr_hex: &str, signer: [u8; 20], new_source: &str, args: Vec<Value>) -> Result<(crate::upgrade::Report, CallResult), String> {
        let c = self.contracts.get(addr_hex).ok_or_else(|| format!("no contract at {addr_hex}"))?;
        match &c.authority {
            None => return Err("this contract is final: its code can never be upgraded".into()),
            Some(a) if *a != hex::encode(signer) => return Err(format!("only the upgrade authority {a} can upgrade this contract")),
            _ => {}
        }
        let old = c.program.clone().expect("compiled");
        let opts = CompileOptions { state_order: old.states.iter().map(|s| s.name.clone()).collect(), ..Default::default() };
        let new = compile(new_source, &opts).map_err(|e| e.to_string())?;
        let report = crate::upgrade::check(&old, &new).map_err(|e| format!("incompatible upgrade: {e}"))?;
        let snapshot = self.clone();
        {
            let c = self.contracts.get_mut(addr_hex).expect("exists");
            c.source = new_source.to_string();
            c.language = new.version;
            c.state_order = new.states.iter().map(|s| s.name.clone()).collect();
            c.program = Some(Arc::new(new));
            c.code_version += 1;
        }
        let r = self.run(addr_hex, Mode::Upgrade, "upgrade", signer, args, 0, 150 + new_source.len() as u64);
        if r.result.is_err() {
            let keep = self.height;
            *self = snapshot;
            self.height = keep;
        }
        Ok((report, r))
    }

    /// Hands the upgrade authority to `new_authority`, or renounces it (`None`: final forever).
    pub fn set_authority(&mut self, addr_hex: &str, signer: [u8; 20], new_authority: Option<[u8; 20]>) -> Result<(), String> {
        let c = self.contracts.get_mut(addr_hex).ok_or_else(|| format!("no contract at {addr_hex}"))?;
        match &c.authority {
            None => Err("this contract is already final".into()),
            Some(a) if *a != hex::encode(signer) => Err(format!("only the upgrade authority {a} can change it")),
            _ => {
                c.authority = new_authority.map(hex::encode);
                self.height += 1;
                Ok(())
            }
        }
    }

    pub fn program(&self, addr_hex: &str) -> Option<&Program> {
        self.contracts.get(addr_hex).and_then(|c| c.program.as_deref())
    }

    /// Decodes a contract's storage by state variable (for playgrounds and `tccl state`).
    pub fn describe_state(&self, addr_hex: &str, hrp: &str) -> Vec<StateEntry> {
        let Some(c) = self.contracts.get(addr_hex) else { return Vec::new() };
        let Some(p) = c.program.as_deref() else { return Vec::new() };
        let raw = |key: &[u8]| c.storage.get(&hex::encode(key)).and_then(|v| hex::decode(v).ok());
        let decode = |b: Vec<u8>| borsh::from_slice::<Value>(&b).ok();
        let mut out = Vec::new();
        for (i, s) in p.states.iter().enumerate() {
            let var = (i as u16).to_be_bytes();
            let role = p.roles.iter().any(|r| r.state as usize == i);
            let ty = if role { "role".to_string() } else { p.type_name(&s.ty) };
            match &s.ty {
                Type::Map(kt, vt) => {
                    let prefix = [&[1u8][..], &var[..]].concat();
                    let pfx = hex::encode(&prefix);
                    let mut items = Vec::new();
                    for (k, v) in c.storage.range(pfx.clone()..) {
                        if !k.starts_with(&pfx) {
                            break;
                        }
                        let key = hex::decode(&k[pfx.len()..]).ok().and_then(decode);
                        let val = hex::decode(v).ok().and_then(decode);
                        if let (Some(key), Some(val)) = (key, val) {
                            items.push((display_typed(p, &key, kt, hrp), display_typed(p, &val, vt, hrp)));
                        }
                    }
                    let value = if role {
                        items.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>().join(", ")
                    } else {
                        format!("{} entries", items.len())
                    };
                    out.push(StateEntry { name: s.name.clone(), ty, value, items });
                }
                Type::List(inner) => {
                    let len = raw(&[&[2u8][..], &var[..]].concat()).and_then(|b| b.try_into().ok()).map(u64::from_be_bytes).unwrap_or(0);
                    let mut items = Vec::new();
                    for idx in 0..len.min(1000) {
                        if let Some(v) = raw(&[&[3u8][..], &var[..], &idx.to_be_bytes()[..]].concat()).and_then(decode) {
                            items.push((idx.to_string(), display_typed(p, &v, inner, hrp)));
                        }
                    }
                    out.push(StateEntry { name: s.name.clone(), ty, value: format!("{len} items"), items });
                }
                t => {
                    let v = raw(&[&[0u8][..], &var[..]].concat()).and_then(decode).unwrap_or_else(|| p.default_value(t));
                    out.push(StateEntry { name: s.name.clone(), ty, value: display_typed(p, &v, t, hrp), items: Vec::new() });
                }
            }
        }
        out
    }
}
