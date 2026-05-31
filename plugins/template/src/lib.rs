//! Template Maxima plugin. Copy this directory to start a new plugin.
//!
//! Build: `cargo build -p maxima-plugin-template`
//! Use:   `load_plugin("target/debug/libmaxima_plugin_template")$ plugin_double(21);`

use maxima_plugin::{maxima_plugin, Expr, Environment, guard};

/// `plugin_double(n)` -> `2*n`; noun form for non-integers.
fn plugin_double(args: &[Expr], _env: &mut Environment) -> Expr {
    guard("plugin_double", args, || match args.first() {
        Some(Expr::Integer(n)) => Expr::int(n * 2),
        _ => Expr::call("plugin_double", args.to_vec()),
    })
}

maxima_plugin!(register = |env| {
    env.register_native("plugin_double", plugin_double, 1, Some(1));
});
