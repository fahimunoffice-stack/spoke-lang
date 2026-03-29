// src/error.rs
// All compiler errors with human-readable messages

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SpokeError {
    // ── Lexer errors ──────────────────────────────────────────────────────────
    #[error("Unexpected character '{ch}' at line {line}, col {col}")]
    UnexpectedChar { ch: char, line: usize, col: usize },

    #[error("Unterminated string starting at line {line}, col {col}")]
    UnterminatedString { line: usize, col: usize },

    #[error("Invalid number '{value}' at line {line}, col {col}")]
    InvalidNumber { value: String, line: usize, col: usize },

    // ── Parser errors ─────────────────────────────────────────────────────────
    #[error("Expected {expected} but found {found} at line {line}")]
    UnexpectedToken {
        expected: String,
        found: String,
        line: usize,
    },

    #[error("Unexpected end of file — were you in the middle of writing something?")]
    UnexpectedEof,

    // ── Semantic errors ───────────────────────────────────────────────────────
    #[error(
        "Unknown field '{field}' on {entity} at line {line}.\n\
         {entity} has: {available}\n\
         Did you mean '{suggestion}'?"
    )]
    UnknownField {
        field: String,
        entity: String,
        available: String,
        suggestion: String,
        line: usize,
    },

    #[error(
        "Entity '{name}' is referenced but never declared.\n\
         Declare it with: {name} has ..."
    )]
    UndeclaredEntity { name: String },

    #[error(
        "Circular dependency detected: {chain}\n\
         Entities cannot depend on each other in a circle."
    )]
    CircularDependency { chain: String },

    // ── IO errors ─────────────────────────────────────────────────────────────
    #[error("Cannot read file '{path}': {reason}")]
    FileRead { path: String, reason: String },

    #[error("Cannot write output to '{path}': {reason}")]
    FileWrite { path: String, reason: String },
}

/// Pretty-print a SpokeError with colors for terminal output
pub fn print_error(err: &SpokeError) {
    use colored::Colorize;
    eprintln!("{} {}", "error:".red().bold(), err);
}

pub fn print_warning(msg: &str) {
    use colored::Colorize;
    eprintln!("{} {}", "warning:".yellow().bold(), msg);
}

pub fn print_success(msg: &str) {
    use colored::Colorize;
    println!("{} {}", "✓".green().bold(), msg);
}
