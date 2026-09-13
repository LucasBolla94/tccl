//! Modules: compile-time libraries.
//!
//! A **module** is source code that a contract includes with `use`. It can declare
//! constants, records, enums, interfaces, events, state, roles, helpers (`fn`),
//! actions and views. Its code is compiled *into* the contract: a module has no
//! address, no balance and is never deployed or called on its own. Its state
//! lives inside the contract's storage and can only be changed by the module's
//! own functions; its actions and views become part of the contract interface.
//!
//! A **published contract** is what has an address on chain; other contracts
//! reach it only through an `interface` and a call.
//!
//! Standard modules (`use std.token`) ship with the compiler and are frozen per
//! language version: the hash of each source is recorded in
//! [`crate::program::ModuleRef`]. Local modules are sections at the end of the
//! same source file (`module mathx`); `tccl bundle` appends them from files.

/// Standard modules of language version 2: `(name, source)`.
pub const STD_MODULES: &[(&str, &str)] =
    &[("token", include_str!("../std/token.tccl")), ("items", include_str!("../std/items.tccl")), ("payments", include_str!("../std/payments.tccl"))];

pub fn std_source(name: &str) -> Option<&'static str> {
    STD_MODULES.iter().find(|(n, _)| *n == name).map(|(_, s)| *s)
}

/// A local module file, resolved by name (`use mathx` → `mathx.tccl`).
pub struct Bundle {
    /// Single source text ready to deploy.
    pub source: String,
    /// `(first line, file name)` of each part, to map error lines back to files.
    pub parts: Vec<(u32, String)>,
}

/// Appends the local modules used by `main` (and not already present in it) as
/// `module` sections, producing one deployable source.
pub fn bundle(main: &str, main_name: &str, resolve: &dyn Fn(&str) -> Option<String>) -> Result<Bundle, String> {
    let mut source = main.trim_end_matches('\n').to_string();
    let mut parts = vec![(1u32, main_name.to_string())];
    let declared: Vec<String> =
        main.lines().filter_map(|l| l.strip_prefix("module ")).map(|n| n.split('#').next().unwrap_or("").trim().to_string()).collect();
    for line in main.lines() {
        let Some(rest) = line.strip_prefix("use ") else { continue };
        let name = rest.split('#').next().unwrap_or("").split_whitespace().next().unwrap_or("");
        if name.is_empty() || name.starts_with("std.") || declared.iter().any(|d| d == name) {
            continue;
        }
        let text = resolve(name).ok_or_else(|| format!("module '{name}' not found (looked for {name}.tccl)"))?;
        let first = text.lines().find(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#')).unwrap_or("");
        if first.trim() != format!("module {name}") {
            return Err(format!("{name}.tccl must start with 'module {name}'"));
        }
        source.push_str("\n\n");
        parts.push((source.lines().count() as u32 + 1, format!("{name}.tccl")));
        source.push_str(text.trim_end_matches('\n'));
    }
    source.push('\n');
    Ok(Bundle { source, parts })
}

impl Bundle {
    /// File and line inside that file of a line of the bundled source.
    pub fn locate(&self, line: u32) -> (String, u32) {
        let (start, name) = self.parts.iter().rev().find(|(s, _)| *s <= line).cloned().unwrap_or((1, String::new()));
        (name, line - start + 1)
    }
}
