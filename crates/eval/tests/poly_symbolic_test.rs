// S3: resultant / discriminant with symbolic (non-integer) coefficients.
use maxima_eval::eval_str;
fn run(s: &str) -> String { eval_str(s) }

fn norm(s: &str) -> String { s.chars().filter(|c| !c.is_whitespace()).collect() }

// ---------- Symbolic discriminant ----------

#[test] fn disc_quadratic_symbolic() {
    // b^2 - 4*a*c
    let r = run("discriminant(a*x^2+b*x+c, x);");
    // term-order may vary; check the multiset of terms
    let n = norm(&r);
    assert!(n.contains("b^2") && n.contains("4*a*c"), "got: {}", r);
}

#[test] fn disc_monic_quadratic_symbolic() {
    let r = norm(&run("discriminant(x^2+b*x+c, x);"));
    assert!(r.contains("b^2") && r.contains("4*c"), "got: {}", r);
}

#[test] fn disc_depressed_cubic() {
    // discriminant(x^3+p*x+q) = -4*p^3 - 27*q^2
    let r = norm(&run("discriminant(x^3+p*x+q, x);"));
    assert!(r.contains("4*p^3") && r.contains("27*q^2"), "got: {}", r);
}

// ---------- Symbolic resultant ----------

#[test] fn resultant_symbolic_simple() {
    // resultant(x^2+a, x+b, x) = a + b^2
    let r = norm(&run("resultant(x^2+a, x+b, x);"));
    assert!(r.contains("b^2") && r.contains("a"), "got: {}", r);
}

#[test] fn resultant_linear_symbolic() {
    // resultant(x-p, x-q, x) = p - q (up to sign)
    let r = run("resultant(x-p, x-q, x);");
    assert!(r == "p-q" || r == "q-p" || r == "-p+q" || r == "-q+p", "got: {}", r);
}

// ---------- Numerical agreement: symbolic == integer instantiation ----------

#[test] fn disc_symbolic_matches_integer() {
    // disc(2x^2+7x+3) = 25 both ways
    assert_eq!(run("subst([a=2,b=7,c=3], discriminant(a*x^2+b*x+c,x));"), "25");
    assert_eq!(run("discriminant(2*x^2+7*x+3, x);"), "25");
}

#[test] fn resultant_symbolic_matches_integer() {
    assert_eq!(run("subst([a=5,b=2], resultant(x^2+a,x+b,x));"), "9");
    assert_eq!(run("resultant(x^2+5, x+2, x);"), "9");
}

// ---------- Integer path still reduces (regression for the 50/2 bug) ----------

#[test] fn disc_integer_reduced() {
    assert_eq!(run("discriminant(2*x^2+7*x+3, x);"), "25");
    assert_eq!(run("discriminant(x^2-5*x+6, x);"), "1");
    assert_eq!(run("discriminant(3*x^2+x+1, x);"), "-11");
}

// ---------- Non-polynomial guard ----------

#[test] fn disc_nonpolynomial_noun() {
    let r = run("discriminant(sin(x)+a, x);");
    assert!(r.contains("discriminant"), "should stay noun, got: {}", r);
}
