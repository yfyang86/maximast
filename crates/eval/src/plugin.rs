//! Dynamic Rust plugin loading.
//!
//! A plugin is a `cdylib` built against this crate that exports two
//! `#[no_mangle] extern "C"` symbols:
//!
//! ```ignore
//! #[no_mangle]
//! pub extern "C" fn maxima_plugin_abi() -> *const std::os::raw::c_char {
//!     maxima_eval::plugin::MAXIMA_PLUGIN_ABI_CSTR.as_ptr()
//! }
//!
//! #[no_mangle]
//! pub extern "C" fn maxima_plugin_register(env: &mut maxima_eval::Environment) {
//!     env.register_native("plugin_double", plugin_double, 1, Some(1));
//! }
//! ```
//!
//! The boundary uses the **Rust ABI** (we pass `&mut Environment`), which is
//! only stable when the plugin and host are built with the same toolchain and
//! the same version of this crate. [`MAXIMA_PLUGIN_ABI`] encodes both; the
//! host refuses to call a plugin whose string does not match, turning an
//! otherwise-undefined situation into a clean error.

use std::ffi::CStr;
use std::os::raw::c_char;
use maxima_core::Expr;
use crate::env::Environment;

/// Run a native-function body inside the plugin's own panic runtime so a
/// panic can never unwind across the dynamic-library boundary. This is
/// mandatory: the host cannot catch a "foreign exception" thrown by a
/// separately-compiled `.so` — it would abort the whole process. On panic
/// this returns the noun form `name(args...)`.
///
/// Plugin authors should wrap every registered function body in this:
/// ```ignore
/// fn my_fn(args: &[Expr], _env: &mut Environment) -> Expr {
///     maxima_eval::plugin::guard("my_fn", args, || { /* real work */ })
/// }
/// ```
pub fn guard<F: FnOnce() -> Expr>(name: &str, args: &[Expr], body: F) -> Expr {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)) {
        Ok(result) => result,
        Err(_) => {
            eprintln!("plugin: native function `{}` panicked; returning noun form", name);
            Expr::call(name, args.to_vec())
        }
    }
}

/// ABI/version descriptor. Encodes the plugin-API revision, this crate's
/// version, and the rustc version (captured by `build.rs`). Host and plugin
/// observe the same value because they compile the same crate; a mismatch
/// means the plugin is binary-incompatible and must not be invoked.
pub const MAXIMA_PLUGIN_ABI: &str = concat!(
    "maxima-plugin-abi/1;core=",
    env!("CARGO_PKG_VERSION"),
    ";rustc=",
    env!("MAXIMA_RUSTC_VERSION"),
);

/// Null-terminated form of [`MAXIMA_PLUGIN_ABI`] for plugins to return from
/// their `maxima_plugin_abi` export.
pub const MAXIMA_PLUGIN_ABI_CSTR: &CStr = unsafe {
    // Built from the same const with an appended NUL, so it is valid UTF-8
    // with no interior NULs.
    CStr::from_bytes_with_nul_unchecked(
        concat!(
            "maxima-plugin-abi/1;core=",
            env!("CARGO_PKG_VERSION"),
            ";rustc=",
            env!("MAXIMA_RUSTC_VERSION"),
            "\0"
        )
        .as_bytes(),
    )
};

const ABI_SYMBOL: &[u8] = b"maxima_plugin_abi";
const REGISTER_SYMBOL: &[u8] = b"maxima_plugin_register";
const SET_INTERNER_SYMBOL: &[u8] = b"maxima_plugin_set_interner";

type AbiFn = unsafe extern "C" fn() -> *const c_char;
type RegisterFn = unsafe extern "C" fn(&mut Environment);
type SetInternerFn = unsafe extern "C" fn(*mut std::sync::Mutex<maxima_core::InternTable>);

