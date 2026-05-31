// S1: dynamic Rust plugin loading. Exercises the loader against the
// `maxima-test-plugin` cdylib fixture: value calls, symbolic round-tripping
// across the (separately-compiled) boundary, panic containment, and errors.
use maxima_eval::{eval_str_with_env, Environment};
use std::path::PathBuf;
use std::process::Command;

fn plugin_path() -> Option<String> {
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target");
    let name = format!(
        "{}maxima_test_plugin.{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_EXTENSION
    );
    for profile in ["debug", "release"] {
        let p = target.join(profile).join(&name);
        if p.is_file() {
            return Some(p.display().to_string());
        }
    }
    None
}

/// Locate the fixture cdylib, building it once if necessary.
fn ensure_plugin() -> Option<String> {
    if let Some(p) = plugin_path() {
        return Some(p);
    }
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let _ = Command::new(cargo)
        .args(["build", "-p", "maxima-test-plugin"])
        .status();
    plugin_path()
}

#[test]
fn plugin_load_call_and_symbols() {
    let Some(path) = ensure_plugin() else {
        eprintln!("skipping: maxima-test-plugin cdylib not available");
        return;
    };
    let mut env = Environment::new();

    // Load succeeds and is idempotent.
    assert_eq!(eval_str_with_env(&format!("load_plugin(\"{}\");", path), &mut env), "true");
    assert_eq!(eval_str_with_env(&format!("load_plugin(\"{}\");", path), &mut env), "true");

    // Numeric call across the boundary.
    assert_eq!(eval_str_with_env("plugin_double(21);", &mut env), "42");
    assert_eq!(eval_str_with_env("plugin_double(2+3);", &mut env), "10");

    // Symbols round-trip both ways despite the plugin's own interner copy.
    assert_eq!(eval_str_with_env("plugin_double(x);", &mut env), "plugin_double(x)");
    assert_eq!(eval_str_with_env("plugin_double(a+b);", &mut env), "plugin_double(a+b)");

    // The plugin is listed.
    assert!(eval_str_with_env("loaded_plugins();", &mut env).contains("maxima_test_plugin"));
}

#[test]
fn plugin_panic_is_contained() {
    let Some(path) = ensure_plugin() else { return; };
    let mut env = Environment::new();
    eval_str_with_env(&format!("load_plugin(\"{}\");", path), &mut env);
    // A panicking plugin fn returns the noun form, and the session survives.
    assert_eq!(eval_str_with_env("plugin_boom();", &mut env), "plugin_boom()");
    assert_eq!(eval_str_with_env("plugin_double(50);", &mut env), "100");
}

#[test]
fn plugin_missing_returns_false() {
    let mut env = Environment::new();
    assert_eq!(
        eval_str_with_env("load_plugin(\"definitely_not_a_real_plugin\");", &mut env),
        "false"
    );
}
