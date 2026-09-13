//! Errors of the compiler and the virtual machine.

use std::fmt;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pos {
    pub line: u32,
    pub col: u32,
}

impl fmt::Display for Pos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

/// A compile error with the source position where it was detected.
///
/// `Display` is `line L:C: message` (unchanged since version 1). `help` carries an
/// optional suggested fix computed by the version 2 compiler (e.g. "did you mean
/// 'balances'?"); see [`crate::diagnostics`] for explanations in several languages.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("line {pos}: {message}")]
pub struct CompileError {
    pub pos: Pos,
    pub message: String,
    pub help: Option<String>,
}

impl CompileError {
    pub fn new(pos: Pos, message: impl Into<String>) -> Self {
        CompileError { pos, message: message.into(), help: None }
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

/// Runtime errors. Any error aborts the call and reverts every change it made.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum VmError {
    #[error("out of fuel")]
    OutOfFuel,
    #[error("requirement failed: {0}")]
    Require(String),
    #[error("integer overflow")]
    Overflow,
    #[error("division by zero")]
    DivisionByZero,
    #[error("index {index} out of bounds (length {len})")]
    IndexOutOfBounds { index: i128, len: u64 },
    #[error("value too large")]
    TooLarge,
    #[error("call depth limit reached")]
    CallDepth,
    #[error("unknown function '{0}'")]
    UnknownFunction(String),
    #[error("function '{0}' cannot be called this way")]
    NotCallable(String),
    #[error("wrong arguments: {0}")]
    BadArguments(String),
    #[error("function does not accept TCN (not payable)")]
    NotPayable,
    #[error("state cannot be modified in a view")]
    ReadOnly,
    #[error("invalid amount")]
    BadAmount,
    #[error("insufficient contract balance")]
    InsufficientBalance,
    #[error("contract cannot be destroyed while it still has storage ({0} entries)")]
    StorageNotEmpty(u64),
    #[error("host error: {0}")]
    Host(String),
    #[error("internal type error: {0}")]
    Type(String),
    // ---- version 2 ----
    #[error("re-entrant call: contract {0} is already running in this transaction")]
    Reentrancy(String),
    #[error("contract call depth limit reached")]
    ContractDepth,
    #[error("no contract at {0}")]
    NoContract(String),
    #[error("interface mismatch: {0}")]
    InterfaceMismatch(String),
    #[error("memory limit reached ({0} bytes)")]
    MemoryLimit(u64),
    #[error("transition not allowed: {enum_name} cannot go from {from} to {to}")]
    TransitionNotAllowed { enum_name: String, from: String, to: String },
    #[error("destroy() is only allowed when the contract is called directly by a transaction")]
    DestroyInNestedCall,
    #[error("not supported: {0}")]
    Unsupported(String),
}

impl VmError {
    /// Stable short code of the error (for documentation and tools).
    pub fn code(&self) -> &'static str {
        match self {
            VmError::OutOfFuel => "R001",
            VmError::Require(_) => "R002",
            VmError::Overflow => "R003",
            VmError::DivisionByZero => "R004",
            VmError::IndexOutOfBounds { .. } => "R005",
            VmError::TooLarge => "R006",
            VmError::CallDepth => "R007",
            VmError::UnknownFunction(_) => "R008",
            VmError::NotCallable(_) => "R009",
            VmError::BadArguments(_) => "R010",
            VmError::NotPayable => "R011",
            VmError::ReadOnly => "R012",
            VmError::BadAmount => "R013",
            VmError::InsufficientBalance => "R014",
            VmError::StorageNotEmpty(_) => "R015",
            VmError::Host(_) => "R016",
            VmError::Type(_) => "R017",
            VmError::Reentrancy(_) => "R018",
            VmError::ContractDepth => "R019",
            VmError::NoContract(_) => "R020",
            VmError::InterfaceMismatch(_) => "R021",
            VmError::MemoryLimit(_) => "R022",
            VmError::TransitionNotAllowed { .. } => "R023",
            VmError::DestroyInNestedCall => "R024",
            VmError::Unsupported(_) => "R025",
        }
    }
}