/// Resolve a plugin path: try the name as given, then with the platform's
/// dynamic-library extension appended, searching (in order) the directories in
/// the `MAXIMA_PLUGIN_PATH` env var (`:`-separated) and the env's search paths.
fn resolve_plugin_path(name: &str, env: &Environment) -> Option<String> {
    let ext = std::env::consts::DLL_EXTENSION; // "so" | "dylib" | "dll"
    let with_ext = |p: &str| {
        if std::path::Path::new(p).extension().is_some() {
            vec![p.to_string()]
        } else {
            vec![p.to_string(), format!("{}.{}", p, ext)]
        }
    };

    // A bare plugin name like "specfun" also resolves to the cdylib output name
    // `libmaxima_specfun.<ext>` in the usual build/search directories.
    let lib_name = format!("libmaxima_{}.{}", name, ext);
    let mut candidates = Vec::new();
    if name.starts_with('/') || name.starts_with("./") || name.starts_with("../") {
        candidates.extend(with_ext(name));
    } else {
        candidates.extend(with_ext(name));
        for d in ["target/release", "target/debug", "."] {
            candidates.push(format!("{}/{}", d, lib_name));
        }
        if let Ok(paths) = std::env::var("MAXIMA_PLUGIN_PATH") {
            for dir in paths.split(':').filter(|d| !d.is_empty()) {
                candidates.extend(with_ext(&format!("{}/{}", dir, name)));
                candidates.push(format!("{}/{}", dir, lib_name));
            }
        }
        for dir in &env.search_paths {
            candidates.extend(with_ext(&format!("{}/{}", dir, name)));
            candidates.push(format!("{}/{}", dir, lib_name));
        }
    }
    candidates.into_iter().find(|p| std::path::Path::new(p).is_file())
}

/// Load a plugin, validate its ABI, run its registration entry point, and
/// keep the library alive for the session. Returns the resolved path on
/// success or a human-readable error.
pub fn load_plugin(name: &str, env: &mut Environment) -> Result<String, String> {
    let path = resolve_plugin_path(name, env)
        .ok_or_else(|| format!("plugin not found: {}", name))?;

    let canonical = std::fs::canonicalize(&path)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.clone());

    if env.loaded_plugin_paths.iter().any(|p| p == &canonical) {
        return Ok(canonical); // idempotent: already loaded
    }

    // SAFETY: loading arbitrary native code is inherently unsafe; the ABI
    // check below is our guard against calling a binary-incompatible plugin.
    let lib = unsafe { libloading::Library::new(&path) }
        .map_err(|e| format!("cannot open plugin {}: {}", path, e))?;

    // 1. Validate the ABI string before trusting anything else in the library.
    let plugin_abi = unsafe {
        let f: libloading::Symbol<AbiFn> = lib
            .get(ABI_SYMBOL)
            .map_err(|_| format!("plugin {} missing `maxima_plugin_abi` symbol", path))?;
        let ptr = f();
        if ptr.is_null() {
            return Err(format!("plugin {} returned a null ABI string", path));
        }
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    };
    if plugin_abi != MAXIMA_PLUGIN_ABI {
        return Err(format!(
            "plugin ABI mismatch for {}:\n  plugin: {}\n  host:   {}",
            path, plugin_abi, MAXIMA_PLUGIN_ABI
        ));
    }

    // 2. Hand the plugin our symbol interner so symbols (in arguments and in
    //    results) are consistent across the boundary. Optional: a numeric-only
    //    plugin may omit it, but then any symbolic Expr it produces or reads
    //    would be mis-resolved, so we warn.
    unsafe {
        match lib.get::<SetInternerFn>(SET_INTERNER_SYMBOL) {
            Ok(set_interner) => set_interner(maxima_core::interner_ptr()),
            Err(_) => eprintln!(
                "load_plugin: warning: {} does not share the host interner; \
                 symbolic results from it may be incorrect",
                path
            ),
        }
    }

    // 3. Run the registration entry point.
    unsafe {
        let register: libloading::Symbol<RegisterFn> = lib
            .get(REGISTER_SYMBOL)
            .map_err(|_| format!("plugin {} missing `maxima_plugin_register` symbol", path))?;
        register(env);
    }

    // 4. Keep the library alive — its function pointers are now registered.
    env.loaded_plugins.push(lib);
    env.loaded_plugin_paths.push(canonical.clone());
    Ok(canonical)
}
