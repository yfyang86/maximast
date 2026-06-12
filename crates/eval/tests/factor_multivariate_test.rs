// V8.0 S7 (stretch): minimal multivariate factor — extracts the monomial GCD
// across all terms, and runs the univariate `factor` on what's left when
// possible. Doesn't ship a full Hensel-lifting multivariate factoriser;
// that is the V9.0 carry-forward.
use maxima_eval::eval_str;
fn run(s: &str) -> String { eval_str(s) }
fn norm(s: &str) -> String { s.chars().filter(|c| !c.is_whitespace()).collect() }

#[test] fn extracts_common_monomial_factor() {
    // x^2*y - x*y^2 = x*y*(x - y)
    assert_eq!(norm(&run("factor_multivariate(x^2*y - x*y^2);")), "x*y*(x-y)");
}

#[test] fn extracts_higher_degree_monomial() {
    // x^4*y^2 - x^2*y^4 = x^2*y^2*(x^2 - y^2)
    let r = norm(&run("factor_multivariate(x^4*y^2 - x^2*y^4);"));
    assert_eq!(r, "x^2*y^2*(x^2-y^2)");
}

#[test] fn univariate_input_uses_existing_factor() {
    // After extracting the common x, the rest is univariate; the host's
    // factor handles it.
    assert_eq!(norm(&run("factor_multivariate(x^3 + x);")), "x*(1+x^2)");
}

#[test] fn no_common_factor_returns_unchanged_form() {
    // x^2 + y^2 + 1 has no common factor and is not univariate; the function
    // leaves it as-is (without going noun).
    let r = norm(&run("factor_multivariate(x^2 + y^2 + 1);"));
    // Three terms in some canonical order.
    assert!(r.contains("x^2") && r.contains("y^2") && r.contains("1"));
}

#[test] fn purely_numeric_input_unchanged() {
    assert_eq!(run("factor_multivariate(42);"), "42");
}

#[test] fn extracts_partial_when_rest_is_multivariate() {
    // 2*x^2 + 6*x*y + 4*x*y^2 has x in every term. The function should
    // factor x out, leaving a multivariate polynomial unchanged.
    let r = norm(&run("factor_multivariate(2*x^2 + 6*x*y + 4*x*y^2);"));
    // x * (something with both vars)
    assert!(r.starts_with("x*"));
}

#[test] fn handles_zero() {
    assert_eq!(run("factor_multivariate(x - x);"), "0");
}
