//! **Frozen** language version 1 compiler — byte-for-byte the compiler deployed on
//! The Coin (thecoin@8f620ea, `crates/tccl`). Programs compiled here are part of
//! chain history: every node must keep producing exactly the same bytes, errors
//! and positions to replay old deployments. Do not change behaviour in this
//! module; fix bugs only in newer language versions. The differential tests in
//! `crates/tccl-compat` compare it with the reference implementation.

pub mod ast;
pub mod checker;
pub mod lexer;
pub mod parser;
