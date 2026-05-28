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
