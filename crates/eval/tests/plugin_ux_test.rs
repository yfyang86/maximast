// S5: plugin loading UX & robustness — multi-plugin coexistence, kill(all)
// survival, search-path resolution, and the missing-symbol error path.
use maxima_eval::{eval_str_with_env, Environment};
use std::path::PathBuf;
use std::process::Command;

fn target_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target")
}

fn lib_name(pkg_lib: &str) -> String {
    format!("{}{}.{}", std::env::consts::DLL_PREFIX, pkg_lib, std::env::consts::DLL_EXTENSION)
}

/// Find the cdylib for the given crate-lib name, building the package if needed.
fn artifact(pkg: &str, lib: &str) -> Option<String> {
    let find = || {
        for profile in ["debug", "release"] {
            let p = target_dir().join(profile).join(lib_name(lib));
            if p.is_file() {
                return Some(p.display().to_string());
            }
        }
        None
    };
    if let Some(p) = find() {
        return Some(p);
    }
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let _ = Command::new(cargo).args(["build", "-p", pkg]).status();
    find()
}

fn load(env: &mut Environment, path: &str) -> String {
    eval_str_with_env(&format!("load_plugin(\"{}\");", path), env)
}

#[test]
fn multiple_plugins_coexist() {
    let (Some(tp), Some(op)) = (
        artifact("maxima-test-plugin", "maxima_test_plugin"),
        artifact("maxima-orthopoly", "maxima_orthopoly"),
    ) else {
        eprintln!("skipping: plugin cdylibs not available");
        return;
    };
    let mut env = Environment::new();
    assert_eq!(load(&mut env, &tp), "true");
    assert_eq!(load(&mut env, &op), "true");

    // Functions from both plugins are callable in the same session.
    assert_eq!(eval_str_with_env("plugin_double(21);", &mut env), "42");
    let leg = eval_str_with_env("legendre_p(2, x);", &mut env);
    assert!(leg.contains("x^2"), "got: {}", leg);

    // Both plugins are listed.
    let listed = eval_str_with_env("loaded_plugins();", &mut env);
    assert!(listed.contains("maxima_test_plugin") && listed.contains("maxima_orthopoly"), "got: {}", listed);
}

#[test]
fn native_fns_survive_kill_all() {
    let Some(tp) = artifact("maxima-test-plugin", "maxima_test_plugin") else { return; };
    let mut env = Environment::new();
    load(&mut env, &tp);
    assert_eq!(eval_str_with_env("plugin_double(21);", &mut env), "42");

    // kill(all) clears user state but native (plugin) functions persist.
    eval_str_with_env("f(x):=x+1;", &mut env);
    eval_str_with_env("kill(all);", &mut env);
    assert_eq!(eval_str_with_env("plugin_double(21);", &mut env), "42");
    // A user-defined function, by contrast, is gone.
    assert!(eval_str_with_env("f(3);", &mut env).contains("f"));
}

#[test]
fn search_path_resolves_bare_name() {
    let Some(tp) = artifact("maxima-test-plugin", "maxima_test_plugin") else { return; };
    // Directory containing the artifact.
    let dir = PathBuf::from(&tp).parent().unwrap().display().to_string();
    let mut env = Environment::new();
    env.search_paths.push(dir);
    // Load by bare library name (no path, no extension) via the search path.
    assert_eq!(load(&mut env, "libmaxima_test_plugin"), "true");
    assert_eq!(eval_str_with_env("plugin_double(10);", &mut env), "20");
}

#[test]
fn missing_register_symbol_is_rejected() {
    let Some(bad) = artifact("maxima-bad-plugin", "maxima_bad_plugin") else { return; };
    let mut env = Environment::new();
    // The fixture omits maxima_plugin_register; loading must fail gracefully.
    assert_eq!(load(&mut env, &bad), "false");
}
