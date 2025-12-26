//! Integration tests for the full pipeline

use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;
use tauri_codegen::config::{Config, InputConfig, NamingConfig, OutputConfig};
use tauri_codegen::pipeline::Pipeline;

/// Create a test config with temp directories
fn create_test_config(source_dir: PathBuf, output_dir: PathBuf) -> Config {
    Config {
        input: InputConfig {
            source_dir,
            exclude: vec!["tests".to_string(), "target".to_string()],
        },
        output: OutputConfig {
            types_file: output_dir.join("types.ts"),
            commands_file: output_dir.join("commands.ts"),
        },
        naming: NamingConfig::default(),
    }
}

#[test]
fn test_full_pipeline_simple() {
    let temp = tempdir().unwrap();
    let src_dir = temp.path().join("src");
    let output_dir = temp.path().join("generated");

    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&output_dir).unwrap();

    // Create a simple Rust file with commands and types
    let code = r#"
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub name: String,
}

#[tauri::command]
pub fn get_user(id: i32) -> Result<User, String> {
    unimplemented!()
}

#[tauri::command]
pub fn list_users() -> Vec<User> {
    vec![]
}
"#;

    fs::write(src_dir.join("commands.rs"), code).unwrap();

    let config = create_test_config(src_dir, output_dir.clone());
    let pipeline = Pipeline::new(false);

    let result = pipeline.run(&config);
    assert!(result.is_ok(), "Pipeline should succeed: {:?}", result.err());

    // Verify output files exist
    assert!(output_dir.join("types.ts").exists());
    assert!(output_dir.join("commands.ts").exists());

    // Verify types.ts content
    let types_content = fs::read_to_string(output_dir.join("types.ts")).unwrap();
    assert!(types_content.contains("export interface User"));
    assert!(types_content.contains("id: number"));
    assert!(types_content.contains("name: string"));

    // Verify commands.ts content
    let commands_content = fs::read_to_string(output_dir.join("commands.ts")).unwrap();
    assert!(commands_content.contains("export async function getUser"));
    assert!(commands_content.contains("export async function listUsers"));
    assert!(commands_content.contains("import { invoke }"));
}

#[test]
fn test_pipeline_with_multiple_files() {
    let temp = tempdir().unwrap();
    let src_dir = temp.path().join("src");
    let output_dir = temp.path().join("generated");

    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&output_dir).unwrap();

    // types.rs
    let types_code = r#"
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct Item {
    pub id: i32,
    pub title: String,
}
"#;
    fs::write(src_dir.join("types.rs"), types_code).unwrap();

    // commands.rs
    let commands_code = r#"
use crate::types::{User, Item};

#[tauri::command]
pub fn get_user(id: i32) -> User {
    unimplemented!()
}

#[tauri::command]
pub fn get_item(id: i32) -> Item {
    unimplemented!()
}
"#;
    fs::write(src_dir.join("commands.rs"), commands_code).unwrap();

    let config = create_test_config(src_dir, output_dir.clone());
    let pipeline = Pipeline::new(false);

    let result = pipeline.run(&config);
    assert!(result.is_ok(), "Pipeline should succeed: {:?}", result.err());

    // Verify both types are in output
    let types_content = fs::read_to_string(output_dir.join("types.ts")).unwrap();
    assert!(types_content.contains("export interface User"));
    assert!(types_content.contains("export interface Item"));

    // Verify both commands are in output
    let commands_content = fs::read_to_string(output_dir.join("commands.ts")).unwrap();
    assert!(commands_content.contains("export async function getUser"));
    assert!(commands_content.contains("export async function getItem"));
}

#[test]
fn test_pipeline_excludes_directories() {
    let temp = tempdir().unwrap();
    let src_dir = temp.path().join("src");
    let output_dir = temp.path().join("generated");
    let tests_dir = src_dir.join("tests");

    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&tests_dir).unwrap();
    fs::create_dir_all(&output_dir).unwrap();

    // Main file
    let main_code = r#"
use serde::Serialize;

#[derive(Serialize)]
pub struct User {
    pub id: i32,
}

