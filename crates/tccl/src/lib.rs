//! # TCCL — The Coin Cloud Language
//!
//! A small, statically typed, indentation-based language for smart contracts
//! on The Coin, and the deterministic virtual machine that runs them.
//!
//! ```text
//! contract Counter
//!
//! state count: int
//!
//! action increment(by: int):
//!     require by > 0, "by must be positive"
//!     count += by
//!
//! view get() -> int:
//!     return count
//! ```
//!
//! * [`compile`] — source → type-checked [`Program`] for language version 1
//!   (frozen, [`v1`]) or 2 (current, [`checker`])
//! * [`vm::execute`] — runs a program against a [`vm::Host`] with fuel metering
//! * [`modules`] — standard library (`use std.token`) and bundling of local modules
//! * [`upgrade`] — compatibility rules for replacing a contract's code
//! * [`sim`] — in-memory chain for local testing (`tccl run`, `tccl test`, playground)
//! * [`diagnostics`] — explanations and suggested fixes for errors
//! * [`ring`] — linkable ring signatures used by privacy contracts
//! * [`abi`] — argument parsing and JSON rendering
//!
//! The compiler is part of the consensus rules: a deployment transaction
//! carries source code, and every node compiles it identically.

pub mod abi;
pub mod ast;
pub mod checker;
pub mod diagnostics;
pub mod error;
pub mod modules;
pub mod ops;
pub mod parser;
pub mod program;
pub mod ring;
pub mod scenario;
pub mod sim;
pub mod upgrade;
pub mod v1;
pub mod vm;

pub use checker::{compile, CompileOptions};
pub use error::{CompileError, VmError};
pub use program::{Program, Type, Value};

/// Version 1 modules kept at their historical paths.
pub use v1::lexer;
