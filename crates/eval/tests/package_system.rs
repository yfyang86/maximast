use maxima_eval::{eval_str_with_env, Environment, NativeFn};
use maxima_core::Expr;

// ==================== Package System Tests ====================

#[test]
fn test_load_and_require() {
    let dir = tempfile::tempdir().unwrap();
    let mac_path = dir.path().join("mypkg.mac");
    std::fs::write(&mac_path, "myfn(x) := x^2 + 1;\n").unwrap();

    let mut env = Environment::new();
    env.search_paths.push(dir.path().display().to_string());

    let result = eval_str_with_env("load(\"mypkg\");", &mut env);
    assert!(result.contains("mypkg"), "load should return path, got: {}", result);

    let result = eval_str_with_env("myfn(3);", &mut env);
    assert_eq!(result, "10");

    // loaded_files should include the file
    let loaded = eval_str_with_env("loaded_files();", &mut env);
    assert!(loaded.contains("mypkg"), "loaded_files should list mypkg, got: {}", loaded);

    // require should NOT re-load (function should still work)
    let result = eval_str_with_env("require(\"mypkg\");", &mut env);
    assert!(result.contains("mypkg"));

    let result = eval_str_with_env("myfn(5);", &mut env);
    assert_eq!(result, "26");
}

#[test]
fn test_load_nonexistent() {
    let mut env = Environment::new();
    let result = eval_str_with_env("load(\"nonexistent_file_xyz\");", &mut env);
    assert_eq!(result, "false");
}

#[test]
fn test_file_search() {
    let dir = tempfile::tempdir().unwrap();
    let mac_path = dir.path().join("findme.mac");
    std::fs::write(&mac_path, "1;\n").unwrap();

    let mut env = Environment::new();
    env.search_paths.push(dir.path().display().to_string());

    let result = eval_str_with_env("file_search(\"findme\");", &mut env);
    assert!(result.contains("findme.mac"), "file_search should find the file, got: {}", result);

    let result = eval_str_with_env("file_search(\"nope\");", &mut env);
    assert_eq!(result, "false");
}

#[test]
fn test_setup_autoload() {
    let dir = tempfile::tempdir().unwrap();
    let mac_path = dir.path().join("autofns.mac");
    std::fs::write(&mac_path, "autofn1(x) := x + 100;\nautofn2(x) := x * 2;\n").unwrap();

    let mut env = Environment::new();
    env.search_paths.push(dir.path().display().to_string());

    // Register autoload
    eval_str_with_env("setup_autoload(\"autofns\", autofn1, autofn2);", &mut env);

    // Call autofn1 — should trigger autoload
    let result = eval_str_with_env("autofn1(5);", &mut env);
    assert_eq!(result, "105");

    // autofn2 should also be available now (loaded by same file)
    let result = eval_str_with_env("autofn2(7);", &mut env);
    assert_eq!(result, "14");
}

#[test]
fn test_nested_load() {
    let dir = tempfile::tempdir().unwrap();
    // inner.mac defines a function
    let inner_path = dir.path().join("inner.mac");
    std::fs::write(&inner_path, "inner_fn(x) := x * 3;\n").unwrap();

    // outer.mac loads inner.mac
    let outer_path = dir.path().join("outer.mac");
    std::fs::write(&outer_path, "load(\"inner\");\nouter_fn(x) := inner_fn(x) + 1;\n").unwrap();

    let mut env = Environment::new();
    env.search_paths.push(dir.path().display().to_string());

    eval_str_with_env("load(\"outer\");", &mut env);
    let result = eval_str_with_env("outer_fn(10);", &mut env);
    assert_eq!(result, "31");
}

#[test]
fn test_load_pathname() {
    let dir = tempfile::tempdir().unwrap();
    let mac_path = dir.path().join("checkpath.mac");
    // The file just defines a trivial function; we check load_pathname from outside
    std::fs::write(&mac_path, "pathfn(x) := x;\n").unwrap();

    let mut env = Environment::new();
    env.search_paths.push(dir.path().display().to_string());

    // Before loading, load_pathname should be false
    let result = eval_str_with_env("load_pathname();", &mut env);
    assert_eq!(result, "false");

    eval_str_with_env("load(\"checkpath\");", &mut env);

    // After load completes, load_pathname should be restored to false
    let result = eval_str_with_env("load_pathname();", &mut env);
    assert_eq!(result, "false");
}

#[test]
fn test_native_function_registration() {
    fn native_double(args: &[Expr], _env: &mut Environment) -> Expr {
        match &args[0] {
            Expr::Integer(n) => Expr::Integer(n * 2),
            other => other.clone(),
        }
    }

    let mut env = Environment::new();
    env.register_native("native_double", native_double as NativeFn, 1, Some(1));

    let result = eval_str_with_env("native_double(21);", &mut env);
    assert_eq!(result, "42");

    // Wrong number of args: returns noun form
    let result = eval_str_with_env("native_double(1, 2);", &mut env);
    assert!(result.contains("native_double"), "should return noun form, got: {}", result);
}

#[test]
fn test_native_overrides_maxima() {
    fn native_myfn(_args: &[Expr], _env: &mut Environment) -> Expr {
        Expr::Integer(999)
    }

    let mut env = Environment::new();

    // Define a Maxima function first
    eval_str_with_env("myfn(x) := x + 1;", &mut env);
    let result = eval_str_with_env("myfn(5);", &mut env);
    assert_eq!(result, "6");

    // Register native — should take priority
    env.register_native("myfn", native_myfn as NativeFn, 1, Some(1));
    let result = eval_str_with_env("myfn(5);", &mut env);
    assert_eq!(result, "999");
}

#[test]
fn test_file_search_maxima() {
    let mut env = Environment::new();
    env.search_paths.push("/custom/path".to_string());
    let result = eval_str_with_env("file_search_maxima();", &mut env);
    assert!(result.contains("/custom/path"), "should list custom path, got: {}", result);
}

#[test]
fn test_kill_all_preserves_native() {
    fn native_fn(_args: &[Expr], _env: &mut Environment) -> Expr {
        Expr::Integer(42)
    }

    let mut env = Environment::new();
    env.register_native("preserved_fn", native_fn as NativeFn, 0, Some(0));
    eval_str_with_env("kill(all);", &mut env);

    // Native function should survive kill(all)
    let result = eval_str_with_env("preserved_fn();", &mut env);
    assert_eq!(result, "42");
}
