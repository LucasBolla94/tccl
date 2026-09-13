//! TCCL for the browser: the same compiler, VM and simulator as the command-line
//! tool, compiled to WebAssembly.
//!
//! The playground runs this module inside a Web Worker. Requests and responses are
//! JSON strings passed through linear memory:
//!
//! 1. `tccl_alloc(len)` → pointer; the host writes the UTF-8 request there;
//! 2. `tccl_request(ptr, len)` → pointer to `[u32 little-endian length][JSON]`;
//! 3. the host reads the response and frees both buffers with `tccl_free`.
//!
//! Everything is a local simulation with fictitious accounts: nothing is signed,
//! no network is contacted and no recovery phrase or key is ever used.

use serde_json::{json, Value as Json};
use std::cell::RefCell;
use std::collections::BTreeMap;
use tccl::abi::{display_typed, parse_arg_in};
use tccl::diagnostics::{explain_code, explain_compile, explain_runtime, Lang};
use tccl::program::{FnKind, Program, Type, Value};
use tccl::sim::{account, CallResult, DeployOptions, Simulator};
use tccl::{compile, CompileOptions};

/// Fictitious accounts available in the playground.
pub const ACCOUNTS: &[&str] = &["alice", "bob", "carol", "dave", "erin"];
const HRP: &str = "tcr";
const MAX_CONTRACTS: usize = 32;
const MAX_SCENARIO_LINES: usize = 1_000;

/// Example contracts shipped with the playground: `(file, source)`.
pub const EXAMPLES: &[(&str, &str)] = &[
    ("counter.tccl", include_str!("../../../examples/counter.tccl")),
    ("tip_jar.tccl", include_str!("../../../examples/tip_jar.tccl")),
    ("cloud_coin.tccl", include_str!("../../../examples/cloud_coin.tccl")),
    ("orders.tccl", include_str!("../../../examples/orders.tccl")),
    ("pool.tccl", include_str!("../../../examples/pool.tccl")),
    ("coin_flip.tccl", include_str!("../../../examples/coin_flip.tccl")),
    ("tickets.tccl", include_str!("../../../examples/tickets.tccl")),
    ("deals.tccl", include_str!("../../../examples/deals.tccl")),
    ("crowdfund.tccl", include_str!("../../../examples/crowdfund.tccl")),
    ("escrow.tccl", include_str!("../../../examples/escrow.tccl")),
    ("token.tccl", include_str!("../../../examples/token.tccl")),
    ("poll.tccl", include_str!("../../../examples/poll.tccl")),
    ("savings.tccl", include_str!("../../../examples/savings.tccl")),
    ("treasury.tccl", include_str!("../../../examples/treasury.tccl")),
    ("names.tccl", include_str!("../../../examples/names.tccl")),
    ("shop.tccl", include_str!("../../../examples/shop.tccl")),
    ("private_pool.tccl", include_str!("../../../examples/private_pool.tccl")),
    ("counter_v2.tccl", include_str!("../../../examples/counter_v2.tccl")),
];

thread_local! {
    static SESSION: RefCell<Simulator> = RefCell::new(Simulator::new());
}

fn lang_of(req: &Json) -> Lang {
    Lang::parse(req.get("lang").and_then(Json::as_str).unwrap_or("en"))
}

fn account_json(name: &str) -> String {
    tccl::abi::format_address(&account(name), HRP)
}

fn contract_address(hex_addr: &str) -> String {
    let mut a = [0u8; 20];
    if let Ok(b) = hex::decode(hex_addr) {
        if b.len() == 20 {
            a.copy_from_slice(&b);
        }
    }
    tccl::abi::format_address(&a, HRP)
}

fn name_of(sim: &Simulator, addr: &[u8; 20]) -> Option<String> {
    if let Some(n) = ACCOUNTS.iter().find(|n| account(n) == *addr) {
        return Some(n.to_string());
    }
    let h = hex::encode(addr);
    sim.contracts.get(&h).and_then(|c| c.program.as_ref()).map(|p| format!("{} contract", p.name))
}

fn compile_error_json(src: &str, e: &tccl::CompileError, lang: Lang) -> Json {
    let x = explain_compile(e, lang);
    json!({
        "line": e.pos.line,
        "col": e.pos.col,
        "message": e.message,
        "help": e.help,
        "code": x.code,
        "title": x.title,
        "explanation": x.explanation,
        "fix": x.fix,
        "source_line": src.lines().nth(e.pos.line.saturating_sub(1) as usize).unwrap_or(""),
    })
}

