// Bundle 2 / 1a: general cubic (Cardano) and quartic (Ferrari) radical solve.
// Every root returned by the solver is numerically verified inside the engine
// (expr_to_complex: |p(r)| < 1e-6 over Complex64); a root that fails, or a
// factor that can't be radical-solved, collapses the whole result to a noun.
// So "returns a non-noun list of the right length" already asserts every root
// is correct. Real-radical roots are additionally float-checked here.
use maxima_eval::{eval_str_with_env, Environment};

fn run(s: &str) -> String {
    let mut env = Environment::new();
    eval_str_with_env(s, &mut env).split_whitespace().collect()
}

fn count_roots(s: &str) -> usize {
    s.matches("x=").count()
}

/// f64 of `float(rhs(solve(p,x)[idx]))` — only valid for real-radical roots.
fn root_float(poly: &str, idx: usize) -> f64 {
    let mut env = Environment::new();
    let out = eval_str_with_env(&format!("float(rhs(solve({poly},x)[{idx}]));"), &mut env);
    out.trim().parse::<f64>().unwrap_or_else(|_| panic!("not a float: {out}"))
}

#[test]
fn pure_cube() {
    // x^3 = 2: real cube root + two complex
    let r = run("solve(x^3-2, x);");
    assert!(r.contains("2^(1/3)") && r.contains("%i"), "got {r}");
    assert_eq!(count_roots(&r), 3);
}

#[test]
fn general_cardano_real_radical() {
    // p≠0, D>0: one real root in real radicals, two complex
    let r = run("solve(x^3+x+1, x);");
    assert!(!r.contains("solve("), "got {r}");
    assert_eq!(count_roots(&r), 3);
    assert!((root_float("x^3+x+1", 1) + 0.6823278).abs() < 1e-6);
    assert!((root_float("x^3+3*x-2", 1) - 0.5960716).abs() < 1e-6);
}

#[test]
fn casus_irreducibilis_solved() {
    // D<0, three real roots: now solved via complex radicals (each verified),
    // no longer a noun. (x^3-3x+1 → 2cos(2πk/9 ± ...))
    let r = run("solve(x^3-3*x+1, x);");
    assert!(!r.contains("solve("), "casus should solve, got {r}");
    assert_eq!(count_roots(&r), 3);
}

#[test]
fn biquadratic_quartic() {
    // depressed quartic with q=0 (biquadratic in y), B=0
    let r = run("solve(x^4-5*x^2+6, x);");
    assert!(r.contains("sqrt(2)") && r.contains("sqrt(3)"), "got {r}");
    assert_eq!(count_roots(&r), 4);
}

#[test]
fn shifted_biquadratic_quartic() {
    // (x-1)^4 - 2: depresses to y^4 = 2 (q=0 with B≠0). Real roots 1±2^(1/4).
    let r = run("solve(x^4-4*x^3+6*x^2-4*x-1, x);");
    assert!(!r.contains("solve("), "got {r}");
    assert_eq!(count_roots(&r), 4);
    assert!((root_float("x^4-4*x^3+6*x^2-4*x-1", 1) - 2.1892071).abs() < 1e-6);
    assert!((root_float("x^4-4*x^3+6*x^2-4*x-1", 2) + 0.1892071).abs() < 1e-6);
}

#[test]
fn general_ferrari_quartic() {
    // x^4+x+1: irreducible over Q, resolvent cubic is casus irreducibilis —
    // Ferrari still succeeds (the complex-radical cubic solver supplies t0).
    let r = run("solve(x^4+x+1, x);");
    assert!(!r.contains("solve("), "got {r}");
    assert_eq!(count_roots(&r), 4);
}

#[test]
fn quartic_with_linear_factors_still_works() {
    // factoring path unaffected: (x-1)(x-2)(x-3)(x-4)
    let r = run("solve(x^4-10*x^3+35*x^2-50*x+24, x);");
    assert!(!r.contains("solve("), "got {r}");
    for k in ["x=1", "x=2", "x=3", "x=4"] {
        assert!(r.contains(k), "missing {k} in {r}");
    }
}
