// Fixture: Complex types for testing type generation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub struct Wrapper<T> {
    pub data: T,
    pub metadata: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
pub struct Pair<K, V> {
    pub key: K,
    pub value: V,
}

#[derive(Serialize, Deserialize)]
pub struct Pagination {
    pub page: u32,
    pub per_page: u32,
    pub total: u64,
}

#[derive(Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub pagination: Pagination,
}

#[derive(Serialize, Deserialize)]
pub struct NestedData {
    pub users: Vec<Option<User>>,
    pub tags: HashMap<String, Vec<String>>,
    pub config: Option<HashMap<String, i32>>,
}

#[derive(Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub profile: UserProfile,
}

#[derive(Serialize, Deserialize)]
pub struct UserProfile {
    pub first_name: String,
    pub last_name: String,
    pub avatar_url: Option<String>,
}

#[derive(Serialize)]
pub enum Message {
    Text(String),
    Image { url: String, width: u32, height: u32 },
    File { name: String, size: u64 },
    Reaction(String, i32),
}

#[derive(Serialize)]
pub enum ApiResponse<T> {
    Success { data: T },
    Error { code: i32, message: String },
}

#[derive(Serialize, Deserialize)]
pub struct TupleStruct(pub i32, pub String);

