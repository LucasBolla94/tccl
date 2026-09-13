//! `tccl` — TCCL developer tool: create, check, inspect, simulate and test contracts locally.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use tccl::abi::{display_typed, parse_arg_in};
use tccl::diagnostics::{explain_code, explain_compile, explain_runtime, Lang};
use tccl::modules::{bundle, Bundle};
use tccl::program::{FnKind, Program, Type, Value, LANGUAGE_VERSION};
use tccl::sim::{account, DeployOptions, Simulator};
use tccl::{compile, CompileError, CompileOptions};

const USAGE: &str = "\
tccl — The Coin Cloud Language tool

USAGE:
  tccl new <name>                        Create a project with a contract and a test scenario
  tccl check <file.tccl>                 Compile and show the interface, permissions and effects
  tccl abi <file.tccl>                   Print the interface as JSON
  tccl bundle <file.tccl> [-o <out>]     Include local modules (use mylib → mylib.tccl) in one deployable file
  tccl run <file.tccl> [options] deploy [args...]
  tccl run <file.tccl> [options] call <function> [args...]
  tccl run <file.tccl> [options] view <function> [args...]
  tccl run <file.tccl> [options] upgrade [args...]      Replace the code (upgrade authority only)
  tccl run <file.tccl> [options] authority <@account|none>
  tccl run <file.tccl> [options] state   Show the decoded state of the contract
  tccl test [files or folders...]        Run *.scenario files (default: current folder)
  tccl explain <code>                    Explain an error code (C006, R018, ...)
  tccl bench                             Measure engine speed and memory on this machine
  tccl ring keygen [--seed <hex32>]      Create a ring (privacy) key pair
  tccl ring sign --secret <hex> --ring <pk1,pk2,...> --index <i> --message <0xhex>
  tccl version

OPTIONS:
  --language <1|2>   Language version (default: 2; 1 = frozen version deployed on The Coin v0.2)
  --lang <en|pt|es>  Language of explanations (default: en, or TCCL_LANG)

RUN OPTIONS:
  --state <file>     Simulator state file (default: tccl-state.json)
  --from <name>      Fictitious account calling (default: alice); every account starts with 1 000 000 TCN
  --value <amount>   TCN sent with the call, e.g. 5tcn or 500000000 (motes)
  --height <n>       Set the simulated block height before the call
  --contract <hex>   Contract address in the simulator (default: the last deployed)
  --final            deploy: no upgrade authority, the code can never change

This is a local simulation: nothing is signed or published. Deploy to a network with thecoin-wallet.
Arguments are parsed using the declared parameter types:
  int 42 | 2.5tcn   bool true   text \"hello\"   bytes 0xabcd   address tc1... | @alice | $contract
  list [1, 2]      record {field: value}      enum Variant
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            if !e.is_empty() {
                eprintln!("error: {e}");
            }
            ExitCode::FAILURE
        }
    }
}

fn read_source(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))
}

fn take_opt(args: &mut Vec<String>, name: &str) -> Option<String> {
    let pos = args.iter().position(|a| a == name)?;
    if pos + 1 >= args.len() {
        return None;
    }
    let v = args.remove(pos + 1);
    args.remove(pos);
    Some(v)
}

fn take_flag(args: &mut Vec<String>, name: &str) -> bool {
    match args.iter().position(|a| a == name) {
        Some(p) => {
            args.remove(p);
            true
        }
        None => false,
    }
}

struct Common {
    language: u16,
    lang: Lang,
}

fn common(args: &mut Vec<String>) -> Result<Common, String> {
    let language = match take_opt(args, "--language") {
        Some(v) => v.parse().map_err(|_| format!("invalid --language {v}"))?,
        None => LANGUAGE_VERSION,
    };
    let lang = Lang::parse(&take_opt(args, "--lang").or_else(|| std::env::var("TCCL_LANG").ok()).unwrap_or_else(|| "en".into()));
    Ok(Common { language, lang })
}

