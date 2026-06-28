// V13 2e: real-line definite integrals of rational functions by residues
// (contour over the upper half-plane), realised as exact partial fractions over
// Q with each irreducible quadratic integrated by ∫(Bx+C)/((x-α)²+ω²)^m.
use maxima_eval::eval_str;
fn run(s: &str) -> String { eval_str(s) }

#[test] fn simple_irreducible_quadratic() {
    // ∫_{-∞}^{∞} 1/(x²+2x+5) dx = π/2  (poles −1±2i)
    assert_eq!(run("integrate(1/(x^2+2*x+5), x, minf, inf);"), "(1/2)*%pi");
}
#[test] fn product_of_quadratics() {
    assert_eq!(run("integrate(1/((x^2+1)*(x^2+4)), x, minf, inf);"), "(1/6)*%pi");
    assert_eq!(run("integrate(1/((x^2+1)*(x^2+9)), x, minf, inf);"), "(1/12)*%pi");
}
#[test] fn repeated_quadratic() {
    // ∫ 1/(x²+1)² = π/2, ∫ 1/(x²+1)³ = 3π/8  (reduction formula)
    assert_eq!(run("integrate(1/(x^2+1)^2, x, minf, inf);"), "(1/2)*%pi");
    assert_eq!(run("integrate(1/(x^2+1)^3, x, minf, inf);"), "(3/8)*%pi");
}
#[test] fn numerator_with_x() {
    // ∫ x²/(x²+1)² = π/2; odd numerator part integrates to 0
    assert_eq!(run("integrate(x^2/(x^2+1)^2, x, minf, inf);"), "(1/2)*%pi");
    assert_eq!(run("integrate((x+1)/(x^2+2*x+5), x, minf, inf);"), "0");
}
#[test] fn real_pole_diverges_to_noun() {
    // 1/(x²−1) has real poles ⇒ divergent ⇒ noun (never a wrong finite value)
    assert!(run("integrate(1/(x^2-1), x, minf, inf);").contains("integrate"));
}

// Fourier/Jordan integrals: ∫ trig(ax)·P/Q over the real line.
#[test] fn fourier_cosine() {
    assert_eq!(run("integrate(cos(x)/(x^2+1), x, minf, inf);"), "%pi*exp(-1)");
    assert_eq!(run("integrate(cos(2*x)/(x^2+1), x, minf, inf);"), "%pi*exp(-2)");
    assert_eq!(run("integrate(cos(x)/(x^2+4), x, minf, inf);"), "(1/2)*%pi*exp(-2)");
}
#[test] fn fourier_sine() {
    assert_eq!(run("integrate(sin(x)/(x^2+1), x, minf, inf);"), "0"); // odd
    assert_eq!(run("integrate(x*sin(x)/(x^2+1), x, minf, inf);"), "%pi*exp(-1)");
}

// Unit-circle integrals: ∫_0^{2π} c/(a+b·trig θ) dθ = c·2π/√(a²−b²).
#[test] fn unit_circle_cosine() {
    assert_eq!(run("integrate(1/(2+cos(x)), x, 0, 2*%pi);"), "2*%pi/sqrt(3)");
    assert_eq!(run("integrate(1/(5+4*cos(x)), x, 0, 2*%pi);"), "(2/3)*%pi");
    assert_eq!(run("integrate(3/(2+cos(x)), x, 0, 2*%pi);"), "6*%pi/sqrt(3)");
}
#[test] fn unit_circle_sine() {
    assert_eq!(run("integrate(1/(2+sin(x)), x, 0, 2*%pi);"), "2*%pi/sqrt(3)");
}

// Biquadratic denominators irreducible over ℚ (x⁴+px²+q): the algebraic-number
// case ∫1/(x⁴+1)=π/√2, via ℝ-factorisation + residues. Closed form needs only
// the surds √q and √(2√q+p) — no general algebraic-number arithmetic.
#[test] fn biquadratic_unit() {
    assert_eq!(run("integrate(1/(x^4+1), x, minf, inf);"), "%pi/sqrt(2)");
    assert_eq!(run("integrate(x^2/(x^4+1), x, minf, inf);"), "%pi/sqrt(2)");
}
#[test] fn biquadratic_perfect_square_q() {
    // q=9,16 fold √q→3,4 — previously overflowed the LRT log path; the closed
    // form sidesteps it.
    assert_eq!(run("integrate(1/(x^4+9), x, minf, inf);"), "(1/3)*%pi/sqrt(6)");
    assert_eq!(run("integrate(1/(x^4+16), x, minf, inf);"), "(1/4)*%pi/(2*sqrt(2))");
}
#[test] fn biquadratic_nonsquare_q() {
    assert_eq!(run("integrate(1/(x^4+5), x, minf, inf);"), "%pi/(sqrt(5)*sqrt(2*sqrt(5)))");
    assert_eq!(run("integrate(1/(x^4+2), x, minf, inf);"), "%pi/(sqrt(2)*sqrt(2*sqrt(2)))");
}
#[test] fn biquadratic_leading_coeff() {
    assert_eq!(run("integrate(1/(2*x^4+2), x, minf, inf);"), "(1/2)*%pi/sqrt(2)");
}