fn kind(k: FnKind) -> &'static str {
    match k {
        FnKind::Init => "init",
        FnKind::Action => "action",
        FnKind::View => "view",
        FnKind::Internal => "fn",
        FnKind::Upgrade => "upgrade",
    }
}

fn interface_json(p: &Program) -> Json {
    let functions: Vec<Json> = p
        .functions
        .iter()
        .enumerate()
        .filter(|(_, f)| f.kind != FnKind::Internal)
        .map(|(i, f)| {
            let only = p.access.iter().find(|a| a.function as usize == i).map(|a| {
                a.rules
                    .iter()
                    .map(|r| match r {
                        tccl::program::AccessRule::Role(v) | tccl::program::AccessRule::Address(v) => p.states[*v as usize].name.clone(),
                    })
                    .collect::<Vec<_>>()
            });
            json!({
                "name": f.name, "kind": kind(f.kind), "payable": f.payable,
                "params": f.params.iter().map(|(n, t)| json!({"name": n, "type": p.type_name(t)})).collect::<Vec<_>>(),
                "returns": p.type_name(&f.ret), "only": only,
            })
        })
        .collect();
    json!({
        "name": p.name,
        "language": p.version,
        "bytes": p.to_bytes().len(),
        "code_hash": hex::encode(p.code_hash()),
        "functions": functions,
        "events": p.events.iter().map(|e| json!({"name": e.name, "fields": e.fields.iter().map(|(n, t)| format!("{n}: {}", p.type_name(t))).collect::<Vec<_>>()})).collect::<Vec<_>>(),
        "state": p.states.iter().map(|s| json!({"name": s.name, "type": p.type_name(&s.ty)})).collect::<Vec<_>>(),
        "records": p.records.iter().map(|r| json!({"name": r.name, "fields": r.fields.iter().map(|(n, t)| format!("{n}: {}", p.type_name(t))).collect::<Vec<_>>()})).collect::<Vec<_>>(),
        "enums": p.enums.iter().map(|e| json!({"name": e.name, "variants": e.variants, "transitions": e.transitions.as_ref().map(|t| e.variants.iter().zip(t).map(|(v, n)| json!({"from": v, "to": n.iter().map(|i| e.variants[*i as usize].clone()).collect::<Vec<_>>()})).collect::<Vec<_>>())})).collect::<Vec<_>>(),
        "roles": p.roles.iter().map(|r| r.name.clone()).collect::<Vec<_>>(),
        "modules": p.modules.iter().map(|m| m.name.clone()).collect::<Vec<_>>(),
        "effects": p.effects,
    })
}

fn parse_value(sim: &Simulator, program: &Program, raw: &str, t: &Type) -> Result<Value, String> {
    let raw = raw.trim();
    if matches!(t, Type::Address | Type::Interface(_)) {
        if let Some(n) = raw.strip_prefix('@') {
            return Ok(Value::Address(account(n)));
        }
        if let Some(h) = raw.strip_prefix('$') {
            let c = sim.contracts.keys().find(|k| k.starts_with(h)).ok_or_else(|| format!("no contract starting with {h}"))?;
            let mut a = [0u8; 20];
            a.copy_from_slice(&hex::decode(c).map_err(|e| e.to_string())?);
            return Ok(Value::Address(a));
        }
    }
    parse_arg_in(program, raw, t)
}

fn args_of(sim: &Simulator, program: &Program, function: &str, req: &Json) -> Result<Vec<Value>, String> {
    let raw: Vec<String> = req.get("args").and_then(Json::as_array).map(|a| a.iter().map(|x| x.as_str().map(str::to_string).unwrap_or_else(|| x.to_string())).collect()).unwrap_or_default();
    let params = match program.find(function) {
        Some((_, f)) => f.params.clone(),
        None if matches!(function, "init" | "upgrade") => Vec::new(),
        None => return Err(format!("contract has no function '{function}'")),
    };
    if raw.len() != params.len() {
        return Err(format!("{function} expects {} argument(s), {} given", params.len(), raw.len()));
    }
    params.iter().zip(&raw).map(|((n, t), a)| parse_value(sim, program, a, t).map_err(|e| format!("argument '{n}': {e}"))).collect()
}

fn value_of(req: &Json) -> Result<u64, String> {
    match req.get("value").and_then(Json::as_str).map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(0),
        Some(v) => match tccl::abi::parse_arg(v, &Type::Int)? {
            Value::Int(i) if i >= 0 && i <= u64::MAX as i128 => Ok(i as u64),
            _ => Err("value must be a non-negative amount".into()),
        },
    }
}

