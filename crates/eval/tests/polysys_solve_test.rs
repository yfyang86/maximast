// V8.0 S4: polysys_solve via lex-Gröbner triangulation.
//
// Coverage is bounded by what the host's univariate `solve` can do — the
// triangulation cascade is only as strong as its leaf solver. Quadratics,
// linears, and explicit factorisations are in; biquadratics/quartics
// without explicit factorisation are not yet (they go noun).
use maxima_eval::eval_str;
fn run(s: &str) -> String { eval_str(s) }
fn norm(s: &str) -> String { s.chars().filter(|c| !c.is_whitespace()).collect() }

#[test] fn linear_system_unique_solution() {
    let r = norm(&run("polysys_solve([x + y - 3, 2*x - y], [x, y]);"));
    assert_eq!(r, "[[x=1,y=2]]");
}

#[test] fn accepts_equation_form() {
    let r = norm(&run("polysys_solve([x = 1, y = 2], [x, y]);"));
    assert_eq!(r, "[[x=1,y=2]]");
}

#[test] fn quadratic_with_substitution_two_points() {
    // x^2 = 4 and y = x → (2,2) and (-2,-2).
    let r = norm(&run("polysys_solve([x^2 - 4, y - x], [x, y]);"));
    assert!(r.contains("x=2") && r.contains("y=2"));
    assert!(r.contains("x=-2") && r.contains("y=-2"));
}

#[test] fn hyperbola_line_double_root() {
    // x*y = 1, x + y = 2 → (y-1)^2 = 0, x = y = 1.
    let r = norm(&run("polysys_solve([x*y - 1, x + y - 2], [x, y]);"));
    assert_eq!(r, "[[x=1,y=1]]");
}

#[test] fn inconsistent_system_yields_empty() {
    // {x = 0, x = 1}: no solutions.
    let r = norm(&run("polysys_solve([x, x - 1], [x, y]);"));
    assert_eq!(r, "[]");
}

#[test] fn non_polynomial_input_goes_noun() {
    let r = run("polysys_solve([sin(x), y - 1], [x, y]);");
    assert!(r.contains("polysys_solve"), "expected noun, got: {}", r);
}

#[test] fn unsupported_univariate_falls_back_to_noun() {
    // The host can't (yet) solve y^4 + y^2 - 1 = 0 symbolically; honest
    // failure mode is the noun form, not a wrong result.
    let r = run("polysys_solve([x^2 + y^2 - 1, x - y^2], [x, y]);");
    assert!(r.contains("polysys_solve"), "expected noun, got: {}", r);
}
