use maxima_core::{Expr, SymbolId};
use crate::simp::simplify;
use num::{BigInt, BigRational, One, Zero, Signed};

pub(crate) fn eval_poly_func(name: &str, args: &[Expr]) -> Option<Expr> {
    match name {
        "resultant" => {
            if args.len() == 3 {
                if let Expr::Symbol(var_id) = &args[2] {
                    resultant(&args[0], &args[1], *var_id)
                } else { None }
            } else { None }
        }
        "discriminant" => {
            if args.len() == 2 {
                if let Expr::Symbol(var_id) = &args[1] {
                    discriminant(&args[0], *var_id)
                } else { None }
            } else { None }
        }
        "content" => {
            if args.len() == 2 {
                if let Expr::Symbol(var_id) = &args[1] {
                    content_poly(&args[0], *var_id)
                } else { None }
            } else { None }
        }
        "primpart" => {
            if args.len() == 2 {
                if let Expr::Symbol(var_id) = &args[1] {
                    primpart(&args[0], *var_id)
                } else { None }
            } else { None }
        }
        _ => None,
    }
}

fn resultant(p: &Expr, q: &Expr, var: SymbolId) -> Option<Expr> {
    // Fast path: integer/rational coefficients via maxima_poly.
    if let (Some(pp), Some(pq)) = (
        maxima_poly::expr_to_poly(&crate::eval::expand(p), var),
        maxima_poly::expr_to_poly(&crate::eval::expand(q), var),
    ) {
        let res = maxima_poly::resultant(&pp, &pq);
        return Some(coeff_to_expr(&res));
    }
    // Symbolic-coefficient fallback (e.g. resultant(x^2+a, x+b, x)).
    let vexpr = Expr::Symbol(var);
    let pp = crate::poly_expr::PolyExpr::from_expr(p, &vexpr)?;
    let pq = crate::poly_expr::PolyExpr::from_expr(q, &vexpr)?;
    crate::poly_expr::resultant(&pp, &pq)
}

fn discriminant(p: &Expr, var: SymbolId) -> Option<Expr> {
    // Fast path: integer/rational coefficients via maxima_poly.
    if let Some(pp) = maxima_poly::expr_to_poly(&crate::eval::expand(p), var) {
        let deg = pp.degree()?;
        if deg < 2 { return Some(Expr::int(0)); }
        let dp = pp.derivative();
        let res = maxima_poly::resultant(&pp, &dp);
        let res_expr = coeff_to_expr(&res);
        let lc = pp.leading_coeff();
        let lc_expr = coeff_to_expr(&lc);
        let sign = if (deg * (deg - 1) / 2) % 2 == 0 { 1 } else { -1 };
        let signed = if sign == -1 { simplify(&Expr::neg(res_expr)) } else { res_expr };
        return Some(divide_reduced(&signed, &lc_expr));
    }
    // Symbolic-coefficient fallback (e.g. discriminant(a*x^2+b*x+c, x)).
    let vexpr = Expr::Symbol(var);
    let pp = crate::poly_expr::PolyExpr::from_expr(p, &vexpr)?;
    crate::poly_expr::discriminant(&pp, &vexpr)
}

fn content_poly(p: &Expr, var: SymbolId) -> Option<Expr> {
    let pp = maxima_poly::expr_to_poly(&crate::eval::expand(p), var)?;
    let c = pp.content();
    Some(coeff_to_expr(&c))
}

fn primpart(p: &Expr, var: SymbolId) -> Option<Expr> {
    let pp = maxima_poly::expr_to_poly(&crate::eval::expand(p), var)?;
    let c = pp.content();
    if c.is_one() || c.is_zero() { return Some(maxima_poly::poly_to_expr(&pp)); }
    // Divide each coefficient by the content
    let inv_c = match &c {
        maxima_poly::Coeff::Int(n) => maxima_poly::Coeff::Rat(1, *n),
        maxima_poly::Coeff::Rat(n, d) => maxima_poly::Coeff::Rat(*d, *n),
    };
    let prim = pp.scale(&inv_c);
    Some(maxima_poly::poly_to_expr(&prim))
}

/// Divide `num` by `den`, reducing to a fully simplified integer/rational when
/// both are numeric (a bare `simplify(div(...))` leaves e.g. 50/2 unreduced).
fn divide_reduced(num: &Expr, den: &Expr) -> Expr {
    if let (Expr::Integer(n), Expr::Integer(d)) = (num, den) {
        if *d != 0 {
            let sign = if (*n < 0) ^ (*d < 0) { -1 } else { 1 };
            let (na, da) = (n.unsigned_abs(), d.unsigned_abs());
            let g = gcd_u64(na, da).max(1);
            let nn = (na / g) as i64 * sign;
            let dd = (da / g) as i64;
            return if dd == 1 { Expr::int(nn) } else { Expr::Rational { num: nn, den: dd } };
        }
    }
    simplify(&Expr::div(num.clone(), den.clone()))
}

fn coeff_to_expr(c: &maxima_poly::Coeff) -> Expr {
    match c {
        maxima_poly::Coeff::Int(n) => Expr::int(*n),
        maxima_poly::Coeff::Rat(n, d) => {
            if *d == 1 { Expr::int(*n) }
            else { Expr::Rational { num: *n, den: *d } }
        }
    }
}

