use maxima_eval::eval_str;
fn run(s: &str) -> String { eval_str(s) }

// defrule + apply1
#[test] fn defrule_apply1() {
    assert_eq!(
        run("matchdeclare(a, true); defrule(r1, sin(a)^2, 1-cos(a)^2); apply1(sin(x)^2, r1);"),
        "1-cos(x)^2"
    );
}

// Regression: tellsimp rules must actually fire (were stored but never applied).
#[test] fn tellsimp_constant() {
    assert_eq!(run("tellsimp(foo(0), 42); foo(0);"), "42");
}

#[test] fn tellsimp_unaffected() {
    // A non-matching call must pass through unchanged.
    assert_eq!(run("tellsimp(foo(0), 42); foo(1);"), "foo(1)");
}

#[test] fn tellsimp_pattern() {
    // Pattern variable binding in tellsimp.
    assert_eq!(run("matchdeclare(a, true); tellsimp(g(a,a), a); g(5,5);"), "5");
}
