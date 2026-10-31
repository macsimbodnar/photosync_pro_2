//! The layer the front-ends share, for the GUI crate (DEC-067). `main.rs` keeps
//! its own `mod` lines, so the binary and its unit tests do not change: these
//! files compile once here, with no tests, and once in the binary.
#![allow(dead_code)]
#![allow(clippy::new_without_default)]
pub mod app;
pub mod args;
pub mod flow;
pub mod system;
pub mod text;
pub mod worker;