pub(crate) fn eval_sturm_func(name: &str, args: &[Expr]) -> Option<Expr> {
    match name {
        "sturm" => {
            // sturm(poly[, x]) → the Sturm sequence as a list of polynomials.
            if args.is_empty() { return None; }
            let var_id = find_main_var(&args[0])?;
            let p = maxima_poly::expr_to_poly(&crate::eval::expand(&args[0]), var_id)?;
            let seq = sturm_sequence(&p);
            Some(Expr::list(seq.iter().map(maxima_poly::poly_to_expr).collect()))
        }
        "nroots" => {
            if matches!(args.first(), Some(Expr::Symbol(_))) {
                return None; // Need poly, not a bare var
            }
            let var_id = find_main_var(&args[0])?;
            let p = maxima_poly::expr_to_poly(&crate::eval::expand(&args[0]), var_id)?;
            let count = if args.len() >= 3 {
                // nroots(poly, lo, hi) — count distinct real roots in [lo, hi].
                let lo = crate::helpers::to_f64(&args[1])?;
                let hi = crate::helpers::to_f64(&args[2])?;
                sturm_count(&p, lo, hi)
            } else {
                // nroots(poly) — all real roots (over the Cauchy root bound).
                let b = root_bound(&p) + 1.0;
                sturm_count(&p, -b, b)
            };
            Some(Expr::int(count as i64))
        }
        "realroots" => {
            // Exact real-root isolation. Factor over Q: each linear factor gives
            // an exact rational root; each higher (irreducible) factor's real
            // roots are irrational and isolated by Sturm bisection in exact
            // rational arithmetic, returned as a rational within eps. Result is
            // Maxima-style `[x = r, ...]` of exact rationals (no f64). eps is a
            // rational (default 10^-10).
            if args.is_empty() { return None; }
            let var_id = find_main_var(&args[0])?;
            let p = maxima_poly::expr_to_poly(&crate::eval::expand(&args[0]), var_id)?;
            let eps = match args.get(1) {
                Some(a) => crate::helpers::expr_to_bigrat(a)?.abs(),
                None => BigRational::new(BigInt::one(), BigInt::from(10).pow(10)),
            };
            if eps.is_zero() { return None; }
            let mut roots: Vec<BigRational> = Vec::new();
            for (f, _m) in &maxima_poly::factor_poly(&p) {
                match f.degree() {
                    Some(0) | None => continue,
                    Some(1) => {
                        // a·x + b = 0 → root −b/a (exact rational)
                        let a = coeff_bigrat(&f.leading_coeff());
                        let b = coeff_bigrat(&f.constant_term());
                        roots.push(-b / a);
                    }
                    Some(_) => roots.extend(isolate_exact(f, &eps)),
                }
            }
            roots.sort();
            roots.dedup();
            let var = Expr::Symbol(var_id);
            let eqs: Vec<Expr> = roots.iter().map(|r| Expr::List {
                op: maxima_core::Operator::MEqual,
                simplified: false,
                args: vec![var.clone(), crate::helpers::bigrat_to_expr(r)],
            }).collect();
            Some(Expr::list(eqs))
        }
        _ => None,
    }
}

fn find_main_var(expr: &Expr) -> Option<maxima_core::SymbolId> {
    match expr {
        Expr::Symbol(id) => Some(*id),
        Expr::List { args, .. } => {
            for a in args {
                if let Some(id) = find_main_var(a) {
                    let name = maxima_core::resolve(id);
                    if name.len() == 1 && name.chars().next().unwrap().is_alphabetic() {
                        return Some(id);
                    }
                }
            }
            args.iter().find_map(|a| find_main_var(a))
        }
        _ => None,
    }
}

fn sturm_sequence(p: &maxima_poly::Poly) -> Vec<maxima_poly::Poly> {
    let mut seq = vec![p.clone(), p.derivative()];
    loop {
        let n = seq.len();
        if seq[n-1].is_zero() { break; }
        if let Some((_, rem)) = seq[n-2].divmod(&seq[n-1]) {
            let neg_rem = rem.neg();
            if neg_rem.is_zero() { break; }
            seq.push(neg_rem);
        } else { break; }
    }
    seq
}

fn sign_changes(seq: &[maxima_poly::Poly], x: f64) -> usize {
    let vals: Vec<f64> = seq.iter().map(|p| eval_poly_f64(p, x)).filter(|v| v.abs() > 1e-15).collect();
    let mut changes = 0;
    for i in 1..vals.len() {
        if vals[i-1] * vals[i] < 0.0 { changes += 1; }
    }
    changes
}

fn eval_poly_f64(p: &maxima_poly::Poly, x: f64) -> f64 {
    let mut result = 0.0;
    for (e, c) in &p.terms {
        let cv = match c {
            maxima_poly::Coeff::Int(n) => *n as f64,
            maxima_poly::Coeff::Rat(n, d) => *n as f64 / *d as f64,
        };
        result += cv * x.powi(*e as i32);
    }
    result
}