fn from_of(req: &Json) -> Result<[u8; 20], String> {
    let from = req.get("from").and_then(Json::as_str).unwrap_or("alice").trim_start_matches('@');
    if !ACCOUNTS.contains(&from) {
        return Err(format!("unknown fictitious account '{from}' (use {})", ACCOUNTS.join(", ")));
    }
    Ok(account(from))
}

fn snapshot(sim: &Simulator) -> Json {
    let contracts: BTreeMap<String, Json> = sim
        .contracts
        .iter()
        .map(|(h, c)| {
            (
                h.clone(),
                json!({
                    "name": c.program.as_ref().map(|p| p.name.clone()),
                    "address": contract_address(h),
                    "balance": sim.balances.get(h).copied().unwrap_or(0),
                    "deposit": c.deposit,
                    "code_version": c.code_version,
                    "upgrade_authority": c.authority.as_ref().map(|a| { let mut x = [0u8; 20]; x.copy_from_slice(&hex::decode(a).unwrap_or_else(|_| vec![0; 20])); name_of(sim, &x).unwrap_or_else(|| tccl::abi::format_address(&x, HRP)) }),
                    "state": sim.describe_state(h, HRP),
                }),
            )
        })
        .collect();
    let accounts: Vec<Json> = ACCOUNTS.iter().map(|n| json!({"name": n, "address": account_json(n), "balance": sim.balance_of(&account(n))})).collect();
    json!({"height": sim.height, "accounts": accounts, "contracts": contracts})
}

fn outcome_json(sim: &Simulator, r: &CallResult, program: Option<&Program>, before: Json, lang: Lang) -> Json {
    let events: Vec<Json> = r
        .events
        .iter()
        .map(|e| {
            let p = sim.contracts.get(&e.contract).and_then(|c| c.program.clone());
            let fields: Vec<Json> = e
                .fields
                .iter()
                .map(|(n, v)| {
                    let shown = p
                        .as_ref()
                        .and_then(|p| p.events.iter().find(|d| d.name == e.name).and_then(|d| d.fields.iter().find(|(f, _)| f == n)).map(|(_, t)| display_typed(p, v, t, HRP)))
                        .unwrap_or_else(|| tccl::abi::display(v, HRP));
                    json!([n, shown])
                })
                .collect();
            json!({"contract": contract_address(&e.contract), "contract_name": p.map(|p| p.name.clone()), "name": e.name, "fields": fields})
        })
        .collect();
    let (result, error) = match &r.result {
        Ok(Value::Unit) => (Json::Null, Json::Null),
        Ok(v) => (json!(program.map_or_else(|| tccl::abi::display(v, HRP), |p| display_typed(p, v, &r.ret, HRP))), Json::Null),
        Err(e) => {
            let x = explain_runtime(e, lang);
            (Json::Null, json!({"message": e.to_string(), "code": x.code, "title": x.title, "explanation": x.explanation, "fix": x.fix}))
        }
    };
    json!({
        "ok": r.result.is_ok(),
        "result": result,
        "error": error,
        "fuel_used": r.fuel_used,
        "fee": r.fee,
        "events": events,
        "before": before,
        "after": snapshot(sim),
    })
}

