// S1: Associative-Commutative pattern matching edge cases.
use maxima_eval::eval_str;
fn run(s: &str) -> String { eval_str(s) }

// ---------- Commutativity ----------

#[test] fn ac_sum_order_independent() {
    // pattern 1+p must match x+1 (terms in any order)
    assert_eq!(
        run("matchdeclare(p,true)$ defrule(r, 1+p, p)$ apply1(x+1, r);"),
        "x"
    );
}

#[test] fn ac_product_order_independent() {
    assert_eq!(
        run("matchdeclare(u,true)$ defrule(r, 2*u, u)$ apply1(y*2, r);"),
        "y"
    );
}

// ---------- Rest variable absorbs leftover ----------

#[test] fn ac_rest_absorbs_two_terms() {
    // a+b on x+y+z: a takes one, b (last var) absorbs the rest.
    // Result list [a,b] with a=x, b=y+z; recursion then rewrites y+z too.
    let r = run("matchdeclare(a,true,b,true)$ defrule(r, a+b, ff(a,b))$ apply1(x+y+z, r);");
    // ff(x, y+z) — but the inner y+z may be rewritten recursively into ff(y,z).
    assert!(r.starts_with("ff("), "got: {}", r);
}

#[test] fn ac_rest_single_leftover() {
    assert_eq!(
        run("matchdeclare(a,true,b,true)$ defrule(r, a+b, gg(a,b))$ apply1(p+q, r);"),
        "gg(p,q)"
    );
}

// ---------- Subset matching (pattern smaller than subject, no rest var) ----------

#[test] fn ac_subset_pythagorean() {
    // sin(c)^2+cos(c)^2 (2 structured terms, no bare rest var) fires inside a
    // larger sum, leaving other terms intact.
    assert_eq!(
        run("matchdeclare(c,true)$ defrule(s, sin(c)^2+cos(c)^2, 1)$ apply1(sin(t)^2+cos(t)^2+w, s);"),
        "1+w"
    );
}

#[test] fn ac_subset_buried() {
    // The matching subset is not contiguous / not first.
    assert_eq!(
        run("matchdeclare(c,true)$ defrule(s, sin(c)^2+cos(c)^2, 1)$ apply1(a+sin(t)^2+b+cos(t)^2, s);"),
        "1+a+b"
    );
}

// ---------- Repeated pattern variables (non-linear) ----------

#[test] fn ac_repeated_var_consistent() {
    assert_eq!(
        run("matchdeclare(a,true)$ defrule(r, foo(a,a), a)$ apply1(foo(5,5), r);"),
        "5"
    );
}

#[test] fn ac_repeated_var_mismatch() {
    // foo(5,6) must NOT match foo(a,a)
    assert_eq!(
        run("matchdeclare(a,true)$ defrule(r, foo(a,a), a)$ apply1(foo(5,6), r);"),
        "foo(5,6)"
    );
}

// ---------- Predicate filtering ----------

#[test] fn ac_predicate_integerp_accepts() {
    let r = run("matchdeclare(n,integerp,x,true)$ defrule(r, x^n, captured(x,n))$ apply1(y^3, r);");
    assert!(r.contains("captured") && r.contains("3"), "got: {}", r);
}

#[test] fn ac_predicate_integerp_rejects() {
    // exponent is a symbol, not an integer → predicate fails → no rewrite
    let r = run("matchdeclare(n,integerp,x,true)$ defrule(r, x^n, captured)$ apply1(y^m, r);");
    assert!(r.contains("y") && r.contains("m"), "got: {}", r);
}

// ---------- matchdeclare paired form ----------

#[test] fn matchdeclare_pairs() {
    // Two var/predicate pairs in one matchdeclare call.
    let r = run("matchdeclare(a,true,b,true)$ defrule(r, a*b, prod(a,b))$ apply1(p*q, r);");
    assert!(r.starts_with("prod("), "got: {}", r);
}

// ---------- No spurious matches ----------

#[test] fn ac_no_match_passes_through() {
    assert_eq!(
        run("matchdeclare(c,true)$ defrule(s, sin(c)^2+cos(c)^2, 1)$ apply1(x+y, s);"),
        "x+y"
    );
}

#[test] fn ac_constant_in_pattern() {
    // Pattern with a literal constant term: 2+a matches 2+w → w, but 3+w no.
    assert_eq!(
        run("matchdeclare(a,true)$ defrule(r, 2+a, a)$ apply1(2+w, r);"),
        "w"
    );
    // 5+w has no literal `2` term, so `2+a` must NOT match.
    assert_eq!(
        run("matchdeclare(a,true)$ defrule(r, 2+a, hit)$ apply1(5+w, r);"),
        "5+w"
    );
}

// ---------- Resource cap: pathological input terminates ----------

#[test] fn ac_pathological_terminates() {
    // 5 all-true consuming vars + impossible rest predicate over a 14-term sum.
    // Must terminate (budget cap) and return unchanged, not hang.
    let r = run("matchdeclare(a,true,b,true,c,true,d,true,e,true,z,integerp)$ \
                 defrule(r, a+b+c+d+e+z, hit)$ \
                 apply1(p1+p2+p3+p4+p5+p6+p7+p8+p9+p10+p11+p12+p13+p14, r);");
    assert!(!r.contains("hit"), "should not match (z needs integer), got: {}", r);
}

// ---------- tellsimp with AC (S2) ----------
// NOTE: avoid sin^2+cos^2 here — that is a BUILT-IN simplification and would
// pass even without the rule. Use a rule with no built-in equivalent.

#[test] fn tellsimp_ac_plus_fires() {
    assert_eq!(
        run("matchdeclare(a,true,b,true)$ tellsimp(a+b+99, zzz)$ x+y+99;"),
        "zzz"
    );
}

#[test] fn tellsimp_ac_times_fires() {
    assert_eq!(
        run("matchdeclare(u,true)$ tellsimp(7*u, sevenfold)$ 7*w;"),
        "sevenfold"
    );
}

#[test] fn tellsimp_fires_in_subexpression() {
    // Rule on a sum must fire when the sum is nested inside a product.
    assert_eq!(
        run("matchdeclare(a,true,b,true)$ tellsimp(a+b+99, zzz)$ k*(x+y+99);"),
        "k*zzz"
    );
}

#[test] fn tellsimp_nonterminating_is_capped() {
    // h(a) -> h(a+1) never reaches a fixpoint; the iteration cap must stop it
    // at a bounded value rather than hang.
    let r = run("matchdeclare(a,true)$ tellsimp(h(a), h(a+1))$ h(0);");
    assert!(r.starts_with("h("), "should be bounded h(...), got: {}", r);
}

#[test] fn tellsimp_swap_rule_is_fixpoint() {
    // a+b -> b+a is a no-op after commutative simplification; must not loop.
    let r = run("matchdeclare(a,true,b,true)$ tellsimp(a+b, b+a)$ x+y;");
    assert!(r == "x+y" || r == "y+x", "got: {}", r);
}
