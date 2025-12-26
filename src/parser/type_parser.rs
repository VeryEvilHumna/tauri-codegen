use crate::models::{EnumVariant, RustEnum, RustStruct, StructField, VariantData};
use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;
use syn::{Fields, Item, ItemEnum, ItemStruct};

use super::type_extractor::{parse_type, parse_type_with_context};

/// Parse a Rust source file and extract structs and enums
pub fn parse_types(content: &str, source_file: &Path) -> Result<(Vec<RustStruct>, Vec<RustEnum>)> {
    let syntax = syn::parse_file(content)?;
    let mut structs = Vec::new();
    let mut enums = Vec::new();

    for item in syntax.items {
        match item {
            Item::Struct(item_struct) => {
                if is_serializable(&item_struct.attrs) {
                    if let Some(s) = parse_struct(&item_struct, source_file) {
                        structs.push(s);
                    }
                }
            }
            Item::Enum(item_enum) => {
                if is_serializable(&item_enum.attrs) {
                    if let Some(e) = parse_enum(&item_enum, source_file) {
                        enums.push(e);
                    }
                }
            }
            Item::Mod(module) => {
                // Also parse types inside modules
                if let Some((_, items)) = module.content {
                    for mod_item in items {
                        match mod_item {
                            Item::Struct(item_struct) => {
                                if is_serializable(&item_struct.attrs) {
                                    if let Some(s) = parse_struct(&item_struct, source_file) {
                                        structs.push(s);
                                    }
                                }
                            }
                            Item::Enum(item_enum) => {
                                if is_serializable(&item_enum.attrs) {
                                    if let Some(e) = parse_enum(&item_enum, source_file) {
                                        enums.push(e);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok((structs, enums))
}

/// Check if a type has Serialize or Deserialize derive attribute
/// This indicates the type is meant for serialization and should be exported
fn is_serializable(attrs: &[syn::Attribute]) -> bool {
    for attr in attrs {
        if let syn::Meta::List(meta_list) = &attr.meta {
            if meta_list.path.is_ident("derive") {
                let tokens = meta_list.tokens.to_string();
                if tokens.contains("Serialize") || tokens.contains("Deserialize") {
                    return true;
                }
            }
        }
    }
    false
}

/// Parse a struct into our RustStruct representation
fn parse_struct(item: &ItemStruct, source_file: &Path) -> Option<RustStruct> {
    let name = item.ident.to_string();

    // Extract generic type parameters
    let generics: Vec<String> = item
        .generics
        .params
        .iter()
        .filter_map(|param| {
            if let syn::GenericParam::Type(type_param) = param {
                Some(type_param.ident.to_string())
            } else {
                None
            }
        })
        .collect();

    // Create a set for efficient lookup when parsing field types
    let generic_params: HashSet<String> = generics.iter().cloned().collect();

    let fields = match &item.fields {
        Fields::Named(named) => named
            .named
            .iter()
            .filter_map(|field| {
                let field_name = field.ident.as_ref()?.to_string();
                let field_type = parse_type_with_context(&field.ty, &generic_params);

                // Check for serde rename attribute
                let final_name = get_serde_rename(&field.attrs).unwrap_or(field_name);

                Some(StructField {
                    name: final_name,
                    ty: field_type,
                })
            })
            .collect(),
        Fields::Unnamed(unnamed) => {
            // Tuple struct - use numbered field names
            unnamed
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, field)| StructField {
                    name: format!("field{}", i),
                    ty: parse_type_with_context(&field.ty, &generic_params),
                })
                .collect()
        }
        Fields::Unit => Vec::new(),
    };

    Some(RustStruct {
        name,
        generics,
        fields,
        source_file: source_file.to_path_buf(),
    })
}

/// Parse an enum into our RustEnum representation
fn parse_enum(item: &ItemEnum, source_file: &Path) -> Option<RustEnum> {
    let name = item.ident.to_string();

    let variants = item
        .variants
        .iter()
        .map(|variant| {
            let variant_name = variant.ident.to_string();

            // Check for serde rename attribute
            let final_name = get_serde_rename(&variant.attrs).unwrap_or(variant_name);

            let data = match &variant.fields {
                Fields::Unit => VariantData::Unit,
                Fields::Unnamed(unnamed) => {
                    let types = unnamed.unnamed.iter().map(|f| parse_type(&f.ty)).collect();
                    VariantData::Tuple(types)
                }
                Fields::Named(named) => {
                    let fields = named
                        .named
                        .iter()
                        .filter_map(|field| {
                            let field_name = field.ident.as_ref()?.to_string();
                            let final_name = get_serde_rename(&field.attrs).unwrap_or(field_name);
                            Some(StructField {
                                name: final_name,
                                ty: parse_type(&field.ty),
                            })
                        })
                        .collect();
                    VariantData::Struct(fields)
                }
            };

            EnumVariant {
                name: final_name,
                data,
            }
        })
        .collect();

    Some(RustEnum {
        name,
        variants,
        source_file: source_file.to_path_buf(),
    })
}

/// Get the serde rename value from attributes if present
fn get_serde_rename(attrs: &[syn::Attribute]) -> Option<String> {
    for attr in attrs {
        if let syn::Meta::List(meta_list) = &attr.meta {
            if meta_list.path.is_ident("serde") {
                let tokens = meta_list.tokens.to_string();
                // Look for rename = "..."
                if let Some(start) = tokens.find("rename") {
                    let rest = &tokens[start..];
                    if let Some(eq_pos) = rest.find('=') {
                        let after_eq = rest[eq_pos + 1..].trim();
                        // Extract the string value
                        if let Some(quote_start) = after_eq.find('"') {
                            let after_quote = &after_eq[quote_start + 1..];
                            if let Some(quote_end) = after_quote.find('"') {
                                return Some(after_quote[..quote_end].to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::RustType;
    use std::path::PathBuf;

    fn test_path() -> PathBuf {
        PathBuf::from("test.rs")
    }

    #[test]
    fn test_parse_simple_struct() {
        let code = r#"
            #[derive(Serialize)]
            pub struct User {
                pub id: i32,
                pub name: String,
            }
        "#;

        let (structs, enums) = parse_types(code, &test_path()).unwrap();
        assert_eq!(structs.len(), 1);
        assert_eq!(enums.len(), 0);

        let user = &structs[0];
        assert_eq!(user.name, "User");
        assert_eq!(user.fields.len(), 2);
        assert_eq!(user.fields[0].name, "id");
        assert_eq!(user.fields[1].name, "name");
    }

    #[test]
    fn test_parse_struct_with_generics() {
        let code = r#"
            #[derive(Serialize)]
            pub struct Wrapper<T> {
                pub data: T,
                pub count: i32,
            }
        "#;

        let (structs, _) = parse_types(code, &test_path()).unwrap();
        assert_eq!(structs.len(), 1);

        let wrapper = &structs[0];
        assert_eq!(wrapper.name, "Wrapper");
        assert_eq!(wrapper.generics, vec!["T"]);
        assert_eq!(wrapper.fields.len(), 2);

        // data field should be Generic(T)
        match &wrapper.fields[0].ty {
            RustType::Generic(name) => assert_eq!(name, "T"),
            other => panic!("Expected Generic(T), got {:?}", other),
        }
    }

    #[test]
    fn test_parse_struct_with_multiple_generics() {
        let code = r#"
            #[derive(Serialize, Deserialize)]
            pub struct Pair<K, V> {
                pub key: K,
                pub value: V,
            }
        "#;

        let (structs, _) = parse_types(code, &test_path()).unwrap();
        assert_eq!(structs.len(), 1);

        let pair = &structs[0];
        assert_eq!(pair.generics, vec!["K", "V"]);
    }

    #[test]
    fn test_parse_tuple_struct() {
        let code = r#"
            #[derive(Serialize)]
            pub struct Point(i32, i32);
        "#;

        let (structs, _) = parse_types(code, &test_path()).unwrap();
        assert_eq!(structs.len(), 1);

        let point = &structs[0];
        assert_eq!(point.name, "Point");
        assert_eq!(point.fields.len(), 2);
        assert_eq!(point.fields[0].name, "field0");
        assert_eq!(point.fields[1].name, "field1");
    }

    #[test]
    fn test_parse_simple_enum() {
        let code = r#"
            #[derive(Serialize)]
            pub enum Status {
                Active,
                Inactive,
                Pending,
            }
        "#;

        let (structs, enums) = parse_types(code, &test_path()).unwrap();
        assert_eq!(structs.len(), 0);
        assert_eq!(enums.len(), 1);

        let status = &enums[0];
        assert_eq!(status.name, "Status");
        assert_eq!(status.variants.len(), 3);
        assert_eq!(status.variants[0].name, "Active");
        assert_eq!(status.variants[1].name, "Inactive");
        assert_eq!(status.variants[2].name, "Pending");

        // All should be unit variants
        for variant in &status.variants {
            match &variant.data {
                VariantData::Unit => {}
                other => panic!("Expected Unit, got {:?}", other),
            }
        }
    }

    #[test]
    fn test_parse_enum_with_tuple_data() {
        let code = r#"
            #[derive(Serialize)]
            pub enum Message {
                Text(String),
                Number(i32),
                Pair(String, i32),
            }
        "#;

        let (_, enums) = parse_types(code, &test_path()).unwrap();
        assert_eq!(enums.len(), 1);

        let message = &enums[0];
        assert_eq!(message.variants.len(), 3);

        match &message.variants[0].data {
            VariantData::Tuple(types) => {
                assert_eq!(types.len(), 1);
            }
            other => panic!("Expected Tuple, got {:?}", other),
        }

        match &message.variants[2].data {
            VariantData::Tuple(types) => {
                assert_eq!(types.len(), 2);
            }
            other => panic!("Expected Tuple with 2 elements, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_enum_with_struct_variant() {
        let code = r#"
            #[derive(Serialize)]
            pub enum UserRole {
                Admin { permissions: Vec<String> },
                User,
                Guest,
            }
        "#;

        let (_, enums) = parse_types(code, &test_path()).unwrap();
        assert_eq!(enums.len(), 1);

        let role = &enums[0];

        match &role.variants[0].data {
            VariantData::Struct(fields) => {
                assert_eq!(fields.len(), 1);
                assert_eq!(fields[0].name, "permissions");
            }
            other => panic!("Expected Struct variant, got {:?}", other),
        }

        match &role.variants[1].data {
            VariantData::Unit => {}
            other => panic!("Expected Unit, got {:?}", other),
        }
    }

    #[test]
    fn test_serde_rename_field() {
        let code = r#"
            #[derive(Serialize)]
            pub struct User {
                #[serde(rename = "userId")]
                pub id: i32,
                pub name: String,
            }
        "#;

        let (structs, _) = parse_types(code, &test_path()).unwrap();
        assert_eq!(structs.len(), 1);

        let user = &structs[0];
        assert_eq!(user.fields[0].name, "userId");
        assert_eq!(user.fields[1].name, "name");
    }

    #[test]
    fn test_serde_rename_variant() {
        let code = r#"
            #[derive(Serialize)]
            pub enum Status {
                #[serde(rename = "ACTIVE")]
                Active,
                #[serde(rename = "INACTIVE")]
                Inactive,
            }
        "#;

        let (_, enums) = parse_types(code, &test_path()).unwrap();
        assert_eq!(enums.len(), 1);

        let status = &enums[0];
        assert_eq!(status.variants[0].name, "ACTIVE");
        assert_eq!(status.variants[1].name, "INACTIVE");
    }

    #[test]
    fn test_ignore_non_serializable() {
        let code = r#"
            pub struct NotExported {
                pub id: i32,
            }

            #[derive(Debug)]
            pub struct AlsoNotExported {
                pub name: String,
            }

            #[derive(Serialize)]
            pub struct Exported {
                pub data: String,
            }
        "#;

        let (structs, _) = parse_types(code, &test_path()).unwrap();
        assert_eq!(structs.len(), 1);
        assert_eq!(structs[0].name, "Exported");
    }

    #[test]
    fn test_parse_types_in_mod() {
        let code = r#"
            mod types {
                #[derive(Serialize)]
                pub struct InnerType {
                    pub value: i32,
                }

                #[derive(Deserialize)]
                pub enum InnerEnum {
                    A,
                    B,
                }
            }
        "#;

        let (structs, enums) = parse_types(code, &test_path()).unwrap();
        assert_eq!(structs.len(), 1);
        assert_eq!(enums.len(), 1);
        assert_eq!(structs[0].name, "InnerType");
        assert_eq!(enums[0].name, "InnerEnum");
    }

    #[test]
    fn test_deserialize_also_works() {
        let code = r#"
            #[derive(Deserialize)]
            pub struct Request {
                pub data: String,
            }
        "#;

        let (structs, _) = parse_types(code, &test_path()).unwrap();
        assert_eq!(structs.len(), 1);
        assert_eq!(structs[0].name, "Request");
    }

    #[test]
    fn test_source_file_is_set() {
        let code = r#"
            #[derive(Serialize)]
            pub struct Test {
                pub value: i32,
            }
        "#;

        let path = PathBuf::from("src/types.rs");
        let (structs, _) = parse_types(code, &path).unwrap();
        assert_eq!(structs.len(), 1);
        assert_eq!(structs[0].source_file, path);
    }

    #[test]
    fn test_complex_field_types() {
        let code = r#"
            #[derive(Serialize)]
            pub struct Complex {
                pub items: Vec<Item>,
                pub optional: Option<String>,
                pub map: HashMap<String, i32>,
                pub nested: Vec<Option<User>>,
            }
        "#;

        let (structs, _) = parse_types(code, &test_path()).unwrap();
        assert_eq!(structs.len(), 1);
        assert_eq!(structs[0].fields.len(), 4);

        match &structs[0].fields[0].ty {
            RustType::Vec(_) => {}
            other => panic!("Expected Vec, got {:?}", other),
        }

        match &structs[0].fields[1].ty {
            RustType::Option(_) => {}
            other => panic!("Expected Option, got {:?}", other),
        }

        match &structs[0].fields[2].ty {
            RustType::HashMap { .. } => {}
            other => panic!("Expected HashMap, got {:?}", other),
        }
    }
}