fn handle(req: &Json) -> Result<Json, String> {
    let op = req.get("op").and_then(Json::as_str).ok_or("missing op")?;
    let lang = lang_of(req);
    let text = |k: &str| req.get(k).and_then(Json::as_str).map(str::to_string).ok_or_else(|| format!("missing '{k}'"));
    SESSION.with(|cell| {
        let mut sim = cell.borrow_mut();
        match op {
            "version" => Ok(json!({"tccl": env!("CARGO_PKG_VERSION"), "language": tccl::program::LANGUAGE_VERSION, "mode": "simulation"})),
            "examples" => Ok(json!({
                "examples": EXAMPLES.iter().map(|(n, s)| json!({"name": n, "source": s})).collect::<Vec<_>>(),
                "std": tccl::modules::STD_MODULES.iter().map(|(n, s)| json!({"name": format!("std.{n}"), "source": s})).collect::<Vec<_>>(),
            })),
            "compile" => {
                let source = text("source")?;
                let version = req.get("language").and_then(Json::as_u64).unwrap_or(tccl::program::LANGUAGE_VERSION as u64) as u16;
                match compile(&source, &CompileOptions { version, ..Default::default() }) {
                    Ok(p) => Ok(json!({"ok": true, "interface": interface_json(&p)})),
                    Err(e) => Ok(json!({"ok": false, "error": compile_error_json(&source, &e, lang)})),
                }
            }
            "reset" => {
                *sim = Simulator::new();
                Ok(snapshot(&sim))
            }
            "snapshot" => Ok(snapshot(&sim)),
            "deploy" => {
                if sim.contracts.len() >= MAX_CONTRACTS {
                    return Err(format!("the playground session holds at most {MAX_CONTRACTS} contracts; reset it"));
                }
                let source = text("source")?;
                let program = match compile(&source, &CompileOptions::default()) {
                    Ok(p) => p,
                    Err(e) => return Ok(json!({"ok": false, "compile_error": compile_error_json(&source, &e, lang)})),
                };
                let args = args_of(&sim, &program, "init", req)?;
                let before = snapshot(&sim);
                let final_code = req.get("final").and_then(Json::as_bool).unwrap_or(false);
                let (addr, r) = sim.deploy_with(&source, from_of(req)?, args, value_of(req)?, &DeployOptions { final_code, ..Default::default() })?;
                let mut out = outcome_json(&sim, &r, Some(&program), before, lang);
                if r.result.is_ok() {
                    out["contract"] = json!(addr);
                    out["address"] = json!(contract_address(&addr));
                    out["interface"] = interface_json(&program);
                }
                Ok(out)
            }
            "call" | "view" => {
                let contract = text("contract")?;
                let function = text("function")?;
                let program = sim.program(&contract).cloned().ok_or_else(|| format!("no contract {contract} in this session"))?;
                let args = args_of(&sim, &program, &function, req)?;
                let before = snapshot(&sim);
                let r = if op == "call" { sim.call(&contract, from_of(req)?, &function, args, value_of(req)?)? } else { sim.view(&contract, &function, args)? };
                Ok(outcome_json(&sim, &r, Some(&program), before, lang))
            }
            "upgrade" => {
                let contract = text("contract")?;
                let source = text("source")?;
                let old = sim.program(&contract).cloned().ok_or("no such contract")?;
                let opts = CompileOptions { state_order: old.states.iter().map(|s| s.name.clone()).collect(), ..Default::default() };
                let preview = match compile(&source, &opts) {
                    Ok(p) => p,
                    Err(e) => return Ok(json!({"ok": false, "compile_error": compile_error_json(&source, &e, lang)})),
                };
                let args = args_of(&sim, &preview, "upgrade", req)?;
                let before = snapshot(&sim);
                let (report, r) = sim.upgrade(&contract, from_of(req)?, &source, args)?;
                let mut out = outcome_json(&sim, &r, Some(&preview), before, lang);
                out["report"] = json!(report);
                out["interface"] = interface_json(&preview);
                Ok(out)
            }
            "authority" => {
                let contract = text("contract")?;
                let to = text("to")?;
                let new = if to == "none" { None } else { Some(account(to.trim_start_matches('@'))) };
                sim.set_authority(&contract, from_of(req)?, new)?;
                Ok(snapshot(&sim))
            }
            "height" => {
                let h = req.get("height").and_then(Json::as_u64).ok_or("missing height")?;
                sim.height = h.clamp(1, 1_000_000_000);
                Ok(snapshot(&sim))
            }
            "export" => Ok(json!({"state": sim.save()})),
            "import" => {
                let state = text("state")?;
                if state.len() > 4_000_000 {
                    return Err("session file too large".into());
                }
                let loaded = Simulator::load(&state)?;
                if loaded.contracts.len() > MAX_CONTRACTS {
                    return Err(format!("the playground session holds at most {MAX_CONTRACTS} contracts"));
                }
                *sim = loaded;
                Ok(snapshot(&sim))
            }
            "scenario" => {
                let script = text("script")?;
                if script.lines().count() > MAX_SCENARIO_LINES {
                    return Err(format!("scenarios are limited to {MAX_SCENARIO_LINES} lines in the playground"));
                }
                let files: BTreeMap<String, String> = req
                    .get("files")
                    .and_then(Json::as_object)
                    .map(|o| o.iter().filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string()))).collect())
                    .unwrap_or_default();
                let read = |name: &str| {
                    files.get(name).cloned().or_else(|| EXAMPLES.iter().find(|(n, _)| *n == name).map(|(_, s)| s.to_string())).ok_or_else(|| format!("file {name} is not open in the playground"))
                };
                Ok(json!(tccl::scenario::run(&script, &read)))
            }
            "explain" => {
                let code = text("code")?;
                Ok(json!(explain_code(&code, lang)))
            }
            other => Err(format!("unknown op '{other}'")),
        }
    })
}

