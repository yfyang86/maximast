use maxima_core::{Expr, Operator, resolve};
use crate::simp::simplify;
use crate::helpers::{contains_var, to_i64};

pub(crate) fn eval_laplace(name: &str, args: &[Expr], env: &mut crate::env::Environment) -> Option<Expr> {
    match name {
        "laplace" => {
            if args.len() == 3 {
                if let (Expr::Symbol(_t_id), Expr::Symbol(_s_id)) = (&args[1], &args[2]) {
                    return Some(laplace_transform(&args[0], &args[1], &args[2]));
                }
            }
            None
        }
        "ilt" => {
            if args.len() == 3 {
                if let (Expr::Symbol(_s_id), Expr::Symbol(_t_id)) = (&args[1], &args[2]) {
                    // meval folds residual numeric factors (e.g. 3·sin(3t)/3)
                    // that the structural simplifier leaves alone.
                    let r = inverse_laplace(&args[0], &args[1], &args[2]);
                    return Some(crate::eval::meval(&r, env));
                }
            }
            None
        }
        _ => None,
    }
}

fn laplace_transform(f: &Expr, t: &Expr, s: &Expr) -> Expr {
    // Linearity: L{a*f + b*g} = a*L{f} + b*L{g}
    if let Expr::List { op: Operator::MPlus, args, .. } = f {
        let terms: Vec<Expr> = args.iter().map(|a| laplace_transform(a, t, s)).collect();
        if terms.iter().any(|r| is_noun_laplace(r)) {
            return Expr::call("laplace", vec![f.clone(), t.clone(), s.clone()]);
        }
        return simplify(&Expr::List { op: Operator::MPlus, simplified: false, args: terms });
    }

    // Factor out constants: L{c*f(t)} = c*L{f(t)}
    if let Expr::List { op: Operator::MTimes, args, .. } = f {
        let (constant, dependent): (Vec<&Expr>, Vec<&Expr>) =
            args.iter().partition(|a| !contains_var(a, t));
        if !constant.is_empty() && !dependent.is_empty() {
            let c = if constant.len() == 1 { constant[0].clone() }
                else { simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: constant.into_iter().cloned().collect() }) };
            let inner = if dependent.len() == 1 { dependent[0].clone() }
                else { Expr::List { op: Operator::MTimes, simplified: false, args: dependent.into_iter().cloned().collect() } };
            let lt = laplace_transform(&inner, t, s);
            if !is_noun_laplace(&lt) {
                return simplify(&Expr::mul(c, lt));
            }
        }
    }

    // Table entries
    if let Some(result) = laplace_table(f, t, s) {
        return result;
    }

    // Shift theorem: L{exp(a*t)*f(t)} = F(s-a)
    if let Expr::List { op: Operator::MTimes, args, .. } = f {
        for (i, arg) in args.iter().enumerate() {
            if let Some(a) = extract_exp_coeff(arg, t) {
                let rest: Vec<Expr> = args.iter().enumerate()
                    .filter(|(j, _)| *j != i).map(|(_, e)| e.clone()).collect();
                let inner = if rest.len() == 1 { rest[0].clone() }
                    else { Expr::List { op: Operator::MTimes, simplified: false, args: rest } };
                let shifted_s = simplify(&Expr::sub(s.clone(), a));
                let lt = laplace_transform(&inner, t, &shifted_s);
                if !is_noun_laplace(&lt) {
                    return lt;
                }
            }
        }
    }

    Expr::call("laplace", vec![f.clone(), t.clone(), s.clone()])
}