/// Reads a contract and the local modules it uses.
fn load(path: &str) -> Result<Bundle, String> {
    let main = read_source(path)?;
    let dir = Path::new(path).parent().map(Path::to_path_buf).unwrap_or_default();
    let name = Path::new(path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.to_string());
    bundle(&main, &name, &|m| std::fs::read_to_string(dir.join(format!("{m}.tccl"))).ok())
}

/// Prints a compile error with the source line, a caret, the explanation and the fix.
fn render_error(b: &Bundle, e: &CompileError, lang: Lang) -> String {
    let (file, line) = b.locate(e.pos.line);
    let x = explain_compile(e, lang);
    let mut out = format!("error[{}]: {}\n  --> {file}:{line}:{}\n", x.code, e.message, e.pos.col);
    if let Some(text) = b.source.lines().nth(e.pos.line.saturating_sub(1) as usize) {
        let gutter = line.to_string().len();
        out.push_str(&format!("{:gutter$} |\n{line} | {text}\n{:gutter$} | {}^\n", "", "", " ".repeat(e.pos.col.saturating_sub(1) as usize)));
    }
    out.push_str(&format!("  = {}: {}\n  = {}: {}\n", ["why", "por quê", "por qué"][lang as usize], x.explanation, ["fix", "correção", "corrección"][lang as usize], x.fix));
    out
}

fn compile_file(path: &str, c: &Common, state_order: Vec<String>) -> Result<(Bundle, Program), String> {
    let b = load(path)?;
    let opts = CompileOptions { version: c.language, state_order, ..Default::default() };
    match compile(&b.source, &opts) {
        Ok(p) => Ok((b, p)),
        Err(e) => {
            eprint!("{}", render_error(&b, &e, c.lang));
            Err(String::new())
        }
    }
}

