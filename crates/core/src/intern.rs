use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::sync::atomic::{AtomicPtr, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(u32);

#[doc(hidden)]
pub struct InternTable {
    to_id: HashMap<String, SymbolId>,
    to_name: Vec<String>,
}

// Each dynamically loaded object (the host binary and every plugin `.so`)
// gets its own copy of this `static`. To keep symbols consistent across the
// plugin boundary, a plugin "adopts" the host's table at load time via
// `adopt_interner`, after which `table()` returns the host's instance for
// both. Without adoption, each object uses its own LOCAL table.
static LOCAL: LazyLock<Mutex<InternTable>> = LazyLock::new(|| {
    Mutex::new(InternTable {
        to_id: HashMap::new(),
        to_name: Vec::new(),
    })
});

static SHARED: AtomicPtr<Mutex<InternTable>> = AtomicPtr::new(std::ptr::null_mut());

#[inline]
fn table() -> &'static Mutex<InternTable> {
    let p = SHARED.load(Ordering::Acquire);
    if p.is_null() {
        LazyLock::force(&LOCAL)
    } else {
        // SAFETY: `p` was set by `adopt_interner` from another object's
        // `interner_ptr()`, which points at that object's `'static` LOCAL
        // table — valid for the rest of the process.
        unsafe { &*p }
    }
}

/// Pointer to this object's own interner table. The host passes this to a
/// plugin so the plugin can `adopt_interner` it and share one symbol table.
pub fn interner_ptr() -> *mut Mutex<InternTable> {
    LazyLock::force(&LOCAL) as *const Mutex<InternTable> as *mut Mutex<InternTable>
}

/// Adopt another object's interner table as this object's symbol table.
/// Call once, at plugin load, before any interning in this object.
///
/// # Safety
/// `ptr` must come from `interner_ptr()` of a host/object that outlives this
/// one (it lives for the whole process), and must not be null.
pub unsafe fn adopt_interner(ptr: *mut Mutex<InternTable>) {
    SHARED.store(ptr, Ordering::Release);
}

pub fn intern(name: &str) -> SymbolId {
    let mut table = table().lock().unwrap();
    if let Some(&id) = table.to_id.get(name) {
        return id;
    }
    let id = SymbolId(table.to_name.len() as u32);
    table.to_name.push(name.to_string());
    table.to_id.insert(name.to_string(), id);
    id
}

pub fn resolve(id: SymbolId) -> String {
    let table = table().lock().unwrap();
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
