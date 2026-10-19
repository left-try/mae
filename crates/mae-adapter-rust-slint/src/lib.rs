//! Native Windows & Desktop Rust + Slint Target Adapter (`crates/mae-adapter-rust-slint`).
//!
//! Implements `TargetAdapterPort` from `mae-domain` to scaffold and emit genuine
//! native Rust + Slint desktop applications with strict Logic-UI separation and
//! Windows GUI subsystem configuration (`#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]`).

pub mod codegen;

pub use codegen::{RustSlintAdapterConfig, RustSlintTargetAdapter};