fn laplace_table(f: &Expr, t: &Expr, s: &Expr) -> Option<Expr> {
    // L{1} = 1/s
    if !contains_var(f, t) {
        return Some(simplify(&Expr::mul(f.clone(), Expr::pow(s.clone(), Expr::int(-1)))));
    }

    // L{t} = 1/s^2
    if f == t {
        return Some(simplify(&Expr::pow(s.clone(), Expr::int(-2))));
    }

    // L{t^n} = n!/s^(n+1)
    if let Expr::List { op: Operator::MExpt, args, .. } = f {
        if args.len() == 2 && args[0] == *t {
            if let Some(n) = to_i64(&args[1]) {
                if n >= 0 {
                    let fact = factorial(n);
                    return Some(simplify(&Expr::div(
                        Expr::int(fact),
                        Expr::pow(s.clone(), Expr::int(n + 1)))));
                }
            }
        }
    }

    // L{exp(a*t)} = 1/(s-a)
    if let Some(a) = extract_exp_coeff(f, t) {
        return Some(simplify(&Expr::pow(
            Expr::sub(s.clone(), a), Expr::int(-1))));
    }

    // Named functions of t
    if let Expr::List { op: Operator::Named(id), args: fa, .. } = f {
        let fname = resolve(*id);
        if fa.len() == 1 {
            // L{sin(w*t)} = w/(s^2+w^2)
            // L{cos(w*t)} = s/(s^2+w^2)
            // L{sinh(w*t)} = w/(s^2-w^2)
            // L{cosh(w*t)} = s/(s^2-w^2)
            if let Some(w) = extract_linear_coeff(&fa[0], t) {
                let w2 = simplify(&Expr::mul(w.clone(), w.clone()));
                let s2 = Expr::pow(s.clone(), Expr::int(2));
                match fname.as_str() {
                    "sin" => return Some(simplify(&Expr::div(w, Expr::add(s2, w2)))),
                    "cos" => return Some(simplify(&Expr::div(s.clone(), Expr::add(s2, w2)))),
                    "sinh" => return Some(simplify(&Expr::div(w, Expr::sub(s2, w2)))),
                    "cosh" => return Some(simplify(&Expr::div(s.clone(), Expr::sub(s2, w2)))),
                    _ => {}
                }
            }
            // L{exp(a*t)} already handled above
        }
    }

    None
}

fn inverse_laplace(f: &Expr, s: &Expr, t: &Expr) -> Expr {
    // Linearity
    if let Expr::List { op: Operator::MPlus, args, .. } = f {
        let terms: Vec<Expr> = args.iter().map(|a| inverse_laplace(a, s, t)).collect();
        if terms.iter().any(|r| is_noun_ilt(r)) {
            return Expr::call("ilt", vec![f.clone(), s.clone(), t.clone()]);
        }
        return simplify(&Expr::List { op: Operator::MPlus, simplified: false, args: terms });
    }

    // Factor out constants
    if let Expr::List { op: Operator::MTimes, args, .. } = f {
        let (constant, dependent): (Vec<&Expr>, Vec<&Expr>) =
            args.iter().partition(|a| !contains_var(a, s));
        if !constant.is_empty() && !dependent.is_empty() {
            let c = if constant.len() == 1 { constant[0].clone() }
                else { simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: constant.into_iter().cloned().collect() }) };
            let inner = if dependent.len() == 1 { dependent[0].clone() }
                else { Expr::List { op: Operator::MTimes, simplified: false, args: dependent.into_iter().cloned().collect() } };
            let ilt = inverse_laplace(&inner, s, t);
            if !is_noun_ilt(&ilt) {
                return simplify(&Expr::mul(c, ilt));
            }
        }
    }

    if let Some(result) = ilt_table(f, s, t) {
        return result;
    }

    // General rational F(s) = N/D via exact partial fractions over Q, each term
    // inverted by the standard transform pairs (real poles → t^j·e^(at);
    // irreducible quadratics → damped sin/cos).
    if let Some(result) = ilt_rational(f, s, t) {
        return result;
    }

    Expr::call("ilt", vec![f.clone(), s.clone(), t.clone()])
}

