//! Module resolver - resolves types based on imports and module structure

use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use syn::{Item, UseTree};

/// Represents a parsed file with its imports and local types
#[derive(Debug, Default)]
pub struct FileScope {
    /// Module path (e.g., ["crate", "commands"] for src/commands.rs)
    pub module_path: Vec<String>,
    /// Types defined locally in this file (name -> kind)
    pub local_types: HashMap<String, TypeKind>,
    /// Imports: local name -> full path
    pub imports: HashMap<String, ImportedType>,
    /// Wildcard imports (use something::*)
    pub wildcard_imports: Vec<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    Struct,
    Enum,
}

#[derive(Debug, Clone)]
pub struct ImportedType {
    /// Full module path (e.g., ["crate", "internal", "UserRole"])
    pub path: Vec<String>,
}

/// Module resolver that tracks all files and their scopes
#[derive(Debug, Default)]
pub struct ModuleResolver {
    /// File path -> FileScope
    pub files: HashMap<PathBuf, FileScope>,
    /// Type name -> list of files that define it
    pub type_locations: HashMap<String, Vec<PathBuf>>,
    /// Module path -> file path (e.g., ["crate", "internal"] -> src/internal.rs)
    pub module_to_file: HashMap<Vec<String>, PathBuf>,
}

impl ModuleResolver {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse a file and extract its scope (imports, local types, submodules)
    pub fn parse_file(&mut self, path: &Path, content: &str, base_path: &Path) -> Result<()> {
        let syntax = syn::parse_file(content)?;

        let mut scope = FileScope {
            module_path: self.path_to_module(path, base_path),
            ..Default::default()
        };

        for item in &syntax.items {
            match item {
                Item::Use(item_use) => {
                    self.parse_use_tree(&item_use.tree, &mut scope, Vec::new());
                }
                Item::Struct(s) => {
                    let name = s.ident.to_string();
                    scope.local_types.insert(name.clone(), TypeKind::Struct);
                    self.type_locations
                        .entry(name)
                        .or_default()
                        .push(path.to_path_buf());
                }
                Item::Enum(e) => {
                    let name = e.ident.to_string();
                    scope.local_types.insert(name.clone(), TypeKind::Enum);
                    self.type_locations
                        .entry(name)
                        .or_default()
                        .push(path.to_path_buf());
                }
                _ => {}
            }
        }

        self.module_to_file
            .insert(scope.module_path.clone(), path.to_path_buf());
        self.files.insert(path.to_path_buf(), scope);

        Ok(())
    }

    /// Parse use tree recursively
    fn parse_use_tree(&self, tree: &UseTree, scope: &mut FileScope, mut prefix: Vec<String>) {
        match tree {
            UseTree::Path(path) => {
                prefix.push(path.ident.to_string());
                self.parse_use_tree(&path.tree, scope, prefix);
            }
            UseTree::Name(name) => {
                let type_name = name.ident.to_string();
                prefix.push(type_name.clone());
                scope
                    .imports
                    .insert(type_name, ImportedType { path: prefix });
            }
            UseTree::Rename(rename) => {
                let original_name = rename.ident.to_string();
                let alias = rename.rename.to_string();
                prefix.push(original_name);
                scope.imports.insert(alias, ImportedType { path: prefix });
            }
            UseTree::Glob(_) => {
                scope.wildcard_imports.push(prefix);
            }
            UseTree::Group(group) => {
                for item in &group.items {
                    self.parse_use_tree(item, scope, prefix.clone());
                }
            }
        }
    }

    /// Convert file path to module path
    fn path_to_module(&self, path: &Path, base_path: &Path) -> Vec<String> {
        let relative = path.strip_prefix(base_path).unwrap_or(path);
        let mut parts: Vec<String> = vec!["crate".to_string()];

        for component in relative.components() {
            if let std::path::Component::Normal(s) = component {
                let s = s.to_string_lossy();
                if s == "mod.rs" || s == "lib.rs" || s == "main.rs" {
                    continue;
                }
                let name = s.trim_end_matches(".rs");
                parts.push(name.to_string());
            }
        }

        parts
    }

