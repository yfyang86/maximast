//! Authoring kit for Maxima Rust plugins.
//!
//! A plugin is a `cdylib` that exports three `extern "C"` symbols the host
//! calls at load time. Writing them by hand is error-prone (the ABI string,
//! the interner handoff, the registration signature must all be exact), so
//! this crate's [`maxima_plugin!`] macro generates them. A complete plugin:
//!
//! ```ignore
//! use maxima_plugin::{maxima_plugin, Expr, Environment, guard};
//!
//! fn my_double(args: &[Expr], _env: &mut Environment) -> Expr {
//!     // `guard` contains any panic inside the plugin's own runtime — a panic
//!     // must never unwind across the .so boundary.
//!     guard("my_double", args, || match args.first() {
//!         Some(Expr::Integer(n)) => Expr::int(n * 2),
//!         _ => Expr::call("my_double", args.to_vec()),
//!     })
//! }
//!
//! maxima_plugin!(register = |env| {
//!     env.register_native("my_double", my_double, 1, Some(1));
//! });
//! ```
//!
//! ```toml
//! # Cargo.toml
//! [lib]
//! crate-type = ["cdylib"]
//! [dependencies]
//! maxima-plugin = { path = "../../crates/plugin-sdk" }
//! ```
//!
//! Build with `cargo build -p <your-plugin>`, then from Maxima:
//! `load_plugin("target/debug/libyour_plugin")`.
//!
//! **The plugin must be built from this workspace with the same toolchain as
//! the host.** The boundary uses the Rust ABI, which is not stable across
//! compiler or crate versions; the host checks the plugin's ABI string and
//! refuses to load a mismatched one.

// Re-export the crates and types plugin authors need, so a plugin only needs
// `maxima-plugin` as a dependency. `$crate` paths in the macro resolve here.
pub use maxima_core;
pub use maxima_eval;
pub use maxima_core::{Expr, Operator};
pub use maxima_eval::{Environment, NativeFn};
pub use maxima_eval::plugin::guard;
pub use maxima_eval::simp::simplify;
pub use maxima_eval::meval;

use std::ffi::CStr;

/// The ABI descriptor the generated `maxima_plugin_abi` export returns.
#[doc(hidden)]
pub fn abi_cstr() -> &'static CStr {
    maxima_eval::MAXIMA_PLUGIN_ABI_CSTR
}

/// Generate the three `extern "C"` exports a Maxima plugin must provide:
/// `maxima_plugin_abi` (version guard), `maxima_plugin_set_interner` (shared
/// symbol table handoff), and `maxima_plugin_register` (runs your closure).
///
/// `register` takes a non-capturing closure `|env| { ... }` that registers
/// native functions on the host `Environment`.
#[macro_export]
macro_rules! maxima_plugin {
    (register = $reg:expr $(,)?) => {
        #[no_mangle]
        pub extern "C" fn maxima_plugin_abi() -> *const ::std::os::raw::c_char {
            $crate::abi_cstr().as_ptr()
        }

        #[no_mangle]
        pub extern "C" fn maxima_plugin_set_interner(
            ptr: *mut ::std::sync::Mutex<$crate::maxima_core::InternTable>,
        ) {
            // SAFETY: the host passes its own `interner_ptr()`, valid for the
            // whole process; called once at load before any interning here.
            unsafe { $crate::maxima_core::adopt_interner(ptr) };
        }

        #[no_mangle]
        pub extern "C" fn maxima_plugin_register(
            env: &mut $crate::maxima_eval::Environment,
        ) {
            let register: fn(&mut $crate::maxima_eval::Environment) = $reg;
            register(env);
        }
    };
}