#[tauri::command]
pub fn get_user(id: i32) -> User {
    unimplemented!()
}
"#;
    fs::write(src_dir.join("main.rs"), main_code).unwrap();

    // Test file (should be excluded)
    let test_code = r#"
use serde::Serialize;

#[derive(Serialize)]
pub struct TestType {
    pub data: String,
}

#[tauri::command]
pub fn test_command() -> TestType {
    unimplemented!()
}
"#;
    fs::write(tests_dir.join("test.rs"), test_code).unwrap();

    let config = create_test_config(src_dir, output_dir.clone());
    let pipeline = Pipeline::new(false);

    let result = pipeline.run(&config);
    assert!(result.is_ok());

    // Verify User is included but TestType is not
    let types_content = fs::read_to_string(output_dir.join("types.ts")).unwrap();
    assert!(types_content.contains("export interface User"));
    assert!(!types_content.contains("TestType"));

    // Verify get_user is included but test_command is not
    let commands_content = fs::read_to_string(output_dir.join("commands.ts")).unwrap();
    assert!(commands_content.contains("export async function getUser"));
    assert!(!commands_content.contains("testCommand"));
}

#[test]
fn test_pipeline_with_naming_config() {
    let temp = tempdir().unwrap();
    let src_dir = temp.path().join("src");
    let output_dir = temp.path().join("generated");

    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&output_dir).unwrap();

    let code = r#"
use serde::Serialize;

#[derive(Serialize)]
pub struct User {
    pub id: i32,
}

#[tauri::command]
pub fn get_user(id: i32) -> User {
    unimplemented!()
}
"#;
    fs::write(src_dir.join("lib.rs"), code).unwrap();

    let config = Config {
        input: InputConfig {
            source_dir: src_dir,
            exclude: vec![],
        },
        output: OutputConfig {
            types_file: output_dir.join("types.ts"),
            commands_file: output_dir.join("commands.ts"),
        },
        naming: NamingConfig {
            type_prefix: "I".to_string(),
            type_suffix: "".to_string(),
            function_prefix: "".to_string(),
            function_suffix: "Cmd".to_string(),
        },
    };

    let pipeline = Pipeline::new(false);
    let result = pipeline.run(&config);
    assert!(result.is_ok());

    let types_content = fs::read_to_string(output_dir.join("types.ts")).unwrap();
    assert!(types_content.contains("export interface IUser"));

    let commands_content = fs::read_to_string(output_dir.join("commands.ts")).unwrap();
    assert!(commands_content.contains("export async function getUserCmd"));
}

#[test]
fn test_pipeline_empty_source() {
    let temp = tempdir().unwrap();
    let src_dir = temp.path().join("src");
    let output_dir = temp.path().join("generated");

    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&output_dir).unwrap();

    // No Rust files

    let config = create_test_config(src_dir, output_dir.clone());
    let pipeline = Pipeline::new(false);

    let result = pipeline.run(&config);
    assert!(result.is_ok());

    // Files should still be generated (though empty)
    assert!(output_dir.join("types.ts").exists());
    assert!(output_dir.join("commands.ts").exists());
}

#[test]
fn test_pipeline_creates_output_directories() {
    let temp = tempdir().unwrap();
    let src_dir = temp.path().join("src");
    let output_dir = temp.path().join("deeply").join("nested").join("output");

    fs::create_dir_all(&src_dir).unwrap();
    // Don't create output_dir - pipeline should create it

    let code = r#"
use serde::Serialize;

#[derive(Serialize)]
pub struct User {
    pub id: i32,
}

#[tauri::command]
pub fn greet() -> User {
    unimplemented!()
}
"#;
    fs::write(src_dir.join("lib.rs"), code).unwrap();

    let config = create_test_config(src_dir, output_dir.clone());
    let pipeline = Pipeline::new(false);

    let result = pipeline.run(&config);
    assert!(result.is_ok());

    assert!(output_dir.join("types.ts").exists());
    assert!(output_dir.join("commands.ts").exists());
}

