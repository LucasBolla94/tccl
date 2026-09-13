//! Test scenarios: plain-text scripts run against the simulator (`tccl test`).
//!
//! ```text
//! # counter.scenario
//! deploy counter.tccl as c
//! call c increment 5 --from bob
//! expect ok
//! expect event Increased
//! view c get
//! expect result 5
//! call c increment 500
//! expect fail "at most 100"
//! ```
//!
//! Commands (`--from` defaults to `alice`; accounts are fictitious):
//! * `deploy <file> as <name> [args...] [--from A] [--value 5tcn] [--final] [--language 1|2]`
//! * `call <name> <function> [args...] [--from A] [--value 5tcn]`
//! * `view <name> <function> [args...]`
//! * `upgrade <name> <file> [args...] [--from A]` · `authority <name> <@account|none> [--from A]`
//! * `height <n>` · `advance <n>`
//! * `expect ok` · `expect fail ["text"]` · `expect result <value>` · `expect event <Name>`
//!   · `expect balance <@account|name> <amount>` · `expect state <name> <variable> <value>`
//!
//! Arguments use the declared parameter types: `42`, `2.5tcn`, `true`, `"text"`,
//! `0xabcd`, `[1, 2]`, `{field: value}`, `Variant`, `@account` or `$name` (a
//! deployed contract) for addresses.

use crate::abi::{display_typed, parse_arg_in};
use crate::program::{Program, Type, Value};
use crate::sim::{account, CallResult, DeployOptions, Simulator};
use std::collections::BTreeMap;

/// Result of running one scenario.
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct Report {
    pub passed: usize,
    pub failed: Vec<String>,
    /// Human-readable transcript.
    pub log: Vec<String>,
}

impl Report {
    pub fn ok(&self) -> bool {
        self.failed.is_empty()
    }
}