    /// Resolve a type name in the context of a specific file
    pub fn resolve_type(&self, type_name: &str, from_file: &Path) -> Option<PathBuf> {
        let scope = self.files.get(from_file)?;

        // 1. Check if it's a local type
        if scope.local_types.contains_key(type_name) {
            return Some(from_file.to_path_buf());
        }

        // 2. Check explicit imports
        if let Some(imported) = scope.imports.get(type_name) {
            return self.find_type_by_module_path(&imported.path);
        }

        // 3. Check wildcard imports
        for wildcard_path in &scope.wildcard_imports {
            if let Some(file) = self.find_type_in_module(type_name, wildcard_path) {
                return Some(file);
            }
        }

        // 4. Fallback: find any file that defines this type
        if let Some(locations) = self.type_locations.get(type_name) {
            if locations.len() == 1 {
                return Some(locations[0].clone());
            }
            let from_module = &scope.module_path;
            for loc in locations {
                if let Some(loc_scope) = self.files.get(loc) {
                    if loc_scope.module_path.len() >= 2
                        && from_module.len() >= 2
                        && loc_scope.module_path[..loc_scope.module_path.len() - 1]
                            == from_module[..from_module.len() - 1]
                    {
                        return Some(loc.clone());
                    }
                }
            }
            return Some(locations[0].clone());
        }

        None
    }

    /// Find file by module path
    fn find_type_by_module_path(&self, module_path: &[String]) -> Option<PathBuf> {
        if module_path.len() < 2 {
            return None;
        }
        let type_name = &module_path[module_path.len() - 1];
        let mod_path = &module_path[..module_path.len() - 1];

        if let Some(file_path) = self.module_to_file.get(mod_path) {
            if let Some(scope) = self.files.get(file_path) {
                if scope.local_types.contains_key(type_name) {
                    return Some(file_path.clone());
                }
            }
        }

        None
    }