/// Inverse Laplace of a strictly-proper rational F(s)=N(s)/D(s) by partial
/// fractions. D is factored over Q into linear and irreducible-quadratic
/// factors; the PFD numerators are found by an exact linear solve, then each
/// term is inverted: A/(s−a)^j → A·t^(j−1)·e^(at)/(j−1)!, and (Bs+C)/((s+p)²+ω²)
/// → e^(−pt)[B·cos ωt + ((C−Bp)/ω)·sin ωt]. Repeated complex poles → None.
fn ilt_rational(f: &Expr, s: &Expr, t: &Expr) -> Option<Expr> {
    use num::{BigRational, BigInt, Zero};
    let Expr::Symbol(s_id) = s else { return None };
    let (num, den) = split_rational(f, *s_id)?;
    let (dn, dd) = (ddeg(&num), ddeg(&den));
    if dn < 0 || dd < 1 || dn >= dd { return None; } // need a strictly proper fraction

    // Monic denominator (scale numerator to match).
    let lead = den[dd as usize].clone();
    let num: Vec<BigRational> = num.iter().map(|c| c / &lead).collect();

    // Factor D over Q; require every factor degree ≤ 2 (linear or irreducible
    // quadratic). Each is monicised.
    let den_poly = dense_to_poly(&den, *s_id)?;
    let factors = maxima_poly::factor_poly(&den_poly);
    let mut facs: Vec<(Vec<BigRational>, u32)> = Vec::new(); // (monic dense q, mult)
    for (q, m) in &factors {
        let d = q.degree().unwrap_or(0);
        if d == 0 { continue; }
        if d > 2 { return None; }
        let mut dq = dense(q);
        let l = dq[d as usize].clone();
        for c in dq.iter_mut() { *c = &*c / &l; }
        facs.push((dq, *m));
    }
    if facs.is_empty() { return None; }

    // Monic D = ∏ q_i^{m_i}.
    let mut dmon = vec![BigRational::from(BigInt::from(1))];
    for (q, m) in &facs { for _ in 0..*m { dmon = dmul(&dmon, q); } }

    // PFD unknowns: for each factor i, power j=1..=m_i, coeff e=0..deg(q_i)−1.
    // Column (i,j,e) = s^e · (D / q_i^j); RHS = num.
    struct Term { fi: usize, j: u32, e: u32 }
    let mut terms: Vec<Term> = Vec::new();
    let mut cols: Vec<Vec<BigRational>> = Vec::new();
    for (fi, (q, m)) in facs.iter().enumerate() {
        let dq = (q.len() - 1) as u32;
        for j in 1..=*m {
            // cofactor = D / q^j
            let mut cof = dmon.clone();
            for _ in 0..j { cof = ddiv_exact(&cof, q)?; }
            for e in 0..dq {
                let mut col = vec![BigRational::zero(); e as usize];
                col.extend(cof.iter().cloned()); // s^e · cofactor
                cols.push(col);
                terms.push(Term { fi, j, e });
            }
        }
    }
    let n_unknowns = cols.len();
    if n_unknowns != dd as usize { return None; } // PFD slot count must match

    // Build the dd×n system: row = power 0..dd-1, solve cols·x = num.
    let rows = dd as usize;
    let mut mat: Vec<Vec<BigRational>> = (0..rows).map(|r| {
        cols.iter().map(|c| c.get(r).cloned().unwrap_or_else(BigRational::zero)).collect()
    }).collect();
    let mut rhs: Vec<BigRational> = (0..rows)
        .map(|r| num.get(r).cloned().unwrap_or_else(BigRational::zero)).collect();
    let sol = solve_linear(&mut mat, &mut rhs, n_unknowns)?;

    // Invert each term, grouping the unknowns by (factor, power) into a
    // numerator polynomial (constant for linear, B·s+C for quadratic).
    let mut result = Expr::int(0);
    for (fi, (q, m)) in facs.iter().enumerate() {
        let dq = q.len() - 1;
        for j in 1..=*m {
            // numerator coeffs for this (fi,j): index by e
            let mut ncoef = vec![BigRational::zero(); dq];
            for (idx, tm) in terms.iter().enumerate() {
                if tm.fi == fi && tm.j == j { ncoef[tm.e as usize] = sol[idx].clone(); }
            }
            if ncoef.iter().all(|c| c.is_zero()) { continue; }
            let piece = invert_term(q, j, &ncoef, t)?;
            result = simplify(&Expr::add(result, piece));
        }
    }
    Some(simplify(&result))
}

// ---- dense BigRational polynomials (index = power) for the PFD machinery ----
use num::BigRational;

fn dense(p: &maxima_poly::Poly) -> Vec<BigRational> {
    use num::BigInt;
    let d = p.degree().unwrap_or(0) as usize;
    let mut v = vec![BigRational::from(BigInt::from(0)); d + 1];
    for (e, c) in &p.terms {
        v[*e as usize] = match c {
            maxima_poly::Coeff::Int(n) => BigRational::from(BigInt::from(*n)),
            maxima_poly::Coeff::Rat(n, m) => BigRational::new(BigInt::from(*n), BigInt::from(*m)),
        };
    }
    v
}

fn ddeg(v: &[BigRational]) -> i64 {
    use num::Zero;
    (0..v.len()).rev().find(|&i| !v[i].is_zero()).map(|i| i as i64).unwrap_or(-1)
}

fn dpow(p: &[BigRational], n: u32) -> Vec<BigRational> {
    let mut r = vec![BigRational::from(num::BigInt::from(1))];
    for _ in 0..n { r = dmul(&r, p); }
    r
}