fn split_words(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut quote, mut depth) = (false, 0i32);
    for ch in line.chars() {
        match ch {
            '"' => {
                quote = !quote;
                cur.push(ch);
            }
            '[' | '{' if !quote => {
                depth += 1;
                cur.push(ch);
            }
            ']' | '}' if !quote => {
                depth -= 1;
                cur.push(ch);
            }
            ' ' | '\t' if !quote && depth == 0 => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            _ => cur.push(ch),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

struct Opts {
    from: String,
    value: u64,
    final_code: bool,
    language: u16,
}

fn take_opts(words: &mut Vec<String>) -> Result<Opts, String> {
    let mut o = Opts { from: "alice".into(), value: 0, final_code: false, language: crate::program::LANGUAGE_VERSION };
    let mut i = 0;
    while i < words.len() {
        match words[i].as_str() {
            "--from" | "--value" | "--language" => {
                let v = words.get(i + 1).cloned().ok_or_else(|| format!("{} needs a value", words[i]))?;
                match words[i].as_str() {
                    "--from" => o.from = v.trim_start_matches('@').to_string(),
                    "--value" => {
                        o.value = match crate::abi::parse_arg(&v, &Type::Int)? {
                            Value::Int(n) if n >= 0 && n <= u64::MAX as i128 => n as u64,
                            _ => return Err(format!("invalid --value {v}")),
                        }
                    }
                    _ => o.language = v.parse().map_err(|_| format!("invalid --language {v}"))?,
                }
                words.drain(i..i + 2);
            }
            "--final" => {
                o.final_code = true;
                words.remove(i);
            }
            _ => i += 1,
        }
    }
    Ok(o)
}

/// Runs a scenario. `read` loads contract files referenced by the script.
pub fn run(script: &str, read: &dyn Fn(&str) -> Result<String, String>) -> Report {
    let mut report = Report::default();
    let mut sim = Simulator::new();
    let mut names: BTreeMap<String, String> = BTreeMap::new();
    let mut last: Option<(CallResult, Option<Program>)> = None;
    let hrp = "tcr";
    for (n, raw) in script.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let at = format!("line {}", n + 1);
        let step = run_line(line, &mut sim, &mut names, &mut last, read, hrp, &mut report);
        if let Err(e) = step {
            report.failed.push(format!("{at}: {line}: {e}"));
            report.log.push(format!("✘ {line}\n  {e}"));
        }
    }
    report
}

fn resolve_address(word: &str, names: &BTreeMap<String, String>) -> Option<Value> {
    if let Some(a) = word.strip_prefix('@') {
        return Some(Value::Address(account(a)));
    }
    let n = word.strip_prefix('$')?;
    let hex_addr = names.get(n)?;
    let mut a = [0u8; 20];
    a.copy_from_slice(&hex::decode(hex_addr).ok()?);
    Some(Value::Address(a))
}

fn args_for(program: &Program, function: &str, raw: &[String], names: &BTreeMap<String, String>) -> Result<Vec<Value>, String> {
    let params: Vec<(String, Type)> = match program.find(function) {
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
                if let Some(v) = resolve_address(a, names) {
                    return Ok(v);
                }
            }
            parse_arg_in(program, a, t).map_err(|e| format!("argument '{name}': {e}"))
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn run_line(
    line: &str,
    sim: &mut Simulator,
    names: &mut BTreeMap<String, String>,
    last: &mut Option<(CallResult, Option<Program>)>,
    read: &dyn Fn(&str) -> Result<String, String>,
    hrp: &str,
    report: &mut Report,
) -> Result<(), String> {
    let mut words = split_words(line);
    let cmd = words.remove(0);
    let describe = |r: &CallResult, p: Option<&Program>| -> String {
        let mut s = String::new();
        for e in &r.events {
            let fields = e.fields.iter().map(|(k, v)| format!("{k}: {}", crate::abi::display(v, hrp))).collect::<Vec<_>>().join(", ");
            s.push_str(&format!("  event {}({fields})\n", e.name));
        }
        match &r.result {
            Ok(Value::Unit) => s.push_str(&format!("  ok · fuel {}", r.fuel_used)),
            Ok(v) => s.push_str(&format!("  result {} · fuel {}", p.map_or_else(|| crate::abi::display(v, hrp), |p| display_typed(p, v, &r.ret, hrp)), r.fuel_used)),
            Err(e) => s.push_str(&format!("  failed: {e} · fuel {} (reverted)", r.fuel_used)),
        }
        s
    };
    match cmd.as_str() {
        "deploy" => {
            let o = take_opts(&mut words)?;
            if words.len() < 3 || words[1] != "as" {
                return Err("usage: deploy <file> as <name> [args...]".into());
            }
            let file = words.remove(0);
            words.remove(0);
            let name = words.remove(0);
            let src = read(&file)?;
            let program = crate::compile(&src, &crate::CompileOptions { version: o.language, ..Default::default() }).map_err(|e| format!("{file}:{e}"))?;
            let args = args_for(&program, "init", &words, names)?;
            let (addr, r) = sim.deploy_with(&src, account(&o.from), args, o.value, &DeployOptions { language: o.language, final_code: o.final_code })?;
            report.log.push(format!("deploy {file} as {name}\n{}", describe(&r, Some(&program))));
            if r.result.is_ok() {
                names.insert(name, addr);
            }
            *last = Some((r, Some(program)));
        }
        "call" | "view" => {
            let o = take_opts(&mut words)?;
            if words.len() < 2 {
                return Err(format!("usage: {cmd} <name> <function> [args...]"));
            }
            let name = words.remove(0);
            let function = words.remove(0);
            let addr = names.get(&name).cloned().ok_or_else(|| format!("no deployed contract named '{name}'"))?;
            let program = sim.program(&addr).cloned().ok_or("contract missing")?;
            let args = args_for(&program, &function, &words, names)?;
            let r = if cmd == "call" { sim.call(&addr, account(&o.from), &function, args, o.value)? } else { sim.view(&addr, &function, args)? };
            report.log.push(format!("{line}\n{}", describe(&r, Some(&program))));
            *last = Some((r, Some(program)));
        }
        "upgrade" => {
            let o = take_opts(&mut words)?;
            if words.len() < 2 {
                return Err("usage: upgrade <name> <file> [args...]".into());
            }
            let name = words.remove(0);
            let file = words.remove(0);
            let addr = names.get(&name).cloned().ok_or_else(|| format!("no deployed contract named '{name}'"))?;
            let src = read(&file)?;
            let preview = crate::compile(&src, &crate::CompileOptions::default()).map_err(|e| format!("{file}:{e}"))?;
            let args = args_for(&preview, "upgrade", &words, names)?;
            let (rep, r) = sim.upgrade(&addr, account(&o.from), &src, args)?;
            report.log.push(format!(
                "upgrade {name} with {file} (added state: {}; notes: {})\n{}",
                if rep.added_state.is_empty() { "-".into() } else { rep.added_state.join(", ") },
                if rep.notes.is_empty() { "-".into() } else { rep.notes.join("; ") },
                describe(&r, sim.program(&addr))
            ));
            *last = Some((r, sim.program(&addr).cloned()));
        }
        "authority" => {
            let o = take_opts(&mut words)?;
            let [name, who] = words.as_slice() else { return Err("usage: authority <name> <@account|none>".into()) };
            let addr = names.get(name).cloned().ok_or_else(|| format!("no deployed contract named '{name}'"))?;
            let new = if who == "none" { None } else { Some(account(who.trim_start_matches('@'))) };
            sim.set_authority(&addr, account(&o.from), new)?;
            report.log.push(line.to_string());
        }
        "height" | "advance" => {
            let n: u64 = words.first().and_then(|w| w.parse().ok()).ok_or_else(|| format!("usage: {cmd} <blocks>"))?;
            sim.height = if cmd == "height" { n } else { sim.height + n };
            report.log.push(format!("{cmd} {n} (height {})", sim.height));
        }
        "expect" => {
            let what = words.first().cloned().ok_or("usage: expect ok|fail|result|event|balance|state ...")?;
            let rest = &words[1..];
            let (r, program) = last.as_ref().map(|(r, p)| (Some(r), p.as_ref())).unwrap_or((None, None));
            let check = |ok: bool, msg: String| if ok { Ok(()) } else { Err(msg) };
            let outcome = match what.as_str() {
                "ok" => {
                    let r = r.ok_or("nothing to check yet")?;
                    check(r.result.is_ok(), format!("expected success, got: {}", r.result.as_ref().err().map(|e| e.to_string()).unwrap_or_default()))
                }
                "fail" => {
                    let r = r.ok_or("nothing to check yet")?;
                    let needle = rest.join(" ");
                    let needle = needle.trim_matches('"');
                    match &r.result {
                        Ok(_) => Err("expected a failure, but the call succeeded".into()),
                        Err(e) => check(e.to_string().contains(needle), format!("failure '{e}' does not contain '{needle}'")),
                    }
                }
                "result" => {
                    let r = r.ok_or("nothing to check yet")?;
                    let got = r.result.as_ref().map_err(|e| format!("the call failed: {e}"))?;
                    let raw = rest.join(" ");
                    let expected = match (program, &r.ret) {
                        (Some(p), t) if *t != Type::Unit => {
                            resolve_address(&raw, names).filter(|_| matches!(t, Type::Address)).map(Ok).unwrap_or_else(|| parse_arg_in(p, &raw, t))?
                        }
                        _ => return Err("this call returns nothing".into()),
                    };
                    check(*got == expected, format!("expected result {raw}, got {}", program.map_or_else(|| crate::abi::display(got, hrp), |p| display_typed(p, got, &r.ret, hrp))))
                }
                "event" => {
                    let r = r.ok_or("nothing to check yet")?;
                    let name = rest.first().ok_or("usage: expect event <Name>")?;
                    check(r.events.iter().any(|e| &e.name == name), format!("no event {name} (events: {})", r.events.iter().map(|e| e.name.as_str()).collect::<Vec<_>>().join(", ")))
                }
                "balance" => {
                    let [who, amount] = rest else { return Err("usage: expect balance <@account|name> <amount>".into()) };
                    let addr = match resolve_address(who, names).or_else(|| resolve_address(&format!("${who}"), names)) {
                        Some(Value::Address(a)) => a,
                        _ => return Err(format!("unknown account or contract '{who}'")),
                    };
                    let want = match crate::abi::parse_arg(amount, &Type::Int)? {
                        Value::Int(v) => v,
                        _ => unreachable!(),
                    };
                    let got = sim.balance_of(&addr) as i128;
                    check(got == want, format!("expected balance {want}, got {got}"))
                }
                "state" => {
                    if rest.len() < 3 {
                        return Err("usage: expect state <name> <variable> <value>".into());
                    }
                    let addr = names.get(&rest[0]).cloned().ok_or_else(|| format!("no deployed contract named '{}'", rest[0]))?;
                    let entry = sim.describe_state(&addr, hrp).into_iter().find(|e| e.name == rest[1]).ok_or_else(|| format!("no state variable '{}'", rest[1]))?;
                    let want = rest[2..].join(" ");
                    check(entry.value == want, format!("expected {} = {want}, got {}", rest[1], entry.value))
                }
                other => Err(format!("unknown expectation '{other}'")),
            };
            outcome?;
            report.passed += 1;
            report.log.push(format!("✔ {line}"));
        }
        other => return Err(format!("unknown command '{other}'")),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_a_scenario() {
        let files: BTreeMap<&str, String> = [
            ("counter.tccl", include_str!("../examples/counter.tccl").to_string()),
            ("counter_v2.tccl", include_str!("../examples/counter_v2.tccl").to_string()),
        ]
        .into_iter()
        .collect();
        let read = |f: &str| files.get(f).cloned().ok_or_else(|| format!("missing {f}"));
        let script = "deploy counter.tccl as c --from dev\ncall c increment 5 --from bob\nexpect ok\nexpect event Increased\nview c last\nexpect result @bob\ncall c increment 500\nexpect fail \"at most 100\"\nupgrade c counter_v2.tccl --from dev\nexpect ok\ncall c increment 1\nview c get\nexpect result 15\nexpect state c step 10\nexpect balance @bob 1000000tcn\n";
        let r = run(script, &read);
        assert!(r.ok(), "{:#?}", r);
        assert_eq!(r.passed, 8);
        let bad = run("deploy counter.tccl as c\nview c get\nexpect result 1\n", &read);
        assert_eq!(bad.failed.len(), 1);
        assert!(bad.failed[0].contains("expected result 1, got 0"));
    }
}
