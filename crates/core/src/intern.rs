use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(u32);

struct InternTable {
    to_id: HashMap<String, SymbolId>,
    to_name: Vec<String>,
}

static TABLE: LazyLock<Mutex<InternTable>> = LazyLock::new(|| {
    Mutex::new(InternTable {
        to_id: HashMap::new(),
        to_name: Vec::new(),
    })
});

pub fn intern(name: &str) -> SymbolId {
    let mut table = TABLE.lock().unwrap();
    if let Some(&id) = table.to_id.get(name) {
        return id;
    }
    let id = SymbolId(table.to_name.len() as u32);
    table.to_name.push(name.to_string());
    table.to_id.insert(name.to_string(), id);
    id
}

pub fn resolve(id: SymbolId) -> String {
    let table = TABLE.lock().unwrap();
    table.to_name[id.0 as usize].clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intern_roundtrip() {
        let id = intern("x");
        assert_eq!(resolve(id), "x");
    }

    #[test]
    fn intern_same_symbol_same_id() {
        let id1 = intern("foo");
        let id2 = intern("foo");
        assert_eq!(id1, id2);
    }

    #[test]
    fn intern_different_symbols_different_ids() {
        let a = intern("alpha");
        let b = intern("beta");
        assert_ne!(a, b);
    }
}