fn root_bound(p: &maxima_poly::Poly) -> f64 {
    let lc = match p.leading_coeff() {
        maxima_poly::Coeff::Int(n) => n.abs() as f64,
        maxima_poly::Coeff::Rat(n, d) => (n as f64 / d as f64).abs(),
    };
    if lc < 1e-15 { return 1.0; }
    let mut max_ratio = 0.0f64;
    for (_, c) in &p.terms {
        let cv = match c {
            maxima_poly::Coeff::Int(n) => n.abs() as f64,
            maxima_poly::Coeff::Rat(n, d) => (*n as f64 / *d as f64).abs(),
        };
        max_ratio = max_ratio.max(cv / lc);
    }
    1.0 + max_ratio
}

fn sturm_count(p: &maxima_poly::Poly, lo: f64, hi: f64) -> usize {
    let sqf = p.clone(); // ideally square-free, but use as-is for now
    let seq = sturm_sequence(&sqf);
    let v_lo = sign_changes(&seq, lo);
    let v_hi = sign_changes(&seq, hi);
    if v_lo >= v_hi { v_lo - v_hi } else { 0 }
}

fn coeff_bigrat(c: &maxima_poly::Coeff) -> BigRational {
    match c {
        maxima_poly::Coeff::Int(n) => BigRational::from(BigInt::from(*n)),
        maxima_poly::Coeff::Rat(n, d) => BigRational::new(BigInt::from(*n), BigInt::from(*d)),
    }
}

/// A polynomial as (exponent, exact coefficient) terms for rational evaluation.
fn poly_bigrat_terms(p: &maxima_poly::Poly) -> Vec<(usize, BigRational)> {
    p.terms.iter().map(|(e, c)| (*e as usize, coeff_bigrat(c))).collect()
}

fn eval_bigrat(terms: &[(usize, BigRational)], q: &BigRational) -> BigRational {
    let mut s = BigRational::zero();
    for (e, c) in terms { s += c * num::pow(q.clone(), *e); }
    s
}

fn sign_bigrat(x: &BigRational) -> i32 {
    if x.is_positive() { 1 } else if x.is_negative() { -1 } else { 0 }
}

/// Sign variations of the Sturm chain (each as exact terms) evaluated at q.
fn sign_changes_bigrat(seq: &[Vec<(usize, BigRational)>], q: &BigRational) -> usize {
    let mut last = 0i32;
    let mut changes = 0;
    for poly in seq {
        let s = sign_bigrat(&eval_bigrat(poly, q));
        if s != 0 {
            if last != 0 && s != last { changes += 1; }
            last = s;
        }
    }
    changes
}

/// Cauchy bound 1 + max|a_i/a_n| as an exact rational; all real roots lie in
/// (−bound, bound).
fn root_bound_bigrat(terms: &[(usize, BigRational)], deg: usize) -> BigRational {
    let lc = terms.iter().find(|(e, _)| *e == deg).map(|(_, c)| c.clone())
        .unwrap_or_else(BigRational::one);
    let mut maxr = BigRational::zero();
    for (e, c) in terms {
        if *e != deg {
            let r = (c / &lc).abs();
            if r > maxr { maxr = r; }
        }
    }
    maxr + BigRational::one()
}

/// Real roots of a square-free, irreducible-over-Q factor (degree ≥ 2, so the
/// real roots are irrational), each isolated by Sturm bisection in exact
/// rational arithmetic and returned as the rational midpoint of an interval of
/// width < eps. Because the factor is irreducible no rational bisection point
/// is ever a root, so the Sturm count V(lo)−V(hi) is exact at every split.
fn isolate_exact(p: &maxima_poly::Poly, eps: &BigRational) -> Vec<BigRational> {
    let seq: Vec<Vec<(usize, BigRational)>> =
        sturm_sequence(p).iter().map(poly_bigrat_terms).collect();
    let pterms = poly_bigrat_terms(p);
    let deg = p.degree().unwrap_or(0) as usize;
    let bound = root_bound_bigrat(&pterms, deg);
    let neg_bound = -bound.clone();
    if sign_changes_bigrat(&seq, &neg_bound) <= sign_changes_bigrat(&seq, &bound) {
        return vec![];
    }
    let two = BigRational::from(BigInt::from(2));
    let mut intervals = vec![(neg_bound, bound)];
    let mut roots = Vec::new();
    let mut guard = 0;
    while !intervals.is_empty() && guard < 100_000 {
        guard += 1;
        let mut next = Vec::new();
        for (lo, hi) in intervals {
            let v_lo = sign_changes_bigrat(&seq, &lo);
            let v_hi = sign_changes_bigrat(&seq, &hi);
            let n = v_lo.saturating_sub(v_hi);
            if n == 0 { continue; }
            if n == 1 && &hi - &lo < *eps {
                roots.push((&lo + &hi) / &two);
            } else {
                let mid = (&lo + &hi) / &two;
                next.push((lo, mid.clone()));
                next.push((mid, hi));
            }
        }
        intervals = next;
    }
    roots.sort();
    roots
}

fn gcd_u64(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd_u64(b, a % b) }
}
