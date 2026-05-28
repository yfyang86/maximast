use maxima_eval::eval_str;

fn run(s: &str) -> String { eval_str(s) }

// ==================== sqrt simplification ====================

#[test]
fn sqrt_perfect_squares() {
    assert_eq!(run("sqrt(0);"), "0");
    assert_eq!(run("sqrt(1);"), "1");
    assert_eq!(run("sqrt(4);"), "2");
    assert_eq!(run("sqrt(9);"), "3");
    assert_eq!(run("sqrt(16);"), "4");
    assert_eq!(run("sqrt(25);"), "5");
    assert_eq!(run("sqrt(100);"), "10");
    assert_eq!(run("sqrt(10000);"), "100");
}

#[test]
fn sqrt_factor_extraction() {
    assert_eq!(run("sqrt(12);"), "2*sqrt(3)");
    assert_eq!(run("sqrt(18);"), "3*sqrt(2)");
    assert_eq!(run("sqrt(50);"), "5*sqrt(2)");
    assert_eq!(run("sqrt(72);"), "6*sqrt(2)");
    assert_eq!(run("sqrt(48);"), "4*sqrt(3)");
    assert_eq!(run("sqrt(75);"), "5*sqrt(3)");
    assert_eq!(run("sqrt(200);"), "10*sqrt(2)");
}

#[test]
fn sqrt_primes_unchanged() {
    assert_eq!(run("sqrt(2);"), "sqrt(2)");
    assert_eq!(run("sqrt(3);"), "sqrt(3)");
    assert_eq!(run("sqrt(5);"), "sqrt(5)");
    assert_eq!(run("sqrt(7);"), "sqrt(7)");
}

#[test]
fn sqrt_negative_unchanged() {
    assert_eq!(run("sqrt(-1);"), "sqrt(-1)");
    assert_eq!(run("sqrt(-4);"), "sqrt(-4)");
}

#[test]
fn sqrt_rational() {
    assert_eq!(run("sqrt(1/4);"), "1/2");
    assert_eq!(run("sqrt(9/16);"), "3/4");
    assert_eq!(run("sqrt(4/9);"), "2/3");
}

// ==================== sqrt power identities ====================

#[test]
fn sqrt_squared() {
    assert_eq!(run("sqrt(2)^2;"), "2");
    assert_eq!(run("sqrt(3)^2;"), "3");
    assert_eq!(run("sqrt(x)^2;"), "x");
}

#[test]
fn sqrt_higher_powers() {
    assert_eq!(run("sqrt(x)^4;"), "x^2");
    assert_eq!(run("sqrt(x)^6;"), "x^3");
    assert_eq!(run("sqrt(x)^3;"), "x*sqrt(x)");
}

#[test]
fn sqrt_product_same() {
    assert_eq!(run("sqrt(2)*sqrt(2);"), "2");
    assert_eq!(run("sqrt(3)*sqrt(3);"), "3");
    assert_eq!(run("sqrt(x)*sqrt(x);"), "x");
}

// ==================== Integration with sqrt ====================

#[test]
fn integrate_sqrt_x() {
    let r = run("integrate(sqrt(x), x);");
    assert!(!r.contains("integrate"), "should solve sqrt(x), got: {}", r);
}

#[test]
fn integrate_inv_sqrt_x() {
    let r = run("integrate(1/sqrt(x), x);");
    assert!(!r.contains("integrate"), "should solve 1/sqrt(x), got: {}", r);
}

#[test]
fn integrate_rational_product_definite() {
    // ∫_{-∞}^{∞} 1/((x²+1)(x²+4)) dx = π/6
    let r = run("integrate(1/((x^2+1)*(x^2+4)), x, minf, inf);");
    assert!(!r.contains("sqrt"), "should be fully simplified, got: {}", r);
    assert!(r.contains("6") && r.contains("%pi"), "expected pi/6, got: {}", r);
}

#[test]
fn integrate_rational_product_definite_2() {
    // ∫_{-∞}^{∞} 1/((x²+1)(x²+9)) dx = π/12
    let r = run("integrate(1/((x^2+1)*(x^2+9)), x, minf, inf);");
    assert!(!r.contains("sqrt"), "should be fully simplified, got: {}", r);
    assert!(r.contains("12") && r.contains("%pi"), "expected pi/12, got: {}", r);
}

// ==================== ratsimp with sqrt ====================

#[test]
fn ratsimp_sqrt_integer() {
    assert_eq!(run("ratsimp(sqrt(4));"), "2");
    assert_eq!(run("ratsimp(sqrt(9));"), "3");
    assert_eq!(run("ratsimp(sqrt(12));"), "2*sqrt(3)");
}

#[test]
fn ratsimp_sqrt_in_expression() {
    let r = run("ratsimp(%pi*((sqrt(1)+sqrt(4))*sqrt(4))^-1);");
    assert!(!r.contains("sqrt"), "should simplify all sqrts, got: {}", r);
}
