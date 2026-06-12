// V8.0 S5: eliminate(polys, vars) for variable elimination / implicitization.
use maxima_eval::eval_str;
fn run(s: &str) -> String { eval_str(s) }
fn norm(s: &str) -> String { s.chars().filter(|c| !c.is_whitespace()).collect() }

#[test] fn parabola_implicit() {
    // Parametric (x, y) = (t, t^2). Eliminate t → y - x^2 (i.e. x^2 - y = 0).
    assert_eq!(norm(&run("eliminate([x - t, y - t^2], [t]);")), "[x^2-y]");
}

#[test] fn equation_form_parabola() {
    // p = q form.
    assert_eq!(norm(&run("eliminate([x = t, y = t^2], [t]);")), "[x^2-y]");
}

#[test] fn unit_circle_y_subst_eliminate_x() {
    // x^2 + y^2 = 1, x = y + 1. Eliminating x leaves a polynomial in y alone.
    let r = norm(&run("eliminate([x^2 + y^2 - 1, x - y - 1], [x]);"));
    assert!(r.starts_with("[") && r.contains("y") && !r.contains("x"));
}

#[test] fn twisted_cubic_implicit_equations() {
    // (x, y, z) = (t, t^2, t^3). Eliminating t gives the four classical
    // relations of the twisted cubic.
    let r = norm(&run("eliminate([x = t, y = t^2, z = t^3], [t]);"));
    assert!(r.contains("x^2-y"));
    assert!(r.contains("y^3-z^2") || r.contains("z^2-y^3"));
}

#[test] fn empty_eliminate_returns_inputs() {
    // No variables to eliminate -> inputs are returned unchanged (as a list).
    let r = run("eliminate([x^2 + y^2 - 1, x - y], []);");
    assert!(r.contains("x^2") && r.contains("y"));
}

#[test] fn nonpolynomial_input_noun() {
    let r = run("eliminate([cos(s) - x, sin(s) - y], [s]);");
    assert!(r.contains("eliminate"), "expected noun, got: {}", r);
}
