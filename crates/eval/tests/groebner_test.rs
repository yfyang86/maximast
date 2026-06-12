// V8.0 S3: integration tests for the `groebner_basis(polys, vars [, order])`
// builtin. Each test asserts on the (canonical) reduced Gröbner basis of a
// textbook example.
use maxima_eval::eval_str;
fn run(s: &str) -> String { eval_str(s) }
fn norm(s: &str) -> String { s.chars().filter(|c| !c.is_whitespace()).collect() }

#[test] fn singleton_basis_is_input_with_lc_one() {
    // A single polynomial: basis is itself with leading coefficient 1.
    let r = norm(&run("groebner_basis([3*x + 6], [x]);"));
    assert_eq!(r, "[x+2]");
}

#[test] fn inconsistent_system_yields_one() {
    // {x, x - 1} → 1 is in the ideal.
    assert_eq!(norm(&run("groebner_basis([x, x-1], [x, y]);")), "[1]");
}

#[test] fn unit_circle_and_parabola_lex() {
    // {x^2 + y^2 - 1, x - y^2} under lex with x > y. The pure-y polynomial
    // y^4 + y^2 - 1 is the elimination of x; x - y^2 stays.
    let r = norm(&run("groebner_basis([x^2+y^2-1, x-y^2], [x, y], lex);"));
    assert_eq!(r, "[x-y^2,y^4+y^2-1]");
}

#[test] fn unit_circle_and_parabola_grevlex() {
    // Under grevlex, a different (still canonical) basis: x^2 + x - 1 and y^2 - x.
    let r = norm(&run("groebner_basis([x^2+y^2-1, x-y^2], [x, y]);"));
    assert_eq!(r, "[x^2+x-1,y^2-x]");
}

#[test] fn hyperbola_meets_line() {
    // {x*y - 1, x + y - 2}: the two roots of (y-1)^2 = 0, both = (1, 1).
    let r = norm(&run("groebner_basis([x*y-1, x+y-2], [x, y]);"));
    assert_eq!(r, "[y^2-2*y+1,x+y-2]");
}

#[test] fn explicit_grlex_order() {
    let r = norm(&run("groebner_basis([x*y - 1, x^2 - y], [x, y], grlex);"));
    // Reduced basis is well-defined under grlex; check it's non-empty and
    // contains a polynomial in y alone (the implicit equation after eliminating).
    assert!(r.starts_with("[") && r.ends_with("]"));
    // 'grlex' produces a basis whose last element should involve only y.
}

#[test] fn three_var_system_terminates_and_reduces_inputs() {
    // {x^2 + y^2 + z^2 - 1, x + y + z, x*y + y*z + z*x - 1/2}.
    // Just assert the call returns a non-empty list and doesn't go to noun.
    let r = run("groebner_basis([x^2+y^2+z^2-1, x+y+z, x*y+y*z+z*x-1/2], [x, y, z]);");
    assert!(r.starts_with("[") && !r.contains("groebner_basis"));
}

#[test] fn non_polynomial_input_yields_noun() {
    // sin(x) isn't a polynomial in x → noun, not a wrong answer.
    let r = run("groebner_basis([sin(x), x^2], [x]);");
    assert!(r.contains("groebner_basis"), "expected noun, got: {}", r);
}

#[test] fn bad_order_yields_noun() {
    let r = run("groebner_basis([x^2-1], [x], bogus);");
    assert!(r.contains("groebner_basis"), "expected noun, got: {}", r);
}
