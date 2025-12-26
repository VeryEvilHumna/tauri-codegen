//! tauri-codegen - Generate TypeScript bindings from Tauri commands
//!
//! This crate provides functionality to scan Rust source files for Tauri commands
//! and generate corresponding TypeScript bindings.

pub mod cli;
pub mod config;
pub mod generator;
pub mod models;
pub mod parser;
pub mod pipeline;
pub mod resolver;
pub mod scanner;
pub mod utils;

