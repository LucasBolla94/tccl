//! Upgrades: rules for replacing a deployed contract's code.
//!
//! TCCL follows the **upgrade authority** model: a contract is deployed with an
//! authority (by default the deployer) that may replace its code at any time, or
//! with no authority at all (*final*). The authority can hand over its power or
//! renounce it, making the contract final forever. Upgrades are always an
//! explicit, signed transaction by the authority; nothing upgrades implicitly.
//!
//! Users of a contract can see whether it is upgradeable (`is_final(addr)` from
//! other contracts, `upgradeable` in tools) and should treat an upgradeable
//! contract as trusting its authority.
//!
//! Storage is kept across an upgrade, so the new code must read the existing
//! data the same way. [`check`] enforces:
//! * every state variable of the old program still exists with the same name, the
//!   same storage slot (the compiler keeps slots, see
//!   [`crate::CompileOptions::state_order`]) and a compatible type;
//! * records stored in state keep exactly the same fields;
//! * enums stored in state keep their variants in order (new variants may be added
//!   at the end);
//! * the language version does not go backwards.
//!
//! If the new code declares `upgrade():`, it runs once inside the upgrade
//! transaction (called by the authority); if it fails, the whole upgrade is
//! reverted and the old code stays.

use crate::program::{FnKind, Program, Type};

/// What changes between two versions of a contract (shown before upgrading).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct Report {
    pub added_state: Vec<String>,
    pub added_functions: Vec<String>,
    pub removed_functions: Vec<String>,
    pub changed_functions: Vec<String>,
    /// Human-readable notes, e.g. "the new code can send TCN (the old one could not)".
    pub notes: Vec<String>,
    pub runs_upgrade_hook: bool,
}

fn shape_compatible(old: &Program, ot: &Type, new: &Program, nt: &Type, depth: usize) -> Result<(), String> {
    if depth > 64 {
        return Err("types nested too deeply".into());
    }
    match (ot, nt) {
        (Type::List(a), Type::List(b)) => shape_compatible(old, a, new, b, depth + 1),
        (Type::Map(ak, av), Type::Map(bk, bv)) => {
            shape_compatible(old, ak, new, bk, depth + 1)?;
            shape_compatible(old, av, new, bv, depth + 1)
        }
        (Type::Record(a), Type::Record(b)) => {
            let (ra, rb) = (&old.records[*a as usize], &new.records[*b as usize]);
            if ra.fields.len() != rb.fields.len() {
                return Err(format!("record {} must keep exactly the same fields ({} before, {} now)", ra.name, ra.fields.len(), rb.fields.len()));
            }
            for ((na, ta), (nb, tb)) in ra.fields.iter().zip(&rb.fields) {
                if na != nb {
                    return Err(format!("record {}: field '{na}' was renamed or moved (now '{nb}')", ra.name));
                }
                shape_compatible(old, ta, new, tb, depth + 1).map_err(|e| format!("record {}.{na}: {e}", ra.name))?;
            }
            Ok(())
        }
        (Type::Enum(a), Type::Enum(b)) => {
            let (ea, eb) = (&old.enums[*a as usize], &new.enums[*b as usize]);
            if eb.variants.len() < ea.variants.len() || eb.variants[..ea.variants.len()] != ea.variants[..] {
                return Err(format!("enum {} must keep its variants in the same order (new variants can only be added at the end)", ea.name));
            }
            Ok(())
        }
        (Type::Interface(_), Type::Interface(_) | Type::Address) | (Type::Address, Type::Interface(_)) => Ok(()),
        (a, b) if a == b && !a.uses_v2() => Ok(()),
        _ => Err(format!("type changed from {} to {}", old.type_name(ot), new.type_name(nt))),
    }
}