#[test]
fn test_pipeline_with_enums() {
    let temp = tempdir().unwrap();
    let src_dir = temp.path().join("src");
    let output_dir = temp.path().join("generated");

    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&output_dir).unwrap();

    let code = r#"
use serde::Serialize;

#[derive(Serialize)]
pub enum Status {
    Active,
    Inactive,
    Pending,
}

#[tauri::command]
pub fn get_status() -> Status {
    Status::Active
}
"#;
    fs::write(src_dir.join("lib.rs"), code).unwrap();

    let config = create_test_config(src_dir, output_dir.clone());
    let pipeline = Pipeline::new(false);

    let result = pipeline.run(&config);
    assert!(result.is_ok());

    let types_content = fs::read_to_string(output_dir.join("types.ts")).unwrap();
    assert!(types_content.contains("export type Status"));
    assert!(types_content.contains("\"Active\""));
    assert!(types_content.contains("\"Inactive\""));
    assert!(types_content.contains("\"Pending\""));
}

#[test]
fn test_pipeline_with_complex_return_types() {
    let temp = tempdir().unwrap();
    let src_dir = temp.path().join("src");
    let output_dir = temp.path().join("generated");

    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&output_dir).unwrap();

    let code = r#"
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
pub struct User {
    pub id: i32,
}

#[tauri::command]
pub fn get_users() -> Vec<User> {
    vec![]
}

#[tauri::command]
pub fn get_optional_user(id: i32) -> Option<User> {
    None
}

#[tauri::command]
pub fn get_user_result(id: i32) -> Result<User, String> {
    unimplemented!()
}

#[tauri::command]
pub fn get_user_map() -> HashMap<String, User> {
    HashMap::new()
}
"#;
    fs::write(src_dir.join("lib.rs"), code).unwrap();

    let config = create_test_config(src_dir, output_dir.clone());
    let pipeline = Pipeline::new(false);

    let result = pipeline.run(&config);
    assert!(result.is_ok());

    let commands_content = fs::read_to_string(output_dir.join("commands.ts")).unwrap();
    assert!(commands_content.contains("Promise<User[]>"));
    assert!(commands_content.contains("Promise<User | null>"));
    assert!(commands_content.contains("Promise<User>"));
    assert!(commands_content.contains("Promise<Record<string, User>>"));
}

#[test]
fn test_pipeline_verbose_mode() {
    let temp = tempdir().unwrap();
    let src_dir = temp.path().join("src");
    let output_dir = temp.path().join("generated");

    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&output_dir).unwrap();

    let code = r#"
use serde::Serialize;

#[derive(Serialize)]
pub struct User {
    pub id: i32,
}

#[tauri::command]
pub fn greet() -> User {
    unimplemented!()
}
"#;
    fs::write(src_dir.join("lib.rs"), code).unwrap();

    let config = create_test_config(src_dir, output_dir.clone());

    // Run with verbose mode - should not panic
    let pipeline = Pipeline::new(true);
    let result = pipeline.run(&config);
    assert!(result.is_ok());
}

#[test]
fn test_pipeline_filters_only_used_types() {
    let temp = tempdir().unwrap();
    let src_dir = temp.path().join("src");
    let output_dir = temp.path().join("generated");

    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&output_dir).unwrap();

    let code = r#"
use serde::Serialize;

#[derive(Serialize)]
pub struct UsedType {
    pub id: i32,
}

#[derive(Serialize)]
pub struct UnusedType {
    pub name: String,
}

#[tauri::command]
pub fn get_used() -> UsedType {
    unimplemented!()
}
"#;
    fs::write(src_dir.join("lib.rs"), code).unwrap();

    let config = create_test_config(src_dir, output_dir.clone());
    let pipeline = Pipeline::new(false);

    let result = pipeline.run(&config);
    assert!(result.is_ok());

    let types_content = fs::read_to_string(output_dir.join("types.ts")).unwrap();

    // UsedType should be included
    assert!(types_content.contains("export interface UsedType"));

    // UnusedType should NOT be included (not referenced by any command)
    assert!(!types_content.contains("UnusedType"));
}

