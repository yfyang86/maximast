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
