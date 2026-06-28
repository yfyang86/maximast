// V13 3e: Fourier transform F(ω)=∫f(x)e^{-iωx}dx — canonical pairs.
use maxima_eval::eval_str;
fn run(s: &str) -> String { eval_str(s) }

#[test] fn gaussian() {
    assert_eq!(run("fourier_transform(exp(-x^2), x, w);"), "exp(-w^2/4)*sqrt(%pi)");
    assert_eq!(run("fourier_transform(exp(-2*x^2), x, w);"), "exp(-w^2/8)*sqrt(%pi/2)");
}
#[test] fn two_sided_exponential() {
    assert_eq!(run("fourier_transform(exp(-abs(x)), x, w);"), "2/(1+w^2)");
    assert_eq!(run("fourier_transform(exp(-3*abs(x)), x, w);"), "6/(9+w^2)");
}
#[test] fn lorentzian() {
    assert_eq!(run("fourier_transform(1/(x^2+1), x, w);"), "%pi*exp(-abs(w))");
    assert_eq!(run("fourier_transform(1/(x^2+4), x, w);"), "%pi*exp(-2*abs(w))/2");
}
#[test] fn linearity() {
    assert_eq!(run("fourier_transform(2*exp(-x^2), x, w);"), "2*exp(-w^2/4)*sqrt(%pi)");
}

// Rational P/Q via residues (assumes ω>0): F = C(ω) − i·S(ω).
#[test] fn rational_odd_numerator() {
    assert_eq!(run("fourier_transform(x/(x^2+1), x, w);"), "-%i*%pi*exp(-w)");
}
#[test] fn rational_shifted_pole() {
    assert_eq!(run("fourier_transform(1/(x^2-2*x+2), x, w);"), "%pi*exp(-%i*w)*exp(-w)");
}
#[test] fn rational_two_quadratics() {
    assert_eq!(run("fourier_transform(1/((x^2+1)*(x^2+4)), x, w);"),
        "(-1/6)*%pi*exp(-2*w)+(1/3)*%pi*exp(-w)");
}
#[test] fn rational_real_pole_is_noun() {
    assert_eq!(run("fourier_transform(1/(x^3+1), x, w);"), "fourier_transform(1/(1+x^3),x,w)");
}
