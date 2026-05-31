use maxima_core::{Expr, SymbolId};
use crate::simp::simplify;

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
        "nroots" => {
            if args.len() == 3 {
                if let Expr::Symbol(var_id) = &args[0] {
                    return None; // Need poly, not var
                }
                // nroots(poly, lo, hi) — count real roots in [lo, hi]
                // Detect variable from the polynomial
                let var_id = find_main_var(&args[0])?;
                let p = maxima_poly::expr_to_poly(&crate::eval::expand(&args[0]), var_id)?;
                let lo = crate::helpers::to_f64(&args[1])?;
                let hi = crate::helpers::to_f64(&args[2])?;
                let count = sturm_count(&p, lo, hi);
                Some(Expr::int(count as i64))
            } else { None }
        }
        "realroots" => {
            if args.len() >= 1 {
                let var_id = find_main_var(&args[0])?;
                let p = maxima_poly::expr_to_poly(&crate::eval::expand(&args[0]), var_id)?;
                let eps = if args.len() >= 2 { crate::helpers::to_f64(&args[1])? } else { 1e-10 };
                let roots = isolate_real_roots(&p, eps);
                let root_exprs: Vec<Expr> = roots.into_iter()
                    .map(|r| { let i = r.round() as i64; if (r - i as f64).abs() < eps { Expr::int(i) } else { Expr::Float(r) } })
                    .collect();
                Some(Expr::list(root_exprs))
            } else { None }
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

fn isolate_real_roots(p: &maxima_poly::Poly, eps: f64) -> Vec<f64> {
    let bound = root_bound(p);
    let seq = sturm_sequence(p);
    let total = sign_changes(&seq, -bound) - sign_changes(&seq, bound);
    if total == 0 { return vec![]; }

    let mut intervals: Vec<(f64, f64)> = vec![(-bound, bound)];
    let mut roots = Vec::new();

    for _ in 0..100 {
        let mut new_intervals = Vec::new();
        for (lo, hi) in &intervals {
            let n = sign_changes(&seq, *lo) - sign_changes(&seq, *hi);
            if n == 0 { continue; }
            if n == 1 {
                if hi - lo < eps {
                    roots.push((lo + hi) / 2.0);
                } else {
                    let mid = (lo + hi) / 2.0;
                    new_intervals.push((*lo, mid));
                    new_intervals.push((mid, *hi));
                }
            } else {
                let mid = (lo + hi) / 2.0;
                new_intervals.push((*lo, mid));
                new_intervals.push((mid, *hi));
            }
        }
        if new_intervals.is_empty() { break; }
        intervals = new_intervals;
    }
    roots.sort_by(|a, b| a.partial_cmp(b).unwrap());
    roots
}

fn gcd_u64(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd_u64(b, a % b) }
}
