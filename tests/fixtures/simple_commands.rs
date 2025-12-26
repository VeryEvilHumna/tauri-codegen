// Fixture: Simple Tauri commands for testing

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: Option<String>,
}

#[derive(Serialize)]
pub enum Status {
    Active,
    Inactive,
    Pending,
}

#[tauri::command]
pub fn greet(name: String) -> String {
    format!("Hello, {}!", name)
}

#[tauri::command]
pub fn get_user(id: i32) -> Result<User, String> {
    Ok(User {
        id,
        name: "Test".to_string(),
        email: None,
    })
}

#[tauri::command]
pub fn create_user(request: CreateUserRequest) -> Result<User, String> {
    Ok(User {
        id: 1,
        name: request.name,
        email: request.email,
    })
}

#[tauri::command]
pub fn get_all_users() -> Vec<User> {
    vec![]
}

#[tauri::command]
pub fn delete_user(id: i32) -> Result<(), String> {
    Ok(())
}

