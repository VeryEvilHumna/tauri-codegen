// Fixture: Nested modules for testing module resolution

use serde::{Deserialize, Serialize};

pub mod types {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub struct InnerType {
        pub value: i32,
        pub label: String,
    }

    #[derive(Serialize)]
    pub enum InnerEnum {
        First,
        Second,
        Third,
    }
}

pub mod commands {
    use super::types::InnerType;

    #[tauri::command]
    pub fn get_inner() -> InnerType {
        InnerType {
            value: 42,
            label: "test".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct OuterType {
    pub inner: types::InnerType,
    pub status: types::InnerEnum,
}

#[tauri::command]
pub fn get_outer() -> OuterType {
    OuterType {
        inner: types::InnerType {
            value: 1,
            label: "outer".to_string(),
        },
        status: types::InnerEnum::First,
    }
}

