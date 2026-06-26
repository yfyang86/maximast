// Bundle 2 / RootOf: implicit algebraic roots for polynomials unsolvable by
// radicals. solve returns rootof(p,x,k) nouns (one per root, real roots first);
// float/bfloat evaluate them — all roots via Durand–Kerner, real roots refined
// to full bigfloat precision via Newton.
use maxima_eval::{eval_str_with_env, Environment};

fn run(s: &str) -> String {
    let mut env = Environment::new();
    eval_str_with_env(s, &mut env).split_whitespace().collect()
}

fn run_seq(stmts: &[&str]) -> String {
    let mut env = Environment::new();
    let mut out = String::new();
    for s in stmts { out = eval_str_with_env(s, &mut env); }
    out.split_whitespace().collect()
}

/// Evaluate the integer polynomial x^5 - x - 1 at v.
fn quintic(v: f64) -> f64 { v.powi(5) - v - 1.0 }

#[test]
fn solve_quintic_returns_rootof() {
    let r = run("solve(x^5-x-1, x);");
    assert_eq!(r.matches("rootof(").count(), 5, "got {r}");
    assert!(!r.contains("solve("), "should not be a bare noun: {r}");
    // indices 1..5 present
    for k in 1..=5 {
        assert!(r.contains(&format!("x,{k})")), "missing index {k}: {r}");
    }
}

#[test]
fn solvable_polys_stay_radical() {
    // RootOf must not hijack polynomials that have radical solutions.
    assert_eq!(run("solve(x^2-2, x);"), "[x=sqrt(2),x=-sqrt(2)]");
    assert!(!run("solve(x^3-2, x);").contains("rootof"));
    assert!(!run("solve(x^6-1, x);").contains("rootof"));
}

#[test]
fn float_real_root() {
    // k=1 is the (single) real root, ascending-real-first ordering.
    let v: f64 = run("float(rootof(x^5-x-1, x, 1));").parse().unwrap();
    assert!((v - 1.1673039782614187).abs() < 1e-12, "got {v}");
    assert!(quintic(v).abs() < 1e-10);
}

#[test]
fn float_complex_root() {
    // k=2 is the first complex root: "re+im*%i"
    let r = run("float(rootof(x^5-x-1, x, 2));");
    assert!(r.contains("%i"), "expected complex, got {r}");
}

#[test]
fn bfloat_real_root_high_precision() {
    let r = run_seq(&["fpprec: 40;", "bfloat(rootof(x^5-x-1, x, 1));"]);
    // real root of x^5-x-1 to 40 digits
    assert!(r.starts_with("1.16730397826141868425604589985"), "got {r}");
    assert!(r.ends_with("b0"), "got {r}");
    // numeric residual at f64 precision
    let v: f64 = r.replace('b', "e").parse().unwrap();
    assert!(quintic(v).abs() < 1e-10);
}

#[test]
fn mixed_factors_radical_plus_rootof() {
    // (x-1)(x^5-x-1): exact rational root + 5 rootof for the quintic factor
    let r = run("solve((x-1)*(x^5-x-1), x);");
    assert!(r.contains("x=1"), "missing rational root: {r}");
    assert_eq!(r.matches("rootof(").count(), 5, "got {r}");
}

#[test]
fn cube_root_via_rootof() {
    // rootof works when invoked directly even on a radical-solvable poly
    let v: f64 = run("float(rootof(x^3-2, x, 1));").parse().unwrap();
    assert!((v - 2f64.powf(1.0 / 3.0)).abs() < 1e-12, "got {v}");
}
