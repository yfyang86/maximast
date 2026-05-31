// S3: orthogonal-polynomial plugin. Loads the maxima-orthopoly cdylib and
// checks closed forms (symbolic) plus exact numeric values.
use maxima_eval::{eval_str_with_env, Environment};
use std::path::PathBuf;
use std::process::Command;

fn plugin_path() -> Option<String> {
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target");
    let name = format!(
        "{}maxima_orthopoly.{}",
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

fn ensure() -> Option<String> {
    if let Some(p) = plugin_path() {
        return Some(p);
    }
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
