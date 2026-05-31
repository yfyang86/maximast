// S4: residue(f, z, z0).
use maxima_eval::eval_str;
fn run(s: &str) -> String { eval_str(s) }
fn norm(s: &str) -> String { s.chars().filter(|c| !c.is_whitespace()).collect() }

// ---------- Simple poles ----------

#[test] fn residue_simple_pole_unit() {
    assert_eq!(run("residue(1/z, z, 0);"), "1");
    assert_eq!(run("residue(1/(z-3), z, 3);"), "1");
}

#[test] fn residue_simple_pole_rational() {
    // 1/(z^2-1) at z=1: 1/(2z)|_1 = 1/2
    assert_eq!(run("residue(1/(z^2-1), z, 1);"), "1/2");
    // at z=-1: 1/(2z)|_-1 = -1/2
    assert_eq!(run("residue(1/(z^2-1), z, -1);"), "-1/2");
}

#[test] fn residue_partial_fraction_consistency() {
    // z/((z-1)(z-2)) = -1/(z-1) + 2/(z-2); residues are -1 and 2.
    assert_eq!(run("residue(z/((z-1)*(z-2)), z, 1);"), "-1");
    assert_eq!(run("residue(z/((z-1)*(z-2)), z, 2);"), "2");
}

// ---------- Complex poles ----------

#[test] fn residue_complex_pole_i() {
    // 1/(z^2+1) at z=i: 1/(2i) = -i/2
    assert_eq!(norm(&run("residue(1/(z^2+1), z, %i);")), "(-1/2)*%i");
}

#[test] fn residue_complex_pole_minus_i() {
    // at z=-i: i/2
    assert_eq!(norm(&run("residue(1/(z^2+1), z, -%i);")), "(1/2)*%i");
}

// ---------- Higher-order poles ----------

#[test] fn residue_order_three_zero() {
    // 1/z^3 has zero residue (no 1/z term)
    assert_eq!(run("residue(1/z^3, z, 0);"), "0");
}

#[test] fn residue_order_two_exp() {
    // exp(z)/z^2: Laurent coeff of 1/z is exp'(0) = 1
    assert_eq!(run("residue(exp(z)/z^2, z, 0);"), "1");
}

#[test] fn residue_order_two_rational() {
    // 1/(z*(z-1)^2) at z=1 (double pole): d/dz[1/z]|_1 = -1
    assert_eq!(run("residue(1/(z*(z-1)^2), z, 1);"), "-1");
}

// ---------- No pole / analytic ----------

#[test] fn residue_no_pole() {
    // z0 is not a pole → residue 0
    assert_eq!(run("residue(1/(z-1), z, 5);"), "0");
    // analytic function → residue 0
    assert_eq!(run("residue(z^2+1, z, 0);"), "0");
}