/// Checks that `new` can replace `old` while keeping its storage.
pub fn check(old: &Program, new: &Program) -> Result<Report, String> {
    if new.version < old.version {
        return Err(format!("the language version cannot go backwards ({} → {})", old.version, new.version));
    }
    let mut report = Report::default();
    for (i, s) in old.states.iter().enumerate() {
        let Some(n) = new.states.get(i) else {
            return Err(format!("state variable '{}' was removed", s.name));
        };
        if n.name != s.name {
            return Err(format!("state variable '{}' must keep its storage slot (slot {i} is now '{}')", s.name, n.name));
        }
        shape_compatible(old, &s.ty, new, &n.ty, 0).map_err(|e| format!("state variable '{}': {e}", s.name))?;
    }
    report.added_state = new.states[old.states.len()..].iter().map(|s| s.name.clone()).collect();
    let entry = |p: &Program| -> Vec<(String, String)> {
        p.abi()
            .into_iter()
            .filter(|f| f.kind != FnKind::Upgrade)
            .map(|f| {
                let sig = format!(
                    "{:?} {}({}) -> {}{}",
                    f.kind,
                    f.name,
                    f.params.iter().map(|(n, t)| format!("{n}: {t}")).collect::<Vec<_>>().join(", "),
                    f.returns,
                    if f.payable { " payable" } else { "" }
                );
                (f.name, sig)
            })
            .collect()
    };
    let (before, after) = (entry(old), entry(new));
    for (name, sig) in &after {
        match before.iter().find(|(n, _)| n == name) {
            None => report.added_functions.push(name.clone()),
            Some((_, old_sig)) if old_sig != sig => report.changed_functions.push(name.clone()),
            _ => {}
        }
    }
    report.removed_functions = before.iter().filter(|(n, _)| !after.iter().any(|(m, _)| m == n)).map(|(n, _)| n.clone()).collect();
    let (a, b) = (&old.effects, &new.effects);
    let mut note = |was: bool, now: bool, what: &str| {
        if now && !was {
            report.notes.push(format!("the new code can {what} (the old code could not)"));
        }
    };
    if old.version >= 2 {
        note(a.sends_tcn, b.sends_tcn, "send TCN");
        note(a.calls_contracts, b.calls_contracts, "call other contracts");
        note(a.can_destroy, b.can_destroy, "destroy the contract");
        note(a.receives_tcn, b.receives_tcn, "receive TCN");
    }
    if old.name != new.name {
        report.notes.push(format!("the contract name changes from {} to {}", old.name, new.name));
    }
    report.runs_upgrade_hook = new.functions.iter().any(|f| f.kind == FnKind::Upgrade);
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compile, CompileOptions};

    fn build(src: &str, prev: Option<&Program>) -> Program {
        let opts = CompileOptions { state_order: prev.map(|p| p.states.iter().map(|s| s.name.clone()).collect()).unwrap_or_default(), ..Default::default() };
        compile(src, &opts).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn compatible_and_incompatible_upgrades() {
        let v1 = build("contract A\nenum E: X, Y\nrecord R:\n    a: int\nstate e: E\nstate r: R\nstate n: int\naction f():\n    n += 1\n", None);
        let ok = build(
            "contract A\nenum E: X, Y, Z\nrecord R:\n    a: int\nstate extra: text\nstate n: int\nstate r: R\nstate e: E\naction f():\n    n += 2\naction g(to: address):\n    send(to, 1)\n",
            Some(&v1),
        );
        let report = check(&v1, &ok).unwrap();
        assert_eq!(report.added_state, ["extra"]);
        assert_eq!(report.added_functions, ["g"]);
        assert!(report.notes.iter().any(|n| n.contains("send TCN")));

        let reordered_enum = build("contract A\nenum E: Y, X\nrecord R:\n    a: int\nstate e: E\nstate r: R\nstate n: int\naction f():\n    pass\n", Some(&v1));
        assert!(check(&v1, &reordered_enum).unwrap_err().contains("variants"));
        let new_field = build("contract A\nenum E: X, Y\nrecord R:\n    a: int\n    b: int\nstate e: E\nstate r: R\nstate n: int\naction f():\n    pass\n", Some(&v1));
        assert!(check(&v1, &new_field).unwrap_err().contains("same fields"));
        let retyped = build("contract A\nenum E: X, Y\nrecord R:\n    a: int\nstate e: E\nstate r: R\nstate n: text\naction f():\n    pass\n", Some(&v1));
        assert!(check(&v1, &retyped).unwrap_err().contains("type changed"));
        let v1_lang = compile("contract A\nstate n: int\naction f():\n    n += 1\n", &CompileOptions { version: 1, ..Default::default() }).unwrap();
        assert!(check(&ok, &v1_lang).is_err(), "version cannot go backwards");
    }
}
