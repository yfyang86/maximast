// S3: orthogonal-polynomial plugin. Loads the maxima-orthopoly cdylib and
// checks closed forms (symbolic) plus exact numeric values.
use maxima_eval::{eval_str_with_env, Environment};
use std::path::PathBuf;
use std::process::Command;

fn target_dir() -> PathBuf {
    std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target"))
}

fn plugin_path() -> Option<String> {
    let name = format!(
        "{}maxima_orthopoly.{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_EXTENSION
    );
    for profile in ["debug", "release"] {
        let p = target_dir().join(profile).join(&name);
        if p.is_file() {
            return Some(p.display().to_string());
        }
    }
    None
}

fn ensure() -> Option<String> {
    // Always invoke `cargo build` first so a source change to the plugin is
    // picked up. Cargo's incremental build is a no-op when nothing changed,
    // but checking only "does the .so exist?" would silently use a stale
    // artifact from before the source change.
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let _ = Command::new(cargo).args(["build", "-p", "maxima-orthopoly"]).status();
    plugin_path()
}

fn norm(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

#[test]
fn orthopoly_families() {
    let Some(path) = ensure() else {
        eprintln!("skipping: maxima-orthopoly cdylib not available");
        return;
    };
    let mut env = Environment::new();
    assert_eq!(eval_str_with_env(&format!("load_plugin(\"{}\");", path), &mut env), "true");

    let mut run = |s: &str| norm(&eval_str_with_env(s, &mut env));

    // Symbolic closed forms.
    assert_eq!(run("legendre_p(2, x);"), "(3/2)*x^2-1/2");
    assert_eq!(run("legendre_p(3, x);"), "(-3/2)*x+(5/2)*x^3");
    assert_eq!(run("chebyshev_t(3, x);"), "-3*x+4*x^3");
    assert_eq!(run("chebyshev_u(2, x);"), "-1+4*x^2");
    assert_eq!(run("hermite(3, x);"), "-12*x+8*x^3");
    assert_eq!(run("laguerre(2, x);"), "1-2*x+(1/2)*x^2");
    assert_eq!(run("gen_laguerre(2, 1, x);"), "3-3*x+(1/2)*x^2");
    assert_eq!(run("ultraspherical(2, 1, x);"), "-1+4*x^2"); // C_n^1 = U_n
    assert_eq!(run("jacobi_p(2, 1, 1, x);"), "(15/4)*x^2-3/4");

    // Base cases.
    assert_eq!(run("legendre_p(0, x);"), "1");
    assert_eq!(run("chebyshev_t(1, x);"), "x");
}

#[test]
fn orthopoly_exact_numeric() {
    let Some(path) = ensure() else { return; };
    let mut env = Environment::new();
    eval_str_with_env(&format!("load_plugin(\"{}\");", path), &mut env);
    let mut run = |s: &str| norm(&eval_str_with_env(s, &mut env));

    // P_2(1/2) = (3*(1/4)-1)/2 = -1/8
    assert_eq!(run("legendre_p(2, 1/2);"), "-1/8");
    // P_3(1/2) = (5/8 - 3/2)/2 = -7/16
    assert_eq!(run("legendre_p(3, 1/2);"), "-7/16");
    // T_n(1) = 1 for all n
    assert_eq!(run("chebyshev_t(5, 1);"), "1");
    // H_4(0) = 12
    assert_eq!(run("hermite(4, 0);"), "12");
    // L_3(0) = 1 (Laguerre at 0 is 1)
    assert_eq!(run("laguerre(3, 0);"), "1");
}

#[test]
fn orthopoly_symbolic_param_is_noun() {
    let Some(path) = ensure() else { return; };
    let mut env = Environment::new();
    eval_str_with_env(&format!("load_plugin(\"{}\");", path), &mut env);
    // A symbolic parameter cannot be expanded by the rational recurrence;
    // expect the noun form rather than a wrong/garbled result.
    let r = eval_str_with_env("gen_laguerre(2, a, x);", &mut env);
    assert!(r.contains("gen_laguerre"), "expected noun form, got: {}", r);
}

#[test]
fn legendre_q_closed_forms() {
    let Some(path) = ensure() else { return; };
    let mut env = Environment::new();
    eval_str_with_env(&format!("load_plugin(\"{}\");", path), &mut env);
    let mut run = |s: &str| norm(&eval_str_with_env(s, &mut env));

    // Q_0(x) = (1/2) log((1+x)/(1-x))
    assert_eq!(run("legendre_q(0, x);"), "(1/2)*log((1+x)/(1-x))");
    // Q_1(x) = x*Q_0(x) - 1
    let q1 = run("legendre_q(1, x);");
    assert!(q1.contains("log((1+x)/(1-x))") && q1.contains("-1") && q1.contains("x"),
            "got: {}", q1);
    // Q_2(x) = ((3*x^2-1)/4)*log((1+x)/(1-x)) - 3*x/2
    let q2 = run("legendre_q(2, x);");
    assert!(q2.contains("log((1+x)/(1-x))") && q2.contains("x^2") && q2.contains("3"),
            "got: {}", q2);
}

#[test]
fn legendre_q_satisfies_legendre_ode() {
    // Q_n satisfies (1-x^2) y'' - 2x y' + n(n+1) y = 0. Numerically the
    // residual should be ~0 at non-singular points.
    let Some(path) = ensure() else { return; };
    let mut env = Environment::new();
    eval_str_with_env(&format!("load_plugin(\"{}\");", path), &mut env);

    for (n_val, n_n_plus_1) in [(2, 6), (3, 12), (4, 20)] {
        let expr = format!(
            "float(subst(0.3, x, ratsimp((1-x^2)*diff(legendre_q({n}, x), x, 2) \
                - 2*x*diff(legendre_q({n}, x), x) + {nn}*legendre_q({n}, x))));",
            n = n_val, nn = n_n_plus_1
        );
        let r = eval_str_with_env(&expr, &mut env);
        let v: f64 = r.trim().parse().unwrap_or_else(|_| panic!("not numeric: {}", r));
        assert!(v.abs() < 1e-9, "legendre_q({}) ODE residual = {}", n_val, v);
    }
}

#[test]
fn legendre_q_numeric_atanh_at_zero() {
    let Some(path) = ensure() else { return; };
    let mut env = Environment::new();
    eval_str_with_env(&format!("load_plugin(\"{}\");", path), &mut env);

    // Q_0(0) = 0 (log(1/1)/2 = 0). Q_0(0.5) = atanh(0.5) ≈ 0.549306.
    let r = eval_str_with_env("float(subst(0.5, x, legendre_q(0, x)));", &mut env);
    let v: f64 = r.trim().parse().unwrap();
    assert!((v - 0.549_306_144_334_055).abs() < 1e-9, "got: {}", v);
}