/// Split a rational expression in s into (numerator, denominator) dense
/// polynomials. Handles products, integer powers (incl. reciprocals D^(−1)),
/// and bare polynomials — unlike expr_to_cre, which rejects a bare reciprocal.
fn split_rational(f: &Expr, s_id: maxima_core::SymbolId) -> Option<(Vec<BigRational>, Vec<BigRational>)> {
    let one = || vec![BigRational::from(num::BigInt::from(1))];
    match f {
        Expr::List { op: Operator::MTimes, args, .. } => {
            let (mut num, mut den) = (one(), one());
            for a in args {
                let (an, ad) = split_rational(a, s_id)?;
                num = dmul(&num, &an);
                den = dmul(&den, &ad);
            }
            Some((num, den))
        }
        Expr::List { op: Operator::MExpt, args, .. } if args.len() == 2 => {
            let Expr::Integer(e) = &args[1] else { return None };
            let bp = dense(&maxima_poly::expr_to_poly(&args[0], s_id)?);
            if *e >= 0 { Some((dpow(&bp, *e as u32), one())) }
            else { Some((one(), dpow(&bp, (-*e) as u32))) }
        }
        _ => Some((dense(&maxima_poly::expr_to_poly(f, s_id)?), one())),
    }
}

fn dense_to_poly(d: &[BigRational], var: maxima_core::SymbolId) -> Option<maxima_poly::Poly> {
    use num::{Zero, ToPrimitive};
    let mut terms = Vec::new();
    for (e, c) in d.iter().enumerate() {
        if c.is_zero() { continue; }
        let (n, m) = (c.numer().to_i64()?, c.denom().to_i64()?);
        let coeff = if m == 1 { maxima_poly::Coeff::Int(n) } else { maxima_poly::Coeff::Rat(n, m) };
        terms.push((e as u32, coeff));
    }
    terms.sort_by(|a, b| b.0.cmp(&a.0));
    Some(maxima_poly::Poly { var, terms })
}

fn dmul(a: &[BigRational], b: &[BigRational]) -> Vec<BigRational> {
    use num::{BigInt, Zero};
    if a.is_empty() || b.is_empty() { return vec![]; }
    let mut r = vec![BigRational::from(BigInt::from(0)); a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        if x.is_zero() { continue; }
        for (j, y) in b.iter().enumerate() { r[i + j] += x * y; }
    }
    r
}

/// Exact division a / b (b monic-ish); None if it doesn't divide evenly.
fn ddiv_exact(a: &[BigRational], b: &[BigRational]) -> Option<Vec<BigRational>> {
    use num::Zero;
    let (da, db) = (ddeg(a), ddeg(b));
    if db < 0 { return None; }
    if da < db { return if a.iter().all(|c| c.is_zero()) { Some(vec![BigRational::from(num::BigInt::from(0))]) } else { None }; }
    let mut rem = a.to_vec();
    let mut quot = vec![BigRational::from(num::BigInt::from(0)); (da - db + 1) as usize];
    let lead_b = b[db as usize].clone();
    let mut dr = da;
    while dr >= db {
        let coef = &rem[dr as usize] / &lead_b;
        let shift = (dr - db) as usize;
        quot[shift] = coef.clone();
        for j in 0..=(db as usize) {
            rem[shift + j] -= &coef * &b[j];
        }
        dr = ddeg(&rem);
    }
    if !rem.iter().all(|c| c.is_zero()) { return None; }
    Some(quot)
}

/// Solve the square system mat·x = rhs (n×n) over Q; None if singular.
fn solve_linear(mat: &mut [Vec<BigRational>], rhs: &mut [BigRational], n: usize) -> Option<Vec<BigRational>> {
    use num::Zero;
    let rows = mat.len();
    let mut piv_row = vec![usize::MAX; n];
    let mut r = 0;
    for c in 0..n {
        if r >= rows { break; }
        let sel = (r..rows).find(|&i| !mat[i][c].is_zero())?;
        mat.swap(sel, r); rhs.swap(sel, r);
        let p = mat[r][c].clone();
        for j in 0..n { mat[r][j] = &mat[r][j] / &p; }
        rhs[r] = &rhs[r] / &p;
        for i in 0..rows {
            if i != r && !mat[i][c].is_zero() {
                let f = mat[i][c].clone();
                for j in 0..n { mat[i][j] = &mat[i][j] - &(&f * &mat[r][j]); }
                rhs[i] = &rhs[i] - &(&f * &rhs[r]);
            }
        }
        piv_row[c] = r; r += 1;
    }
    let mut x = vec![BigRational::from(num::BigInt::from(0)); n];
    for c in 0..n {
        if piv_row[c] == usize::MAX { return None; } // underdetermined
        x[c] = rhs[piv_row[c]].clone();
    }
    Some(x)
}

