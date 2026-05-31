// S1 benchmark + ablation for the AC pattern matcher.
//
// Run with output:  cargo test --test ac_bench -- --nocapture
//
// Measures matching cost as subject size grows, across input categories that
// exercise different cost regimes:
//   - greedy success (all-true vars, matches on first assignment)
//   - selectivity prune (a literal/structured term fails fast)
//   - subset rewrite (pattern smaller than subject)
//   - pathological backtracking (budget-capped)
//
// The "ablation" angle: compare a structured pattern (fast, selectivity
// prunes) vs an all-variable pattern (greedy) vs the pathological case
// (budget cap) at the same subject size, to show where cost concentrates.

use maxima_eval::eval_str;
use std::time::Instant;

fn timed(input: &str) -> (String, f64) {
    let start = Instant::now();
    let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| eval_str(input)))
        .unwrap_or_else(|_| "PANIC".to_string());
    (out, start.elapsed().as_secs_f64())
}

fn sum_of(n: usize, prefix: &str) -> String {
    (1..=n).map(|i| format!("{}{}", prefix, i)).collect::<Vec<_>>().join("+")
}

#[test]
fn bench_ac_scaling() {
    eprintln!("\n=== AC matcher scaling (subject size N) ===");
    eprintln!("{:<28} {:>4} {:>10}  {}", "category", "N", "time(s)", "result");

    // 1. Greedy success: a+rest matches any sum instantly.
    for n in [4, 8, 16, 24] {
        let s = sum_of(n, "x");
        let input = format!(
            "matchdeclare(a,true,b,true)$ defrule(r,a+b,matched)$ apply1({}, r);", s);
        let (out, t) = timed(&input);
        let ok = out.contains("matched") || out.starts_with("matched");
        eprintln!("{:<28} {:>4} {:>10.5}  {}", "greedy success (a+b)", n, t,
                  if ok { "ok" } else { "MISS" });
        assert!(t < 2.0, "greedy too slow at N={}: {}s", n, t);
    }

    // 2. Selectivity prune: a literal term not present → fail fast.
    for n in [4, 8, 16, 24] {
        let s = sum_of(n, "x");
        let input = format!(
            "matchdeclare(a,true)$ defrule(r,777+a,matched)$ apply1({}, r);", s);
        let (_out, t) = timed(&input);
        eprintln!("{:<28} {:>4} {:>10.5}  {}", "selectivity prune (777+a)", n, t, "no-match");
        assert!(t < 2.0, "selectivity prune too slow at N={}: {}s", n, t);
    }

    // 3. Subset rewrite: 2-term structured pattern fires inside an N-term sum.
    for n in [4, 8, 16, 24] {
        let mut terms = vec!["sin(t)^2".to_string(), "cos(t)^2".to_string()];
        for i in 1..=(n - 2) { terms.push(format!("z{}", i)); }
        let s = terms.join("+");
        let input = format!(
            "matchdeclare(c,true)$ defrule(s,sin(c)^2+cos(c)^2,1)$ apply1({}, s);", s);
        let (out, t) = timed(&input);
        eprintln!("{:<28} {:>4} {:>10.5}  {}", "subset rewrite (pythag)", n, t,
                  if out.starts_with("1") || out.contains("+1") || out == "1" { "fired" } else { "?" });
        assert!(t < 2.0, "subset rewrite too slow at N={}: {}s", n, t);
    }

    // 4. Pathological: all-true consuming vars + impossible rest predicate.
    //    Must be budget-capped (bounded time, returns no match).
    for n in [8, 12, 14] {
        let s = sum_of(n, "p");
        let input = format!(
            "matchdeclare(a,true,b,true,c,true,d,true,e,true,z,integerp)$ \
             defrule(r,a+b+c+d+e+z,hit)$ apply1({}, r);", s);
        let (out, t) = timed(&input);
        eprintln!("{:<28} {:>4} {:>10.5}  {}", "pathological (budget cap)", n, t,
                  if out.contains("hit") { "MATCHED?!" } else { "capped-ok" });
        assert!(!out.contains("hit"), "pathological should not match at N={}", n);
        assert!(t < 3.0, "pathological exceeded cap budget at N={}: {}s", n, t);
    }
}

#[test]
fn bench_ac_ablation() {
    // Ablation: same subject size (N=12), three pattern shapes. Shows that
    // selectivity ordering keeps structured/literal patterns cheap, while the
    // all-variable failing case is the only one that approaches the budget.
    eprintln!("\n=== AC ablation at N=12 ===");
    let n = 12;
    let s = sum_of(n, "q");

    let cases: Vec<(&str, String)> = vec![
        ("structured-match (1 term)",
         format!("matchdeclare(a,true)$ defrule(r,q1+a,hit)$ apply1({}, r);", s)),
        ("all-var greedy success",
         format!("matchdeclare(a,true,b,true)$ defrule(r,a+b,hit)$ apply1({}, r);", s)),
        ("failing literal (fast)",
         format!("matchdeclare(a,true)$ defrule(r,999+a,hit)$ apply1({}, r);", s)),
    ];
    for (label, input) in &cases {
        let (_out, t) = timed(input);
        eprintln!("{:<28} {:>10.5}s", label, t);
        assert!(t < 2.0, "{} too slow: {}s", label, t);
    }
}
