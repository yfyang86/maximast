use maxima_eval::eval_str;
fn run(s: &str) -> String { eval_str(s) }

// Forward Laplace transform
#[test] fn laplace_1() { assert_eq!(run("laplace(1, t, s);"), "1/s"); }
#[test] fn laplace_t() { assert_eq!(run("laplace(t, t, s);"), "1/s^2"); }
#[test] fn laplace_t3() { assert_eq!(run("laplace(t^3, t, s);"), "6/s^4"); }
#[test] fn laplace_exp() { assert_eq!(run("laplace(exp(a*t), t, s);"), "1/(-a+s)"); }
#[test] fn laplace_sin() { assert_eq!(run("laplace(sin(w*t), t, s);"), "w/(s^2+w^2)"); }
#[test] fn laplace_cos() { assert_eq!(run("laplace(cos(w*t), t, s);"), "s/(s^2+w^2)"); }

#[test] fn laplace_linearity() {
    let r = run("laplace(3*t^2+2*t+1, t, s);");
    assert!(r.contains("/s") && r.contains("1/s"), "got: {}", r);
}

#[test] fn laplace_shift() {
    let r = run("laplace(exp(a*t)*sin(w*t), t, s);");
    assert!(r.contains("w") && r.contains("-a+s"), "got: {}", r);
}

#[test] fn laplace_exp_t3() {
    let r = run("laplace(exp(-2*t)*t^3, t, s);");
    assert!(r.contains("6") && r.contains("2+s"), "got: {}", r);
}

// Inverse Laplace transform
#[test] fn ilt_1_over_s() { assert_eq!(run("ilt(1/s, s, t);"), "1"); }
#[test] fn ilt_1_over_s3() {
    let r = run("ilt(1/s^3, s, t);");
    assert!(r.contains("t^2"), "got: {}", r);
}
#[test] fn ilt_exp() { assert_eq!(run("ilt(1/(s-a), s, t);"), "exp(a*t)"); }
#[test] fn ilt_cos() { assert_eq!(run("ilt(s/(s^2+4), s, t);"), "cos(2*t)"); }
#[test] fn ilt_sin() { assert_eq!(run("ilt(3/(s^2+9), s, t);"), "sin(3*t)"); }

#[test] fn roundtrip_exp() {
    let lt = run("laplace(exp(3*t), t, s);");
    let back = run(&format!("ilt({}, s, t);", lt));
    assert_eq!(back, "exp(3*t)");
}

// General rational inverse Laplace via partial fractions over Q (V13 1d).
#[test] fn ilt_pfd_distinct_real() {
    // 6/((s+1)(s+2)(s+3)) → 3e^-t − 6e^-2t + 3e^-3t (each residue exact)
    let r = run("ilt(6/((s+1)*(s+2)*(s+3)), s, t);");
    assert!(!r.contains("ilt"), "should invert, got: {}", r);
    assert!(r.contains("exp(-t)") && r.contains("exp(-2*t)") && r.contains("exp(-3*t)"), "got: {}", r);
}
#[test] fn ilt_sinh() {
    // 1/(s^2-1) = sinh(t) = (e^t - e^-t)/2
    assert_eq!(run("ilt(1/(s^2-1), s, t);"), "(1/2)*exp(t)-(1/2)*exp(-t)");
}
#[test] fn ilt_repeated_pole() {
    // 1/((s-1)^2 (s+2)): repeated real pole → t·e^t term present
    let r = run("ilt(1/((s-1)^2*(s+2)), s, t);");
    assert!(!r.contains("ilt") && r.contains("t*exp(t)"), "got: {}", r);
}
#[test] fn ilt_damped_oscillation() {
    // s/(s^2+2s+5): complex poles −1±2i → e^-t(cos2t − sin2t/2)
    let r = run("ilt(s/(s^2+2*s+5), s, t);");
    assert!(!r.contains("ilt") && r.contains("exp(-t)") && r.contains("cos(2*t)") && r.contains("sin(2*t)"), "got: {}", r);
}
#[test] fn ilt_one_minus_cos() {
    assert_eq!(run("ilt(1/(s*(s^2+1)), s, t);"), "1-cos(t)");
}
#[test] fn ilt_roundtrip_general() {
    // laplace(ilt(F)) reconstructs F for a few rationals
    assert_eq!(run("laplace(ilt(1/(s^2+1), s, t), t, s);"), "1/(1+s^2)");
    assert_eq!(run("laplace(ilt(1/(s^2*(s+1)), s, t), t, s);"), "1/s^2-1/s+1/(1+s)");
}