fn run(mut args: Vec<String>) -> Result<(), String> {
    if args.is_empty() || args[0] == "help" || args[0] == "--help" || args[0] == "-h" {
        print!("{USAGE}");
        return Ok(());
    }
    let cmd = args.remove(0);
    match cmd.as_str() {
        "--version" | "version" => {
            println!("tccl {} (language versions 1–{LANGUAGE_VERSION})", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        "new" => new_project(args),
        "check" => {
            let c = common(&mut args)?;
            let path = args.first().ok_or("usage: tccl check <file.tccl>")?.clone();
            let (b, p) = compile_file(&path, &c, Vec::new())?;
            print_interface(&b, &p);
            Ok(())
        }
        "abi" => {
            let c = common(&mut args)?;
            let path = args.first().ok_or("usage: tccl abi <file.tccl>")?.clone();
            let (_, p) = compile_file(&path, &c, Vec::new())?;
            println!("{}", serde_json::to_string_pretty(&abi_json(&p)).expect("json"));
            Ok(())
        }
        "bundle" => {
            let out = take_opt(&mut args, "-o");
            let path = args.first().ok_or("usage: tccl bundle <file.tccl> [-o <out>]")?;
            let b = load(path)?;
            match out {
                Some(o) => std::fs::write(&o, &b.source).map_err(|e| format!("cannot write {o}: {e}"))?,
                None => print!("{}", b.source),
            }
            Ok(())
        }
        "explain" => {
            let c = common(&mut args)?;
            let code = args.first().ok_or("usage: tccl explain <code>")?;
            let x = explain_code(code, c.lang).ok_or_else(|| format!("unknown error code '{code}'"))?;
            println!("{} — {}\n\n{}\n\n{}", x.code, x.title, x.explanation, x.fix);
            Ok(())
        }
        "run" => run_sim(args),
        "test" => run_tests(args),
        "bench" => bench(),
        "ring" => ring_cmd(args),
        other => Err(format!("unknown command '{other}'\n\n{USAGE}")),
    }
}

fn kind_name(k: FnKind) -> &'static str {
    match k {
        FnKind::Init => "init",
        FnKind::Action => "action",
        FnKind::View => "view",
        FnKind::Internal => "fn",
        FnKind::Upgrade => "upgrade",
    }
}

fn print_interface(b: &Bundle, p: &Program) {
    println!("✔ {} compiles (language {}, {} bytes of source, {} bytes compiled)", p.name, p.version, b.source.len(), p.to_bytes().len());
    let states: Vec<String> = p.states.iter().filter(|s| !p.roles.iter().any(|r| p.states[r.state as usize].name == s.name)).map(|s| format!("{}: {}", s.name, p.type_name(&s.ty))).collect();
    println!("  state: {}", if states.is_empty() { "-".into() } else { states.join(", ") });
    for r in &p.records {
        println!("  record {}({})", r.name, r.fields.iter().map(|(n, t)| format!("{n}: {}", p.type_name(t))).collect::<Vec<_>>().join(", "));
    }
    for e in &p.enums {
        let rule = match &e.transitions {
            None => String::new(),
            Some(t) => format!(
                " [{}]",
                e.variants
                    .iter()
                    .zip(t)
                    .filter(|(_, n)| !n.is_empty())
                    .map(|(v, n)| format!("{v} -> {}", n.iter().map(|i| e.variants[*i as usize].as_str()).collect::<Vec<_>>().join(", ")))
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
        };
        println!("  enum {}: {}{rule}", e.name, e.variants.join(", "));
    }
    for r in &p.roles {
        println!("  role {}", r.name);
    }
    for m in &p.modules {
        println!("  uses {} (source blake3 {})", m.name, &hex::encode(m.source_hash)[..16]);
    }
    for (i, f) in p.functions.iter().enumerate() {
        if f.kind == FnKind::Internal {
            continue;
        }
        let params = f.params.iter().map(|(n, t)| format!("{n}: {}", p.type_name(t))).collect::<Vec<_>>().join(", ");
        let ret = if f.ret == Type::Unit { String::new() } else { format!(" -> {}", p.type_name(&f.ret)) };
        let access = p.access.iter().find(|a| a.function as usize == i).map(|a| format!(" only {}", access_names(p, &a.rules))).unwrap_or_default();
        println!("  {} {}({params}){ret}{}{access}", kind_name(f.kind), f.name, if f.payable { " payable" } else { "" });
    }
    let fx = &p.effects;
    let mut can = Vec::new();
    for (flag, what) in [
        (fx.changes_state, "change state"),
        (fx.receives_tcn, "receive TCN"),
        (fx.sends_tcn, "send TCN"),
        (fx.emits_events, "emit events"),
        (fx.calls_contracts, "call other contracts"),
        (fx.can_destroy, "destroy itself"),
        (fx.has_upgrade_hook, "run upgrade()"),
    ] {
        if flag {
            can.push(what);
        }
    }
    if p.version >= 2 {
        println!("  can: {}", if can.is_empty() { "only read".into() } else { can.join(", ") });
    }
}

fn access_names(p: &Program, rules: &[tccl::program::AccessRule]) -> String {
    rules
        .iter()
        .map(|r| match r {
            tccl::program::AccessRule::Role(v) | tccl::program::AccessRule::Address(v) => p.states[*v as usize].name.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn abi_json(p: &Program) -> serde_json::Value {
    use serde_json::json;
    let functions: Vec<_> = p
        .functions
        .iter()
        .enumerate()
        .filter(|(_, f)| f.kind != FnKind::Internal)
        .map(|(i, f)| {
            json!({
                "name": f.name,
                "kind": kind_name(f.kind),
                "payable": f.payable,
                "params": f.params.iter().map(|(n, t)| json!({"name": n, "type": p.type_name(t)})).collect::<Vec<_>>(),
                "returns": p.type_name(&f.ret),
                "only": p.access.iter().find(|a| a.function as usize == i).map(|a| access_names(p, &a.rules)),
            })
        })
        .collect();
    json!({
        "name": p.name,
        "language": p.version,
        "code_hash": hex::encode(p.code_hash()),
        "functions": functions,
        "events": p.events.iter().map(|e| json!({"name": e.name, "fields": e.fields.iter().map(|(n, t)| json!({"name": n, "type": p.type_name(t)})).collect::<Vec<_>>()})).collect::<Vec<_>>(),
        "state": p.states.iter().map(|s| json!({"name": s.name, "type": p.type_name(&s.ty)})).collect::<Vec<_>>(),
        "records": p.records.iter().map(|r| json!({"name": r.name, "fields": r.fields.iter().map(|(n, t)| json!({"name": n, "type": p.type_name(t)})).collect::<Vec<_>>()})).collect::<Vec<_>>(),
        "enums": p.enums.iter().map(|e| json!({"name": e.name, "variants": e.variants, "transitions": e.transitions})).collect::<Vec<_>>(),
        "roles": p.roles.iter().map(|r| r.name.clone()).collect::<Vec<_>>(),
        "modules": p.modules.iter().map(|m| json!({"name": m.name, "source_blake3": hex::encode(m.source_hash)})).collect::<Vec<_>>(),
        "effects": p.effects,
    })
}

fn new_project(args: Vec<String>) -> Result<(), String> {
    let name = args.first().ok_or("usage: tccl new <name>")?;
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') || name.is_empty() {
        return Err("project names use letters, digits, '-' and '_'".into());
    }
    let dir = PathBuf::from(name);
    if dir.exists() {
        return Err(format!("{name} already exists"));
    }
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let contract: String = name.split(['-', '_']).map(|w| {
        let mut c = w.chars();
        c.next().map(|f| f.to_ascii_uppercase().to_string() + c.as_str()).unwrap_or_default()
    }).collect();
    let src = format!(
        "# {contract}: a starting point. Edit it, then run `tccl check contract.tccl` and `tccl test`.\ncontract {contract}\n\nstate owner: address\nstate count: int\n\nevent Increased(by: address, total: int)\n\ninit():\n    owner = caller\n\naction increment(amount: int):\n    require amount > 0, \"amount must be positive\"\n    count += amount\n    emit Increased(caller, count)\n\naction reset() only owner:\n    count = 0\n\nview get() -> int:\n    return count\n"
    );
    let scenario = "# Run with: tccl test\ndeploy contract.tccl as app --from owner\ncall app increment 5 --from bob\nexpect ok\nexpect event Increased\nview app get\nexpect result 5\ncall app reset --from bob\nexpect fail \"only owner\"\ncall app reset --from owner\nexpect ok\n";
    std::fs::write(dir.join("contract.tccl"), src).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("contract.scenario"), scenario).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(".gitignore"), "tccl-state.json\ntccl-state.last\n").map_err(|e| e.to_string())?;
    println!("created {name}/contract.tccl and {name}/contract.scenario\n  cd {name} && tccl check contract.tccl && tccl test");
    Ok(())
}

fn find_scenarios(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_file() {
        out.push(path.to_path_buf());
    } else if let Ok(entries) = std::fs::read_dir(path) {
        let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            let hidden = p.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.') || n == "target");
            if p.is_dir() && !hidden {
                find_scenarios(&p, out);
            } else if p.extension().is_some_and(|x| x == "scenario") {
                out.push(p);
            }
        }
    }
}

fn run_tests(mut args: Vec<String>) -> Result<(), String> {
    let verbose = take_flag(&mut args, "-v") | take_flag(&mut args, "--verbose");
    let mut files = Vec::new();
    if args.is_empty() {
        find_scenarios(Path::new("."), &mut files);
    }
    for a in &args {
        find_scenarios(Path::new(a), &mut files);
    }
    if files.is_empty() {
        return Err("no .scenario files found".into());
    }
    let (mut passed, mut failed) = (0, 0);
    for f in &files {
        let script = std::fs::read_to_string(f).map_err(|e| format!("{}: {e}", f.display()))?;
        let dir = f.parent().map(Path::to_path_buf).unwrap_or_default();
        let read = |name: &str| load(&dir.join(name).to_string_lossy()).map(|b| b.source);
        let r = tccl::scenario::run(&script, &read);
        passed += r.passed;
        if r.ok() {
            println!("✔ {} ({} checks)", f.display(), r.passed);
        } else {
            failed += r.failed.len();
            println!("✘ {}", f.display());
            for x in &r.failed {
                println!("    {x}");
            }
        }
        if verbose || !r.ok() {
            for l in &r.log {
                println!("    {}", l.replace('\n', "\n    "));
            }
        }
    }
    println!("{passed} checks passed, {failed} failed, {} scenario file(s)", files.len());
    if failed > 0 {
        return Err(String::new());
    }
    Ok(())
}

fn parse_call_args(program: &Program, function: &str, raw: &[String], sim: &Simulator) -> Result<Vec<Value>, String> {
    let params = match program.find(function) {
        Some((_, f)) => f.params.clone(),
        None if function == "init" || function == "upgrade" => Vec::new(),
        None => return Err(format!("contract has no function '{function}'")),
    };
    if raw.len() != params.len() {
        return Err(format!(
            "{function} expects {} argument(s): {}",
            params.len(),
            params.iter().map(|(n, t)| format!("{n}: {}", program.type_name(t))).collect::<Vec<_>>().join(", ")
        ));
    }
    params
        .iter()
        .zip(raw)
        .map(|((name, t), a)| {
            if matches!(t, Type::Address | Type::Interface(_)) {
                if let Some(n) = a.strip_prefix('@') {
                    return Ok(Value::Address(account(n)));
                }
                if let Some(h) = a.strip_prefix('$') {
                    let c = sim.contracts.keys().find(|k| k.starts_with(h)).ok_or_else(|| format!("no contract starting with {h}"))?;
                    let mut arr = [0u8; 20];
                    arr.copy_from_slice(&hex::decode(c).expect("hex"));
                    return Ok(Value::Address(arr));
                }
            }
            parse_arg_in(program, a, t).map_err(|e| format!("argument '{name}': {e}"))
        })
        .collect()
}

fn run_sim(mut args: Vec<String>) -> Result<(), String> {
    let c = common(&mut args)?;
    let state_path = PathBuf::from(take_opt(&mut args, "--state").unwrap_or_else(|| "tccl-state.json".into()));
    let from = take_opt(&mut args, "--from").unwrap_or_else(|| "alice".into());
    let from = from.trim_start_matches('@').to_string();
    let value = match take_opt(&mut args, "--value") {
        Some(v) => match tccl::abi::parse_arg(&v, &Type::Int)? {
            Value::Int(i) if i >= 0 && i <= u64::MAX as i128 => i as u64,
            _ => return Err("invalid --value".into()),
        },
        None => 0,
    };
    let height = take_opt(&mut args, "--height");
    let contract_opt = take_opt(&mut args, "--contract");
    let final_code = take_flag(&mut args, "--final");
    if args.len() < 2 {
        return Err("usage: tccl run <file.tccl> deploy|call|view|upgrade|authority|state ...".into());
    }
    let file = args.remove(0);
    let action = args.remove(0);
    let mut sim = match std::fs::read_to_string(&state_path) {
        Ok(json) => Simulator::load(&json)?,
        Err(_) => Simulator::new(),
    };
    if let Some(h) = height {
        sim.height = h.parse().map_err(|_| "invalid --height")?;
    }
    let caller = account(&from);
    let hrp = "tcr";
    let last_path = state_path.with_extension("last");
    let current = || -> Result<String, String> {
        match &contract_opt {
            Some(a) => Ok(a.clone()),
            None => std::fs::read_to_string(&last_path).map(|s| s.trim().to_string()).map_err(|_| "no deployed contract yet (run deploy first)".to_string()),
        }
    };
    let (result, program) = match action.as_str() {
        "deploy" => {
            let (b, program) = compile_file(&file, &c, Vec::new())?;
            let call_args = parse_call_args(&program, "init", &args, &sim)?;
            let (addr, r) = sim.deploy_with(&b.source, caller, call_args, value, &DeployOptions { language: c.language, final_code })?;
            if r.result.is_ok() {
                println!(
                    "deployed {} at {addr} (simulation) · upgrade authority: {}",
                    program.name,
                    if final_code { "none (final)".to_string() } else { format!("{from} ({})", hex::encode(caller)) }
                );
                std::fs::write(&last_path, &addr).map_err(|e| e.to_string())?;
            }
            (r, program)
        }
        "call" | "view" => {
            if args.is_empty() {
                return Err(format!("usage: tccl run <file> {action} <function> [args...]"));
            }
            let function = args.remove(0);
            let addr = current()?;
            let program = sim.program(&addr).ok_or_else(|| format!("no contract at {addr}"))?.clone();
            let call_args = parse_call_args(&program, &function, &args, &sim)?;
            let r = if action == "call" { sim.call(&addr, caller, &function, call_args, value)? } else { sim.view(&addr, &function, call_args)? };
            (r, program)
        }
        "upgrade" => {
            let addr = current()?;
            let old = sim.program(&addr).ok_or_else(|| format!("no contract at {addr}"))?.clone();
            let (b, preview) = compile_file(&file, &c, old.states.iter().map(|s| s.name.clone()).collect())?;
            let call_args = parse_call_args(&preview, "upgrade", &args, &sim)?;
            let (report, r) = sim.upgrade(&addr, caller, &b.source, call_args)?;
            println!("upgrade report: {}", serde_json::to_string(&report).expect("json"));
            (r, preview)
        }
        "authority" => {
            let addr = current()?;
            let who = args.first().ok_or("usage: tccl run <file> authority <@account|none>")?;
            let new = if who == "none" { None } else { Some(account(who.trim_start_matches('@'))) };
            sim.set_authority(&addr, caller, new)?;
            std::fs::write(&state_path, sim.save()).map_err(|e| e.to_string())?;
            println!("{}", if new.is_none() { "the contract is now final: its code can never change".into() } else { format!("upgrade authority is now {who}") });
            return Ok(());
        }
        "state" => {
            let addr = current()?;
            let c = sim.contracts.get(&addr).ok_or_else(|| format!("no contract at {addr}"))?;
            println!(
                "{} at {addr} · balance {} motes · code version {} · upgrade authority {}",
                sim.program(&addr).map(|p| p.name.as_str()).unwrap_or("?"),
                sim.balance_of(&{
                    let mut a = [0u8; 20];
                    a.copy_from_slice(&hex::decode(&addr).expect("hex"));
                    a
                }),
                c.code_version,
                c.authority.clone().unwrap_or_else(|| "none (final)".into())
            );
            for e in sim.describe_state(&addr, hrp) {
                println!("  {}: {} = {}", e.name, e.ty, e.value);
                for (k, v) in e.items.iter().take(50) {
                    println!("      {k} → {v}");
                }
            }
            return Ok(());
        }
        other => return Err(format!("unknown run action '{other}' (use deploy, call, view, upgrade, authority or state)")),
    };
    for e in &result.events {
        let prog = sim.contracts.get(&e.contract).and_then(|c| c.program.clone());
        let fields = e
            .fields
            .iter()
            .map(|(n, v)| {
                let t = prog.as_ref().and_then(|p| p.events.iter().find(|d| d.name == e.name).and_then(|d| d.fields.iter().find(|(f, _)| f == n)).map(|(_, t)| (p.clone(), t.clone())));
                match t {
                    Some((p, t)) => format!("{n}: {}", display_typed(&p, v, &t, hrp)),
                    None => format!("{n}: {}", tccl::abi::display(v, hrp)),
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        println!("event {}({fields})", e.name);
    }
    match &result.result {
        Ok(v) => {
            if *v != Value::Unit {
                println!("result: {}", display_typed(&program, v, &result.ret, hrp));
            }
            let fee = &result.fee;
            println!("ok · fuel used: {} · height: {} · {from} balance: {} motes", result.fuel_used, sim.height, sim.balance_of(&caller));
            if fee.fee > 0 {
                println!(
                    "estimated on-chain cost (default parameters): fee {} motes for max_fuel {} and ~{} bytes · storage deposit {:+} motes",
                    fee.fee, fee.max_fuel, fee.tx_bytes, fee.deposit_change
                );
            }
        }
        Err(e) => {
            let x = explain_runtime(e, c.lang);
            println!("FAILED[{}]: {e} · fuel used: {} (all changes reverted)\n  = {}\n  = {}", x.code, result.fuel_used, x.explanation, x.fix);
        }
    }
    std::fs::write(&state_path, sim.save()).map_err(|e| format!("cannot write {}: {e}", state_path.display()))?;
    if result.result.is_err() {
        return Err(String::new());
    }
    Ok(())
}

fn bench() -> Result<(), String> {
    use std::time::Instant;
    let workloads = [
        ("loop and arithmetic", "contract B\naction run(n: int) -> int:\n    let s: int = 0\n    for i in range(0, n):\n        s += i * 3 % 7\n    return s\n", 200_000i128),
        ("storage writes", "contract B\nstate m: map[int, int]\naction run(n: int) -> int:\n    for i in range(0, n):\n        m[i] = i + 1\n    return n\n", 3_000),
        ("hashing", "contract B\naction run(n: int) -> int:\n    let h: bytes = 0x00\n    for i in range(0, n):\n        h = blake3(h + to_bytes(i))\n    return len(h)\n", 20_000),
        ("records and lists", "contract B\nrecord P:\n    a: int\n    b: text\naction run(n: int) -> int:\n    let xs: list[P] = []\n    for i in range(0, n):\n        xs.push(P(a: i, b: \"item\"))\n    return len(xs)\n", 4_000),
    ];
    println!("TCCL engine on this machine (tree-walking interpreter, release build = {}):", !cfg!(debug_assertions));
    for (name, src, n) in workloads {
        let mut sim = Simulator::new();
        let (c, r) = sim.deploy(src, account("bench"), vec![], 0)?;
        r.result.map_err(|e| e.to_string())?;
        let start = Instant::now();
        let r = sim.call(&c, account("bench"), "run", vec![Value::Int(n)], 0)?;
        let elapsed = start.elapsed();
        match r.result {
            Ok(_) => println!(
                "  {name:<20} {:>9} fuel in {:>8.2} ms  → {:>6.1} ns per fuel",
                r.fuel_used,
                elapsed.as_secs_f64() * 1e3,
                elapsed.as_nanos() as f64 / r.fuel_used.max(1) as f64
            ),
            Err(e) => println!("  {name:<20} failed: {e}"),
        }
    }
    println!("Fuel prices assume about 20 ns per fuel on a 2 vCPU server. Numbers include simulator overhead.");
    Ok(())
}

fn hex32(s: &str, what: &str) -> Result<[u8; 32], String> {
    let b = hex::decode(s.trim().trim_start_matches("0x")).map_err(|_| format!("{what} must be hex"))?;
    b.try_into().map_err(|_| format!("{what} must be 32 bytes"))
}

fn ring_cmd(mut args: Vec<String>) -> Result<(), String> {
    if args.is_empty() {
        return Err("usage: tccl ring keygen | tccl ring sign ...".into());
    }
    let sub = args.remove(0);
    match sub.as_str() {
        "keygen" => {
            let seed = match take_opt(&mut args, "--seed") {
                Some(s) => hex32(&s, "seed")?,
                None => {
                    let mut s = [0u8; 32];
                    rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut s);
                    s
                }
            };
            let (secret, public) = tccl::ring::keypair_from_seed(&seed);
            println!("secret:    {}", hex::encode(secret));
            println!("public:    0x{}", hex::encode(public));
            println!("key image: 0x{}", hex::encode(tccl::ring::key_image(&secret).expect("valid")));
            Ok(())
        }
        "sign" => {
            let secret = hex32(&take_opt(&mut args, "--secret").ok_or("--secret required")?, "secret")?;
            let ring: Vec<[u8; 32]> = take_opt(&mut args, "--ring").ok_or("--ring required")?.split(',').map(|k| hex32(k, "ring key")).collect::<Result<_, _>>()?;
            let index: usize = take_opt(&mut args, "--index").ok_or("--index required")?.parse().map_err(|_| "invalid --index")?;
            let msg_hex = take_opt(&mut args, "--message").ok_or("--message required")?;
            let msg = hex::decode(msg_hex.trim_start_matches("0x")).map_err(|_| "--message must be hex")?;
            let (sig, ki) = tccl::ring::sign(&msg, &ring, index, &secret, &mut rand::rngs::OsRng).map_err(|e| e.to_string())?;
            println!("signature: 0x{}", hex::encode(sig));
            println!("key image: 0x{}", hex::encode(ki));
            Ok(())
        }
        other => Err(format!("unknown ring command '{other}'")),
    }
}