/// Invert one PFD term numerator(s)/q(s)^j (q monic linear or irreducible
/// quadratic). ncoef indexes the numerator by power (constant, or [C,B]=B·s+C).
fn invert_term(q: &[BigRational], j: u32, ncoef: &[BigRational], t: &Expr) -> Option<Expr> {
    use num::{BigInt, Zero};
    let br = crate::helpers::bigrat_to_expr;
    if q.len() == 2 {
        // q = s + a0 → root r = −a0; A/(s−r)^j → A·t·^(j−1)·e^(rt)/(j−1)!
        let r = -(q[0].clone());
        let a = br(&ncoef[0]);
        let exp = if r.is_zero() { Expr::int(1) }
            else { Expr::call("exp", vec![simplify(&Expr::mul(br(&r), t.clone()))]) };
        let mut piece = Expr::mul(a, exp);
        if j >= 2 {
            let mut fact = BigInt::from(1);
            for i in 1..j { fact *= BigInt::from(i); } // (j−1)!
            piece = Expr::mul(piece,
                Expr::div(Expr::pow(t.clone(), Expr::int((j - 1) as i64)),
                          crate::helpers::bigint_to_expr(&fact)));
        }
        return Some(simplify(&piece));
    }
    if q.len() == 3 && j == 1 {
        // q = s² + b·s + c = (s+p)² + ω², p=b/2, ω²=c−b²/4. Numerator B·s+C.
        let b = q[1].clone();
        let c = q[0].clone();
        let two = BigRational::from(BigInt::from(2));
        let p = &b / &two;                                  // p = b/2
        let w2 = &c - &(&p * &p);                            // ω² = c − p²
        let bb = ncoef.get(1).cloned().unwrap_or_else(|| BigRational::from(BigInt::from(0)));
        let cc = ncoef[0].clone();
        let cm = &cc - &(&bb * &p);                          // C − B·p
        let w = Expr::call("sqrt", vec![br(&w2)]);
        let damp = if p.is_zero() { Expr::int(1) }
            else { Expr::call("exp", vec![simplify(&Expr::mul(br(&(-&p)), t.clone()))]) };
        let wt = simplify(&Expr::mul(w.clone(), t.clone()));
        // B·cos(ωt) + ((C−Bp)/ω)·sin(ωt)
        let cos_part = Expr::mul(br(&bb), Expr::call("cos", vec![wt.clone()]));
        let sin_coeff = simplify(&Expr::div(br(&cm), w));
        let sin_part = Expr::mul(sin_coeff, Expr::call("sin", vec![wt]));
        return Some(simplify(&Expr::mul(damp, simplify(&Expr::add(cos_part, sin_part)))));
    }
    None // repeated complex pole (j≥2 quadratic) — not handled
}

fn ilt_table(f: &Expr, s: &Expr, t: &Expr) -> Option<Expr> {
    // ILT{1/s} = 1
    if *f == Expr::pow(s.clone(), Expr::int(-1)) {
        return Some(Expr::int(1));
    }

    // ILT{1/s^n} = t^(n-1)/(n-1)!
    if let Expr::List { op: Operator::MExpt, args, .. } = f {
        if args.len() == 2 && args[0] == *s {
            if let Some(e) = to_i64(&args[1]) {
                if e < 0 {
                    let n = (-e) as i64;
                    return Some(simplify(&Expr::div(
                        Expr::pow(t.clone(), Expr::int(n - 1)),
                        Expr::int(factorial(n - 1)))));
                }
            }
        }
    }

    // ILT{1/(s-a)} = exp(a*t)
    if let Expr::List { op: Operator::MExpt, args, .. } = f {
        if args.len() == 2 && args[1] == Expr::int(-1) {
            if let Expr::List { op: Operator::MPlus, args: sum_args, .. } = &args[0] {
                // s - a or s + (-a)
                if sum_args.len() == 2 {
                    let (s_part, a_neg) = if sum_args[0] == *s {
                        (true, &sum_args[1])
                    } else if sum_args[1] == *s {
                        (true, &sum_args[0])
                    } else { (false, &sum_args[0]) };
                    if s_part && !contains_var(a_neg, s) {
                        let a = simplify(&Expr::neg(a_neg.clone()));
                        return Some(Expr::call("exp", vec![simplify(&Expr::mul(a, t.clone()))]));
                    }
                }
            }
        }
    }

    // ILT{s/(s^2+w^2)} = cos(w*t)
    // ILT{w/(s^2+w^2)} = sin(w*t)
    if let Some((num, den)) = extract_ratio(f, s) {
        if let Some(w2) = extract_s2_plus_w2(&den, s) {
            if num == *s {
                let w = simplify(&Expr::call("sqrt", vec![w2]));
                return Some(Expr::call("cos", vec![simplify(&Expr::mul(w, t.clone()))]));
            }
            if !contains_var(&num, s) {
                let w = simplify(&Expr::call("sqrt", vec![w2.clone()]));
                // num should be w, so sin(w*t)
                return Some(Expr::call("sin", vec![simplify(&Expr::mul(w, t.clone()))]));
            }
        }
    }

    None
}

