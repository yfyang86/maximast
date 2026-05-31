use std::collections::{HashMap, HashSet};

use maxima_core::{Expr, SymbolId};
use crate::assume::AssumptionDB;
use crate::pattern::PatternState;

#[derive(Debug, Clone)]
pub struct FuncDef {
    pub params: Vec<SymbolId>,
    pub body: Expr,
}

/// Native function signature for Rust plugins.
/// Future: dynamically loaded .so/.dylib plugins will register functions with this type.
pub type NativeFn = fn(&[Expr], &mut Environment) -> Expr;

/// Metadata for a registered native function
#[derive(Clone)]
pub struct NativeFuncDef {
    pub func: NativeFn,
    pub min_args: usize,
    pub max_args: Option<usize>,
}

/// Key for subscripted function definitions like t[n](x)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubscriptKey {
    pub name: SymbolId,
    pub indices: Vec<String>,
}

pub struct Environment {
    /// Variable bindings stack (dynamic scoping)
    scopes: Vec<HashMap<SymbolId, Expr>>,
    /// Maxima-defined function definitions (from f(x):=...)
    pub functions: HashMap<SymbolId, FuncDef>,
    /// Native (Rust plugin) function definitions, keyed by name string.
    /// Strings (not interned SymbolIds) are the key because a dynamically
    /// loaded plugin has its own copy of the symbol interner — its SymbolIds
    /// would not match the host's. The name string is the stable identity
    /// across the plugin boundary.
    pub native_functions: HashMap<String, NativeFuncDef>,
    /// Subscripted function definitions: t[0](x):=1, t[n](x):=...
    pub subscript_fns: HashMap<SubscriptKey, FuncDef>,
    /// Generic subscripted function definitions (symbolic index): t[n](x):=...
    pub subscript_generic_fns: HashMap<SymbolId, (Vec<SymbolId>, FuncDef)>,
    /// Array/subscript values: a[1]:5
    pub array_values: HashMap<(SymbolId, Vec<String>), Expr>,
    /// Input number base (default 10)
    pub ibase: i64,
    /// Output number base (default 10)
    pub obase: i64,
    /// Input/output label counter
    pub label_count: usize,
    /// Stored output labels: %o1, %o2, ...
    pub outputs: Vec<Expr>,
    /// Assumption database
    pub assumptions: AssumptionDB,
    /// Files that have been loaded (canonical paths)
    pub loaded_files: HashSet<String>,
    /// Path of the file currently being loaded (for nested loads)
    pub load_pathname: Option<String>,
    /// Directories to search for .mac files
    pub search_paths: Vec<String>,
    /// Autoload registry: function name → file to load on first call
    pub autoload_registry: HashMap<SymbolId, String>,
    /// Pattern matching state (matchdeclare, defrule, tellsimp)
    pub pattern_state: PatternState,
    /// Dynamically loaded plugin libraries, kept alive for the whole session.
    /// Dropping a `Library` unloads the `.so` and turns every function pointer
    /// it registered into a dangling pointer, so these must never be dropped
    /// while the session runs. Parallel to `loaded_plugin_paths`.
    pub loaded_plugins: Vec<libloading::Library>,
    /// Resolved paths of loaded plugins, for introspection and dedup.
    pub loaded_plugin_paths: Vec<String>,
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            scopes: vec![HashMap::new()],
            functions: HashMap::new(),
            native_functions: HashMap::new(),
            subscript_fns: HashMap::new(),
            subscript_generic_fns: HashMap::new(),
            array_values: HashMap::new(),
            ibase: 10,
            obase: 10,
            label_count: 0,
            outputs: Vec::new(),
            assumptions: AssumptionDB::new(),
            loaded_files: HashSet::new(),
            load_pathname: None,
            search_paths: vec![".".to_string()],
            autoload_registry: HashMap::new(),
            pattern_state: PatternState::default(),
            loaded_plugins: Vec::new(),
            loaded_plugin_paths: Vec::new(),
        }
    }

    pub fn get(&self, sym: SymbolId) -> Option<&Expr> {
        for scope in self.scopes.iter().rev() {
            if let Some(val) = scope.get(&sym) {
                return Some(val);
            }
        }
        None
    }

    pub fn set(&mut self, sym: SymbolId, val: Expr) {
        // Set in the most local scope that already contains the variable,
        // or global scope if not found (matching dynamic scoping semantics)
        for scope in self.scopes.iter_mut().rev() {
            if let std::collections::hash_map::Entry::Occupied(mut e) = scope.entry(sym) {
                e.insert(val);
                return;
            }
        }
        self.scopes.first_mut().unwrap().insert(sym, val);
    }

    pub fn set_local(&mut self, sym: SymbolId, val: Expr) {
        self.scopes.last_mut().unwrap().insert(sym, val);
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn define_function(&mut self, name: SymbolId, def: FuncDef) {
        self.functions.insert(name, def);
    }

    pub fn kill_var(&mut self, sym: SymbolId) {
        for scope in &mut self.scopes {
            scope.remove(&sym);
        }
    }

    pub fn kill_function(&mut self, sym: SymbolId) {
        self.functions.remove(&sym);
    }

    pub fn kill_all(&mut self) {
        for scope in &mut self.scopes {
            scope.clear();
        }
        self.functions.clear();
        self.subscript_fns.clear();
        self.subscript_generic_fns.clear();
        self.array_values.clear();
        self.assumptions.clear();
        self.autoload_registry.clear();
        self.loaded_files.clear();
        // native_functions intentionally preserved across kill(all)
    }

    pub fn register_native(&mut self, name: &str, func: NativeFn, min_args: usize, max_args: Option<usize>) {
        self.native_functions.insert(name.to_string(), NativeFuncDef { func, min_args, max_args });
    }

    pub fn is_file_loaded(&self, path: &str) -> bool {
        self.loaded_files.contains(path)
    }

    pub fn mark_file_loaded(&mut self, path: String) {
        self.loaded_files.insert(path);
    }

    pub fn register_autoload(&mut self, filename: &str, func_names: &[SymbolId]) {
        for &name in func_names {
            self.autoload_registry.insert(name, filename.to_string());
        }
    }

    pub fn list_values(&self) -> Vec<SymbolId> {
        self.scopes
            .first()
            .map(|s| s.keys().copied().collect())
            .unwrap_or_default()
    }

    pub fn list_functions(&self) -> Vec<SymbolId> {
        let mut fns: Vec<SymbolId> = self.functions.keys().copied().collect();
        fns.extend(self.native_functions.keys().map(|n| maxima_core::intern(n)));
        fns
    }

    pub fn next_label(&mut self) -> usize {
        self.label_count += 1;
        self.label_count
    }

    pub fn store_output(&mut self, val: Expr) {
        self.outputs.push(val);
    }

    pub fn last_output(&self) -> Option<&Expr> {
        self.outputs.last()
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maxima_core::{Expr, intern};

    #[test]
    fn env_new_is_empty() {
        let env = Environment::new();
        assert!(env.list_values().is_empty());
        assert!(env.list_functions().is_empty());
        assert_eq!(env.label_count, 0);
        assert!(env.outputs.is_empty());
        assert_eq!(env.ibase, 10);
        assert_eq!(env.obase, 10);
    }

    #[test]
    fn env_set_get() {
        let mut env = Environment::new();
        let x = intern("x");
        env.set(x, Expr::int(42));
        assert_eq!(env.get(x), Some(&Expr::int(42)));
    }

    #[test]
    fn env_get_unset() {
        let env = Environment::new();
        let x = intern("unset_var");
        assert_eq!(env.get(x), None);
    }

    #[test]
    fn env_set_overwrite() {
        let mut env = Environment::new();
        let x = intern("ow");
        env.set(x, Expr::int(1));
        env.set(x, Expr::int(2));
        assert_eq!(env.get(x), Some(&Expr::int(2)));
    }

    #[test]
    fn env_scope_push_pop() {
        let mut env = Environment::new();
        let x = intern("sc");
        env.set(x, Expr::int(10));

        env.push_scope();
        env.set_local(x, Expr::int(20));
        assert_eq!(env.get(x), Some(&Expr::int(20)));

        env.pop_scope();
        assert_eq!(env.get(x), Some(&Expr::int(10)));
    }

    #[test]
    fn env_dynamic_scoping() {
        let mut env = Environment::new();
        let x = intern("dyn");

        env.set(x, Expr::int(1));
        env.push_scope();
        env.set_local(x, Expr::int(2));

        // set() should update the local scope since x exists there
        env.set(x, Expr::int(3));
        assert_eq!(env.get(x), Some(&Expr::int(3)));

        env.pop_scope();
        // Global should still be 1
        assert_eq!(env.get(x), Some(&Expr::int(1)));
    }

    #[test]
    fn env_set_in_global_when_not_local() {
        let mut env = Environment::new();
        let x = intern("gl");
        env.push_scope();
        env.set(x, Expr::int(5)); // no local, goes to global
        env.pop_scope();
        assert_eq!(env.get(x), Some(&Expr::int(5)));
    }

    #[test]
    fn env_kill_var() {
        let mut env = Environment::new();
        let x = intern("kv");
        env.set(x, Expr::int(1));
        env.kill_var(x);
        assert_eq!(env.get(x), None);
    }

    #[test]
    fn env_kill_function() {
        let mut env = Environment::new();
        let f = intern("kf");
        env.define_function(f, FuncDef { params: vec![], body: Expr::int(0) });
        assert!(env.functions.contains_key(&f));
        env.kill_function(f);
        assert!(!env.functions.contains_key(&f));
    }

    #[test]
    fn env_kill_all() {
        let mut env = Environment::new();
        let x = intern("ka1");
        let f = intern("ka2");
        env.set(x, Expr::int(1));
        env.define_function(f, FuncDef { params: vec![], body: Expr::int(0) });
        env.kill_all();
        assert!(env.list_values().is_empty());
        assert!(env.list_functions().is_empty());
    }

    #[test]
    fn env_labels() {
        let mut env = Environment::new();
        assert_eq!(env.next_label(), 1);
        assert_eq!(env.next_label(), 2);
        assert_eq!(env.next_label(), 3);
    }

    #[test]
    fn env_outputs() {
        let mut env = Environment::new();
        assert!(env.last_output().is_none());
        env.store_output(Expr::int(42));
        assert_eq!(env.last_output(), Some(&Expr::int(42)));
        env.store_output(Expr::int(99));
        assert_eq!(env.last_output(), Some(&Expr::int(99)));
    }

    #[test]
    fn env_list_values() {
        let mut env = Environment::new();
        let a = intern("lv_a");
        let b = intern("lv_b");
        env.set(a, Expr::int(1));
        env.set(b, Expr::int(2));
        let vals = env.list_values();
        assert_eq!(vals.len(), 2);
        assert!(vals.contains(&a));
        assert!(vals.contains(&b));
    }

    #[test]
    fn env_list_functions() {
        let mut env = Environment::new();
        let f = intern("lf_f");
        let g = intern("lf_g");
        env.define_function(f, FuncDef { params: vec![], body: Expr::int(0) });
        env.define_function(g, FuncDef { params: vec![], body: Expr::int(0) });
        let fns = env.list_functions();
        assert_eq!(fns.len(), 2);
    }

    #[test]
    fn env_pop_scope_minimum() {
        let mut env = Environment::new();
        // Popping the last scope should be a no-op
        env.pop_scope();
        assert!(env.get(intern("anything")).is_none());
    }

    #[test]
    fn env_nested_scopes() {
        let mut env = Environment::new();
        let x = intern("ns");
        env.set(x, Expr::int(1));
        env.push_scope();
        env.set_local(x, Expr::int(2));
        env.push_scope();
        env.set_local(x, Expr::int(3));
        assert_eq!(env.get(x), Some(&Expr::int(3)));
        env.pop_scope();
        assert_eq!(env.get(x), Some(&Expr::int(2)));
        env.pop_scope();
        assert_eq!(env.get(x), Some(&Expr::int(1)));
    }
}
