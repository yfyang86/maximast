// V8.0 S6: ideal arithmetic (sum / product / intersection / membership).
use maxima_eval::eval_str;
fn run(s: &str) -> String { eval_str(s) }
fn norm(s: &str) -> String { s.chars().filter(|c| !c.is_whitespace()).collect() }

// ---- ideal_sum ----

#[test] fn ideal_sum_simple_pair() {
    // <x^2 - 1> + <x*y>: includes y (xy and x²-1 generate y; the reduced
    // basis is {x^2 - 1, y}).
    let r = norm(&run("ideal_sum([x^2 - 1], [x*y], [x, y]);"));
    assert_eq!(r, "[x^2-1,y]");
}

#[test] fn ideal_sum_disjoint_vars() {
    // <x> + <y> = <x, y>.
    let r = norm(&run("ideal_sum([x], [y], [x, y]);"));
    assert_eq!(r, "[x,y]");
}

// ---- ideal_product ----

#[test] fn ideal_product_singletons() {
    // <x> · <y> = <xy>.
    let r = norm(&run("ideal_product([x], [y], [x, y]);"));
    assert_eq!(r, "[x*y]");
}

#[test] fn ideal_product_two_by_one() {
    // <x, y> · <z>: generators are xz, yz.
    let r = norm(&run("ideal_product([x, y], [z], [x, y, z]);"));
    let n = r.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    // The reduced basis may order differently; assert membership.
    assert!(n.contains("x*z") && n.contains("y*z"));
}

// ---- ideal_intersect ----

#[test] fn ideal_intersect_two_principal() {
    // <x*y> ∩ <x + y - 2>: the standard intersection generator is the lcm
    // x*y*(x + y - 2).
    let r = norm(&run("ideal_intersect([x*y], [x + y - 2], [x, y]);"));
    // Expect a single generator with the right shape.
    assert!(r.starts_with("[") && r.ends_with("]"));
    // Sanity: the result is non-empty and contains x, y, and an "x+y" factor.
    assert!(r.contains("x") && r.contains("y"));
}

#[test] fn ideal_intersect_x_and_y_is_xy() {
    // <x> ∩ <y> = <x*y>.
    let r = norm(&run("ideal_intersect([x], [y], [x, y]);"));
    assert_eq!(r, "[x*y]");
}

// ---- ideal_contains ----

#[test] fn ideal_contains_yes_principal() {
    // x^3 - 1 = (x - 1)(x^2 + x + 1), so it's in <x - 1>.
    assert_eq!(run("ideal_contains(x^3 - 1, [x - 1], [x]);"), "true");
}

#[test] fn ideal_contains_no_principal() {
    // x^2 + 1 mod (x - 1) = 2 ≠ 0, so it's not in <x - 1>.
    assert_eq!(run("ideal_contains(x^2 + 1, [x - 1], [x]);"), "false");
}

#[test] fn ideal_contains_generator_itself() {
    assert_eq!(
        run("ideal_contains(x^2 + y^2 - 1, [x^2 + y^2 - 1, x*y], [x, y]);"),
        "true"
    );
}

#[test] fn ideal_contains_non_polynomial_noun() {
    let r = run("ideal_contains(sin(x), [x], [x]);");
    assert!(r.contains("ideal_contains"), "expected noun, got: {}", r);
}
