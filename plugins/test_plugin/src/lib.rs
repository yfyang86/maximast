//! Test fixture for the plugin loader (see crates/eval/tests/plugin_test.rs).
//! Uses the `maxima-plugin` authoring kit, so it also exercises the macro.

use maxima_plugin::{maxima_plugin, Expr, Environment, guard};

fn plugin_double(args: &[Expr], _env: &mut Environment) -> Expr {
    guard("plugin_double", args, || match args.first() {
        Some(Expr::Integer(n)) => Expr::int(n * 2),
        _ => Expr::call("plugin_double", args.to_vec()),
    })
}

/// Always panics — verifies the host survives a misbehaving plugin because
/// `guard` contains the panic and returns the noun form.
fn plugin_boom(args: &[Expr], _env: &mut Environment) -> Expr {
    guard("plugin_boom", args, || panic!("intentional plugin panic"))
}

maxima_plugin!(register = |env| {
    env.register_native("plugin_double", plugin_double, 1, Some(1));
    env.register_native("plugin_boom", plugin_boom, 0, Some(0));
});