/// Handles one JSON request and returns the JSON response (used by native tests too).
pub fn request(input: &str) -> String {
    let response = match serde_json::from_str::<Json>(input) {
        Ok(req) => match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handle(&req))) {
            Ok(Ok(v)) => v,
            Ok(Err(e)) => json!({"ok": false, "fatal": e}),
            Err(_) => json!({"ok": false, "fatal": "internal error"}),
        },
        Err(e) => json!({"ok": false, "fatal": format!("invalid request: {e}")}),
    };
    response.to_string()
}

#[no_mangle]
pub extern "C" fn tccl_alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(len.max(1));
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// # Safety
/// `ptr` must come from `tccl_alloc(len)` or from `tccl_request` with the same total length.
#[no_mangle]
pub unsafe extern "C" fn tccl_free(ptr: *mut u8, len: usize) {
    drop(unsafe { Vec::from_raw_parts(ptr, 0, len.max(1)) });
}

/// # Safety
/// `ptr` must point to `len` initialized bytes allocated with `tccl_alloc`.
#[no_mangle]
pub unsafe extern "C" fn tccl_request(ptr: *const u8, len: usize) -> *mut u8 {
    let input = unsafe { std::slice::from_raw_parts(ptr, len) };
    let out = request(&String::from_utf8_lossy(input));
    let bytes = out.into_bytes();
    let mut buf = Vec::with_capacity(bytes.len() + 4);
    buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    buf.extend_from_slice(&bytes);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(v: Json) -> Json {
        serde_json::from_str(&request(&v.to_string())).unwrap()
    }

    #[test]
    fn playground_session() {
        let counter = EXAMPLES[0].1;
        let c = call(json!({"op": "compile", "source": "contract A\nstate total: int\nview f() -> int:\n    return totl\n", "lang": "pt"}));
        assert_eq!(c["ok"], false);
        assert_eq!(c["error"]["line"], 4);
        assert_eq!(c["error"]["code"], "C006");
        assert_eq!(c["error"]["title"], "Nome desconhecido");
        let d = call(json!({"op": "deploy", "source": counter, "from": "alice"}));
        assert_eq!(d["ok"], true, "{d}");
        let addr = d["contract"].as_str().unwrap().to_string();
        let r = call(json!({"op": "call", "contract": addr, "function": "increment", "args": ["5"], "from": "bob"}));
        assert_eq!(r["ok"], true);
        assert_eq!(r["events"][0]["name"], "Increased");
        assert_eq!(r["after"]["contracts"][&addr]["state"][0]["value"], "5");
        assert_eq!(r["before"]["contracts"][&addr]["state"][0]["value"], "0");
        assert!(r["fee"]["fee"].as_u64().unwrap() > 0);
        let bad = call(json!({"op": "call", "contract": addr, "function": "increment", "args": ["500"], "lang": "es"}));
        assert_eq!(bad["ok"], false);
        assert_eq!(bad["error"]["code"], "R002");
        let v = call(json!({"op": "view", "contract": addr, "function": "get"}));
        assert_eq!(v["result"], "5");
        let e = call(json!({"op": "export"}));
        call(json!({"op": "reset"}));
        let s = call(json!({"op": "import", "state": e["state"]}));
        assert_eq!(s["contracts"][&addr]["state"][0]["value"], "5");
        let u = call(json!({"op": "upgrade", "contract": addr, "source": EXAMPLES.iter().find(|(n, _)| *n == "counter_v2.tccl").unwrap().1, "from": "alice"}));
        assert_eq!(u["ok"], true, "{u}");
        assert_eq!(call(json!({"op": "call", "contract": addr, "function": "increment", "args": ["1"], "from": "mallory"}))["fatal"], "unknown fictitious account 'mallory' (use alice, bob, carol, dave, erin)");
        let sc = call(json!({"op": "scenario", "script": "deploy counter.tccl as c\ncall c increment 2\nexpect ok\nview c get\nexpect result 2\n"}));
        assert_eq!(sc["passed"], 2);
        assert!(call(json!({"op": "nope"}))["fatal"].is_string());
        assert!(call(json!({"op": "examples"}))["examples"].as_array().unwrap().len() >= 10);
    }
}
