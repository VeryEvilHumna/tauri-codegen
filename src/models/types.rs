use std::path::PathBuf;

use super::RustType;

/// Represents a parsed Rust struct
#[derive(Debug, Clone)]
pub struct RustStruct {
    /// Name of the struct
    pub name: String,
    /// Generic type parameters (e.g., ["T", "U"])
    pub generics: Vec<String>,
    /// Struct fields
    pub fields: Vec<StructField>,
    /// Source file where the struct was found
    pub source_file: PathBuf,
}

/// Represents a struct field
#[derive(Debug, Clone)]
pub struct StructField {
    /// Field name
    pub name: String,
    /// Field type
    pub ty: RustType,
}

/// Represents a parsed Rust enum
#[derive(Debug, Clone)]
pub struct RustEnum {
    /// Name of the enum
    pub name: String,
    /// Enum variants
    pub variants: Vec<EnumVariant>,
    /// Source file where the enum was found
    pub source_file: PathBuf,
}

/// Represents an enum variant
#[derive(Debug, Clone)]
pub struct EnumVariant {
    /// Variant name
    pub name: String,
    /// Variant data (for tuple/struct variants)
    pub data: VariantData,
}

/// Represents the data associated with an enum variant
#[derive(Debug, Clone)]
pub enum VariantData {
    /// Unit variant (no data)
    Unit,
    /// Tuple variant with types
    Tuple(Vec<RustType>),
    /// Struct variant with named fields
    Struct(Vec<StructField>),
}

