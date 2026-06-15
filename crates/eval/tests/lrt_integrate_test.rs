// V8.0 / S1: Lazard–Rioboo–Trager logarithmic part wired into rational
// integration. Verifies the new purely-logarithmic / rational-residue cases
// and confirms the existing atan/log paths are unchanged (no regression).
use maxima_eval::{eval_str_with_env, Environment};

fn run(s: &str) -> String {
    let mut env = Environment::new();
    eval_str_with_env(s, &mut env).split_whitespace().collect()
}

#[test]
fn log_derivative_degree4() {
    // (4x^3+1)/(x^4+x) = d/dx log(x^4+x). Denominator x(x+1)(x^2-x+1).
    let r = run("integrate((4*x^3+1)/(x^4+x), x);");
    assert!(r.contains("log"), "got: {}", r);
    assert!(!r.contains("integrate"), "should be solved: {}", r);
    assert!(!r.contains("atan"), "should be a pure log: {}", r);
}

#[test]
fn log_derivative_irreducible_quartic_factor() {
    // (5x^4+1)/(x^5+x) = d/dx log(x^5+x). x^5+x = x(x^4+1), and x^4+1 is
    // irreducible over Q — LRT gives the clean single log where the old
    // algebraic-factoring path produced a sqrt(2) log+atan form.
    let r = run("integrate((5*x^4+1)/(x^5+x), x);");
    assert!(r.contains("log"), "got: {}", r);
    assert!(!r.contains("integrate"), "should be solved: {}", r);
    assert!(!r.contains("atan") && !r.contains("sqrt"), "should be a pure rational-coeff log: {}", r);
}

#[test]
fn atan_path_unchanged() {
    // LRT must NOT fire here (complex residues); existing atan path stands.
    assert_eq!(run("integrate(1/(x^2+1), x);"), "atan(x)");
}

#[test]
fn linear_partfrac_unchanged() {
    // All-linear denominator still handled by the residue path.
    let r = run("integrate(1/(x^2-1), x);");
    assert!(r.contains("log(-1+x)") && r.contains("log(1+x)"), "got: {}", r);
}