fn extract_exp_coeff(expr: &Expr, t: &Expr) -> Option<Expr> {
    if let Expr::List { op: Operator::Named(id), args, .. } = expr {
        if resolve(*id) == "exp" && args.len() == 1 {
            return extract_linear_coeff(&args[0], t);
        }
    }
    None
}

fn extract_linear_coeff(expr: &Expr, var: &Expr) -> Option<Expr> {
    if expr == var { return Some(Expr::int(1)); }
    if let Expr::List { op: Operator::MTimes, args, .. } = expr {
        if args.len() == 2 {
            if args[0] == *var && !contains_var(&args[1], var) { return Some(args[1].clone()); }
            if args[1] == *var && !contains_var(&args[0], var) { return Some(args[0].clone()); }
        }
    }
    None
}

fn extract_ratio(expr: &Expr, _s: &Expr) -> Option<(Expr, Expr)> {
    if let Expr::List { op: Operator::MTimes, args, .. } = expr {
        let mut num_parts = Vec::new();
        let mut den_parts = Vec::new();
        for a in args {
            if let Expr::List { op: Operator::MExpt, args: pa, .. } = a {
                if pa.len() == 2 {
                    if let Expr::Integer(e) = &pa[1] {
                        if *e < 0 {
                            den_parts.push(if *e == -1 { pa[0].clone() }
                                else { Expr::pow(pa[0].clone(), Expr::int(-e)) });
                            continue;
                        }
                    }
                }
            }
            num_parts.push(a.clone());
        }
        if den_parts.is_empty() { return None; }
        let num = if num_parts.is_empty() { Expr::int(1) }
            else if num_parts.len() == 1 { num_parts.pop().unwrap() }
            else { Expr::List { op: Operator::MTimes, simplified: false, args: num_parts } };
        let den = if den_parts.len() == 1 { den_parts.pop().unwrap() }
            else { Expr::List { op: Operator::MTimes, simplified: false, args: den_parts } };
        Some((num, den))
    } else {
        None
    }
}

fn extract_s2_plus_w2(den: &Expr, s: &Expr) -> Option<Expr> {
    // Only s²+w² with a non-negative w² is an oscillation; s²−w² (negative
    // constant) is sinh/cosh and is left to the general rational inverter.
    let nonneg = |w: &Expr| !matches!(w,
        Expr::Integer(n) if *n < 0) && !matches!(w, Expr::Rational { num, den } if (*num < 0) != (*den < 0));
    if let Expr::List { op: Operator::MPlus, args, .. } = den {
        if args.len() == 2 {
            let s2 = Expr::pow(s.clone(), Expr::int(2));
            if args[0] == s2 && !contains_var(&args[1], s) && nonneg(&args[1]) { return Some(args[1].clone()); }
            if args[1] == s2 && !contains_var(&args[0], s) && nonneg(&args[0]) { return Some(args[0].clone()); }
        }
    }
    None
}

fn is_noun_laplace(e: &Expr) -> bool {
    matches!(e, Expr::List { op: Operator::Named(id), .. } if resolve(*id) == "laplace")
}

fn is_noun_ilt(e: &Expr) -> bool {
    matches!(e, Expr::List { op: Operator::Named(id), .. } if resolve(*id) == "ilt")
}

fn factorial(n: i64) -> i64 {
    (1..=n).product()
}
