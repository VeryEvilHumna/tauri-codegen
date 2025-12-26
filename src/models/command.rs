use std::path::PathBuf;

use super::RustType;

/// Represents a parsed Tauri command
#[derive(Debug, Clone)]
pub struct TauriCommand {
    /// Name of the command (function name)
    pub name: String,
    /// Function arguments
    pub args: Vec<CommandArg>,
    /// Return type (None for functions returning ())
    pub return_type: Option<RustType>,
    /// Source file where the command was found
    pub source_file: PathBuf,
}

/// Represents a function argument
#[derive(Debug, Clone)]
pub struct CommandArg {
    /// Argument name
    pub name: String,
    /// Argument type
    pub ty: RustType,
}

