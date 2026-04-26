//! Module loader: walks a parsed program's `eca` directives, finds the
//! source files, parses them, and merges everything into one flat
//! `Program` for sema/codegen.
//!
//! Resolution order for `eca name`:
//!   1. `$MOTT_STDLIB/<name>.mott` — environment override.
//!   2. `<CARGO_MANIFEST_DIR>/stdlib/<name>.mott` — dev builds.
//!   3. `<importing_file_dir>/<name>.mott` — local user modules.
//!
//! Imports flatten into the AST: each `Function`/`StructDef` from a
//! loaded module gets `module = Some(name)` set. Sema later uses that
//! to validate qualified calls (`math.sqrt(...)` requires both that
//! `math` was imported and that `sqrt` is a function inside `math`).
//!
//! Cycles are caught with a "currently visiting" set: re-entering a
//! module before it's fully loaded means a cycle, which we reject.
//! For now we don't allow them at all; structural recursion via arrays
//! is fine inside a module but not across the import graph.

use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};

use crate::ast::{Item, Program};
use crate::error::{Error, Result};
use crate::lexer::Lexer;
use crate::parser::Parser;

/// Resolve and load all imports transitively starting from `program`,
/// which was parsed from `source_path`. Returns one merged Program with
/// `Item::Import` entries removed (replaced by their module's items
/// tagged with `module = Some(name)`).
pub fn load_imports(program: Program, source_path: &Path) -> Result<Program> {
    let mut loader = Loader {
        loaded: HashSet::new(),
        visiting: HashSet::new(),
        source_dir: source_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".")),
    };
    loader.process(program, None)
}

struct Loader {
    loaded: HashSet<String>,
    visiting: HashSet<String>,
    /// Directory of the entry-point source file. User-relative imports
    /// resolve from here. (Not the directory of the importing module —
    /// that's a deliberate simplification: stdlib lookups are absolute,
    /// user modules are flat next to the entry file.)
    source_dir: PathBuf,
}

impl Loader {
    /// Walk `program`'s items, replacing each `Item::Import` with the
    /// items of the imported module (recursively). `current_module` is
    /// `Some(name)` when we're processing items from a loaded module
    /// — we tag them with that module name so sema can resolve qualified
    /// references back to it.
    fn process(
        &mut self,
        program: Program,
        current_module: Option<&str>,
    ) -> Result<Program> {
        let mut out: Vec<Item> = Vec::new();
        for item in program.items {
            match item {
                Item::Import { module } => {
                    // Imports inside imported modules are also followed,
                    // but we keep the original module-tag of *this*
                    // file's items (the inner module's items get tagged
                    // with the inner module's name).
                    if self.loaded.contains(&module) {
                        // Already merged — skip; sema dedups by qualified
                        // name. Re-imports from multiple files of the
                        // same module are common and harmless.
                        continue;
                    }
                    if self.visiting.contains(&module) {
                        return Err(Error::Sema(format!(
                            "import cycle detected at module `{}`",
                            module
                        )));
                    }
                    self.visiting.insert(module.clone());
                    let path = self.resolve_module(&module)?;
                    let source = std::fs::read_to_string(&path).map_err(|e| {
                        Error::Sema(format!(
                            "cannot read module `{}` at {}: {}",
                            module,
                            path.display(),
                            e
                        ))
                    })?;
                    let tokens = Lexer::new(&source).tokenize()?;
                    let mod_program = Parser::new(tokens).parse()?;
                    let merged = self.process(mod_program, Some(&module))?;
                    out.extend(merged.items);
                    self.visiting.remove(&module);
                    self.loaded.insert(module);
                }
                Item::Function(mut f) => {
                    if f.module.is_none() {
                        f.module = current_module.map(|s| s.to_string());
                    }
                    out.push(Item::Function(f));
                }
                Item::Struct(mut s) => {
                    if s.module.is_none() {
                        s.module = current_module.map(|s| s.to_string());
                    }
                    out.push(Item::Struct(s));
                }
            }
        }
        Ok(Program { items: out })
    }

    #[cfg(test)]
    pub(crate) fn resolve_for_test(&self, name: &str) -> Result<PathBuf> {
        self.resolve_module(name)
    }

    fn resolve_module(&self, name: &str) -> Result<PathBuf> {
        // 1. Env override — handy for tests and alternative installations.
        if let Ok(p) = env::var("MOTT_STDLIB") {
            let candidate = PathBuf::from(p).join(format!("{}.mott", name));
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
        // 2. Dev path — works when running the compiler from the repo.
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let dev = Path::new(manifest_dir)
            .join("stdlib")
            .join(format!("{}.mott", name));
        if dev.is_file() {
            return Ok(dev);
        }
        // 3. User-local module relative to the entry source file.
        let local = self.source_dir.join(format!("{}.mott", name));
        if local.is_file() {
            return Ok(local);
        }
        Err(Error::Sema(format!(
            "module `{}` not found (looked in $MOTT_STDLIB, dev stdlib, and {})",
            name,
            self.source_dir.display()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_stdlib_math() {
        // Dev path: <CARGO_MANIFEST_DIR>/stdlib/math.mott. Our compiler
        // ships this file, so resolution should succeed regardless of
        // env or cwd.
        let loader = Loader {
            loaded: HashSet::new(),
            visiting: HashSet::new(),
            source_dir: PathBuf::from("."),
        };
        let path = loader.resolve_for_test("math").expect("math should resolve");
        assert!(path.ends_with("math.mott"));
    }

    #[test]
    fn rejects_unknown_module() {
        let loader = Loader {
            loaded: HashSet::new(),
            visiting: HashSet::new(),
            source_dir: PathBuf::from("/tmp"),
        };
        let err = loader
            .resolve_for_test("definitely_not_a_real_module_xyz")
            .unwrap_err();
        assert!(format!("{}", err).contains("not found"));
    }

    #[test]
    fn loads_math_stdlib_end_to_end() {
        // Drive the full loader against a Program that imports math.
        // We don't have a real source path, but `source_dir` is just for
        // user-local fallback — stdlib resolves via CARGO_MANIFEST_DIR.
        let src = "eca math\nfnc kort() {}\n";
        let tokens = Lexer::new(src).tokenize().unwrap();
        let prog = Parser::new(tokens).parse().unwrap();
        let merged = load_imports(prog, &PathBuf::from("kort.mott")).unwrap();
        // Should have at least one math-tagged extern function
        // (sqrt, pow, etc.) plus the user's kort.
        let math_funcs: Vec<&str> = merged
            .items
            .iter()
            .filter_map(|i| {
                if let Item::Function(f) = i {
                    if f.module.as_deref() == Some("math") {
                        return Some(f.name.as_str());
                    }
                }
                None
            })
            .collect();
        assert!(math_funcs.contains(&"sqrt"), "got: {:?}", math_funcs);
        assert!(math_funcs.contains(&"pi"), "got: {:?}", math_funcs);
    }
}
