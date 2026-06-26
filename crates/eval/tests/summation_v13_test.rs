// Bundle 3 — summation completion: harmonic / generalized-harmonic sums (2b)
// and generating-function sums (3k). Harmonic closed forms round-trip to the
// exact rational at integer n; generating functions are numerically verified
// before being returned (correct-or-noun).
use maxima_eval::{eval_str_with_env, Environment};

fn run(s: &str) -> String {
    let mut env = Environment::new();
    eval_str_with_env(s, &mut env).split_whitespace().collect()
}

// ---- 2b: harmonic sums ----

#[test]
fn harmonic_finite() {
    assert_eq!(run("sum(1/k, k, 1, n);"), "harmonic(n)");
    assert_eq!(run("sum(1/k^2, k, 1, n);"), "harmonic(n,2)");
    assert_eq!(run("sum(1/k^3, k, 1, n);"), "harmonic(n,3)");
}

#[test]
fn harmonic_evaluates_at_integer() {
    assert_eq!(run("harmonic(5);"), "137/60");
    assert_eq!(run("harmonic(1);"), "1");
    assert_eq!(run("harmonic(0);"), "0");
    assert_eq!(run("harmonic(4, 2);"), "205/144");
}

#[test]
fn harmonic_round_trips() {
    // closed form at n=6 must equal the numeric sum
    assert_eq!(run("sum(1/k, k, 1, 6);"), run("harmonic(6);"));
    assert_eq!(run("sum(1/k^2, k, 1, 6);"), run("harmonic(6, 2);"));
}

#[test]
fn harmonic_infinite() {
    assert_eq!(run("sum(1/k, k, 1, inf);"), "inf"); // divergent
    assert_eq!(run("sum(1/k^2, k, 1, inf);"), "%pi^2/6"); // ζ(2)
    assert_eq!(run("sum(1/k^4, k, 1, inf);"), "%pi^4/90"); // ζ(4)
    assert_eq!(run("sum(1/k^3, k, 1, inf);"), "zeta(3)"); // ζ(3) noun
}

// ---- 3k: generating functions ----

#[test]
fn gf_symbolic_base() {
    assert_eq!(run("sum(x^k, k, 0, inf);"), "1/(1-x)");
    assert_eq!(run("sum(k*x^k, k, 1, inf);"), "x/(1-x)^2");
    assert_eq!(run("sum(q^k, k, 0, inf);"), "1/(1-q)");
}

#[test]
fn gf_numeric_base_converges() {
    assert_eq!(run("sum(k*(1/2)^k, k, 1, inf);"), "2");
    assert_eq!(run("sum(k^2*(1/3)^k, k, 1, inf);"), "3/2");
    assert_eq!(run("sum((1/2)^k, k, 0, inf);"), "2");
}

#[test]
fn gf_divergent_is_noun() {
    // |base| ≥ 1 → divergent → noun, never a wrong finite value
    assert!(run("sum(2^k, k, 0, inf);").contains("sum("));
    assert!(run("sum(k*2^k, k, 1, inf);").contains("sum("));
    assert!(run("sum(k, k, 1, inf);").contains("sum("));
}