    /// Find type in module (for wildcard imports)
    fn find_type_in_module(&self, type_name: &str, module_path: &[String]) -> Option<PathBuf> {
        if let Some(file_path) = self.module_to_file.get(module_path) {
            if let Some(scope) = self.files.get(file_path) {
                if scope.local_types.contains_key(type_name) {
                    return Some(file_path.clone());
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_path() -> PathBuf {
        PathBuf::from("src")
    }

    #[test]
    fn test_resolve_local_type() {
        let mut resolver = ModuleResolver::new();

        let code = r#"
            #[derive(Serialize)]
            pub struct User {
                pub id: i32,
            }
        "#;

        let file_path = PathBuf::from("src/types.rs");
        resolver.parse_file(&file_path, code, &base_path()).unwrap();

        let resolved = resolver.resolve_type("User", &file_path);
        assert_eq!(resolved, Some(file_path));
    }

    #[test]
    fn test_resolve_imported_type() {
        let mut resolver = ModuleResolver::new();

        // First file defines the type
        let types_code = r#"
            pub struct User {
                pub id: i32,
            }
        "#;
        let types_path = PathBuf::from("src/types.rs");
        resolver
            .parse_file(&types_path, types_code, &base_path())
            .unwrap();

        // Second file imports and uses it
        let commands_code = r#"
            use crate::types::User;

            fn get_user() -> User {
                unimplemented!()
            }
        "#;
        let commands_path = PathBuf::from("src/commands.rs");
        resolver
            .parse_file(&commands_path, commands_code, &base_path())
            .unwrap();

        let resolved = resolver.resolve_type("User", &commands_path);
        assert_eq!(resolved, Some(types_path));
    }

    #[test]
    fn test_resolve_renamed_import() {
        let mut resolver = ModuleResolver::new();

        let types_code = r#"
            pub struct User {
                pub id: i32,
            }
        "#;
        let types_path = PathBuf::from("src/types.rs");
        resolver
            .parse_file(&types_path, types_code, &base_path())
            .unwrap();

        let commands_code = r#"
            use crate::types::User as MyUser;
        "#;
        let commands_path = PathBuf::from("src/commands.rs");
        resolver
            .parse_file(&commands_path, commands_code, &base_path())
            .unwrap();

        // Should resolve the renamed type
        let resolved = resolver.resolve_type("MyUser", &commands_path);
        assert_eq!(resolved, Some(types_path));
    }

    #[test]
    fn test_resolve_wildcard_import() {
        let mut resolver = ModuleResolver::new();

        let types_code = r#"
            pub struct User {
                pub id: i32,
            }
            pub struct Item {
                pub name: String,
            }
        "#;
        let types_path = PathBuf::from("src/types.rs");
        resolver
            .parse_file(&types_path, types_code, &base_path())
            .unwrap();

        let commands_code = r#"
            use crate::types::*;
        "#;
        let commands_path = PathBuf::from("src/commands.rs");
        resolver
            .parse_file(&commands_path, commands_code, &base_path())
            .unwrap();

        let resolved = resolver.resolve_type("User", &commands_path);
        assert_eq!(resolved, Some(types_path.clone()));

        let resolved = resolver.resolve_type("Item", &commands_path);
        assert_eq!(resolved, Some(types_path));
    }

    #[test]
    fn test_resolve_type_fallback_single_location() {
        let mut resolver = ModuleResolver::new();

        let types_code = r#"
            pub struct User {
                pub id: i32,
            }
        "#;
        let types_path = PathBuf::from("src/types.rs");
        resolver
            .parse_file(&types_path, types_code, &base_path())
            .unwrap();

        // Commands file doesn't import User explicitly
        let commands_code = r#"
            fn some_function() {}
        "#;
        let commands_path = PathBuf::from("src/commands.rs");
        resolver
            .parse_file(&commands_path, commands_code, &base_path())
            .unwrap();

        // Should still find User via fallback (only one location)
        let resolved = resolver.resolve_type("User", &commands_path);
        assert_eq!(resolved, Some(types_path));
    }

    #[test]
    fn test_path_to_module_simple() {
        let resolver = ModuleResolver::new();

        let path = PathBuf::from("src/types.rs");
        let module = resolver.path_to_module(&path, &base_path());

        assert_eq!(module, vec!["crate", "types"]);
    }

    #[test]
    fn test_path_to_module_nested() {
        let resolver = ModuleResolver::new();

        let path = PathBuf::from("src/api/commands.rs");
        let module = resolver.path_to_module(&path, &base_path());

        assert_eq!(module, vec!["crate", "api", "commands"]);
    }

    #[test]
    fn test_path_to_module_mod_rs() {
        let resolver = ModuleResolver::new();

        let path = PathBuf::from("src/api/mod.rs");
        let module = resolver.path_to_module(&path, &base_path());

        assert_eq!(module, vec!["crate", "api"]);
    }

    #[test]
    fn test_path_to_module_main_rs() {
        let resolver = ModuleResolver::new();

        let path = PathBuf::from("src/main.rs");
        let module = resolver.path_to_module(&path, &base_path());

        assert_eq!(module, vec!["crate"]);
    }

    #[test]
    fn test_parse_use_tree_simple() {
        let mut resolver = ModuleResolver::new();

        let code = r#"
            use crate::types::User;
            use crate::models::Item;
        "#;

        let path = PathBuf::from("src/commands.rs");
        resolver.parse_file(&path, code, &base_path()).unwrap();

        let scope = resolver.files.get(&path).unwrap();
        assert!(scope.imports.contains_key("User"));
        assert!(scope.imports.contains_key("Item"));
    }

    #[test]
    fn test_parse_use_tree_grouped() {
        let mut resolver = ModuleResolver::new();

        let code = r#"
            use crate::types::{User, Item, Status};
        "#;

        let path = PathBuf::from("src/commands.rs");
        resolver.parse_file(&path, code, &base_path()).unwrap();

        let scope = resolver.files.get(&path).unwrap();
        assert!(scope.imports.contains_key("User"));
        assert!(scope.imports.contains_key("Item"));
        assert!(scope.imports.contains_key("Status"));
    }

    #[test]
    fn test_type_locations_multiple_files() {
        let mut resolver = ModuleResolver::new();

        let code1 = r#"
            pub struct Data { pub value: i32 }
        "#;
        let path1 = PathBuf::from("src/a.rs");
        resolver.parse_file(&path1, code1, &base_path()).unwrap();

        let code2 = r#"
            pub struct Data { pub name: String }
        "#;
        let path2 = PathBuf::from("src/b.rs");
        resolver.parse_file(&path2, code2, &base_path()).unwrap();

        // Both should be recorded
        let locations = resolver.type_locations.get("Data").unwrap();
        assert_eq!(locations.len(), 2);
        assert!(locations.contains(&path1));
        assert!(locations.contains(&path2));
    }

    #[test]
    fn test_parse_struct_and_enum() {
        let mut resolver = ModuleResolver::new();

        let code = r#"
            pub struct User { pub id: i32 }
            pub enum Status { Active, Inactive }
        "#;

        let path = PathBuf::from("src/types.rs");
        resolver.parse_file(&path, code, &base_path()).unwrap();

        let scope = resolver.files.get(&path).unwrap();
        assert_eq!(scope.local_types.get("User"), Some(&TypeKind::Struct));
        assert_eq!(scope.local_types.get("Status"), Some(&TypeKind::Enum));
    }
}
