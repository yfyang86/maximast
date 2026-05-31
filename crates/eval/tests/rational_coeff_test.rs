// S4b: exact rational-coefficient like-term collection in simplify_plus.
use maxima_eval::eval_str;
fn run(s: &str) -> String { eval_str(s) }

// ---------- Cancellation to zero ----------

#[test] fn rat_coeff_cancels() {
    assert_eq!(run("(1/2)*x + (-1/2)*x;"), "0");
    assert_eq!(run("(2/3)*y - (2/3)*y;"), "0");
}

#[test] fn rat_coeff_imag_cancels() {
    // The residue-sum case that motivated this sprint.
    assert_eq!(run("(-1/2)*%i + (1/2)*%i;"), "0");
}

// ---------- Exact rational sums (must NOT become floats) ----------

#[test] fn rat_coeff_sum_exact() {
    assert_eq!(run("(1/3)*x + (1/3)*x;"), "(2/3)*x");
    assert_eq!(run("(1/2)*x + (1/3)*x;"), "(5/6)*x");
    assert_eq!(run("(2/3)*y - (1/6)*y;"), "(1/2)*y");
}

#[test] fn rat_coeff_to_integer() {
    // (1/2)x + (1/2)x = x  (rational result that reduces to integer coeff 1)
    assert_eq!(run("(1/2)*x + (1/2)*x;"), "x");
    // (3/2)x + (1/2)x = 2x
    assert_eq!(run("(3/2)*x + (1/2)*x;"), "2*x");
}

// ---------- Integer coefficients still work (no regression) ----------

#[test] fn int_coeff_unchanged() {
    assert_eq!(run("x + x;"), "2*x");
    assert_eq!(run("3*x - x;"), "2*x");
    assert_eq!(run("2*%i + 3*%i;"), "5*%i");
    assert_eq!(run("5*z - 5*z;"), "0");
}

// ---------- Float coefficients still collect (no regression) ----------

#[test] fn float_coeff_unchanged() {
    assert_eq!(run("0.5*x + 0.5*x;"), "x");
}

// ---------- Mixed with a constant term ----------

#[test] fn rat_coeff_with_constant() {
    // (1/2)x + (1/2)x + 3 = x + 3
    let r = run("(1/2)*x + (1/2)*x + 3;");
    assert!(r.contains("x") && r.contains("3"), "got: {}", r);
}

// ---------- Pythagorean with rational coefficient ----------

#[test] fn pythagorean_rational_coeff() {
    // (1/2)sin(t)^2 + (1/2)cos(t)^2 = 1/2
    assert_eq!(run("(1/2)*sin(t)^2 + (1/2)*cos(t)^2;"), "1/2");
}
