//! Factorial/Gamma/Pochhammer/binomial rewriting — the simplification layer for
//! hypergeometric closed forms (R2 installment 2).
//!
//! - `makefact` rewrites binomial / pochhammer / gamma into factorials.
//! - `minfactorial` collapses ratios of factorials whose arguments differ by an
//!   integer into finite products (e.g. n!/(n−2)! → n·(n−1)).

use maxima_core::{Expr, Operator, intern};
use crate::simp::simplify;

fn fact(x: Expr) -> Expr { Expr::call("factorial", vec![x]) }

fn named<'a>(e: &'a Expr, name: &str, arity: usize) -> Option<&'a [Expr]> {
    if let Expr::List { op: Operator::Named(id), args, .. } = e {
        if *id == intern(name) && args.len() == arity { return Some(args); }
    }
    None
}

/// Rewrite binomial / pochhammer / gamma into factorials, recursively.
pub fn makefact(e: &Expr) -> Expr {
    if let Some(a) = named(e, "binomial", 2) {
        let (x, y) = (makefact(&a[0]), makefact(&a[1]));
        return Expr::div(fact(x.clone()), Expr::mul(fact(y.clone()), fact(Expr::sub(x, y))));
    }
    if let Some(a) = named(e, "pochhammer", 2) {
        // (a)_n = Γ(a+n)/Γ(a) = (a+n−1)!/(a−1)!
        let (x, n) = (makefact(&a[0]), makefact(&a[1]));
        return Expr::div(
            fact(Expr::sub(Expr::add(x.clone(), n), Expr::int(1))),
            fact(Expr::sub(x, Expr::int(1))),
        );
    }
    if let Some(a) = named(e, "gamma", 1) {
        return fact(Expr::sub(makefact(&a[0]), Expr::int(1)));
    }
    match e {
        Expr::List { op, args, .. } => Expr::List {
            op: *op,
            simplified: false,
            args: args.iter().map(makefact).collect(),
        },
        _ => e.clone(),
    }
}

/// If `simplify(a − b)` is a nonnegative integer ≤ 64, return it.
fn int_diff(a: &Expr, b: &Expr) -> Option<i64> {
    match simplify(&Expr::sub(a.clone(), b.clone())) {
        Expr::Integer(d) if (0..=64).contains(&d) => Some(d),
        _ => None,
    }
}

fn as_factorial(e: &Expr) -> Option<Expr> {
    named(e, "factorial", 1).map(|a| a[0].clone())
}

/// Collapse factorial ratios with integer-differing arguments into products.
/// Works on a single Times node: collects factorial args in the numerator and
/// denominator (the latter as `factorial(·)^(−1)`), cancels matching pairs.
pub fn minfactorial(e: &Expr) -> Expr {
    // Recurse into children first.
    let e = match e {
        Expr::List { op, args, .. } => Expr::List {
            op: *op,
            simplified: false,
            args: args.iter().map(minfactorial).collect(),
        },
        _ => e.clone(),
    };
    let args = match &e {
        Expr::List { op: Operator::MTimes, args, .. } => args.clone(),
        _ => return e,
    };

    // Split factors into numerator factorials, denominator factorials, and rest.
    let mut num_f: Vec<Expr> = Vec::new();
    let mut den_f: Vec<Expr> = Vec::new();
    let mut rest: Vec<Expr> = Vec::new();
    for f in &args {
        if let Some(arg) = as_factorial(f) { num_f.push(arg); continue; }
        if let Expr::List { op: Operator::MExpt, args: pa, .. } = f {
            if pa.len() == 2 {
                if let Expr::Integer(-1) = &pa[1] {
                    // Reciprocal: the base may be a single factorial or a product.
                    if let Some(arg) = as_factorial(&pa[0]) { den_f.push(arg); continue; }
                    if let Expr::List { op: Operator::MTimes, args: ba, .. } = &pa[0] {
                        for sub in ba {
                            if let Some(sa) = as_factorial(sub) { den_f.push(sa); }
                            else { rest.push(Expr::pow(sub.clone(), Expr::int(-1))); }
                        }
                        continue;
                    }
                }
            }
        }
        rest.push(f.clone());
    }

    // Cancel each denominator factorial against a numerator factorial whose
    // argument exceeds it by a nonnegative integer: A!/B! = ∏_{i=1}^{A−B}(B+i).
    let mut produced: Vec<Expr> = Vec::new();
    let mut den_keep: Vec<Expr> = Vec::new();
    for b in den_f {
        let mut matched = None;
        for (i, a) in num_f.iter().enumerate() {
            if let Some(d) = int_diff(a, &b) { matched = Some((i, d)); break; }
        }
        match matched {
            Some((i, d)) => {
                num_f.remove(i);
                for t in 1..=d { produced.push(Expr::add(b.clone(), Expr::int(t))); }
            }
            None => den_keep.push(b),
        }
    }

    // Reassemble: rest · produced · (remaining numerator factorials) / (kept denom).
    let mut out = Expr::int(1);
    for r in rest { out = Expr::mul(out, r); }
    for p in produced { out = Expr::mul(out, p); }
    for a in num_f { out = Expr::mul(out, fact(a)); }
    for b in den_keep { out = Expr::div(out, fact(b)); }
    simplify(&out)
}

#[cfg(test)]
mod tests {
    use crate::eval::eval_str;
    fn run(s: &str) -> String { eval_str(s) }

    #[test] fn pochhammer_expands() { assert_eq!(run("pochhammer(a,3);"), "a*(1+a)*(2+a)"); }
    #[test] fn gamma_integer() { assert_eq!(run("gamma(5);"), "24"); }
    #[test] fn gamma_half() { assert_eq!(run("gamma(1/2);"), "%pi^(1/2)"); }
    #[test] fn minfactorial_ratio() { assert_eq!(run("minfactorial(factorial(n)/factorial(n-2));"), "n*(-1+n)"); }
    #[test] fn makefact_then_min() {
        assert_eq!(run("minfactorial(makefact(binomial(n,2)));"), "(1/2)*n*(-1+n)");
    }
}
