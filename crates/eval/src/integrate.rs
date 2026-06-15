use maxima_core::{Expr, Operator, resolve};

use crate::helpers::{
    to_f64, to_i64, subst, contains_var,
};
use crate::simp::simplify;
use crate::eval::{meval, diff_once, expand, ratsimp, extract_fraction};

/// Hermite reduction for rational integration with repeated factors.
/// Uses per-factor reduction: for each irreducible factor q_i with multiplicity k >= 2,
/// iteratively reduce: ∫ A/q_i^j = -t/(q_i^{j-1}) + ∫ (s + t')/q_i^{j-1}
/// where s*q_i + t*q_i' = A (extended GCD, since q_i is square-free so gcd(q_i, q_i')=1).
fn integrate_hermite(
    num: &maxima_poly::Poly,
    factors: &[(maxima_poly::Poly, u32)],
    var_id: maxima_core::SymbolId,
    var: &Expr,
) -> Option<Expr> {
    use maxima_poly::Coeff;

    let max_mult = factors.iter().map(|(_, m)| *m).max().unwrap_or(0);
    if max_mult <= 1 {
        return None;
    }

    // Full denominator
    let full_den = factors.iter()
        .map(|(f, m)| poly_pow_helper(f, *m))
        .reduce(|a, b| a.mul(&b))
        .unwrap_or(maxima_poly::Poly::constant(var_id, Coeff::one()));

    // Partial fraction approach: decompose A/D into sum of A_i/q_i^k_i terms,
    // then reduce each term with multiplicity > 1.
    //
    // For simplicity, we use the "evaluate at root" method for partial fractions
    // when all factors are linear, and fall back to extended-GCD approach otherwise.

    // Collect rational part terms and remaining (square-free) integrand
    let mut rational_parts: Vec<Expr> = Vec::new();
    let mut remaining_num = num.clone();
    let mut remaining_den = full_den.clone();


    // Process each factor with multiplicity > 1
    for (qi, ki) in factors {
        if *ki <= 1 { continue; }

        let qi_prime = qi.derivative();

        // For multiplicity k, reduce step by step from k down to 1
        for j in (2..=*ki).rev() {
            // We need to extract the A_j/(q_i^j) part from remaining_num/remaining_den
            // where remaining_den contains q_i^j as a factor.
            //
            // Partial fraction: remaining_num/remaining_den = ... + A_j/q_i^j + ...
            // A_j = remaining_num * (remaining_den / q_i^j) ^{-1} mod q_i
            //
            // Simpler approach: compute cofactor = remaining_den / q_i^j
            let qi_pow_j = poly_pow_helper(qi, j);
            let cofactor = match remaining_den.exact_div(&qi_pow_j) {
                Some(c) => c,
                None => continue,
            };

            // A_j = remaining_num / cofactor mod q_i
            // For linear q_i = (x - r), A_j = remaining_num(r) / cofactor(r)
            if qi.degree() == Some(1) {
                let a_coeff = qi.leading_coeff();
                let b_coeff = qi.constant_term();
                let root = match b_coeff.neg().div(&a_coeff) {
                    Some(r) => r,
                    None => continue,
                };

                let num_at_root = remaining_num.eval_at(&root);
                let cof_at_root = cofactor.eval_at(&root);
                if cof_at_root.is_zero() { continue; }

                let a_j = match num_at_root.div(&cof_at_root) {
                    Some(v) => v,
                    None => continue,
                };

                if a_j.is_zero() { continue; }

                // Now reduce ∫ a_j / q_i^j dx using Hermite:
                // Extended GCD: s*q_i + t*q_i' = 1 (since q_i is irreducible hence square-free)
                let (g, s_raw, t_raw) = maxima_poly::poly_extended_gcd(qi, &qi_prime);
                if g.is_zero() { continue; }

                // Scale by a_j/g
                // For linear q_i = a*x + b, q_i' = a, gcd = 1
                // extended GCD gives s=0, t=1/a (or similar)
                // Result: ∫ a_j/(a*x+b)^j = -a_j*t / ((j-1)*(a*x+b)^{j-1}) + ∫ (a_j*(s+t')) / (a*x+b)^{j-1}
                // For linear: t' = 0, so remaining = ∫ a_j*s / (a*x+b)^{j-1}

                // Scale: we want s*q_i + t*q_i' = a_j (not just g)
                let a_j_poly = maxima_poly::Poly::constant(var_id, a_j.clone());
                let scale = match a_j_poly.exact_div(&g) {
                    Some(sc) => sc,
                    None => continue,
                };

                let t_scaled = t_raw.mul(&scale);
                let s_scaled = s_raw.mul(&scale);
                let t_prime = t_scaled.derivative();

                // Rational part contribution: -t_scaled / ((j-1) * q_i^{j-1})
                let jm1 = (j - 1) as i64;
                let rat_num_p = t_scaled.neg();
                let rat_den_p = poly_pow_helper(qi, j - 1).scale(&Coeff::Int(jm1));

                // Normalize rational coefficients
                let lcd = compute_lcd(& rat_num_p, &rat_den_p);
                let (rn, rd) = if lcd > 1 {
                    (rat_num_p.scale(&Coeff::Int(lcd)), rat_den_p.scale(&Coeff::Int(lcd)))
                } else {
                    (rat_num_p, rat_den_p)
                };

                let rn_expr = maxima_poly::poly_to_expr(&rn);
                let rd_expr = maxima_poly::poly_to_expr(&rd);
                rational_parts.push(simplify(&Expr::div(rn_expr, rd_expr)));

                // Remove a_j/q_i^j from the fraction and add back (s+t')/q_i^{j-1}:
                // remaining_num/remaining_den - a_j/q_i^j + (s+t')/q_i^{j-1}
                // = (remaining_num - a_j*cofactor + (s+t')*cofactor*qi) / remaining_den
                let new_remaining_num = remaining_num.sub(&cofactor.scale(&a_j));
                let s_plus_tp = s_scaled.add(&t_prime);
                let qi_cofactor = cofactor.mul(qi);
                remaining_num = new_remaining_num.add(&s_plus_tp.mul(&qi_cofactor));
            } else {
                // Non-linear factor: skip for now
                continue;
            }
        }
    }

    if rational_parts.is_empty() {
        return None;
    }

    // Now remaining_num / remaining_den should have only square-free denominator factors
    // Simplify: cancel common factors from remaining
    let g = maxima_poly::poly_gcd(&remaining_num, &remaining_den);
    if !g.is_constant() || !g.leading_coeff().is_one() {
        if let (Some(rn), Some(rd)) = (remaining_num.exact_div(&g), remaining_den.exact_div(&g)) {
            remaining_num = rn;
            remaining_den = rd;
        }
    }

    // Normalize remaining fraction
    let lcd_all = compute_lcd(&remaining_num, &remaining_den);
    if lcd_all > 1 {
        remaining_num = remaining_num.scale(&Coeff::Int(lcd_all));
        remaining_den = remaining_den.scale(&Coeff::Int(lcd_all));
    }
    if matches!(remaining_den.leading_coeff(), Coeff::Int(n) if n < 0) {
        remaining_num = remaining_num.neg();
        remaining_den = remaining_den.neg();
    }

    // Cancel integer content
    let nc = remaining_num.content();
    let dc = remaining_den.content();
    if let (Coeff::Int(nci), Coeff::Int(dci)) = (&nc, &dc) {
        let g_int = crate::helpers::gcd_i64(nci.unsigned_abs(), dci.unsigned_abs()) as i64;
        if g_int > 1 {
            remaining_num = remaining_num.exact_div(&maxima_poly::Poly::constant(var_id, Coeff::Int(g_int)))
                .unwrap_or(remaining_num);
            remaining_den = remaining_den.exact_div(&maxima_poly::Poly::constant(var_id, Coeff::Int(g_int)))
                .unwrap_or(remaining_den);
        }
    }

    let remaining_expr = simplify(&Expr::div(
        maxima_poly::poly_to_expr(&remaining_num),
        maxima_poly::poly_to_expr(&remaining_den),
    ));

    // Integrate the remaining square-free part
    let remaining_int = if !remaining_num.is_zero() && remaining_den.degree().unwrap_or(0) >= 1 {
        let rem_factors = maxima_poly::factor_poly(&remaining_den);
        let all_linear = rem_factors.iter().all(|(f, m)| f.degree() == Some(1) && *m == 1);

        if all_linear && rem_factors.len() > 1 {
            integrate_partfrac_linear(&remaining_num, &rem_factors, var)
                .unwrap_or_else(|| table_integrate(&remaining_expr, var))
        } else if rem_factors.len() == 1 && rem_factors[0].0.degree() == Some(1) {
            let fi = &rem_factors[0].0;
            let a = fi.leading_coeff();
            if remaining_num.is_constant() {
                let c = remaining_num.leading_coeff();
                if let Some(ca) = c.div(&a) {
                    let ca_expr = coeff_to_expr(&ca);
                    simplify(&Expr::mul(ca_expr, Expr::call("log", vec![maxima_poly::poly_to_expr(fi)])))
                } else {
                    table_integrate(&remaining_expr, var)
                }
            } else {
                table_integrate(&remaining_expr, var)
            }
        } else {
            table_integrate(&remaining_expr, var)
        }
    } else if remaining_num.is_zero() {
        Expr::int(0)
    } else {
        table_integrate(&remaining_expr, var)
    };

    let mut all_parts = rational_parts;
    let rem_str = remaining_int.to_string();
    if rem_str != "0" {
        all_parts.push(remaining_int);
    }

    if all_parts.is_empty() {
        return Some(Expr::int(0));
    }
    if all_parts.len() == 1 {
        return Some(all_parts.remove(0));
    }

    Some(simplify(&Expr::List {
        op: Operator::MPlus,
        simplified: false,
        args: all_parts,
    }))
}

/// Integrate a partial fraction with all-linear square-free denominator factors.
/// ∫ P(x) / ((x-r1)(x-r2)...) dx = sum of c_i * log(x - r_i)
/// where c_i = P(r_i) / product_{j≠i} (r_i - r_j)
fn integrate_partfrac_linear(
    num: &maxima_poly::Poly,
    factors: &[(maxima_poly::Poly, u32)],
    _var: &Expr,
) -> Option<Expr> {
    let mut terms = Vec::new();
    for (i, (fi, _)) in factors.iter().enumerate() {
        let a = fi.leading_coeff();
        let b = fi.constant_term();
        let root = b.neg().div(&a)?;

        let num_at = num.eval_at(&root);
        let mut den_at = maxima_poly::Coeff::one();
        for (j, (fj, _)) in factors.iter().enumerate() {
            if j != i {
                den_at = den_at.mul(&fj.eval_at(&root));
            }
        }
        let residue = num_at.div(&den_at)?;
        if residue.is_zero() { continue; }

        let c_expr = coeff_to_expr(&residue);
        let a_expr = coeff_to_expr(&a);
        let log_term = Expr::call("log", vec![maxima_poly::poly_to_expr(fi)]);
        terms.push(simplify(&Expr::div(Expr::mul(c_expr, log_term), a_expr)));
    }
    if terms.is_empty() { return Some(Expr::int(0)); }
    Some(simplify(&Expr::List {
        op: Operator::MPlus,
        simplified: false,
        args: terms,
    }))
}

pub(crate) fn coeff_to_expr(c: &maxima_poly::Coeff) -> Expr {
    match c {
        maxima_poly::Coeff::Int(n) => Expr::int(*n),
        maxima_poly::Coeff::Rat(n, d) => Expr::Rational { num: *n, den: *d },
    }
}

/// Lazard–Rioboo–Trager logarithmic part for a proper, square-free rational
/// function num/den (Bronstein, Symbolic Integration I, ch. 2). Computes the
/// log part as Σ c_i·log(v_i) from a resultant, without factoring the
/// denominator over an extension field.
///
/// Fires only when the rational-residue logs account for the entire
/// denominator degree (i.e. the antiderivative is purely logarithmic with
/// rational residues) and the result differentiates back to the integrand.
/// Otherwise returns None, leaving the existing partfrac/atan path in charge —
/// so this can never regress a case the older code already handled.
fn try_lrt_log_integrate(
    num: &maxima_poly::Poly,
    den: &maxima_poly::Poly,
    var: &Expr,
) -> Option<Expr> {
    let deg_den = den.degree().unwrap_or(0);
    if deg_den < 1 || num.degree().unwrap_or(0) >= deg_den {
        return None;
    }
    // Require a square-free denominator (Hermite reduction has run upstream).
    if !maxima_poly::poly_gcd(den, &den.derivative()).is_constant() {
        return None;
    }

    let logs = maxima_poly::lazard_rioboo_trager(num, den);
    if logs.is_empty() {
        return None;
    }
    // Full logarithmic coverage: rational residues span the whole denominator.
    let cover: u32 = logs.iter().map(|(_, v)| v.degree().unwrap_or(0)).sum();
    if cover != deg_den {
        return None;
    }

    let mut terms = Vec::new();
    for (c, v) in &logs {
        let log_term = Expr::call("log", vec![maxima_poly::poly_to_expr(v)]);
        terms.push(simplify(&Expr::mul(coeff_to_expr(c), log_term)));
    }
    let result = simplify(&Expr::List {
        op: Operator::MPlus,
        simplified: false,
        args: terms,
    });

    let integrand = simplify(&Expr::div(
        maxima_poly::poly_to_expr(num),
        maxima_poly::poly_to_expr(den),
    ));
    if verify_antiderivative(&result, &integrand, var) {
        Some(result)
    } else {
        None
    }
}

/// Numeric check that d/dx(antideriv) ≈ integrand at several sample points.
/// Used as the mandatory verification gate before returning a closed form.
fn verify_antiderivative(antideriv: &Expr, integrand: &Expr, var: &Expr) -> bool {
    let var_id = match var {
        Expr::Symbol(id) => *id,
        _ => return false,
    };
    let d = diff_once(antideriv, var);
    // A wide spread so that domain-restricted integrands (e.g. √(x²-1),
    // √(2x-x²)) still land enough valid points to verify.
    let samples = [
        0.23f64, 0.37, 0.6, 0.85, 1.3, 1.7, 2.1, 2.6, 3.4, 4.3,
        -0.4, -0.8, -1.3, -1.7, -2.6, -3.4,
    ];
    let mut checked = 0;
    for &v in &samples {
        let (Some(dv), Some(iv)) = (num_eval(&d, var_id, v), num_eval(integrand, var_id, v)) else {
            continue;
        };
        if !dv.is_finite() || !iv.is_finite() {
            continue;
        }
        if (dv - iv).abs() > 1e-6 * (1.0 + iv.abs()) {
            return false;
        }
        checked += 1;
    }
    checked >= 2
}

/// Minimal numeric evaluator for verification (handles the operators that
/// appear in elementary antiderivatives).
fn num_eval(e: &Expr, var: maxima_core::SymbolId, val: f64) -> Option<f64> {
    match e {
        Expr::Integer(n) => Some(*n as f64),
        Expr::Float(f) => Some(*f),
        Expr::Rational { num, den } => Some(*num as f64 / *den as f64),
        Expr::Symbol(id) => {
            if *id == var {
                Some(val)
            } else {
                match resolve(*id).as_str() {
                    "%pi" => Some(std::f64::consts::PI),
                    "%e" => Some(std::f64::consts::E),
                    _ => None,
                }
            }
        }
        Expr::List { op, args, .. } => match op {
            Operator::MPlus => {
                let mut s = 0.0;
                for a in args {
                    s += num_eval(a, var, val)?;
                }
                Some(s)
            }
            Operator::MTimes => {
                let mut p = 1.0;
                for a in args {
                    p *= num_eval(a, var, val)?;
                }
                Some(p)
            }
            Operator::MExpt if args.len() == 2 => {
                let b = num_eval(&args[0], var, val)?;
                let ex = num_eval(&args[1], var, val)?;
                Some(b.powf(ex))
            }
            Operator::Named(id) if args.len() == 1 => {
                let a = num_eval(&args[0], var, val)?;
                match resolve(*id).as_str() {
                    "log" => Some(a.ln()),
                    "exp" => Some(a.exp()),
                    "sin" => Some(a.sin()),
                    "cos" => Some(a.cos()),
                    "tan" => Some(a.tan()),
                    "atan" => Some(a.atan()),
                    "sqrt" => Some(a.sqrt()),
                    _ => None,
                }
            }
            _ => None,
        },
        _ => None,
    }
}

/// Integrate A(x) / (product of linear and irreducible quadratic factors).
/// Uses partial fractions: for each linear factor (x-r), compute residue → log.
/// For each quadratic factor (ax²+bx+c), determine (Px+Q)/(ax²+bx+c) coefficients
/// and integrate: log + atan terms.
fn integrate_partfrac_mixed(
    num: &maxima_poly::Poly,
    factors: &[(maxima_poly::Poly, u32)],
    var: &Expr,
) -> Option<Expr> {
    use maxima_poly::Coeff;

    let mut terms: Vec<Expr> = Vec::new();

    // For each linear factor: residue method
    for (i, (fi, _)) in factors.iter().enumerate() {
        if fi.degree() != Some(1) { continue; }
        let a = fi.leading_coeff();
        let b = fi.constant_term();
        let root = b.neg().div(&a)?;
        let num_at = num.eval_at(&root);
        let mut den_at = Coeff::one();
        for (j, (fj, _)) in factors.iter().enumerate() {
            if j != i { den_at = den_at.mul(&fj.eval_at(&root)); }
        }
        let residue = num_at.div(&den_at)?;
        if residue.is_zero() { continue; }
        let c_expr = coeff_to_expr(&residue);
        let a_expr = coeff_to_expr(&a);
        let log_term = simplify(&Expr::div(
            Expr::mul(c_expr, Expr::call("log", vec![maxima_poly::poly_to_expr(fi)])),
            a_expr));
        terms.push(log_term);
    }

    // For each quadratic factor: solve for (Px+Q)/(ax²+bx+c)
    for (i, (qi, _)) in factors.iter().enumerate() {
        if qi.degree() != Some(2) { continue; }

        let a_c = qi.terms.iter().find(|(e,_)| *e==2).map(|(_,c)| c.clone()).unwrap_or(Coeff::zero());
        let b_c = qi.terms.iter().find(|(e,_)| *e==1).map(|(_,c)| c.clone()).unwrap_or(Coeff::zero());
        let c_c = qi.terms.iter().find(|(e,_)| *e==0).map(|(_,c)| c.clone()).unwrap_or(Coeff::zero());

        let (ai, bi, ci) = match (&a_c, &b_c, &c_c) {
            (Coeff::Int(a), Coeff::Int(b), Coeff::Int(c)) => (*a, *b, *c),
            _ => return None,
        };

        // Cofactor = product of all other factors
        let mut cofactor = maxima_poly::Poly::constant(num.var, Coeff::one());
        for (j, (fj, _)) in factors.iter().enumerate() {
            if j != i { cofactor = cofactor.mul(fj); }
        }

        // We need: num = (P*x+Q) * cofactor + ... (other factor contributions)
        // Approach: subtract linear factor contributions from num, divide by cofactor mod qi
        // Simpler: evaluate at two points to get P and Q
        //
        // For irreducible quadratic ax²+bx+c with disc < 0, use:
        // P and Q satisfy: num(x) ≡ (P*x+Q)*cofactor(x) mod qi(x)
        // Compute num mod qi, cofactor mod qi, then solve
        let num_mod = poly_mod(num, qi);
        let cof_mod = poly_mod(&cofactor, qi);

        // num_mod = (P*x+Q) * cof_mod  mod qi
        // cof_mod is degree ≤ 1: cof_mod = cx+d
        // (P*x+Q)(cx+d) = Pcx² + (Pd+Qc)x + Qd
        // mod qi = ax²+bx+e: Pcx² ≡ Pc*(-bx-e)/a (replace x² = (-bx-e)/a from qi=0)
        // This gets complex. Use evaluation at two points instead.

        // num = (Px+Q)*cofactor + qi*something
        // So: num mod qi = (Px+Q)*(cofactor mod qi)
        // Invert (cofactor mod qi) to get (Px+Q)
        if cof_mod.is_zero() { return None; }

        // Invert cof_mod modulo qi using extended GCD
        let (g, inv, _) = maxima_poly::poly_extended_gcd(&cof_mod, qi);
        if g.is_zero() || !g.is_constant() {
            return None;
        }

        // (Px+Q) = num_mod * inv / g   mod qi
        let pxq_scaled = num_mod.mul(&inv);
        let pxq = if g.is_constant() && g.leading_coeff().is_one() {
            pxq_scaled
        } else {
            match pxq_scaled.exact_div(&g) {
                Some(r) => r,
                None => return None,
            }
        };
        // Reduce mod qi
        let pxq = poly_mod(&pxq, qi);

        // Extract P and Q coefficients
        let p_coeff = pxq.terms.iter().find(|(e,_)| *e == 1).map(|(_,c)| c.clone()).unwrap_or(Coeff::zero());
        let q_coeff = pxq.terms.iter().find(|(e,_)| *e == 0).map(|(_,c)| c.clone()).unwrap_or(Coeff::zero());

        // ∫ (Px+Q)/(ax²+bx+c) dx
        // = (P/(2a)) * log(ax²+bx+c) + (2aQ-Pb)/(a*sqrt(4ac-b²)) * atan((2ax+b)/sqrt(4ac-b²))
        // P and Q may be rational
        let (pn, pd) = match &p_coeff {
            Coeff::Int(n) => (*n, 1i64),
            Coeff::Rat(n, d) => (*n, *d),
        };
        let (qn, qd) = match &q_coeff {
            Coeff::Int(n) => (*n, 1i64),
            Coeff::Rat(n, d) => (*n, *d),
        };
        let disc = 4*ai*ci - bi*bi;

        if pn != 0 {
            // log part: P/(2a) = pn/(pd*2*ai)
            let log_num = pn;
            let log_den = pd * 2 * ai;
            let g = crate::helpers::gcd_i64(log_num.unsigned_abs(), log_den.unsigned_abs()) as i64;
            let (ln, ld) = (log_num / g, log_den / g);
            let coeff = if ld == 1 { Expr::int(ln) } else { Expr::Rational { num: ln, den: ld } };
            terms.push(simplify(&Expr::mul(coeff, Expr::call("log", vec![maxima_poly::poly_to_expr(qi)]))));
        }

        // atan part: (2aQ - Pb) / (a * sqrt(disc))
        // = (2*ai*qn/qd - pn/pd*bi) / ai
        // = (2*ai*qn*pd - pn*bi*qd) / (qd*pd*ai)
        let atan_num_n = 2*ai*qn*pd - pn*bi*qd;
        let atan_num_d = qd * pd * ai;
        if atan_num_n != 0 && disc > 0 {
            let g = crate::helpers::gcd_i64(atan_num_n.unsigned_abs(), atan_num_d.unsigned_abs()) as i64;
            let (cn, cd) = (atan_num_n / g, atan_num_d / g);
            let sd = Expr::call("sqrt", vec![Expr::int(disc)]);
            let inner = simplify(&Expr::div(
                Expr::add(Expr::mul(Expr::int(2*ai), var.clone()), Expr::int(bi)),
                sd.clone()));
            let coeff = if cd == 1 { Expr::int(cn) } else { Expr::Rational { num: cn, den: cd } };
            terms.push(simplify(&Expr::div(
                Expr::mul(coeff, Expr::call("atan", vec![inner])),
                sd)));
        }
    }

    if terms.is_empty() { return None; }
    Some(simplify(&Expr::List { op: Operator::MPlus, simplified: false, args: terms }))
}

/// Try integration by substitution: look for u(x) inside f where f = g(u) * u'(x).
fn try_substitution_integrate(f: &Expr, var: &Expr) -> Option<Expr> {
    let mut candidates = Vec::new();
    collect_substitution_candidates(f, var, &mut candidates);
    candidates.dedup_by(|a, b| a == b);

    let new_var = Expr::Symbol(maxima_core::intern("_u_"));

    for u in &candidates {
        let u_prime = diff_once(u, var);
        if u_prime == Expr::int(0) { continue; }

        // Method 1: structural match f = c * u^n * u' (handles most log/exp compositions)
        if let Some(result) = try_power_substitution(f, u, &u_prime, var) {
            return Some(result);
        }

        // Method 2: decompose f into factors, partition into u-dependent and var-dependent,
        // check if the var-dependent part is proportional to u'
        let mut all_factors = Vec::new();
        collect_mult_factors(f, &mut all_factors);

        let mut u_parts = Vec::new();
        let mut var_parts = Vec::new();
        for factor in &all_factors {
            let substituted = subst_power(&new_var, u, factor, var);
            if contains_var(&substituted, var) {
                var_parts.push(factor.clone());
            } else {
                u_parts.push(substituted);
            }
        }

        if !u_parts.is_empty() && !var_parts.is_empty() {
            let var_product = if var_parts.len() == 1 { var_parts[0].clone() }
                else { simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: var_parts.clone() }) };

            // Try: var_product / u_prime = constant
            let scale = simplify(&ratsimp(&Expr::div(var_product.clone(), u_prime.clone())));
            let scale_val = if !contains_var(&scale, var) {
                Some((scale, u_parts.clone()))
            } else {
                // Numeric fallback for trig ratios
                let test_val = Expr::Float(1.5);
                let mut tmp_env = crate::Environment::new();
                let vp_at = meval(&subst(&test_val, var, &var_product), &mut tmp_env);
                let up_at = meval(&subst(&test_val, var, &u_prime), &mut tmp_env);
                if let (Some(a), Some(b)) = (to_f64(&vp_at), to_f64(&up_at)) {
                    if b.abs() > 1e-15 {
                        let ratio = a / b;
                        let rounded = ratio.round() as i64;
                        if (ratio - rounded as f64).abs() < 1e-10 {
                            let test2 = Expr::Float(2.3);
                            let vp2 = meval(&subst(&test2, var, &var_product), &mut tmp_env);
                            let up2 = meval(&subst(&test2, var, &u_prime), &mut tmp_env);
                            if let (Some(a2), Some(b2)) = (to_f64(&vp2), to_f64(&up2)) {
                                if b2.abs() > 1e-15 && ((a2/b2) - rounded as f64).abs() < 1e-10 {
                                    Some((Expr::int(rounded), u_parts.clone()))
                                } else { None }
                            } else { None }
                        } else {
                            // Not integer — check if var_product is a factor of u_prime
                            // If u' = var_product * remainder, scale=1 and remainder goes to u_parts
                            try_factor_match(&var_product, &u_prime, var, &u_parts, &new_var, u)
                        }
                    } else { None }
                } else { None }
            };
            if let Some((scale, final_u_parts)) = scale_val {
                let u_integrand = if final_u_parts.len() == 1 {
                    simplify(&Expr::mul(final_u_parts[0].clone(), scale))
                } else if final_u_parts.is_empty() {
                    scale
                } else {
                    let u_prod = Expr::List { op: Operator::MTimes, simplified: false, args: final_u_parts };
                    simplify(&Expr::mul(u_prod, scale))
                };
                let inner = table_integrate(&u_integrand, &new_var);
                if !inner.to_string().starts_with("integrate") {
                    return Some(simplify(&subst(u, &new_var, &inner)));
                }
            }
        }

        // Method 3: direct ratio test (for cases where f isn't a simple product)
        let ratio = simplify(&Expr::div(f.clone(), u_prime.clone()));
        let ratio_in_u = subst(&new_var, u, &ratio);

        if !contains_var(&ratio_in_u, var) {
            let inner_integral = table_integrate(&ratio_in_u, &new_var);
            if inner_integral.to_string().starts_with("integrate") {
                continue;
            }
            return Some(simplify(&subst(u, &new_var, &inner_integral)));
        }
    }
    None
}

/// Check if f = c * u^n * u' for constant c and integer n.
/// Returns c * u^(n+1)/(n+1) for n≠-1, or c * log(u) for n=-1.
fn try_power_substitution(f: &Expr, u: &Expr, u_prime: &Expr, var: &Expr) -> Option<Expr> {
    let mut factors = Vec::new();
    collect_mult_factors(f, &mut factors);

    let mut constant = Expr::int(1);
    let mut u_power: i64 = 0;
    let mut uprime_count: i64 = 0;
    let mut unmatched = Vec::new();

    let u_simp = simplify(u);
    let up_simp = simplify(u_prime);

    for factor in &factors {
        let fs = simplify(factor);
        if !contains_var(&fs, var) {
            constant = simplify(&Expr::mul(constant, fs));
        } else if fs == u_simp {
            u_power += 1;
        } else if fs == up_simp {
            uprime_count += 1;
        } else if let Expr::List { op: Operator::MExpt, args, .. } = &fs {
            if args.len() == 2 {
                let base = simplify(&args[0]);
                if base == u_simp {
                    if let Some(n) = to_i64(&args[1]) {
                        u_power += n;
                    } else { unmatched.push(fs); }
                } else if base == up_simp {
                    if let Some(n) = to_i64(&args[1]) {
                        uprime_count += n;
                    } else { unmatched.push(fs); }
                } else {
                    // Check if the base is a product that equals u
                    let base_simp = simplify(&base);
                    if base_simp == u_simp {
                        if let Some(n) = to_i64(&args[1]) { u_power += n; }
                        else { unmatched.push(fs); }
                    } else {
                        unmatched.push(fs);
                    }
                }
            } else { unmatched.push(fs); }
        } else {
            unmatched.push(fs);
        }
    }

    // If u' wasn't found exactly, check if u' is a constant multiple of some factor
    if uprime_count == 0 && unmatched.len() == 1 {
        let candidate = &unmatched[0];
        let ratio = simplify(&Expr::div(candidate.clone(), up_simp.clone()));
        if !contains_var(&ratio, var) {
            constant = simplify(&Expr::mul(constant, ratio));
            uprime_count = 1;
            unmatched.clear();
        }
    }
    // Also check if u' is a constant multiple of a product of unmatched factors
    if uprime_count == 0 && !unmatched.is_empty() {
        let product = if unmatched.len() == 1 {
            unmatched[0].clone()
        } else {
            simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: unmatched.clone() })
        };
        let ratio = simplify(&Expr::div(product, up_simp.clone()));
        if !contains_var(&ratio, var) {
            constant = simplify(&Expr::mul(constant, ratio));
            uprime_count = 1;
            unmatched.clear();
        }
    }

    if !unmatched.is_empty() || uprime_count != 1 {
        return None;
    }

    if u_power == -1 {
        Some(simplify(&Expr::mul(constant, Expr::call("log", vec![u.clone()]))))
    } else {
        let new_power = u_power + 1;
        Some(simplify(&Expr::mul(constant,
            Expr::div(Expr::pow(u.clone(), Expr::int(new_power)), Expr::int(new_power)))))
    }
}

/// Check if var_product is a factor of u_prime. If u' = var_product * remainder,
/// return (scale=1, u_parts with remainder added after subst).
fn try_factor_match(
    var_product: &Expr, u_prime: &Expr, var: &Expr,
    u_parts: &[Expr], new_var: &Expr, u: &Expr,
) -> Option<(Expr, Vec<Expr>)> {
    let mut up_factors = Vec::new();
    collect_mult_factors(u_prime, &mut up_factors);

    // Check if var_product matches one of the u_prime factors
    for (i, uf) in up_factors.iter().enumerate() {
        let test_val = Expr::Float(1.7);
        let mut env = crate::Environment::new();
        let vp_at = meval(&subst(&test_val, var, var_product), &mut env);
        let uf_at = meval(&subst(&test_val, var, uf), &mut env);
        if let (Some(a), Some(b)) = (to_f64(&vp_at), to_f64(&uf_at)) {
            if b.abs() > 1e-15 && (a/b - 1.0).abs() < 1e-10 {
                // var_product ≈ up_factors[i], so u' = var_product * remaining
                let remaining: Vec<Expr> = up_factors.iter().enumerate()
                    .filter(|(j, _)| *j != i)
                    .map(|(_, f)| f.clone())
                    .collect();
                // Substitute u into remaining factors
                let rem_subst: Vec<Expr> = remaining.iter()
                    .map(|f| subst(new_var, u, f))
                    .collect();
                // All remaining must be free of var after subst
                if rem_subst.iter().all(|f| !contains_var(f, var)) {
                    let mut final_u_parts = u_parts.to_vec();
                    // u' = var_product * remaining, and dx = du/u'. Thus
                    // ∫ u_parts·var_product dx = ∫ u_parts / remaining du, so the
                    // leftover factors enter as reciprocals (not multiplied in).
                    final_u_parts.extend(
                        rem_subst.into_iter().map(|f| simplify(&Expr::pow(f, Expr::int(-1)))),
                    );
                    return Some((Expr::int(1), final_u_parts));
                }
            }
        }
    }
    None
}

fn collect_mult_factors(expr: &Expr, out: &mut Vec<Expr>) {
    match expr {
        Expr::List { op: Operator::MTimes, args, .. } => {
            for a in args { collect_mult_factors(a, out); }
        }
        // Distribute negative exponents: (a*b)^(-1) → a^(-1), b^(-1)
        Expr::List { op: Operator::MExpt, args, .. }
            if args.len() == 2 && args[1] == Expr::int(-1) =>
        {
            if let Expr::List { op: Operator::MTimes, args: inner, .. } = &args[0] {
                for a in inner {
                    out.push(Expr::pow(a.clone(), Expr::int(-1)));
                }
            } else {
                out.push(expr.clone());
            }
        }
        _ => out.push(expr.clone()),
    }
}

/// Collect candidate subexpressions for substitution.
fn collect_substitution_candidates(expr: &Expr, var: &Expr, out: &mut Vec<Expr>) {
    match expr {
        Expr::List { op: Operator::Named(id), args, .. } if args.len() == 1 => {
            let fname = resolve(*id);
            match fname.as_str() {
                "log" | "exp" | "sin" | "cos" | "tan" | "asin" | "acos" | "atan"
                | "sinh" | "cosh" | "tanh" | "sqrt" => {
                    if contains_var(&args[0], var) {
                        // The function call itself is a candidate
                        out.push(expr.clone());
                        // The argument is also a candidate (for chain rule)
                        if args[0] != *var {
                            out.push(args[0].clone());
                        }
                    }
                }
                _ => {}
            }
            collect_substitution_candidates(&args[0], var, out);
        }
        Expr::List { op: Operator::MExpt, args, .. } if args.len() == 2 => {
            if contains_var(expr, var) {
                // e.g., x^2, exp(x)^2 — the base or whole expr might be a candidate
                if let Expr::List { op: Operator::Named(_), .. } = &args[0] {
                    out.push(args[0].clone());
                }
                // For x^n, try x^n as candidate; for even n, also try x^(n/2)
                if args[0] == *var {
                    if let Some(n) = to_i64(&args[1]) {
                        if n > 1 {
                            out.push(expr.clone());
                        }
                        if n > 2 && n % 2 == 0 {
                            out.push(Expr::pow(var.clone(), Expr::int(n / 2)));
                        }
                    }
                }
            }
            collect_substitution_candidates(&args[0], var, out);
            collect_substitution_candidates(&args[1], var, out);
        }
        Expr::List { args, .. } => {
            for arg in args {
                collect_substitution_candidates(arg, var, out);
            }
        }
        _ => {}
    }
}

/// Like subst but also handles power substitution: when u = var^n,
/// replace var^(k*n) with new_var^k.
fn subst_power(new_var: &Expr, u: &Expr, expr: &Expr, var: &Expr) -> Expr {
    // First try normal subst
    let result = subst(new_var, u, expr);
    if !contains_var(&result, var) {
        return result;
    }
    // If u = var^n, rewrite var powers as powers of new_var
    if let Expr::List { op: Operator::MExpt, args, .. } = u {
        if args.len() == 2 && args[0] == *var {
            if let Some(n) = to_i64(&args[1]) {
                if n >= 2 {
                    return rewrite_powers(new_var, var, n, expr);
                }
            }
        }
    }
    result
}

fn rewrite_powers(new_var: &Expr, var: &Expr, n: i64, expr: &Expr) -> Expr {
    match expr {
        Expr::List { op: Operator::MExpt, args, .. }
            if args.len() == 2 && args[0] == *var =>
        {
            if let Some(k) = to_i64(&args[1]) {
                if k % n == 0 {
                    let new_exp = k / n;
                    if new_exp == 1 { return new_var.clone(); }
                    return Expr::pow(new_var.clone(), Expr::int(new_exp));
                }
            }
            expr.clone()
        }
        Expr::Symbol(_) if expr == var => expr.clone(),
        Expr::List { op, args, simplified } => {
            let new_args: Vec<Expr> = args.iter().map(|a| rewrite_powers(new_var, var, n, a)).collect();
            Expr::List { op: *op, simplified: *simplified, args: new_args }
        }
        _ => expr.clone(),
    }
}

/// Polynomial remainder: a mod b
fn poly_mod(a: &maxima_poly::Poly, b: &maxima_poly::Poly) -> maxima_poly::Poly {
    match a.divmod(b) {
        Some((_, r)) => r,
        None => a.clone(),
    }
}

/// Compute LCD of all denominators in polynomial coefficients (two polys)
fn compute_lcd(p: &maxima_poly::Poly, q: &maxima_poly::Poly) -> i64 {
    let mut lcd = compute_lcd_single(p);
    for (_, c) in &q.terms {
        if let maxima_poly::Coeff::Rat(_, d) = c {
            lcd = lcm_i64(lcd, d.abs());
        }
    }
    lcd
}

/// Compute LCD of all denominators in a single polynomial's coefficients
fn compute_lcd_single(p: &maxima_poly::Poly) -> i64 {
    let mut lcd = 1i64;
    for (_, c) in &p.terms {
        if let maxima_poly::Coeff::Rat(_, d) = c {
            lcd = lcm_i64(lcd, d.abs());
        }
    }
    lcd
}

fn lcm_i64(a: i64, b: i64) -> i64 {
    if a == 0 || b == 0 { return 0; }
    (a / crate::helpers::gcd_i64(a.unsigned_abs(), b.unsigned_abs()) as i64) * b
}

fn poly_pow_helper(p: &maxima_poly::Poly, n: u32) -> maxima_poly::Poly {
    if n == 0 { return maxima_poly::Poly::constant(p.var, maxima_poly::Coeff::one()); }
    let mut result = p.clone();
    for _ in 1..n { result = result.mul(p); }
    result
}

fn get_func_name(expr: &Expr, var: &Expr) -> Option<String> {
    if let Expr::List { op: Operator::Named(id), args, .. } = expr {
        if args.len() == 1 && args[0] == *var {
            return Some(resolve(*id));
        }
    }
    None
}

pub(crate) fn extract_ncexpt(expr: &Expr) -> Option<(Expr, Expr)> {
    if let Expr::List { op: Operator::Named(id), args, .. } = expr {
        if resolve(*id) == "ncexpt" && args.len() == 2 {
            return Some((args[0].clone(), args[1].clone()));
        }
    }
    // A bare symbol x is implicitly x^^1
    if let Expr::Symbol(_) = expr {
        return Some((expr.clone(), Expr::int(1)));
    }
    None
}

pub(crate) fn normalize_abs_arg(expr: &Expr) -> Expr {
    // For sums, if the first term is negative, negate the whole sum
    if let Expr::List { op: Operator::MPlus, args, .. } = expr {
        if let Some(first) = args.first() {
            let is_neg = match first {
                Expr::Integer(n) => *n < 0,
                Expr::List { op: Operator::MTimes, args: f, .. } => {
                    matches!(f.first(), Some(Expr::Integer(n)) if *n < 0)
                }
                _ => false,
            };
            if is_neg {
                return simplify(&Expr::neg(expr.clone()));
            }
        }
    }
    // For products, strip negative coefficient and normalize sum factors
    if let Expr::List { op: Operator::MTimes, args, .. } = expr {
        if let Some(Expr::Integer(n)) = args.first() {
            if *n < 0 {
                let mut pos = args.clone();
                pos[0] = Expr::int(n.abs());
                if pos[0] == Expr::int(1) { pos.remove(0); }
                let result = if pos.len() == 1 { pos.pop().unwrap() }
                else { simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: pos }) };
                return normalize_abs_arg(&result);
            }
        }
        // Check for sum factors with negative leading terms
        let mut new_args = args.clone();
        let mut changed = false;
        for arg in &mut new_args {
            if let Expr::List { op: Operator::MPlus, args: sum_args, .. } = arg {
                if let Some(first) = sum_args.first() {
                    let is_neg = match first {
                        Expr::Integer(n) => *n < 0,
                        Expr::List { op: Operator::MTimes, args: f, .. } => {
                            matches!(f.first(), Some(Expr::Integer(n)) if *n < 0)
                        }
                        _ => false,
                    };
                    if is_neg {
                        *arg = simplify(&Expr::neg(arg.clone()));
                        changed = true;
                    }
                }
            }
        }
        if changed {
            return simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: new_args });
        }
    }
    expr.clone()
}

/// Known definite integral formulas.
pub(crate) fn try_known_definite_integral(f: &Expr, var: &Expr, a: &Expr, b: &Expr) -> Option<Expr> {
    let is_inf = |e: &Expr| matches!(e, Expr::Symbol(id) if { let n = resolve(*id); n == "inf" || n == "infinity" });
    let is_minf = |e: &Expr| matches!(e, Expr::Symbol(id) if resolve(*id) == "minf");

    // ∫_{-∞}^{∞} exp(-x²) dx = √π (Gaussian integral)
    if is_minf(a) && is_inf(b) {
        if let Expr::List { op: Operator::Named(id), args, .. } = f {
            if resolve(*id) == "exp" && args.len() == 1 {
                let inner = simplify(&args[0]);
                // exp(-x²)
                if let Expr::List { op: Operator::MTimes, args: ma, .. } = &inner {
                    if ma.len() == 2 {
                        let is_neg1 = |e: &Expr| matches!(e, Expr::Integer(n) if *n == -1);
                        let is_var_sq = |e: &Expr| matches!(e, Expr::List { op: Operator::MExpt, args, .. } if args.len() == 2 && args[0] == *var && args[1] == Expr::int(2));
                        if (is_neg1(&ma[0]) && is_var_sq(&ma[1])) || (is_neg1(&ma[1]) && is_var_sq(&ma[0])) {
                            return Some(Expr::call("sqrt", vec![Expr::sym("%pi")]));
                        }
                    }
                }
            }
        }
    }

    // ∫_0^∞ x^(2n)·exp(-x²) dx = (2n-1)!!·√π / 2^(n+1)
    // ∫_{-∞}^∞ x^(2n)·exp(-x²) dx = (2n-1)!!·√π / 2^n (double by symmetry for even power)
    if (*a == Expr::int(0) && is_inf(b)) || (is_minf(a) && is_inf(b)) {
        if let Expr::List { op: Operator::MTimes, args: margs, .. } = f {
            if margs.len() == 2 {
                let mut power = None;
                let mut has_gauss = false;
                for arg in margs {
                    if let Expr::List { op: Operator::MExpt, args: pa, .. } = arg {
                        if pa.len() == 2 && pa[0] == *var {
                            if let Some(n) = to_i64(&pa[1]) {
                                if n > 0 && n % 2 == 0 { power = Some(n); }
                            }
                        }
                    }
                    if let Expr::List { op: Operator::Named(eid), args: ea, .. } = arg {
                        if resolve(*eid) == "exp" && ea.len() == 1 {
                            let inner = simplify(&ea[0]);
                            if let Expr::List { op: Operator::MTimes, args: ma, .. } = &inner {
                                if ma.len() == 2 {
                                    let neg1 = ma.iter().any(|e| matches!(e, Expr::Integer(n) if *n == -1));
                                    let xsq = ma.iter().any(|e| matches!(e, Expr::List { op: Operator::MExpt, args, .. } if args.len() == 2 && args[0] == *var && args[1] == Expr::int(2)));
                                    if neg1 && xsq { has_gauss = true; }
                                }
                            }
                        }
                    }
                }
                if has_gauss {
                    if let Some(two_n) = power {
                        let n = two_n / 2;
                        // (2n-1)!! = 1·3·5·...·(2n-1), use i128 to extend range
                        let mut double_fact = 1i128;
                        let mut overflowed = false;
                        for k in 1..=n {
                            match double_fact.checked_mul(2*k as i128 - 1) {
                                Some(v) => double_fact = v,
                                None => { overflowed = true; break; }
                            }
                        }
                        if overflowed { return None; }
                        let from_zero = *a == Expr::int(0);
                        let shift = if from_zero { n + 1 } else { n };
                        if shift >= 63 { return None; }
                        let denom = 1i128 << shift;
                        let g = {
                            let (mut a, mut b) = (double_fact.unsigned_abs(), denom.unsigned_abs());
                            while b != 0 { let t = b; b = a % b; a = t; } a as i128
                        };
                        let num = (double_fact / g) as i64;
                        let den = (denom / g) as i64;
                        return Some(simplify(&Expr::div(
                            Expr::mul(Expr::int(num), Expr::call("sqrt", vec![Expr::sym("%pi")])),
                            Expr::int(den))));
                    }
                }
            }
        }
    }

    // Laplace transform table: ∫₀^∞ exp(-s·x)·cos(b·x) dx = s/(s²+b²)
    //                          ∫₀^∞ exp(-s·x)·sin(b·x) dx = b/(s²+b²)
    if *a == Expr::int(0) && is_inf(b) {
        if let Expr::List { op: Operator::MTimes, args: margs, .. } = f {
            if margs.len() == 2 {
                let mut exp_coeff = None; // s in exp(-sx)
                let mut trig_fn = None;   // "sin" or "cos"
                let mut trig_coeff = None; // b in sin(bx)/cos(bx)
                for arg in margs {
                    if let Expr::List { op: Operator::Named(eid), args: ea, .. } = arg {
                        let fname = resolve(*eid);
                        if fname == "exp" && ea.len() == 1 {
                            let inner = simplify(&ea[0]);
                            // exp(-s*x): extract s
                            if let Expr::List { op: Operator::MTimes, args: ma, .. } = &inner {
                                if ma.len() == 2 {
                                    let (c, v) = if ma[1] == *var { (&ma[0], true) }
                                        else if ma[0] == *var { (&ma[1], true) }
                                        else { (&ma[0], false) };
                                    if v { if let Some(cv) = to_f64(c) { if cv < 0.0 { exp_coeff = Some(-cv); } } }
                                }
                            } else if inner == Expr::neg(var.clone()) { exp_coeff = Some(1.0); }
                        }
                        if (fname == "sin" || fname == "cos") && ea.len() == 1 {
                            trig_fn = Some(fname);
                            if ea[0] == *var {
                                trig_coeff = Some(1.0);
                            } else if let Expr::List { op: Operator::MTimes, args: ta, .. } = &ea[0] {
                                if ta.len() == 2 {
                                    let (c, v) = if ta[1] == *var { (&ta[0], true) }
                                        else if ta[0] == *var { (&ta[1], true) }
                                        else { (&ta[0], false) };
                                    if v { trig_coeff = to_f64(c); }
                                }
                            }
                        }
                    }
                }
                if let (Some(s), Some(trig), Some(b_val)) = (exp_coeff, trig_fn, trig_coeff) {
                    let denom = s * s + b_val * b_val;
                    if denom.abs() > 1e-15 {
                        let result = match trig.as_str() {
                            "cos" => s / denom,
                            "sin" => b_val / denom,
                            _ => return None,
                        };
                        return Some(float_to_expr(result));
                    }
                }
            }
        }
    }

    // ∫₀^∞ exp(-a·x²)·cos(b·x) dx = √(π/a)/2 · exp(-b²/(4a))
    if *a == Expr::int(0) && is_inf(b) {
        if let Expr::List { op: Operator::MTimes, args: margs, .. } = f {
            if margs.len() == 2 {
                let mut gauss_coeff = None; // a in exp(-a·x²)
                let mut cos_coeff = None;   // b in cos(b·x)
                for arg in margs {
                    if let Expr::List { op: Operator::Named(eid), args: ea, .. } = arg {
                        let fname = resolve(*eid);
                        if fname == "exp" && ea.len() == 1 {
                            // Match exp(-a·x²): extract coefficient a
                            let inner = simplify(&ea[0]);
                            if let Expr::List { op: Operator::MTimes, args: ma, .. } = &inner {
                                // Collect x² factor and remaining coefficient
                                let mut has_xsq = false;
                                let mut coeff_parts: Vec<&Expr> = Vec::new();
                                for m in ma {
                                    if !has_xsq {
                                        if let Expr::List { op: Operator::MExpt, args: pa, .. } = m {
                                            if pa.len() == 2 && pa[0] == *var && pa[1] == Expr::int(2) {
                                                has_xsq = true;
                                                continue;
                                            }
                                        }
                                    }
                                    coeff_parts.push(m);
                                }
                                if has_xsq {
                                    // Coefficient is the product of remaining parts (should be negative for convergence)
                                    let neg_a = if coeff_parts.len() == 1 {
                                        coeff_parts[0].clone()
                                    } else {
                                        Expr::List { op: Operator::MTimes, simplified: false, args: coeff_parts.into_iter().cloned().collect() }
                                    };
                                    // neg_a should be negative; a = -neg_a
                                    if let Some(fv) = to_f64(&neg_a) {
                                        if fv < 0.0 { gauss_coeff = Some(-fv); }
                                    }
                                }
                            }
                        }
                        if fname == "cos" && ea.len() == 1 {
                            if ea[0] == *var { cos_coeff = Some(1.0); }
                            else if let Expr::List { op: Operator::MTimes, args: ta, .. } = &ea[0] {
                                if ta.len() == 2 {
                                    let (c, v) = if ta[1] == *var { (&ta[0], true) }
                                        else if ta[0] == *var { (&ta[1], true) }
                                        else { (&ta[0], false) };
                                    if v { cos_coeff = to_f64(c); }
                                }
                            }
                        }
                    }
                }
                if let (Some(a_val), Some(b_val)) = (gauss_coeff, cos_coeff) {
                    // √(π/a)/2 · exp(-b²/(4a))
                    let exp_arg = -(b_val * b_val) / (4.0 * a_val);
                    return Some(simplify(&Expr::mul(
                        Expr::div(Expr::call("sqrt", vec![
                            Expr::div(Expr::sym("%pi"), float_to_expr(a_val))]), Expr::int(2)),
                        Expr::call("exp", vec![float_to_expr(exp_arg)]))));
                }
            }
        }
    }

    // ∫_0^∞ exp(-x) dx = 1
    if *a == Expr::int(0) && is_inf(b) {
        if let Expr::List { op: Operator::Named(id), args, .. } = f {
            if resolve(*id) == "exp" && args.len() == 1 {
                let inner = simplify(&args[0]);
                if inner == Expr::neg(var.clone()) {
                    return Some(Expr::int(1));
                }
                // exp(-a*x): ∫_0^∞ = 1/a for a > 0
                if let Expr::List { op: Operator::MTimes, args: ma, .. } = &inner {
                    if ma.len() == 2 {
                        let (coeff, is_var) = if ma[1] == *var { (&ma[0], true) }
                            else if ma[0] == *var { (&ma[1], true) }
                            else { (&ma[0], false) };
                        if is_var {
                            if let Some(v) = to_f64(coeff) {
                                if v < 0.0 {
                                    return Some(simplify(&Expr::div(Expr::int(1), Expr::neg(coeff.clone()))));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // ∫_0^∞ x^n * exp(-a*x) dx = n! / a^(n+1) (Gamma function)
    if *a == Expr::int(0) && is_inf(b) {
        if let Expr::List { op: Operator::MTimes, args, .. } = f {
            // Look for x^n * exp(-a*x) pattern
            let mut power_expr = None;
            let mut exp_coeff = None;
            let mut constant = Expr::int(1);
            for arg in args {
                if let Expr::List { op: Operator::MExpt, args: pa, .. } = arg {
                    if pa.len() == 2 && pa[0] == *var && !contains_var(&pa[1], var) {
                        power_expr = Some(pa[1].clone());
                        continue;
                    }
                }
                if arg == var { power_expr = Some(Expr::int(1)); continue; }
                if let Expr::List { op: Operator::Named(eid), args: ea, .. } = arg {
                    if resolve(*eid) == "exp" && ea.len() == 1 {
                        // exp(-a*x): extract coefficient a
                        let inner = simplify(&ea[0]);
                        if let Expr::List { op: Operator::MTimes, args: ma, .. } = &inner {
                            if ma.len() == 2 {
                                let (c, _v) = if ma[1] == *var { (&ma[0], &ma[1]) }
                                    else if ma[0] == *var { (&ma[1], &ma[0]) }
                                    else { continue };
                                if let Some(cv) = to_f64(c) {
                                    if cv < 0.0 { exp_coeff = Some(Expr::neg(c.clone())); }
                                }
                            }
                        } else if inner == Expr::neg(var.clone()) {
                            exp_coeff = Some(Expr::int(1));
                        }
                        continue;
                    }
                }
                if !contains_var(arg, var) {
                    constant = simplify(&Expr::mul(constant, arg.clone()));
                }
            }
            if let (Some(n_expr), Some(a_expr)) = (power_expr, exp_coeff) {
                // Result: constant * n! / a^(n+1)
                let n_plus_1 = simplify(&Expr::add(n_expr.clone(), Expr::int(1)));
                // Evaluate factorial for integer n
                let fact = if let Some(ni) = to_i64(&n_expr) {
                    if ni >= 0 { Expr::int(factorial(ni)) } else { Expr::call("factorial", vec![n_expr]) }
                } else {
                    Expr::call("factorial", vec![n_expr])
                };
                let result = simplify(&Expr::div(
                    Expr::mul(constant, fact),
                    Expr::pow(a_expr, n_plus_1),
                ));
                return Some(result);
            }
        }
    }

    None
}

/// Residue-based integration for rational functions over (-∞, ∞).
/// ∫_{-∞}^{∞} P(x)/Q(x) dx = 2πi × Σ residues at upper half-plane poles
/// Residue-based integration: ∫_{-∞}^{∞} P(x)/Q(x) dx = 2πi × Σ(upper half-plane residues)
pub(crate) fn try_residue_integral(f: &Expr, var: &Expr) -> Option<Expr> {
    let (num, den) = extract_fraction(f)?;
    let var_id = if let Expr::Symbol(id) = var { *id } else { return None; };

    let np = maxima_poly::expr_to_poly(&expand(&num), var_id)?;
    let dp = maxima_poly::expr_to_poly(&expand(&den), var_id)?;

    let ndeg = np.degree().unwrap_or(0);
    let ddeg = dp.degree().unwrap_or(0);
    if ddeg < ndeg + 2 { return None; }

    let factors = maxima_poly::factor_poly(&dp);

    // Check: no real roots (all factors must be irreducible quadratics)
    let all_quadratic = factors.iter().all(|(f, _)| {
        if f.degree() == Some(2) {
            let a = get_poly_coeff(f, 2);
            let b = get_poly_coeff(f, 1);
            let c = get_poly_coeff(f, 0);
            4 * a * c - b * b > 0 // negative discriminant → no real roots
        } else {
            false
        }
    });
    if !all_quadratic { return None; }

    // For each irreducible quadratic factor (ax²+bx+c)^m:
    // Upper half-plane root: z₀ = (-b + i√disc)/(2a) where disc = 4ac-b²
    // For simple pole (m=1): residue = P(z₀) / Q'(z₀)
    // We compute 2πi × residue symbolically using the formula:
    //   ∫ P/(ax²+bx+c) = π * P(z₀_real, z₀_imag) / (a * √disc)
    //   (after expanding P(z₀) and taking the real part of 2πi × residue)

    // All simple poles case
    if factors.iter().all(|(_, m)| *m == 1) {
        return residue_simple_poles(&np, &dp, &factors);
    }

    // Handle repeated quadratic: single factor (ax²+c)^n
    if factors.len() == 1 {
        let (qi, m) = &factors[0];
        let a = get_poly_coeff(qi, 2);
        let b = get_poly_coeff(qi, 1);
        let c = get_poly_coeff(qi, 0);
        if b == 0 && a == 1 && c > 0 && np.is_constant() {
            // ∫ k/(x²+c)^m dx[-∞,∞] = k*π*(2m-2)! / (2^(2m-1) * ((m-1)!)² * c^(m-1) * √c)
            let k_val = match np.constant_term() { maxima_poly::Coeff::Int(n) => n, _ => return None };
            let m_val = *m as i64;
            let fact_2m2 = factorial(2*m_val - 2);
            let fact_m1 = factorial(m_val - 1);
            let pow_2 = 1i64 << (2*m_val - 1); // 2^(2m-1)
            let c_pow = c.pow((m_val - 1) as u32);
            // result = k * π * fact_2m2 / (pow_2 * fact_m1² * c^(m-1) * √c)
            return Some(simplify(&Expr::div(
                Expr::mul(Expr::int(k_val * fact_2m2), Expr::sym("%pi")),
                Expr::mul(Expr::int(pow_2 * fact_m1 * fact_m1 * c_pow),
                    Expr::call("sqrt", vec![Expr::int(c)])),
            )));
        }
    }

    None
}

/// Compute ∫_{-∞}^{∞} P(x)/Q(x) dx where Q has only simple irreducible quadratic factors.
/// Uses: result = 2πi × Σ residues at upper half-plane poles
/// For each factor ax²+bx+c: pole z₀ = (-b+i√d)/(2a), d = 4ac-b²
/// Residue = P(z₀) / Q'(z₀)
fn residue_simple_poles(
    np: &maxima_poly::Poly,
    dp: &maxima_poly::Poly,
    factors: &[(maxima_poly::Poly, u32)],
) -> Option<Expr> {
    // For the case of all distinct irreducible quadratics with b=0 (i.e., ax²+c):
    // Poles at ±i√(c/a). Upper pole: z₀ = i√(c/a).
    // P(z₀) = P(i√(c/a)) — evaluate P at the complex root
    // Q'(z₀) = product evaluation at z₀

    // For simplicity, handle the case where all factors have b=0 (centered quadratics)
    let all_centered = factors.iter().all(|(f, _)| get_poly_coeff(f, 1) == 0);
    if !all_centered { return None; }

    let _dp_deriv = dp.derivative();

    // Sum residues: for each factor ax²+c, the upper pole is z₀ = i*sqrt(c/a)
    // Residue = P(z₀) / Q'(z₀)
    // For real P and Q, 2πi × Σ residues is real and equals π × Σ(real part calculations)

    // Use the formula: for P(x)/((x²+a₁)(x²+a₂)...) with simple poles at ±i√aₖ,
    // ∫ = π × Σₖ P(i√aₖ) / (i√aₖ × Πⱼ≠ₖ(aₖ-aⱼ)... )
    // This simplifies for polynomial P.

    // Special case: constant numerator
    if np.is_constant() {
        let p_val = match np.constant_term() { maxima_poly::Coeff::Int(n) => n, _ => return None };
        let _total_num = 0i64;
        let _total_den = 1i64;

        for (_i, (fi, _)) in factors.iter().enumerate() {
            let ai = get_poly_coeff(fi, 2);
            let ci = get_poly_coeff(fi, 0);
            // Residue contribution: 1/(2*ai*i*√(ci/ai)) × 1/Πⱼ≠ᵢ((ci/ai - cⱼ/aⱼ))
            // Simpler: for (x²+αᵢ) factors where αᵢ=cᵢ/aᵢ,
            // residue at z₀=i√αᵢ: 1/(2i√αᵢ × Πⱼ≠ᵢ(-αᵢ+αⱼ))
            // Then 2πi × residue = π/(√αᵢ × Πⱼ≠ᵢ(αⱼ-αᵢ))
            // For single factor: π/√(a*c)
            if factors.len() == 1 {
                let product = ai * ci;
                let sqrt_val = (product as f64).sqrt();
                let sqrt_int = sqrt_val.round() as i64;
                let denom = if sqrt_int > 0 && sqrt_int * sqrt_int == product {
                    Expr::int(sqrt_int)
                } else {
                    Expr::call("sqrt", vec![Expr::int(product)])
                };
                return Some(simplify(&Expr::div(
                    Expr::mul(Expr::int(p_val), Expr::sym("%pi")),
                    denom,
                )));
            }
        }

        // Multiple factors: use partial fractions approach
        // ∫ 1/((x²+a)(x²+b)) = π/(√a+√b) × 1/√(ab)... this gets complex
        // Compute numerically for now if all values are known
        if factors.len() == 2 {
            let a1 = get_poly_coeff(&factors[0].0, 0) as f64 / get_poly_coeff(&factors[0].0, 2) as f64;
            let a2 = get_poly_coeff(&factors[1].0, 0) as f64 / get_poly_coeff(&factors[1].0, 2) as f64;
            if a1 > 0.0 && a2 > 0.0 && (a1 - a2).abs() > 1e-12 {
                // ∫ 1/((x²+a)(x²+b)) = π/((b-a)√b) + π/((a-b)√a) = π(1/(√b) - 1/(√a))/(b-a)
                // = π / ((√a + √b) × √a × √b) = π / (√(ab)(√a+√b))
                // Simpler: = π / (√a × √b × (√a + √b))... no.
                // Actually: residue at i√a: 1/(2i√a × (-a+b)) → contribution: π/(√a(b-a))
                //           residue at i√b: 1/(2i√b × (a-b))  → contribution: -π/(√b(a-b)) = π/(√b(b-a))
                // Total: π(1/√a - 1/√b)/(b-a) = π/(√a×√b×(√a+√b))... hmm
                // Let me just compute: π × (1/(√a₁×(a₂-a₁)) + 1/(√a₂×(a₁-a₂)))
                //   = π × (1/(√a₁(a₂-a₁)) - 1/(√a₂(a₂-a₁)))
                //   = π/(a₂-a₁) × (1/√a₁ - 1/√a₂)
                //   = π/(a₂-a₁) × (√a₂ - √a₁)/(√a₁×√a₂)
                // For a₁=1, a₂=4: π/3 × (2-1)/(1×2) = π/6
                let (a1i, a2i) = (
                    get_poly_coeff(&factors[0].0, 0),
                    get_poly_coeff(&factors[1].0, 0),
                );
                let lc1 = get_poly_coeff(&factors[0].0, 2);
                let lc2 = get_poly_coeff(&factors[1].0, 2);
                if lc1 == 1 && lc2 == 1 && a1i != a2i {
                    // ∫ p/((x²+a₁)(x²+a₂)) dx[-∞,∞] = pπ / (√(a₁a₂) × (√a₁+√a₂))
                    return Some(simplify(&Expr::div(
                        Expr::mul(Expr::int(p_val), Expr::sym("%pi")),
                        Expr::mul(
                            Expr::call("sqrt", vec![Expr::int(a1i * a2i)]),
                            Expr::add(
                                Expr::call("sqrt", vec![Expr::int(a1i)]),
                                Expr::call("sqrt", vec![Expr::int(a2i)]),
                            ),
                        ),
                    )));
                }
            }
        }
    }

    None
}

/// Try factoring an irreducible degree-4 polynomial over Q(√d) and integrating.
/// For x⁴+bx²+c type: try d = b²-4c, factor as (x²+αx+β)(x²-αx+β) where α²=d.
fn try_algebraic_factor_integrate(
    num: &maxima_poly::Poly, den: &maxima_poly::Poly,
    var_id: maxima_core::SymbolId, var: &Expr,
) -> Option<Expr> {
    // Only handle x⁴ + bx² + c (no odd-degree terms)
    let a4 = get_poly_coeff(den, 4);
    let a3 = get_poly_coeff(den, 3);
    let a2 = get_poly_coeff(den, 2);
    let a1 = get_poly_coeff(den, 1);
    let a0 = get_poly_coeff(den, 0);
    if a4 != 1 || a3 != 0 || a1 != 0 { return None; }

    // x⁴ + a2·x² + a0
    // Factor as (x² + α·x + β)(x² - α·x + β) where α² = 2β - a2, β² = a0
    // So β = √a0 (need a0 > 0 and perfect square or we use √a0)
    // and α² = 2β - a2
    // For x⁴+1: a2=0, a0=1 → β=1, α²=2 → α=√2
    // For x⁴+x²+1: a2=1, a0=1 → β=1, α²=2-1=1 → α=1 (rational!)

    // Try: β such that β² = a0, then α² = 2β - a2
    let beta_sq = a0;
    if beta_sq <= 0 { return None; }
    let beta_f = (beta_sq as f64).sqrt();
    let beta = beta_f.round() as i64;
    if beta * beta != beta_sq { return None; } // β must be integer

    let alpha_sq = 2 * beta - a2;
    if alpha_sq <= 0 { return None; }

    // Check if α² is a perfect square (rational factoring)
    let alpha_f = (alpha_sq as f64).sqrt();
    let alpha_int = alpha_f.round() as i64;

    if alpha_int * alpha_int == alpha_sq {
        // Rational factoring! (x²+α·x+β)(x²-α·x+β)
        let f1 = maxima_poly::Poly { var: var_id, terms: vec![
            (2, maxima_poly::Coeff::Int(1)),
            (1, maxima_poly::Coeff::Int(alpha_int)),
            (0, maxima_poly::Coeff::Int(beta)),
        ]};
        let f2 = maxima_poly::Poly { var: var_id, terms: vec![
            (2, maxima_poly::Coeff::Int(1)),
            (1, maxima_poly::Coeff::Int(-alpha_int)),
            (0, maxima_poly::Coeff::Int(beta)),
        ]};
        let factors = vec![(f1, 1u32), (f2, 1u32)];
        return integrate_partfrac_mixed(num, &factors, var);
    }

    // Algebraic factoring over Q(√α²) = Q(√(2β-a2))
    // (x² + √d·x + β)(x² - √d·x + β) where d = alpha_sq
    let d = alpha_sq;
    let sqrt_d = Expr::call("sqrt", vec![Expr::int(d)]);

    // Partial fractions: P(x) / ((x²+√d·x+β)(x²-√d·x+β))
    // Decompose as (Ax+B)/(x²+√d·x+β) + (Cx+D)/(x²-√d·x+β)
    // where A,B,C,D may involve √d. Find them via numeric evaluation.
    {
        let x = var.clone();
        let disc = d - 4 * beta;
        if disc < 0 {
            let delta = -disc; // 4β - d > 0
            let sqrt_d_f = (d as f64).sqrt();

            // Evaluate P(x)/q1(x) and P(x)/q2(x) at test points to find partial fraction coeffs
            // P/(q1·q2) = (Ax+B)/q1 + (Cx+D)/q2
            // → P = (Ax+B)·q2 + (Cx+D)·q1
            // Evaluate at 4 points to get 4 equations in A,B,C,D
            let _env = crate::Environment::new();
            let test_xs = [0.5f64, 1.0, 1.5, 2.0];
            let mut mat = [[0.0f64; 4]; 4];
            let mut rhs = [0.0f64; 4];

            for (row, &xv) in test_xs.iter().enumerate() {
                let q1v = xv*xv + sqrt_d_f*xv + beta as f64;
                let q2v = xv*xv - sqrt_d_f*xv + beta as f64;
                // (Ax+B)·q2 + (Cx+D)·q1 = P(x)
                // A·x·q2 + B·q2 + C·x·q1 + D·q1 = P(x)
                mat[row][0] = xv * q2v; // coeff of A
                mat[row][1] = q2v;       // coeff of B
                mat[row][2] = xv * q1v; // coeff of C
                mat[row][3] = q1v;       // coeff of D

                // Evaluate P at xv
                let p_at = num.eval_at(&maxima_poly::Coeff::Rat((xv * 1000.0) as i64, 1000));
                rhs[row] = match p_at {
                    maxima_poly::Coeff::Int(n) => n as f64,
                    maxima_poly::Coeff::Rat(n, d) => n as f64 / d as f64,
                };
            }

            // Solve 4x4 system via Gaussian elimination
            if let Some(coeffs) = solve_4x4(&mat, &rhs) {
                let [a_f, b_f, c_f, d_f] = coeffs;

                // Build result: integrate each piece
                // ∫ (Ax+B)/(x²+√d·x+β) + ∫ (Cx+D)/(x²-√d·x+β)
                let mut terms = Vec::new();

                for (sign, a_val, bd_val) in [(1.0, a_f, b_f), (-1.0, c_f, d_f)] {
                    let alpha_signed = sign * sqrt_d_f;

                    // ∫ (ax+b)/(x²+αx+β) = (a/2)·log(x²+αx+β) + (2b-aα)/√δ · atan((2x+α)/√δ)
                    let log_coeff = a_val / 2.0;
                    let atan_num = 2.0 * bd_val - a_val * alpha_signed;
                    let sqrt_delta = (delta as f64).sqrt();

                    let q_expr = Expr::add(Expr::add(
                        Expr::pow(x.clone(), Expr::int(2)),
                        Expr::mul(Expr::mul(Expr::int(sign as i64), sqrt_d.clone()), x.clone())),
                        Expr::int(beta));

                    if log_coeff.abs() > 1e-12 {
                        let lc = float_to_expr(log_coeff);
                        terms.push(simplify(&Expr::mul(lc, Expr::call("log", vec![q_expr.clone()]))));
                    }
                    if atan_num.abs() > 1e-12 && sqrt_delta > 1e-12 {
                        let ac = float_to_expr(atan_num / sqrt_delta);
                        let inner_add = Expr::add(Expr::mul(Expr::int(2), x.clone()),
                            Expr::mul(Expr::int(sign as i64), sqrt_d.clone()));
                        let sqrt_d_expr = Expr::call("sqrt", vec![Expr::int(delta)]);
                        terms.push(simplify(&Expr::mul(ac,
                            Expr::call("atan", vec![simplify(&Expr::div(inner_add, sqrt_d_expr))]))));
                    }
                }

                if !terms.is_empty() {
                    return Some(simplify(&if terms.len() == 1 { terms.remove(0) }
                        else { Expr::List { op: Operator::MPlus, simplified: false, args: terms } }));
                }
            }
        }
    }

    if num.is_constant() {
        let c_val = match num.constant_term() {
            maxima_poly::Coeff::Int(n) => n,
            _ => return None,
        };
        let x = var.clone();

        // ∫ c/((x²+√d·x+β)(x²-√d·x+β)) dx
        // By partial fractions over Q(√d):
        // = c/(2d) · [∫ (√d·x + (2β-d))/(x²+√d·x+β) - ∫ (√d·x - (2β-d))/(x²-√d·x+β)]
        // Wait, let me use a simpler approach: direct formula for this specific form.
        //
        // For x⁴+1 (d=2, β=1): the integral is:
        // (1/(2√2))·log((x²+√2·x+1)/(x²-√2·x+1)) + (1/√2)·[atan(√2·x+1)+atan(√2·x-1)]
        //
        // General: for x⁴ + a2·x² + a0 with α²=d:
        // Factor 1: x² + √d·x + β, disc = d - 4β
        // Factor 2: x² - √d·x + β, disc = d - 4β (same)

        let disc = d - 4 * beta; // discriminant of each quadratic

        // Each quadratic has the form x² ± √d·x + β
        // ∫ c/(x² + √d·x + β) = completing square:
        //   x² + √d·x + β = (x + √d/2)² + (β - d/4)
        //   = (x + √d/2)² + (-disc/4)
        // When disc < 0 (both factors irreducible): atan form

        if disc < 0 {
            let delta = -disc; // 4β - d > 0
            let sqrt_delta = Expr::call("sqrt", vec![Expr::int(delta)]);

            // Each factor contributes log + atan terms.
            // By partial fraction decomposition and integration:
            // Result = c/(2√d) · log((x²+√d·x+β)/(x²-√d·x+β))
            //        + c/√δ · [atan((2x+√d)/√δ) + atan((2x-√d)/√δ)]
            // where δ = 4β - d

            let log_part = simplify(&Expr::div(
                Expr::mul(Expr::int(c_val),
                    Expr::call("log", vec![Expr::div(
                        Expr::add(Expr::add(Expr::pow(x.clone(), Expr::int(2)),
                            Expr::mul(sqrt_d.clone(), x.clone())), Expr::int(beta)),
                        Expr::add(Expr::sub(Expr::pow(x.clone(), Expr::int(2)),
                            Expr::mul(sqrt_d.clone(), x.clone())), Expr::int(beta)),
                    )])),
                Expr::mul(Expr::int(2), sqrt_d.clone()),
            ));

            let atan1 = Expr::call("atan", vec![simplify(&Expr::div(
                Expr::add(Expr::mul(Expr::int(2), x.clone()), sqrt_d.clone()),
                sqrt_delta.clone()))]);
            let atan2 = Expr::call("atan", vec![simplify(&Expr::div(
                Expr::sub(Expr::mul(Expr::int(2), x.clone()), sqrt_d.clone()),
                sqrt_delta.clone()))]);

            let atan_part = simplify(&Expr::div(
                Expr::mul(Expr::int(c_val), Expr::add(atan1, atan2)),
                sqrt_delta,
            ));

            return Some(simplify(&Expr::add(log_part, atan_part)));
        }
    }

    None
}

fn solve_4x4(mat: &[[f64; 4]; 4], rhs: &[f64; 4]) -> Option<[f64; 4]> {
    let mut a = *mat;
    let mut b = *rhs;
    for col in 0..4 {
        let mut pivot = col;
        for row in (col + 1)..4 {
            if a[row][col].abs() > a[pivot][col].abs() { pivot = row; }
        }
        if a[pivot][col].abs() < 1e-15 { return None; }
        a.swap(col, pivot);
        b.swap(col, pivot);
        let diag = a[col][col];
        for row in (col + 1)..4 {
            let factor = a[row][col] / diag;
            for k in col..4 { a[row][k] -= factor * a[col][k]; }
            b[row] -= factor * b[col];
        }
    }
    let mut x = [0.0f64; 4];
    for col in (0..4).rev() {
        x[col] = b[col];
        for k in (col + 1)..4 { x[col] -= a[col][k] * x[k]; }
        x[col] /= a[col][col];
    }
    Some(x)
}

fn float_to_expr(v: f64) -> Expr {
    if (v - v.round()).abs() < 1e-10 {
        let n = v.round() as i64;
        return Expr::int(n);
    }
    for d in 1..=12i64 {
        let n = (v * d as f64).round() as i64;
        if ((n as f64 / d as f64) - v).abs() < 1e-10 {
            let g = crate::helpers::gcd_i64(n.unsigned_abs(), d.unsigned_abs()) as i64;
            let (rn, rd) = (n / g, d / g);
            if rd == 1 { return Expr::int(rn); }
            return Expr::Rational { num: rn, den: rd };
        }
    }
    // Try n/√d form: v = p/q * 1/√d or p/q * √d
    for sq in [2i64, 3, 5, 6, 7] {
        let sqrt_sq = (sq as f64).sqrt();
        let ratio = v / sqrt_sq;
        for d in 1..=12i64 {
            let n = (ratio * d as f64).round() as i64;
            if ((n as f64 / d as f64) - ratio).abs() < 1e-10 {
                let g = crate::helpers::gcd_i64(n.unsigned_abs(), d.unsigned_abs()) as i64;
                let (rn, rd) = (n / g, d / g);
                let coeff = if rd == 1 { Expr::int(rn) } else { Expr::Rational { num: rn, den: rd } };
                return simplify(&Expr::mul(coeff, Expr::call("sqrt", vec![Expr::int(sq)])));
            }
        }
        let ratio_inv = v * sqrt_sq;
        for d in 1..=12i64 {
            let n = (ratio_inv * d as f64).round() as i64;
            if ((n as f64 / d as f64) - ratio_inv).abs() < 1e-10 {
                let g = crate::helpers::gcd_i64(n.unsigned_abs(), d.unsigned_abs()) as i64;
                let (rn, rd) = (n / g, d / g);
                let coeff = if rd == 1 { Expr::int(rn) } else { Expr::Rational { num: rn, den: rd } };
                return simplify(&Expr::div(coeff, Expr::call("sqrt", vec![Expr::int(sq)])));
            }
        }
    }
    Expr::Float(v)
}

fn factorial(n: i64) -> i64 {
    (1..=n).product::<i64>().max(1)
}

fn get_poly_coeff(p: &maxima_poly::Poly, exp: u32) -> i64 {
    p.terms.iter()
        .find(|(e, _)| *e == exp)
        .and_then(|(_, c)| if let maxima_poly::Coeff::Int(n) = c { Some(*n) } else { None })
        .unwrap_or(0)
}

pub(crate) fn table_integrate_pub(f: &Expr, var: &Expr) -> Expr {
    table_integrate(f, var)
}

fn normalize_sqrt_powers(f: &Expr, var: &Expr) -> Expr {
    match f {
        // sqrt(x)^n → x^(n/2)
        Expr::List { op: Operator::MExpt, args, .. } if args.len() == 2 => {
            if let Expr::List { op: Operator::Named(id), args: sa, .. } = &args[0] {
                if resolve(*id) == "sqrt" && sa.len() == 1 && sa[0] == *var {
                    if let Some(n) = to_i64(&args[1]) {
                        return Expr::pow(var.clone(), Expr::Rational { num: n, den: 2 });
                    }
                }
            }
            Expr::pow(normalize_sqrt_powers(&args[0], var), normalize_sqrt_powers(&args[1], var))
        }
        // sqrt(x) alone → x^(1/2)
        Expr::List { op: Operator::Named(id), args, .. }
            if resolve(*id) == "sqrt" && args.len() == 1 && args[0] == *var =>
        {
            Expr::pow(var.clone(), Expr::Rational { num: 1, den: 2 })
        }
        // Recurse into + and *
        Expr::List { op: op @ (Operator::MPlus | Operator::MTimes), args, .. } => {
            let new_args: Vec<Expr> = args.iter().map(|a| normalize_sqrt_powers(a, var)).collect();
            Expr::List { op: *op, simplified: false, args: new_args }
        }
        _ => f.clone(),
    }
}

pub(crate) fn table_integrate(f: &Expr, var: &Expr) -> Expr {
    // Normalize sqrt(var)^n → var^(n/2) so the power rule handles it
    let f = &normalize_sqrt_powers(f, var);

    // Try Risch-Norman heuristic first (fast path for transcendental integrands)
    if let Some(result) = crate::risch_norman::risch_norman(f, var) {
        return result;
    }

    match f {
        // ∫ constant dx = constant * x
        Expr::Integer(_) | Expr::Float(_) | Expr::Rational { .. } => {
            return simplify(&Expr::mul(f.clone(), var.clone()));
        }
        // ∫ x dx = x^2/2
        Expr::Symbol(_) if f == var => {
            return simplify(&Expr::div(Expr::pow(var.clone(), Expr::int(2)), Expr::int(2)));
        }
        Expr::List { op, args, .. } => match op {
            // ∫ (a+b) dx = ∫a dx + ∫b dx
            Operator::MPlus => {
                let terms: Vec<Expr> = args.iter().map(|a| table_integrate(a, var)).collect();
                return simplify(&Expr::List {
                    op: Operator::MPlus,
                    simplified: false,
                    args: terms,
                });
            }
            // ∫ c*f dx = c * ∫f dx (if c doesn't contain var)
            Operator::MTimes => {
                let (constant, dependent): (Vec<&Expr>, Vec<&Expr>) =
                    args.iter().partition(|a| !contains_var(a, var));
                if !constant.is_empty() && !dependent.is_empty() {
                    let c = if constant.len() == 1 {
                        constant[0].clone()
                    } else {
                        Expr::List { op: Operator::MTimes, simplified: false, args: constant.into_iter().cloned().collect() }
                    };
                    let f_dep = if dependent.len() == 1 {
                        dependent[0].clone()
                    } else {
                        Expr::List { op: Operator::MTimes, simplified: false, args: dependent.into_iter().cloned().collect() }
                    };
                    let int_f = table_integrate(&f_dep, var);
                    if !int_f.to_string().starts_with("integrate") {
                        return simplify(&Expr::mul(c, int_f));
                    }
                }
                // Product of two functions: exp*sin, exp*cos
                if args.len() == 2 {
                    let (f1, f2) = (&args[0], &args[1]);
                    if let (Some(n1), Some(n2)) = (get_func_name(f1, var), get_func_name(f2, var)) {
                        let x = var.clone();
                        match (n1.as_str(), n2.as_str()) {
                            // ∫ exp(x)*sin(x) = exp(x)*(sin(x)-cos(x))/2
                            ("exp", "sin") | ("sin", "exp") => return simplify(&Expr::div(
                                Expr::mul(Expr::call("exp", vec![x.clone()]),
                                    Expr::sub(Expr::call("sin", vec![x.clone()]), Expr::call("cos", vec![x]))),
                                Expr::int(2),
                            )),
                            // ∫ exp(x)*cos(x) = exp(x)*(sin(x)+cos(x))/2
                            ("exp", "cos") | ("cos", "exp") => return simplify(&Expr::div(
                                Expr::mul(Expr::call("exp", vec![x.clone()]),
                                    Expr::add(Expr::call("sin", vec![x.clone()]), Expr::call("cos", vec![x]))),
                                Expr::int(2),
                            )),
                            // #9: ∫ sec(u)*tan(u) = sec(u)
                            ("sec", "tan") | ("tan", "sec") =>
                                return Expr::call("sec", vec![x]),
                            // #10: ∫ csc(u)*cot(u) = -csc(u)
                            ("csc", "cot") | ("cot", "csc") =>
                                return Expr::neg(Expr::call("csc", vec![x])),
                            // #57: ∫ sech(u)*tanh(u) = -sech(u)
                            ("sech", "tanh") | ("tanh", "sech") =>
                                return Expr::neg(Expr::call("sech", vec![x])),
                            // #58: ∫ csch(u)*coth(u) = -csch(u)
                            ("csch", "coth") | ("coth", "csch") =>
                                return Expr::neg(Expr::call("csch", vec![x])),
                            _ => {}
                        }
                    }
                }
                // ∫ f(x)^n * g(x) where g is related to f' (trig substitution patterns)
                if args.len() == 2 {
                    for (base_idx, other_idx) in [(0,1), (1,0)] {
                        if let Expr::List { op: Operator::MExpt, args: pa, .. } = &args[base_idx] {
                            if pa.len() == 2 {
                                if let (Expr::List { op: Operator::Named(fid), args: fa, .. }, Some(n)) =
                                    (&pa[0], to_i64(&pa[1])) {
                                    if fa.len() == 1 && fa[0] == *var && n != 0 {
                                        if let Expr::List { op: Operator::Named(gid), args: ga, .. } = &args[other_idx] {
                                            if ga.len() == 1 && ga[0] == *var {
                                                let fname = resolve(*fid);
                                                let gname = resolve(*gid);
                                                let x = var.clone();
                                                // #165: ∫ sec^n*tan = sec^n/n
                                                if fname == "sec" && gname == "tan" {
                                                    return simplify(&Expr::div(
                                                        Expr::pow(Expr::call("sec", vec![x]), Expr::int(n)),
                                                        Expr::int(n)));
                                                }
                                                // #166: ∫ csc^n*cot = -csc^n/n
                                                if fname == "csc" && gname == "cot" {
                                                    return simplify(&Expr::div(
                                                        Expr::neg(Expr::pow(Expr::call("csc", vec![x]), Expr::int(n))),
                                                        Expr::int(n)));
                                                }
                                                // ∫ cos^n*sin = -cos^(n+1)/(n+1) — but n=-1 needs the log form.
                                                if fname == "cos" && gname == "sin" {
                                                    if n == -1 {
                                                        // ∫ sin/cos = -log(cos)
                                                        return Expr::neg(Expr::call("log", vec![Expr::call("cos", vec![x])]));
                                                    }
                                                    return simplify(&Expr::div(
                                                        Expr::neg(Expr::pow(Expr::call("cos", vec![x]), Expr::int(n+1))),
                                                        Expr::int(n+1)));
                                                }
                                                // ∫ cosh^n*sinh = cosh^(n+1)/(n+1)
                                                if fname == "cosh" && gname == "sinh" {
                                                    if n == -1 {
                                                        // ∫ sinh/cosh = log(cosh)
                                                        return Expr::call("log", vec![Expr::call("cosh", vec![x])]);
                                                    }
                                                    return simplify(&Expr::div(
                                                        Expr::pow(Expr::call("cosh", vec![x.clone()]), Expr::int(n+1)),
                                                        Expr::int(n+1)));
                                                }
                                                // ∫ sinh^n*cosh = sinh^(n+1)/(n+1)
                                                if fname == "sinh" && gname == "cosh" {
                                                    if n == -1 {
                                                        // ∫ cosh/sinh = log|sinh|
                                                        return Expr::call("log", vec![Expr::call("abs", vec![Expr::call("sinh", vec![x])])]);
                                                    }
                                                    return simplify(&Expr::div(
                                                        Expr::pow(Expr::call("sinh", vec![x.clone()]), Expr::int(n+1)),
                                                        Expr::int(n+1)));
                                                }
                                                // ∫ sin^n*cos = sin^(n+1)/(n+1)
                                                if fname == "sin" && gname == "cos" {
                                                    if n == -1 {
                                                        // ∫ cos/sin = log|sin|
                                                        return Expr::call("log", vec![Expr::call("abs", vec![Expr::call("sin", vec![x])])]);
                                                    }
                                                    return simplify(&Expr::div(
                                                        Expr::pow(Expr::call("sin", vec![x]), Expr::int(n+1)),
                                                        Expr::int(n+1)));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                // ∫ sin²cos² = x/8 - sin(4x)/32
                if args.len() == 2 {
                    let is_sin2 = |e: &Expr| matches!(e, Expr::List { op: Operator::MExpt, args, .. }
                        if args.len() == 2 && args[1] == Expr::int(2)
                        && matches!(&args[0], Expr::List { op: Operator::Named(id), args: fa, .. }
                            if fa.len() == 1 && fa[0] == *var && resolve(*id) == "sin"));
                    let is_cos2 = |e: &Expr| matches!(e, Expr::List { op: Operator::MExpt, args, .. }
                        if args.len() == 2 && args[1] == Expr::int(2)
                        && matches!(&args[0], Expr::List { op: Operator::Named(id), args: fa, .. }
                            if fa.len() == 1 && fa[0] == *var && resolve(*id) == "cos"));
                    if (is_sin2(&args[0]) && is_cos2(&args[1])) || (is_cos2(&args[0]) && is_sin2(&args[1])) {
                        let x = var.clone();
                        return simplify(&Expr::sub(
                            Expr::div(x.clone(), Expr::int(8)),
                            Expr::div(Expr::call("sin", vec![Expr::mul(Expr::int(4), x)]), Expr::int(32)),
                        ));
                    }
                }
                // #56: ∫ f^(-1)*g where f^(-1)*g = d/dx[-1/f] patterns
                if args.len() == 2 {
                    for (a, b) in [(&args[0], &args[1]), (&args[1], &args[0])] {
                        // Check a = f^(-1), b = g, and f^(-1)*g is a known derivative
                        if let Expr::List { op: Operator::MExpt, args: pa, .. } = a {
                            if pa.len() == 2 && pa[1] == Expr::int(-1) {
                                if let Expr::List { op: Operator::Named(fid), args: fa, .. } = &pa[0] {
                                    if fa.len() == 1 && fa[0] == *var {
                                        if let Expr::List { op: Operator::Named(gid), args: ga, .. } = b {
                                            if ga.len() == 1 && ga[0] == *var {
                                                let fname = resolve(*fid);
                                                let gname = resolve(*gid);
                                                let x = var.clone();
                                                // ∫ (1/cosh)*tanh = -1/cosh = -sech
                                                if fname == "cosh" && gname == "tanh" {
                                                    return Expr::neg(Expr::pow(Expr::call("cosh", vec![x]), Expr::int(-1)));
                                                }
                                                // ∫ (1/sinh)*coth = -1/sinh = -csch
                                                if fname == "sinh" && gname == "coth" {
                                                    return Expr::neg(Expr::pow(Expr::call("sinh", vec![x]), Expr::int(-1)));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                // #167: ∫ sec*csc = log|tan(x)|
                if args.len() == 2 {
                    if let (Some(n1), Some(n2)) = (get_func_name(&args[0], var), get_func_name(&args[1], var)) {
                        if (n1 == "sec" && n2 == "csc") || (n1 == "csc" && n2 == "sec") {
                            return Expr::call("log", vec![Expr::call("abs", vec![Expr::call("tan", vec![var.clone()])])]);
                        }
                    }
                }
                // #34-36: Product-to-sum for sin(ax)*sin(bx), cos(ax)*cos(bx), sin(ax)*cos(bx)
                if args.len() == 2 {
                    if let (
                        Expr::List { op: Operator::Named(id1), args: fa1, .. },
                        Expr::List { op: Operator::Named(id2), args: fa2, .. },
                    ) = (&args[0], &args[1]) {
                        let n1 = resolve(*id1);
                        let n2 = resolve(*id2);
                        if (n1 == "sin" || n1 == "cos") && (n2 == "sin" || n2 == "cos")
                            && fa1.len() == 1 && fa2.len() == 1
                            && fa1[0] != fa2[0]
                            && contains_var(&fa1[0], var) && contains_var(&fa2[0], var) {
                            let a_arg = &fa1[0];
                            let b_arg = &fa2[0];
                            let sum_arg = simplify(&Expr::add(a_arg.clone(), b_arg.clone()));
                            let diff_arg = simplify(&Expr::sub(a_arg.clone(), b_arg.clone()));
                            match (n1.as_str(), n2.as_str()) {
                                ("sin", "sin") => {
                                    // sin(a)*sin(b) = (cos(a-b) - cos(a+b))/2
                                    let int_cos_diff = table_integrate(&Expr::call("cos", vec![diff_arg.clone()]), var);
                                    let int_cos_sum = table_integrate(&Expr::call("cos", vec![sum_arg]), var);
                                    return simplify(&Expr::div(
                                        Expr::sub(int_cos_diff, int_cos_sum), Expr::int(2)));
                                }
                                ("cos", "cos") => {
                                    // cos(a)*cos(b) = (cos(a-b) + cos(a+b))/2
                                    let int_cos_diff = table_integrate(&Expr::call("cos", vec![diff_arg.clone()]), var);
                                    let int_cos_sum = table_integrate(&Expr::call("cos", vec![sum_arg]), var);
                                    return simplify(&Expr::div(
                                        Expr::add(int_cos_diff, int_cos_sum), Expr::int(2)));
                                }
                                ("sin", "cos") | ("cos", "sin") => {
                                    // sin(a)*cos(b) = (sin(a+b) + sin(a-b))/2
                                    let int_sin_sum = table_integrate(&Expr::call("sin", vec![sum_arg]), var);
                                    let int_sin_diff = table_integrate(&Expr::call("sin", vec![diff_arg.clone()]), var);
                                    return simplify(&Expr::div(
                                        Expr::add(int_sin_sum, int_sin_diff), Expr::int(2)));
                                }
                                _ => {}
                            }
                        }
                    }
                }
                // #183: ∫ exp(x)*tanh(x) = exp(x) - 2*atan(exp(x))
                if args.len() == 2 {
                    if let (Some(n1), Some(n2)) = (get_func_name(&args[0], var), get_func_name(&args[1], var)) {
                        let x = var.clone();
                        if (n1 == "exp" && n2 == "tanh") || (n1 == "tanh" && n2 == "exp") {
                            return simplify(&Expr::sub(
                                Expr::call("exp", vec![x.clone()]),
                                Expr::mul(Expr::int(2), Expr::call("atan", vec![Expr::call("exp", vec![x])])),
                            ));
                        }
                    }
                }
                // #179-180: ∫ x*exp(x)*sin(x), ∫ x*exp(x)*cos(x) — by-parts on 3-factor product
                if args.len() == 3 {
                    let mut x_factor = false;
                    let mut exp_factor = false;
                    let mut trig_name = String::new();
                    for a in args {
                        if a == var { x_factor = true; }
                        else if let Some(name) = get_func_name(a, var) {
                            match name.as_str() {
                                "exp" => { exp_factor = true; }
                                "sin" | "cos" => { trig_name = name; }
                                _ => {}
                            }
                        }
                    }
                    if x_factor && exp_factor && !trig_name.is_empty() {
                        let x = var.clone();
                        // ∫ x*exp(x)*sin(x) = (1/2)*exp(x)*(cos(x) - x*cos(x) + x*sin(x))
                        // ∫ x*exp(x)*cos(x) = (1/2)*exp(x)*(x*cos(x) - sin(x) + x*sin(x))
                        // By parts: u=x, dv=exp*trig → v = exp*(sin-cos)/2 or exp*(sin+cos)/2
                        if trig_name == "sin" {
                            return simplify(&Expr::div(
                                Expr::mul(Expr::call("exp", vec![x.clone()]),
                                    Expr::add(
                                        Expr::sub(
                                            Expr::call("cos", vec![x.clone()]),
                                            Expr::mul(x.clone(), Expr::call("cos", vec![x.clone()])),
                                        ),
                                        Expr::mul(x.clone(), Expr::call("sin", vec![x])),
                                    )),
                                Expr::int(2),
                            ));
                        }
                        if trig_name == "cos" {
                            return simplify(&Expr::div(
                                Expr::mul(Expr::call("exp", vec![x.clone()]),
                                    Expr::add(
                                        Expr::sub(
                                            Expr::mul(x.clone(), Expr::call("cos", vec![x.clone()])),
                                            Expr::call("sin", vec![x.clone()]),
                                        ),
                                        Expr::mul(x.clone(), Expr::call("sin", vec![x])),
                                    )),
                                Expr::int(2),
                            ));
                        }
                    }
                }
                // Integration by parts: ∫ x*f(x) dx patterns
                if args.len() == 2 || (args.len() == 3 && matches!(args[0], Expr::Integer(_))) {
                    let (x_power, func) = if args.len() == 2 && args.iter().any(|a| a == var) {
                        let other = args.iter().find(|a| *a != var).cloned();
                        (1, other)
                    } else if args.len() == 2 {
                        // Check for x^n * f(x)
                        if let Expr::List { op: Operator::MExpt, args: pa, .. } = &args[0] {
                            if pa.len() == 2 && pa[0] == *var {
                                if let Some(n) = to_i64(&pa[1]) {
                                    (n, Some(args[1].clone()))
                                } else { (0, None) }
                            } else { (0, None) }
                        } else { (0, None) }
                    } else {
                        (0, None)
                    };
                    if x_power == 1 {
                        if let Some(Expr::List { op: Operator::Named(fid), args: fa, .. }) = &func {
                            if fa.len() == 1 && fa[0] == *var {
                                let fname = resolve(*fid);
                                let x = var.clone();
                                match fname.as_str() {
                                    "exp" => return simplify(&Expr::mul(
                                        Expr::sub(x.clone(), Expr::int(1)),
                                        Expr::call("exp", vec![x]),
                                    )),
                                    "sin" => return simplify(&Expr::sub(
                                        Expr::call("sin", vec![x.clone()]),
                                        Expr::mul(x.clone(), Expr::call("cos", vec![x])),
                                    )),
                                    "cos" => return simplify(&Expr::add(
                                        Expr::call("cos", vec![x.clone()]),
                                        Expr::mul(x.clone(), Expr::call("sin", vec![x])),
                                    )),
                                    "log" => return simplify(&Expr::sub(
                                        Expr::div(Expr::mul(Expr::pow(x.clone(), Expr::int(2)), Expr::call("log", vec![x.clone()])), Expr::int(2)),
                                        Expr::div(Expr::pow(x, Expr::int(2)), Expr::int(4)),
                                    )),
                                    // #62: ∫ u*asin(u) = ((2u²-1)/4)*asin(u) + u*sqrt(1-u²)/4
                                    "asin" => return simplify(&Expr::add(
                                        Expr::div(
                                            Expr::mul(Expr::sub(Expr::mul(Expr::int(2), Expr::pow(x.clone(), Expr::int(2))), Expr::int(1)),
                                                      Expr::call("asin", vec![x.clone()])),
                                            Expr::int(4)),
                                        Expr::div(
                                            Expr::mul(x.clone(), Expr::call("sqrt", vec![Expr::sub(Expr::int(1), Expr::pow(x, Expr::int(2)))])),
                                            Expr::int(4)),
                                    )),
                                    // #63: ∫ u*acos(u) = ((2u²-1)/4)*acos(u) - u*sqrt(1-u²)/4
                                    "acos" => return simplify(&Expr::sub(
                                        Expr::div(
                                            Expr::mul(Expr::sub(Expr::mul(Expr::int(2), Expr::pow(x.clone(), Expr::int(2))), Expr::int(1)),
                                                      Expr::call("acos", vec![x.clone()])),
                                            Expr::int(4)),
                                        Expr::div(
                                            Expr::mul(x.clone(), Expr::call("sqrt", vec![Expr::sub(Expr::int(1), Expr::pow(x, Expr::int(2)))])),
                                            Expr::int(4)),
                                    )),
                                    // #64: ∫ u*atan(u) = ((u²+1)/2)*atan(u) - u/2
                                    "atan" => return simplify(&Expr::sub(
                                        Expr::div(
                                            Expr::mul(Expr::add(Expr::pow(x.clone(), Expr::int(2)), Expr::int(1)),
                                                      Expr::call("atan", vec![x.clone()])),
                                            Expr::int(2)),
                                        Expr::div(x, Expr::int(2)),
                                    )),
                                    _ => {}
                                }
                            }
                        }
                    }
                    // #47: ∫ x^n*log(x) = x^(n+1)/((n+1)²) * ((n+1)*log(x) - 1) for n ≠ -1
                    if x_power >= 1 {
                        if let Some(Expr::List { op: Operator::Named(fid), args: fa, .. }) = &func {
                            if fa.len() == 1 && fa[0] == *var && resolve(*fid) == "log" {
                                let n = x_power;
                                let np1 = n + 1;
                                let x = var.clone();
                                return simplify(&Expr::div(
                                    Expr::mul(
                                        Expr::pow(x.clone(), Expr::int(np1)),
                                        Expr::sub(
                                            Expr::mul(Expr::int(np1), Expr::call("log", vec![x])),
                                            Expr::int(1),
                                        ),
                                    ),
                                    Expr::int(np1 * np1),
                                ));
                            }
                        }
                    }
                    // ∫ x^n * f(a*x) dx — by parts with linear substitution
                    if x_power >= 1 {
                        if let Some(Expr::List { op: Operator::Named(fid), args: fa, .. }) = &func {
                            if fa.len() == 1 {
                                if let Expr::List { op: Operator::MTimes, args: inner, .. } = &fa[0] {
                                    let (consts, vars): (Vec<&Expr>, Vec<&Expr>) =
                                        inner.iter().partition(|a| !contains_var(a, var));
                                    if vars.len() == 1 && *vars[0] == *var && !consts.is_empty() {
                                        let a_val = if consts.len() == 1 { consts[0].clone() }
                                            else { simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: consts.into_iter().cloned().collect() }) };
                                        let fname = resolve(*fid);
                                        let _x = var.clone();
                                        // #42: ∫ x*exp(a*x) = (1/a²)(ax-1)*exp(ax)
                                        if fname == "exp" && x_power == 1 {
                                            let ax = fa[0].clone();
                                            return simplify(&Expr::div(
                                                Expr::mul(
                                                    Expr::sub(ax.clone(), Expr::int(1)),
                                                    Expr::call("exp", vec![ax]),
                                                ),
                                                Expr::pow(a_val, Expr::int(2)),
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // ∫ x^2*sin(x), x^2*cos(x), x^2*exp(x) (by parts twice)
                    if x_power == 2 || x_power == 3 {
                        if let Some(Expr::List { op: Operator::Named(fid), args: fa, .. }) = &func {
                            if fa.len() == 1 && fa[0] == *var {
                                let fname = resolve(*fid);
                                let x = var.clone();
                                if fname == "exp" {
                                    // ∫ x^n*exp(x) = exp(x) * Σ (-1)^k * n!/(n-k)! * x^(n-k)
                                    let n = x_power;
                                    let mut terms = Vec::new();
                                    let mut coeff = 1i64;
                                    for k in 0..=n {
                                        let sign = if k % 2 == 0 { 1i64 } else { -1 };
                                        terms.push(Expr::mul(Expr::int(sign * coeff), Expr::pow(x.clone(), Expr::int(n - k))));
                                        if k < n { coeff *= n - k; }
                                    }
                                    let poly = simplify(&Expr::List { op: Operator::MPlus, simplified: false, args: terms });
                                    return simplify(&Expr::mul(poly, Expr::call("exp", vec![x])));
                                }
                                if fname == "sin" && x_power == 2 {
                                    return simplify(&Expr::add(
                                        Expr::add(
                                            Expr::neg(Expr::mul(Expr::pow(x.clone(), Expr::int(2)), Expr::call("cos", vec![x.clone()]))),
                                            Expr::mul(Expr::int(2), Expr::mul(x.clone(), Expr::call("sin", vec![x.clone()]))),
                                        ),
                                        Expr::mul(Expr::int(2), Expr::call("cos", vec![x])),
                                    ));
                                }
                                if fname == "cos" && x_power == 2 {
                                    return simplify(&Expr::add(
                                        Expr::add(
                                            Expr::mul(Expr::pow(x.clone(), Expr::int(2)), Expr::call("sin", vec![x.clone()])),
                                            Expr::mul(Expr::int(2), Expr::mul(x.clone(), Expr::call("cos", vec![x.clone()]))),
                                        ),
                                        Expr::neg(Expr::mul(Expr::int(2), Expr::call("sin", vec![x]))),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            // ∫ x^n dx = x^(n+1)/(n+1)
            Operator::MExpt if args.len() == 2 && args[0] == *var => {
                if let Some(n) = to_f64(&args[1]) {
                    if (n + 1.0).abs() > 1e-15 {
                        let new_exp = simplify(&Expr::add(args[1].clone(), Expr::int(1)));
                        return simplify(&Expr::div(
                            Expr::pow(var.clone(), new_exp.clone()),
                            new_exp,
                        ));
                    } else {
                        // ∫ x^(-1) dx = log(x)
                        return Expr::call("log", vec![var.clone()]);
                    }
                }
            }
            // ∫ f(x) dx for known functions
            Operator::Named(id) if args.len() == 1 && args[0] == *var => {
                let fname = resolve(*id);
                let x = var.clone();
                match fname.as_str() {
                    "sin" => return Expr::neg(Expr::call("cos", vec![x])),
                    "cos" => return Expr::call("sin", vec![x]),
                    "exp" => return Expr::call("exp", vec![x]),
                    "tan" => return Expr::neg(Expr::call("log", vec![Expr::call("cos", vec![x])])),
                    "log" => return simplify(&Expr::sub(
                        Expr::mul(x.clone(), Expr::call("log", vec![x.clone()])),
                        x,
                    )),
                    "sinh" => return Expr::call("cosh", vec![x]),
                    "cosh" => return Expr::call("sinh", vec![x]),
                    "sqrt" => return simplify(&Expr::mul(
                        Expr::Rational { num: 2, den: 3 },
                        Expr::pow(x, Expr::Rational { num: 3, den: 2 }))),
                    "tanh" => return Expr::call("log", vec![Expr::call("cosh", vec![x])]),
                    "coth" => return Expr::call("log", vec![
                        Expr::call("abs", vec![Expr::call("sinh", vec![x])])
                    ]),
                    "cot" => return Expr::call("log", vec![
                        Expr::call("abs", vec![Expr::call("sin", vec![x])])
                    ]),
                    "asin" => return simplify(&Expr::add(
                        Expr::mul(x.clone(), Expr::call("asin", vec![x.clone()])),
                        Expr::call("sqrt", vec![Expr::sub(Expr::int(1), Expr::pow(x, Expr::int(2)))]),
                    )),
                    "acos" => return simplify(&Expr::sub(
                        Expr::mul(x.clone(), Expr::call("acos", vec![x.clone()])),
                        Expr::call("sqrt", vec![Expr::sub(Expr::int(1), Expr::pow(x, Expr::int(2)))]),
                    )),
                    "atan" => return simplify(&Expr::sub(
                        Expr::mul(x.clone(), Expr::call("atan", vec![x.clone()])),
                        Expr::div(Expr::call("log", vec![Expr::add(Expr::int(1), Expr::pow(x, Expr::int(2)))]), Expr::int(2)),
                    )),
                    "sec" => return Expr::call("log", vec![
                        Expr::add(Expr::call("sec", vec![x.clone()]), Expr::call("tan", vec![x]))
                    ]),
                    "csc" => return Expr::neg(Expr::call("log", vec![
                        Expr::add(Expr::call("csc", vec![x.clone()]), Expr::call("cot", vec![x]))
                    ])),
                    _ => {}
                }
            }
            // ∫ f(x)^n dx for known functions
            // ∫ f(a*x) dx — linear substitution: F(a*x)/a
            Operator::Named(id) if args.len() == 1 => {
                if let Expr::List { op: Operator::MTimes, args: inner, .. } = &args[0] {
                    let (consts, vars): (Vec<&Expr>, Vec<&Expr>) =
                        inner.iter().partition(|a| !contains_var(a, var));
                    if vars.len() == 1 && *vars[0] == *var && !consts.is_empty() {
                        let a = if consts.len() == 1 { consts[0].clone() }
                                else { simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: consts.into_iter().cloned().collect() }) };
                        let fname = resolve(*id);
                        let ax = args[0].clone();
                        let result = match fname.as_str() {
                            "sin" => Some(Expr::neg(Expr::call("cos", vec![ax]))),
                            "cos" => Some(Expr::call("sin", vec![ax])),
                            "exp" => Some(Expr::call("exp", vec![ax])),
                            "tan" => Some(Expr::neg(Expr::call("log", vec![Expr::call("cos", vec![ax])]))),
                            "sec" => Some(Expr::call("log", vec![
                                Expr::add(Expr::call("sec", vec![ax.clone()]), Expr::call("tan", vec![ax]))])),
                            "csc" => Some(Expr::neg(Expr::call("log", vec![
                                Expr::add(Expr::call("csc", vec![ax.clone()]), Expr::call("cot", vec![ax]))]))),
                            "cot" => Some(Expr::call("log", vec![
                                Expr::call("abs", vec![Expr::call("sin", vec![ax])])])),
                            "sinh" => Some(Expr::call("cosh", vec![ax])),
                            "cosh" => Some(Expr::call("sinh", vec![ax])),
                            "tanh" => Some(Expr::call("log", vec![Expr::call("cosh", vec![ax])])),
                            _ => None,
                        };
                        if let Some(r) = result {
                            return simplify(&Expr::div(r, a));
                        }
                    }
                }
            }
            // ∫ f(x)^n dx for function powers
            Operator::MExpt if args.len() == 2 => {
                if let Expr::List { op: Operator::Named(fid), args: fa, .. } = &args[0] {
                    if fa.len() == 1 && fa[0] == *var {
                        let fname = resolve(*fid);
                        let x = var.clone();
                        if let Some(n) = to_i64(&args[1]) {
                            if n == 2 {
                                match fname.as_str() {
                                    "log" => return simplify(&Expr::add(
                                        Expr::sub(
                                            Expr::mul(x.clone(), Expr::pow(Expr::call("log", vec![x.clone()]), Expr::int(2))),
                                            Expr::mul(Expr::int(2), Expr::mul(x.clone(), Expr::call("log", vec![x.clone()]))),
                                        ),
                                        Expr::mul(Expr::int(2), x),
                                    )),
                                    "sin" => return simplify(&Expr::sub(
                                        Expr::div(x.clone(), Expr::int(2)),
                                        Expr::div(Expr::call("sin", vec![Expr::mul(Expr::int(2), x)]), Expr::int(4)),
                                    )),
                                    "cos" => return simplify(&Expr::add(
                                        Expr::div(x.clone(), Expr::int(2)),
                                        Expr::div(Expr::call("sin", vec![Expr::mul(Expr::int(2), x)]), Expr::int(4)),
                                    )),
                                    "sec" => return Expr::call("tan", vec![x]),
                                    "csc" => return Expr::neg(Expr::call("cot", vec![x])),
                                    "tan" => return simplify(&Expr::sub(
                                        Expr::call("tan", vec![x.clone()]), x)),
                                    "cot" => return simplify(&Expr::sub(
                                        Expr::neg(Expr::call("cot", vec![x.clone()])), x)),
                                    "sech" => return Expr::call("tanh", vec![x]),
                                    "csch" => return Expr::neg(Expr::call("coth", vec![x])),
                                    _ => {}
                                }
                            }
                            if n == 3 {
                                match fname.as_str() {
                                    "log" => return simplify(&Expr::add(
                                        Expr::sub(
                                            Expr::sub(
                                                Expr::mul(x.clone(), Expr::pow(Expr::call("log", vec![x.clone()]), Expr::int(3))),
                                                Expr::mul(Expr::int(3), Expr::mul(x.clone(), Expr::pow(Expr::call("log", vec![x.clone()]), Expr::int(2)))),
                                            ),
                                            Expr::neg(Expr::mul(Expr::int(6), Expr::mul(x.clone(), Expr::call("log", vec![x.clone()])))),
                                        ),
                                        Expr::neg(Expr::mul(Expr::int(6), x)),
                                    )),
                                    // #22: ∫ sin³(u) = -(1/3)(2+sin²(u))cos(u)
                                    "sin" => return simplify(&Expr::mul(
                                        Expr::Rational { num: -1, den: 3 },
                                        Expr::mul(
                                            Expr::add(Expr::int(2), Expr::pow(Expr::call("sin", vec![x.clone()]), Expr::int(2))),
                                            Expr::call("cos", vec![x]),
                                        ),
                                    )),
                                    // #23: ∫ cos³(u) = (1/3)(2+cos²(u))sin(u)
                                    "cos" => return simplify(&Expr::mul(
                                        Expr::Rational { num: 1, den: 3 },
                                        Expr::mul(
                                            Expr::add(Expr::int(2), Expr::pow(Expr::call("cos", vec![x.clone()]), Expr::int(2))),
                                            Expr::call("sin", vec![x]),
                                        ),
                                    )),
                                    // #24: ∫ tan³(u) = tan²(u)/2 + log|cos(u)|
                                    "tan" => return simplify(&Expr::add(
                                        Expr::div(Expr::pow(Expr::call("tan", vec![x.clone()]), Expr::int(2)), Expr::int(2)),
                                        Expr::call("log", vec![Expr::call("abs", vec![Expr::call("cos", vec![x])])]),
                                    )),
                                    // #25: ∫ cot³(u) = -cot²(u)/2 - log|sin(u)|
                                    "cot" => return simplify(&Expr::sub(
                                        Expr::div(Expr::neg(Expr::pow(Expr::call("cot", vec![x.clone()]), Expr::int(2))), Expr::int(2)),
                                        Expr::call("log", vec![Expr::call("abs", vec![Expr::call("sin", vec![x])])]),
                                    )),
                                    // #26: ∫ sec³(u) = (1/2)sec(u)tan(u) + (1/2)log|sec(u)+tan(u)|
                                    "sec" => return simplify(&Expr::add(
                                        Expr::div(Expr::mul(Expr::call("sec", vec![x.clone()]), Expr::call("tan", vec![x.clone()])), Expr::int(2)),
                                        Expr::div(Expr::call("log", vec![Expr::call("abs", vec![
                                            Expr::add(Expr::call("sec", vec![x.clone()]), Expr::call("tan", vec![x]))])]), Expr::int(2)),
                                    )),
                                    // #27: ∫ csc³(u) = -(1/2)csc(u)cot(u) + (1/2)log|csc(u)-cot(u)|
                                    "csc" => return simplify(&Expr::add(
                                        Expr::div(Expr::neg(Expr::mul(Expr::call("csc", vec![x.clone()]), Expr::call("cot", vec![x.clone()]))), Expr::int(2)),
                                        Expr::div(Expr::call("log", vec![Expr::call("abs", vec![
                                            Expr::sub(Expr::call("csc", vec![x.clone()]), Expr::call("cot", vec![x]))])]), Expr::int(2)),
                                    )),
                                    _ => {}
                                }
                            }
                            if n == 4 {
                                match fname.as_str() {
                                    "sin" => return simplify(&Expr::add(
                                        Expr::sub(
                                            Expr::div(Expr::mul(Expr::int(3), x.clone()), Expr::int(8)),
                                            Expr::div(Expr::call("sin", vec![Expr::mul(Expr::int(2), x.clone())]), Expr::int(4)),
                                        ),
                                        Expr::div(Expr::call("sin", vec![Expr::mul(Expr::int(4), x)]), Expr::int(32)),
                                    )),
                                    "cos" => return simplify(&Expr::add(
                                        Expr::add(
                                            Expr::div(Expr::mul(Expr::int(3), x.clone()), Expr::int(8)),
                                            Expr::div(Expr::call("sin", vec![Expr::mul(Expr::int(2), x.clone())]), Expr::int(4)),
                                        ),
                                        Expr::div(Expr::call("sin", vec![Expr::mul(Expr::int(4), x)]), Expr::int(32)),
                                    )),
                                    _ => {}
                                }
                            }
                            // General reduction formulas for n >= 4 (sec/csc/tan/cot) or n >= 5 (sin/cos)
                            if n >= 4 {
                                let x = var.clone();
                                let nm1 = n - 1;
                                let nm2 = n - 2;
                                match fname.as_str() {
                                    // #28: ∫ sin^n = -sin^(n-1)*cos/n + (n-1)/n * ∫ sin^(n-2)
                                    "sin" => {
                                        let first = simplify(&Expr::div(
                                            Expr::neg(Expr::mul(
                                                Expr::pow(Expr::call("sin", vec![x.clone()]), Expr::int(nm1)),
                                                Expr::call("cos", vec![x.clone()]))),
                                            Expr::int(n)));
                                        let rest = table_integrate(
                                            &Expr::pow(Expr::call("sin", vec![x]), Expr::int(nm2)), var);
                                        return simplify(&Expr::add(first,
                                            Expr::mul(Expr::Rational { num: nm1, den: n }, rest)));
                                    }
                                    // #29: ∫ cos^n = cos^(n-1)*sin/n + (n-1)/n * ∫ cos^(n-2)
                                    "cos" => {
                                        let first = simplify(&Expr::div(
                                            Expr::mul(
                                                Expr::pow(Expr::call("cos", vec![x.clone()]), Expr::int(nm1)),
                                                Expr::call("sin", vec![x.clone()])),
                                            Expr::int(n)));
                                        let rest = table_integrate(
                                            &Expr::pow(Expr::call("cos", vec![x]), Expr::int(nm2)), var);
                                        return simplify(&Expr::add(first,
                                            Expr::mul(Expr::Rational { num: nm1, den: n }, rest)));
                                    }
                                    // #30: ∫ tan^n = tan^(n-1)/(n-1) - ∫ tan^(n-2)
                                    "tan" => {
                                        let first = simplify(&Expr::div(
                                            Expr::pow(Expr::call("tan", vec![x.clone()]), Expr::int(nm1)),
                                            Expr::int(nm1)));
                                        let rest = table_integrate(
                                            &Expr::pow(Expr::call("tan", vec![x]), Expr::int(nm2)), var);
                                        return simplify(&Expr::sub(first, rest));
                                    }
                                    // #31: ∫ cot^n = -cot^(n-1)/(n-1) - ∫ cot^(n-2)
                                    "cot" => {
                                        let first = simplify(&Expr::div(
                                            Expr::neg(Expr::pow(Expr::call("cot", vec![x.clone()]), Expr::int(nm1))),
                                            Expr::int(nm1)));
                                        let rest = table_integrate(
                                            &Expr::pow(Expr::call("cot", vec![x]), Expr::int(nm2)), var);
                                        return simplify(&Expr::sub(first, rest));
                                    }
                                    // #32: ∫ sec^n = tan*sec^(n-2)/(n-1) + (n-2)/(n-1) * ∫ sec^(n-2)
                                    "sec" => {
                                        let first = simplify(&Expr::div(
                                            Expr::mul(
                                                Expr::call("tan", vec![x.clone()]),
                                                Expr::pow(Expr::call("sec", vec![x.clone()]), Expr::int(nm2))),
                                            Expr::int(nm1)));
                                        let rest = table_integrate(
                                            &Expr::pow(Expr::call("sec", vec![x]), Expr::int(nm2)), var);
                                        return simplify(&Expr::add(first,
                                            Expr::mul(Expr::Rational { num: nm2, den: nm1 }, rest)));
                                    }
                                    // #33: ∫ csc^n = -cot*csc^(n-2)/(n-1) + (n-2)/(n-1) * ∫ csc^(n-2)
                                    "csc" => {
                                        let first = simplify(&Expr::div(
                                            Expr::neg(Expr::mul(
                                                Expr::call("cot", vec![x.clone()]),
                                                Expr::pow(Expr::call("csc", vec![x.clone()]), Expr::int(nm2)))),
                                            Expr::int(nm1)));
                                        let rest = table_integrate(
                                            &Expr::pow(Expr::call("csc", vec![x]), Expr::int(nm2)), var);
                                        return simplify(&Expr::add(first,
                                            Expr::mul(Expr::Rational { num: nm2, den: nm1 }, rest)));
                                    }
                                    _ => {}
                                }
                            }
                            if n == -1 {
                                match fname.as_str() {
                                    "cosh" => return Expr::call("atan", vec![
                                        Expr::call("sinh", vec![x])]),
                                    "sinh" => return Expr::call("log", vec![
                                        Expr::call("abs", vec![
                                            Expr::call("tanh", vec![
                                                Expr::div(x, Expr::int(2))])])]),
                                    _ => {}
                                }
                            }
                            if n == -2 {
                                match fname.as_str() {
                                    "cosh" => return Expr::call("tanh", vec![x]),
                                    "sinh" => return Expr::neg(Expr::call("coth", vec![x])),
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        },
        _ => {}
    }
    // Try recognizing common forms
    // ∫ 1/(1+x^2) dx = atan(x)
    if let Some((num, den)) = extract_fraction(f) {
        if num == Expr::int(1) {
            // Check for sqrt in denominator: 1/sqrt(expr)
            if let Expr::List { op: Operator::Named(fid), args: fa, .. } = &den {
                if resolve(*fid) == "sqrt" && fa.len() == 1 {
                    if let Expr::Symbol(_) = var {
                        let var_id = if let Expr::Symbol(id) = var { *id } else { unreachable!() };
                        let inner = expand(&fa[0]);
                        if let Some(poly) = maxima_poly::expr_to_poly(&inner, var_id) {
                            if poly.degree() == Some(2) {
                                let a_c = poly.terms.iter().find(|(e,_)| *e == 2).map(|(_,c)| c.clone()).unwrap_or(maxima_poly::Coeff::zero());
                                let b_c = poly.terms.iter().find(|(e,_)| *e == 1).map(|(_,c)| c.clone()).unwrap_or(maxima_poly::Coeff::zero());
                                let c_c = poly.terms.iter().find(|(e,_)| *e == 0).map(|(_,c)| c.clone()).unwrap_or(maxima_poly::Coeff::zero());
                                if b_c.is_zero() {
                                    if let (maxima_poly::Coeff::Int(ai), maxima_poly::Coeff::Int(ci)) = (&a_c, &c_c) {
                                        // #15: ∫ 1/sqrt(a²-u²) = asin(u/a)
                                        if *ai < 0 && *ci > 0 {
                                            let a_sq = *ci;
                                            let a_val = (a_sq as f64).sqrt();
                                            let a_int = a_val.round() as i64;
                                            if a_int * a_int == a_sq && -ai == 1 {
                                                if a_sq == 1 {
                                                    return Expr::call("asin", vec![var.clone()]);
                                                }
                                                return Expr::call("asin", vec![
                                                    simplify(&Expr::div(var.clone(), Expr::int(a_int)))
                                                ]);
                                            }
                                        }
                                        // #72: ∫ 1/sqrt(1+u²) = asinh / log (monic only;
                                        // non-monic leading coeff handled by the verified
                                        // quadratic-radical path, which accounts for √a).
                                        if *ai == 1 && *ci > 0 {
                                            if *ai == 1 && *ci == 1 {
                                                return Expr::call("asinh", vec![var.clone()]);
                                            }
                                            return Expr::call("log", vec![
                                                Expr::add(var.clone(),
                                                    Expr::call("sqrt", vec![
                                                        Expr::add(Expr::int(*ci), Expr::pow(var.clone(), Expr::int(2)))]))
                                            ]);
                                        }
                                        // #81: ∫ 1/sqrt(u²-a²) = log|u + sqrt(u²-a²)| (monic only)
                                        if *ai == 1 && *ci < 0 {
                                            if *ai == 1 && *ci == -1 {
                                                return Expr::call("acosh", vec![var.clone()]);
                                            }
                                            return Expr::call("log", vec![
                                                Expr::call("abs", vec![
                                                    Expr::add(var.clone(),
                                                        Expr::call("sqrt", vec![
                                                            Expr::add(Expr::int(*ci), Expr::pow(var.clone(), Expr::int(2)))]))])
                                            ]);
                                        }
                                    }
                                }
                                // Completing the square for b ≠ 0:
                                // ax²+bx+c = a(x+b/(2a))² + (c - b²/(4a))
                                // ∫ 1/sqrt(a(u+h)²+k) where u=x, h=b/(2a), k=c-b²/(4a)
                                if let (maxima_poly::Coeff::Int(ai), maxima_poly::Coeff::Int(bi), maxima_poly::Coeff::Int(ci)) = (&a_c, &b_c, &c_c) {
                                    if *ai == 1 && *bi != 0 {
                                        // x²+bx+c = (x+b/2)² + (c - b²/4)
                                        let h_num = *bi;
                                        let h_den = 2i64;
                                        // k = c - b²/4
                                        let k_num = 4 * ci - bi * bi;
                                        let k_den = 4i64;
                                        // u = x + b/2
                                        let u_expr = simplify(&Expr::add(var.clone(), Expr::Rational { num: h_num, den: h_den }));
                                        if k_num > 0 {
                                            // ∫ 1/sqrt(u²+k) = asinh(u/√k) or log(u+sqrt(u²+k))
                                            if k_num == k_den {
                                                return Expr::call("asinh", vec![u_expr]);
                                            }
                                            return Expr::call("log", vec![
                                                Expr::add(u_expr.clone(),
                                                    Expr::call("sqrt", vec![
                                                        Expr::add(Expr::Rational { num: k_num, den: k_den },
                                                            Expr::pow(u_expr, Expr::int(2)))]))
                                            ]);
                                        } else if k_num < 0 {
                                            // ∫ 1/sqrt(u²-|k|) = acosh or log
                                            return Expr::call("log", vec![
                                                Expr::call("abs", vec![
                                                    Expr::add(u_expr.clone(),
                                                        Expr::call("sqrt", vec![
                                                            Expr::add(Expr::Rational { num: k_num, den: k_den },
                                                                Expr::pow(u_expr, Expr::int(2)))]))])
                                            ]);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let expanded_den = expand(&den);
            if let Expr::Symbol(_) = var {
                let var_id = if let Expr::Symbol(id) = var { *id } else { unreachable!() };
                if let Some(poly) = maxima_poly::expr_to_poly(&expanded_den, var_id) {
                    if poly.degree() == Some(2) {
                        let a = poly.terms.iter().find(|(e,_)| *e == 2).map(|(_,c)| c.clone()).unwrap_or(maxima_poly::Coeff::zero());
                        let b = poly.terms.iter().find(|(e,_)| *e == 1).map(|(_,c)| c.clone()).unwrap_or(maxima_poly::Coeff::zero());
                        let c = poly.terms.iter().find(|(e,_)| *e == 0).map(|(_,c)| c.clone()).unwrap_or(maxima_poly::Coeff::zero());
                        // #16: ∫ 1/(a²+u²) = (1/a)*atan(u/a)
                        if b.is_zero() {
                            if let (maxima_poly::Coeff::Int(ai), maxima_poly::Coeff::Int(ci)) = (&a, &c) {
                                if *ai > 0 && *ci > 0 {
                                    if *ai == 1 && *ci == 1 {
                                        return Expr::call("atan", vec![var.clone()]);
                                    }
                                    let a_sq = *ci;
                                    let a_val = (a_sq as f64).sqrt();
                                    let a_int = a_val.round() as i64;
                                    if a_int * a_int == a_sq && *ai == 1 {
                                        return simplify(&Expr::div(
                                            Expr::call("atan", vec![simplify(&Expr::div(var.clone(), Expr::int(a_int)))]),
                                            Expr::int(a_int),
                                        ));
                                    }
                                }
                                if *ai == -1 && *ci == 1 {
                                    return Expr::call("atanh", vec![var.clone()]);
                                }
                            }
                        }
                        // ∫ 1/(ax²+bx+c) via completing the square
                        // = (2/sqrt(4ac-b²)) * atan((2ax+b)/sqrt(4ac-b²)) when 4ac > b²
                        if let (maxima_poly::Coeff::Int(ai), maxima_poly::Coeff::Int(bi), maxima_poly::Coeff::Int(ci)) = (&a, &b, &c) {
                            let disc = 4 * ai * ci - bi * bi;
                            if disc > 0 {
                                let sqrt_disc = (disc as f64).sqrt();
                                let sqrt_d_int = sqrt_disc.round() as i64;
                                if sqrt_d_int * sqrt_d_int == disc {
                                    // Exact: 2/sqrt_d * atan((2a*x+b)/sqrt_d)
                                    let inner = simplify(&Expr::div(
                                        Expr::add(Expr::mul(Expr::int(2 * ai), var.clone()), Expr::int(*bi)),
                                        Expr::int(sqrt_d_int),
                                    ));
                                    return simplify(&Expr::mul(
                                        Expr::Rational { num: 2, den: sqrt_d_int },
                                        Expr::call("atan", vec![inner]),
                                    ));
                                } else {
                                    // Irrational discriminant: use sqrt
                                    let sd = Expr::call("sqrt", vec![Expr::int(disc)]);
                                    let inner = simplify(&Expr::div(
                                        Expr::add(Expr::mul(Expr::int(2 * ai), var.clone()), Expr::int(*bi)),
                                        sd.clone(),
                                    ));
                                    return simplify(&Expr::div(
                                        Expr::mul(Expr::int(2), Expr::call("atan", vec![inner])),
                                        sd,
                                    ));
                                }
                            }
                        }
                    }
                    // ∫ 1/(linear) = log(linear)/coeff
                    if poly.degree() == Some(1) {
                        let a = poly.leading_coeff();
                        return simplify(&Expr::div(
                            Expr::call("log", vec![den.clone()]),
                            match a {
                                maxima_poly::Coeff::Int(n) => Expr::int(n),
                                maxima_poly::Coeff::Rat(n, d) => Expr::Rational { num: n, den: d },
                            },
                        ));
                    }
                }
            }
        }
    }

    // Special: x/(x^2+a) → log(x^2+a)/2  (when numerator is derivative of denominator / 2)
    if let Some((num, den)) = extract_fraction(f) {
        if let Expr::Symbol(var_id) = var {
            let num_expanded = expand(&num);
            let den_expanded = expand(&den);
            if let (Some(np), Some(dp)) = (
                maxima_poly::expr_to_poly(&num_expanded, *var_id),
                maxima_poly::expr_to_poly(&den_expanded, *var_id),
            ) {
                // Check if num is proportional to derivative of den
                let dp_deriv = dp.derivative();
                if !dp_deriv.is_zero() && np.degree().unwrap_or(0) + 1 == dp.degree().unwrap_or(0) {
                    // Check if np = c * dp'
                    if let Some(q) = dp_deriv.exact_div(&np) {
                        if q.is_constant() {
                            let c = q.constant_term();
                            let den_expr = maxima_poly::poly_to_expr(&dp);
                            return simplify(&Expr::div(
                                Expr::call("log", vec![den_expr]),
                                match c {
                                    maxima_poly::Coeff::Int(n) => Expr::int(n),
                                    maxima_poly::Coeff::Rat(n, d) => Expr::Rational { num: n, den: d },
                                },
                            ));
                        }
                    }
                    // Also check np * c = dp'
                    if let Some(q) = np.exact_div(&dp_deriv) {
                        if q.is_constant() {
                            let c = q.constant_term();
                            let den_expr = maxima_poly::poly_to_expr(&dp);
                            let c_expr = match c {
                                maxima_poly::Coeff::Int(n) => Expr::int(n),
                                maxima_poly::Coeff::Rat(n, d) => Expr::Rational { num: n, den: d },
                            };
                            return simplify(&Expr::mul(c_expr, Expr::call("log", vec![den_expr])));
                        }
                    }
                }
            }
        }
    }

    // Try rational function integration via partial fractions
    if let Some((num, den)) = extract_fraction(f) {
        if let Expr::Symbol(var_id) = var {
            let num_expanded = expand(&num);
            let den_expanded = expand(&den);
            if let (Some(mut np), Some(mut dp)) = (
                maxima_poly::expr_to_poly(&num_expanded, *var_id),
                maxima_poly::expr_to_poly(&den_expanded, *var_id),
            ) {
                // First: cancel common factors
                let g = maxima_poly::poly_gcd(&np, &dp);
                if !g.is_constant() {
                    if let (Some(n2), Some(d2)) = (np.exact_div(&g), dp.exact_div(&g)) {
                        np = n2;
                        dp = d2;
                    }
                }
                // If denominator reduced to constant, integrate numerator directly
                if dp.is_constant() {
                    let num_expr = maxima_poly::poly_to_expr(&np);
                    let c = dp.constant_term();
                    let c_expr = match c {
                        maxima_poly::Coeff::Int(n) => Expr::int(n),
                        maxima_poly::Coeff::Rat(n, d) => Expr::Rational { num: n, den: d },
                    };
                    return simplify(&Expr::div(table_integrate(&num_expr, var), c_expr));
                }
                // Polynomial division: if deg(num) >= deg(den), extract polynomial part
                if np.degree().unwrap_or(0) >= dp.degree().unwrap_or(0) {
                    if let Some((quot, rem)) = np.divmod(&dp) {
                        let poly_int = table_integrate(&maxima_poly::poly_to_expr(&quot), var);
                        if rem.is_zero() {
                            return poly_int;
                        }
                        let rem_expr = simplify(&Expr::div(
                            maxima_poly::poly_to_expr(&rem),
                            maxima_poly::poly_to_expr(&dp),
                        ));
                        let rem_int = table_integrate(&rem_expr, var);
                        if !rem_int.to_string().starts_with("integrate") {
                            return simplify(&Expr::add(poly_int, rem_int));
                        }
                        np = rem;
                    }
                }
                // Factor denominator
                if dp.degree().unwrap_or(0) >= 1 {
                    let factors = maxima_poly::factor_poly(&dp);

                    // Handle repeated factors via Hermite reduction
                    let has_repeated = factors.iter().any(|(_, m)| *m >= 2);
                    if has_repeated {
                        // Single irreducible quadratic with multiplicity n≥2: ∫ k/(x²+c)^n
                        // Reduction: ∫ 1/(x²+c)^n = x/(2c(n-1)(x²+c)^(n-1)) + (2n-3)/(2c(n-1)) * ∫ 1/(x²+c)^(n-1)
                        if factors.len() == 1 && factors[0].0.degree() == Some(2) && factors[0].1 >= 2 && np.is_constant() {
                            let qi = &factors[0].0;
                            let b_c = qi.terms.iter().find(|(e,_)| *e==1).map(|(_,c)| c.clone()).unwrap_or(maxima_poly::Coeff::zero());
                            let c_c = qi.terms.iter().find(|(e,_)| *e==0).map(|(_,c)| c.clone()).unwrap_or(maxima_poly::Coeff::zero());
                            let a_lc = get_poly_coeff(qi, 2);
                            if b_c.is_zero() && a_lc == 1 {
                                if let maxima_poly::Coeff::Int(c_val) = &c_c {
                                    if let maxima_poly::Coeff::Int(k) = np.constant_term() {
                                        let qi_expr = maxima_poly::poly_to_expr(qi);
                                        // Iterative reduction from n down to 1
                                        let mut result = table_integrate(
                                            &simplify(&Expr::div(Expr::int(k), qi_expr.clone())), var);
                                        for j in 2..=factors[0].1 {
                                            let jj = j as i64;
                                            // ∫ 1/(x²+c)^j = x/(2c(j-1)(x²+c)^(j-1)) + (2j-3)/(2c(j-1)) * ∫ 1/(x²+c)^(j-1)
                                            let rat_part = simplify(&Expr::div(
                                                var.clone(),
                                                Expr::mul(
                                                    Expr::int(2 * c_val * (jj - 1)),
                                                    Expr::pow(qi_expr.clone(), Expr::int(jj - 1)),
                                                ),
                                            ));
                                            let coeff = Expr::Rational { num: 2*jj - 3, den: 2 * c_val * (jj - 1) };
                                            result = simplify(&Expr::add(rat_part, Expr::mul(coeff, result)));
                                        }
                                        return simplify(&Expr::mul(Expr::int(k), result));
                                    }
                                }
                            }
                        }
                        if let Some(result) = integrate_hermite(&np, &factors, *var_id, var) {
                            return result;
                        }
                    }
                    // Single linear factor with multiplicity
                    if factors.len() == 1 && factors[0].0.degree() == Some(1) {
                        let (f, m) = &factors[0];
                        let a_coeff = f.leading_coeff();
                        let linear_expr = maxima_poly::poly_to_expr(f);
                        let num_val = match np.constant_term() {
                            maxima_poly::Coeff::Int(n) => Expr::int(n),
                            maxima_poly::Coeff::Rat(n, d) => Expr::Rational { num: n, den: d },
                        };
                        let a_expr = match &a_coeff {
                            maxima_poly::Coeff::Int(n) => Expr::int(*n),
                            maxima_poly::Coeff::Rat(n, d) => Expr::Rational { num: *n, den: *d },
                        };
                        if *m == 1 {
                            // ∫ c/(ax+b) = c/a * log(ax+b)
                            return simplify(&Expr::div(
                                Expr::mul(num_val, Expr::call("log", vec![linear_expr])),
                                a_expr,
                            ));
                        } else {
                            // ∫ c/(ax+b)^n = c / (a*(1-n)) * (ax+b)^(1-n)
                            let new_exp = 1 - (*m as i64);
                            return simplify(&Expr::div(
                                Expr::mul(num_val, Expr::pow(linear_expr, Expr::int(new_exp))),
                                Expr::mul(a_expr, Expr::int(-new_exp)),
                            ));
                        }
                    }
                    // Multiple distinct linear factors → partial fractions then integrate each
                    if factors.len() > 1 && factors.iter().all(|(f, m)| f.degree() == Some(1) && *m == 1) {
                        let mut terms = Vec::new();
                        for (fi, _) in &factors {
                            let a = fi.leading_coeff();
                            let b = fi.constant_term();
                            if let Some(root) = b.neg().div(&a) {
                                let num_at_root = np.eval_at(&root);
                                let mut denom_at_root = maxima_poly::Coeff::one();
                                for (fj, _) in &factors {
                                    if fj != fi {
                                        denom_at_root = denom_at_root.mul(&fj.eval_at(&root));
                                    }
                                }
                                if let Some(residue) = num_at_root.div(&denom_at_root) {
                                    let coeff_expr = match residue {
                                        maxima_poly::Coeff::Int(n) => Expr::int(n),
                                        maxima_poly::Coeff::Rat(n, d) => Expr::Rational { num: n, den: d },
                                    };
                                    let a_expr = match &a {
                                        maxima_poly::Coeff::Int(n) => Expr::int(*n),
                                        maxima_poly::Coeff::Rat(n, d) => Expr::Rational { num: *n, den: *d },
                                    };
                                    let linear_expr = maxima_poly::poly_to_expr(fi);
                                    terms.push(simplify(&Expr::div(
                                        Expr::mul(coeff_expr, Expr::call("log", vec![linear_expr])),
                                        a_expr,
                                    )));
                                }
                            }
                        }
                        if !terms.is_empty() {
                            return simplify(&Expr::List {
                                op: Operator::MPlus,
                                simplified: false,
                                args: terms,
                            });
                        }
                    }
                    // Handle factors with linear + quadratic terms via partial fractions
                    if factors.iter().all(|(f, m)| *m == 1 && (f.degree() == Some(1) || f.degree() == Some(2))) {
                        if let Some(result) = integrate_partfrac_mixed(&np, &factors, var) {
                            return result;
                        }
                    }
                    // Lazard–Rioboo–Trager: purely-logarithmic part with rational
                    // residues over a square-free denominator (handles irreducible
                    // factors of degree ≥ 3 that the partfrac path above misses).
                    if factors.iter().all(|(_, m)| *m == 1) {
                        if let Some(result) = try_lrt_log_integrate(&np, &dp, var) {
                            return result;
                        }
                    }
                    // Try algebraic factoring for irreducible polynomials of degree >= 4
                    for (fi, mi) in &factors {
                        if fi.degree().unwrap_or(0) >= 4 && *mi == 1 {
                            if let Some(result) = try_algebraic_factor_integrate(&np, fi, *var_id, var) {
                                return result;
                            }
                        }
                    }
                }
            }
        }
    }

    // ∫ 1/(x·√(x²-a)) = (1/√a)·atan(√(x²-a)/√a) or acos(√a/x)/√a
    if let Some((num, den)) = extract_fraction(f) {
        if num == Expr::int(1) {
            if let Expr::Symbol(var_id) = var {
                // Check for x*sqrt(x²+c) form in denominator
                let den_expanded = expand(&den);
                let mut den_factors = Vec::new();
                collect_mult_factors(&den_expanded, &mut den_factors);
                if den_factors.len() == 2 {
                    let (maybe_x, maybe_sqrt) = if den_factors[0] == *var {
                        (&den_factors[0], &den_factors[1])
                    } else if den_factors[1] == *var {
                        (&den_factors[1], &den_factors[0])
                    } else { (&den_factors[0], &den_factors[1]) };

                    if *maybe_x == *var {
                        if let Expr::List { op: Operator::Named(sid), args: sa, .. } = maybe_sqrt {
                            if resolve(*sid) == "sqrt" && sa.len() == 1 {
                                let inner = expand(&sa[0]);
                                if let Some(poly) = maxima_poly::expr_to_poly(&inner, *var_id) {
                                    let ai = get_poly_coeff(&poly, 2);
                                    let bi = get_poly_coeff(&poly, 1);
                                    let ci = get_poly_coeff(&poly, 0);
                                    if ai == 1 && bi == 0 && ci < 0 {
                                        // 1/(x·√(x²-|c|)) = atan(√(x²-|c|)/√|c|) / √|c|
                                        let abs_c = -ci;
                                        let sqrt_c = Expr::call("sqrt", vec![Expr::int(abs_c)]);
                                        return simplify(&Expr::div(
                                            Expr::call("atan", vec![
                                                Expr::div(maybe_sqrt.clone(), sqrt_c.clone())]),
                                            sqrt_c));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // #84: ∫ sqrt(a²-x²) = x*sqrt(a²-x²)/2 + a²/2*asin(x/a)
    // #67: ∫ sqrt(a²+x²) = x*sqrt(a²+x²)/2 + a²/2*log(x+sqrt(a²+x²))
    if let Expr::List { op: Operator::Named(fid), args: fa, .. } = f {
        if resolve(*fid) == "sqrt" && fa.len() == 1 {
            if let Expr::Symbol(var_id) = var {
                let inner = expand(&fa[0]);
                if let Some(poly) = maxima_poly::expr_to_poly(&inner, *var_id) {
                    if poly.degree() == Some(2) {
                        let ai = get_poly_coeff(&poly, 2);
                        let bi = get_poly_coeff(&poly, 1);
                        let ci = get_poly_coeff(&poly, 0);
                        if bi == 0 && ai != 0 {
                            let sqrt_expr = f.clone();
                            let x = var.clone();
                            if ai == -1 && ci > 0 {
                                // sqrt(c - x²): ∫ = x*sqrt(c-x²)/2 + c/2*asin(x/√c)
                                let sqrt_c = if ci == 1 { Expr::int(1) }
                                    else { Expr::call("sqrt", vec![Expr::int(ci)]) };
                                return simplify(&Expr::add(
                                    Expr::div(Expr::mul(x.clone(), sqrt_expr), Expr::int(2)),
                                    Expr::div(Expr::mul(Expr::int(ci),
                                        Expr::call("asin", vec![Expr::div(x, sqrt_c)])), Expr::int(2)),
                                ));
                            }
                            if ai == 1 && ci > 0 {
                                // sqrt(x²+c): ∫ = x*sqrt(x²+c)/2 + c/2*log(x+sqrt(x²+c))
                                return simplify(&Expr::add(
                                    Expr::div(Expr::mul(x.clone(), sqrt_expr.clone()), Expr::int(2)),
                                    Expr::div(Expr::mul(Expr::int(ci),
                                        Expr::call("log", vec![Expr::add(x, sqrt_expr)])), Expr::int(2)),
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    // #70: ∫ √(x²+c)/x² = -√(x²+c)/x + log(x+√(x²+c))
    // #89: ∫ 1/(x·√(a-x²)) = (-1/√a)·log|(√a+√(a-x²))/x|
    if let Some((num_expr, den_expr)) = extract_fraction(f) {
        if num_expr == Expr::int(1) {
            if let Expr::Symbol(var_id) = var {
                let mut den_factors = Vec::new();
                collect_mult_factors(&den_expr, &mut den_factors);
                // 1/(x·√(a-x²)) pattern
                if den_factors.len() == 2 {
                    let mut x_factor = false;
                    let mut sqrt_inner = None;
                    for df in &den_factors {
                        if df == var { x_factor = true; }
                        if let Expr::List { op: Operator::Named(sid), args: sa, .. } = df {
                            if resolve(*sid) == "sqrt" && sa.len() == 1 {
                                sqrt_inner = Some(&sa[0]);
                            }
                        }
                    }
                    if x_factor {
                        if let Some(inner) = sqrt_inner {
                            if let Some(p) = maxima_poly::expr_to_poly(&expand(inner), *var_id) {
                                let ai = get_poly_coeff(&p, 2);
                                let bi = get_poly_coeff(&p, 1);
                                let ci = get_poly_coeff(&p, 0);
                                if ai == -1 && bi == 0 && ci > 0 {
                                    // 1/(x·√(c-x²)) = (-1/√c)·log|(√c+√(c-x²))/x|
                                    let sqrt_c = Expr::call("sqrt", vec![Expr::int(ci)]);
                                    let sqrt_expr = Expr::call("sqrt", vec![inner.clone()]);
                                    return simplify(&Expr::div(
                                        Expr::neg(Expr::call("log", vec![Expr::call("abs", vec![
                                            Expr::div(Expr::add(sqrt_c.clone(), sqrt_expr), var.clone())])])),
                                        sqrt_c));
                                }
                            }
                        }
                    }
                }
                // √(x²+c)/x² = 1·(x²)^(-1)·sqrt(...) — check for x^(-2) * sqrt
                let x_sq_inv = den_factors.iter().any(|e| {
                    matches!(e, Expr::List { op: Operator::MExpt, args: pa, .. }
                        if pa.len() == 2 && pa[0] == *var && pa[1] == Expr::int(2))
                });
                if x_sq_inv {
                    if let Expr::List { op: Operator::Named(sid), args: sa, .. } = &num_expr {
                        if resolve(*sid) == "sqrt" && sa.len() == 1 {
                            if let Some(p) = maxima_poly::expr_to_poly(&expand(&sa[0]), *var_id) {
                                if p.degree() == Some(2) && get_poly_coeff(&p, 2) == 1 && get_poly_coeff(&p, 1) == 0 {
                                    let _ci = get_poly_coeff(&p, 0);
                                    let x = var.clone();
                                    let sqrt_e = num_expr.clone();
                                    // -√(x²+c)/x + log(x+√(x²+c))
                                    return simplify(&Expr::add(
                                        Expr::neg(Expr::div(sqrt_e.clone(), x.clone())),
                                        Expr::call("log", vec![Expr::add(x, sqrt_e)])));
                                }
                            }
                        }
                    }
                }
            }
        }
        // Euler substitution for ∫ 1/((x+a)·√(x²+c)) — removed: formulas were incorrect.
        // Falls through to noun form. Correct implementation requires full Euler substitution
        // engine (future work).
    }

    // #70: ∫ √(x²+c)/x² dx = -√(x²+c)/x + log(x+√(x²+c))
    // Detect as MTimes product: x^(-2) * sqrt(x²+c)
    if let Expr::List { op: Operator::MTimes, args, .. } = f {
        if args.len() == 2 {
            if let Expr::Symbol(var_id) = var {
                for (a_idx, b_idx) in [(0,1), (1,0)] {
                    let is_x_neg2 = matches!(&args[a_idx], Expr::List { op: Operator::MExpt, args: pa, .. }
                        if pa.len() == 2 && pa[0] == *var && pa[1] == Expr::int(-2));
                    if is_x_neg2 {
                        if let Expr::List { op: Operator::Named(sid), args: sa, .. } = &args[b_idx] {
                            if resolve(*sid) == "sqrt" && sa.len() == 1 {
                                if let Some(p) = maxima_poly::expr_to_poly(&expand(&sa[0]), *var_id) {
                                    if p.degree() == Some(2) && get_poly_coeff(&p, 2) == 1
                                        && get_poly_coeff(&p, 1) == 0 {
                                        let x = var.clone();
                                        let sqrt_e = args[b_idx].clone();
                                        return simplify(&Expr::add(
                                            Expr::neg(Expr::div(sqrt_e.clone(), x.clone())),
                                            Expr::call("log", vec![Expr::add(x, sqrt_e)])));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // #68: ∫ x²√(x²+c) = (x/8)(2x²+c)√(x²+c) - (c²/8)log(x+√(x²+c))
    if let Expr::List { op: Operator::MTimes, args, .. } = f {
        if args.len() == 2 {
            if let Expr::Symbol(var_id) = var {
                for (a_idx, b_idx) in [(0,1), (1,0)] {
                    // Check: x² * sqrt(x²+c)
                    let is_x_sq = |e: &Expr| matches!(e, Expr::List { op: Operator::MExpt, args: pa, .. }
                        if pa.len() == 2 && pa[0] == *var && pa[1] == Expr::int(2));
                    if is_x_sq(&args[a_idx]) {
                        if let Expr::List { op: Operator::Named(sid), args: sa, .. } = &args[b_idx] {
                            if resolve(*sid) == "sqrt" && sa.len() == 1 {
                                if let Some(p) = maxima_poly::expr_to_poly(&expand(&sa[0]), *var_id) {
                                    if p.degree() == Some(2) && get_poly_coeff(&p, 2) == 1
                                        && get_poly_coeff(&p, 1) == 0 {
                                        let ci = get_poly_coeff(&p, 0);
                                        let x = var.clone();
                                        let sqrt_e = args[b_idx].clone();
                                        return simplify(&Expr::sub(
                                            Expr::div(Expr::mul(x.clone(),
                                                Expr::mul(Expr::add(Expr::mul(Expr::int(2), Expr::pow(x.clone(), Expr::int(2))), Expr::int(ci)),
                                                    sqrt_e.clone())), Expr::int(8)),
                                            Expr::div(Expr::mul(Expr::int(ci * ci),
                                                Expr::call("log", vec![Expr::add(x, sqrt_e)])), Expr::int(8))));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Detect x^n/sqrt(x²+c) and sqrt(x²+c)/x patterns
    if let Some((num_expr, den_expr)) = extract_fraction(f) {
        if let Expr::Symbol(var_id) = var {
            // x²/sqrt(x²+c): #72 formula
            if let Some(np) = maxima_poly::expr_to_poly(&expand(&num_expr), *var_id) {
                if np.degree() == Some(2) && np.leading_coeff() == maxima_poly::Coeff::Int(1)
                    && np.terms.len() == 1 {
                    if let Expr::List { op: Operator::Named(sid), args: sa, .. } = &den_expr {
                        if resolve(*sid) == "sqrt" && sa.len() == 1 {
                            if let Some(dp) = maxima_poly::expr_to_poly(&expand(&sa[0]), *var_id) {
                                if dp.degree() == Some(2) {
                                    let ai = get_poly_coeff(&dp, 2);
                                    let bi = get_poly_coeff(&dp, 1);
                                    let ci = get_poly_coeff(&dp, 0);
                                    if ai == 1 && bi == 0 {
                                        let x = var.clone();
                                        let sqrt_expr = den_expr.clone();
                                        // x²/√(x²+c) = (x/2)√(x²+c) - (c/2)log(x+√(x²+c))
                                        return simplify(&Expr::sub(
                                            Expr::div(Expr::mul(x.clone(), sqrt_expr.clone()), Expr::int(2)),
                                            Expr::div(Expr::mul(Expr::int(ci),
                                                Expr::call("log", vec![Expr::add(x, sqrt_expr)])),
                                                Expr::int(2)),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // sqrt(x)*log(x): substitution u=sqrt(x), x=u², dx=2u du
    // → ∫ u*log(u²)*2u du = 4∫ u²*log(u) du = 4*u³(3log(u)-1)/9
    // = (4/9)*x^(3/2)*(3*log(√x)-1) = (2/9)*x^(3/2)*(3*log(x)-2)
    if let Expr::List { op: Operator::MTimes, args, .. } = f {
        if args.len() == 2 {
            let (maybe_sqrt, maybe_log) = (&args[0], &args[1]);
            let check = |sq: &Expr, lg: &Expr| -> Option<Expr> {
                let is_sqrt_x = matches!(sq, Expr::List { op: Operator::Named(id), args: fa, .. }
                    if resolve(*id) == "sqrt" && fa.len() == 1 && fa[0] == *var)
                    || matches!(sq, Expr::List { op: Operator::MExpt, args: pa, .. }
                    if pa.len() == 2 && pa[0] == *var
                    && (pa[1] == Expr::Rational { num: 1, den: 2 }));
                let is_log_x = matches!(lg, Expr::List { op: Operator::Named(id), args: fa, .. }
                    if resolve(*id) == "log" && fa.len() == 1 && fa[0] == *var);
                if is_sqrt_x && is_log_x {
                    let x = var.clone();
                    // (2/9)*x^(3/2)*(3*log(x)-2)
                    Some(simplify(&Expr::mul(
                        Expr::Rational { num: 2, den: 9 },
                        Expr::mul(
                            Expr::pow(x.clone(), Expr::Rational { num: 3, den: 2 }),
                            Expr::sub(Expr::mul(Expr::int(3), Expr::call("log", vec![x])), Expr::int(2)),
                        ),
                    )))
                } else { None }
            };
            if let Some(r) = check(maybe_sqrt, maybe_log) { return r; }
            if let Some(r) = check(maybe_log, maybe_sqrt) { return r; }
        }
    }

    // Try substitution: look for u(x) where f = g(u)*u'
    if let Some(result) = try_substitution_integrate(f, var) {
        return result;
    }

    // Try Risch tower-based integration (for log/exp compositions)
    if let Some(result) = crate::risch_integrate::risch_integrate(f, var) {
        return result;
    }

    // Named nonelementary antiderivatives (erf/erfi, li, Ei, Si, Ci).
    if let Some(result) = try_named_nonelementary(f, var) {
        return result;
    }

    // Quadratic-radical integrands ∫ R(x, √(ax²+bx+c)) dx (S3).
    if let Some(result) = try_quadratic_radical_integrate(f, var) {
        return result;
    }

    // Can't integrate — return noun form
    Expr::call("integrate", vec![f.clone(), var.clone()])
}

/// Map canonical provably-nonelementary integrands to the named special
/// functions introduced in S7. Every returned form is differentiated back and
/// checked numerically before being accepted (noun form beats a wrong answer).
fn try_named_nonelementary(f: &Expr, var: &Expr) -> Option<Expr> {
    let var_id = match var {
        Expr::Symbol(id) => *id,
        _ => return None,
    };

    // Separate constant factors from var-dependent factors.
    let mut factors = Vec::new();
    collect_mult_factors(f, &mut factors);
    let mut konst = Expr::int(1);
    let mut rest: Vec<Expr> = Vec::new();
    for fac in &factors {
        if contains_var(fac, var) {
            rest.push(fac.clone());
        } else {
            konst = simplify(&Expr::mul(konst, fac.clone()));
        }
    }

    let candidate = match rest.len() {
        1 => named_single(&rest[0], var, var_id),
        2 => named_ratio(&rest[0], &rest[1], var, var_id),
        _ => None,
    }?;

    let scaled = simplify(&Expr::mul(konst, candidate));
    if verify_antiderivative(&scaled, f, var) {
        Some(scaled)
    } else {
        None
    }
}

/// Single var-dependent factor: exp(quadratic) → erf/erfi, 1/log(x) → li.
fn named_single(g: &Expr, var: &Expr, var_id: maxima_core::SymbolId) -> Option<Expr> {
    // 1/log(x) = log(x)^(-1)  → expintegral_li(x)
    if let Expr::List { op: Operator::MExpt, args, .. } = g {
        if args.len() == 2 && args[1] == Expr::int(-1) {
            if let Expr::List { op: Operator::Named(id), args: la, .. } = &args[0] {
                if resolve(*id) == "log" && la.len() == 1 && la[0] == *var {
                    return Some(Expr::call("expintegral_li", vec![var.clone()]));
                }
            }
        }
    }
    // exp(quadratic(x)) → erf/erfi via completing the square.
    if let Expr::List { op: Operator::Named(id), args, .. } = g {
        if resolve(*id) == "exp" && args.len() == 1 {
            return gaussian_integral(&args[0], var, var_id);
        }
    }
    None
}

/// ∫ exp(a*x^2 + b*x + c) dx via completing the square → erf (a<0) / erfi (a>0).
fn gaussian_integral(exponent: &Expr, var: &Expr, var_id: maxima_core::SymbolId) -> Option<Expr> {
    let poly = maxima_poly::expr_to_poly(exponent, var_id)?;
    if poly.degree()? != 2 {
        return None;
    }
    let a = get_coeff(&poly, 2);
    let b = get_coeff(&poly, 1);
    let c = get_coeff(&poly, 0);
    let a_sign = to_f64(&coeff_to_expr(&a))?;
    if a_sign == 0.0 {
        return None;
    }

    let a_expr = coeff_to_expr(&a);
    let b_expr = coeff_to_expr(&b);
    let c_expr = coeff_to_expr(&c);
    // shift = b/(2a), s = x + shift
    let shift = simplify(&Expr::div(b_expr.clone(), Expr::mul(Expr::int(2), a_expr.clone())));
    let s = simplify(&Expr::add(var.clone(), shift));
    // K = c - b^2/(4a)
    let k = simplify(&Expr::sub(
        c_expr,
        Expr::div(Expr::pow(b_expr, Expr::int(2)), Expr::mul(Expr::int(4), a_expr.clone())),
    ));
    let exp_k = if k == Expr::int(0) {
        Expr::int(1)
    } else {
        Expr::call("exp", vec![k])
    };
    let pi = Expr::sym("%pi");

    if a_sign > 0.0 {
        // ∫ exp(a s^2) ds = (1/2) sqrt(pi/a) erfi(sqrt(a) s)
        let sqrt_a = simplify(&Expr::call("sqrt", vec![a_expr]));
        let coeff = simplify(&Expr::div(
            Expr::call("sqrt", vec![pi]),
            Expr::mul(Expr::int(2), sqrt_a.clone()),
        ));
        let arg = simplify(&Expr::mul(sqrt_a, s));
        Some(simplify(&Expr::mul(
            Expr::mul(coeff, exp_k),
            Expr::call("erfi", vec![arg]),
        )))
    } else {
        // a<0: ∫ exp(-|a| s^2) ds = (1/2) sqrt(pi/|a|) erf(sqrt(|a|) s)
        let neg_a = simplify(&Expr::neg(a_expr));
        let sqrt_a = simplify(&Expr::call("sqrt", vec![neg_a]));
        let coeff = simplify(&Expr::div(
            Expr::call("sqrt", vec![pi]),
            Expr::mul(Expr::int(2), sqrt_a.clone()),
        ));
        let arg = simplify(&Expr::mul(sqrt_a, s));
        Some(simplify(&Expr::mul(
            Expr::mul(coeff, exp_k),
            Expr::call("erf", vec![arg]),
        )))
    }
}

/// Two var-dependent factors: g(k*x) * x^(-1) → Ei / Si / Ci.
fn named_ratio(p: &Expr, q: &Expr, var: &Expr, var_id: maxima_core::SymbolId) -> Option<Expr> {
    // Identify which factor is x^(-1) and which is the numerator function.
    let is_inv_x = |e: &Expr| {
        matches!(e, Expr::List { op: Operator::MExpt, args, .. }
            if args.len() == 2 && args[0] == *var && args[1] == Expr::int(-1))
    };
    let (numer, _) = if is_inv_x(q) {
        (p, q)
    } else if is_inv_x(p) {
        (q, p)
    } else {
        return None;
    };

    if let Expr::List { op: Operator::Named(id), args, .. } = numer {
        if args.len() != 1 {
            return None;
        }
        // argument must be linear k*x with k constant (k=1 included)
        let inner = &args[0];
        let lin = maxima_poly::expr_to_poly(inner, var_id)?;
        if lin.degree()? != 1 || !get_coeff(&lin, 0).is_zero() {
            return None;
        }
        let name = resolve(*id);
        let mapped = match name.as_str() {
            "exp" => "expintegral_ei",
            "sin" => "expintegral_si",
            "cos" => "expintegral_ci",
            _ => return None,
        };
        return Some(Expr::call(mapped, vec![inner.clone()]));
    }
    None
}

fn get_coeff(poly: &maxima_poly::Poly, deg: u32) -> maxima_poly::Coeff {
    poly.terms
        .iter()
        .find(|(e, _)| *e == deg)
        .map(|(_, c)| c.clone())
        .unwrap_or(maxima_poly::Coeff::zero())
}

/// Fully evaluate a purely numeric (constant) expression to a reduced
/// Integer/Rational. `simplify`/`ratsimp` leave forms like `4/4` as `4·4⁻¹`
/// which `to_f64` cannot read; `meval` reduces them.
fn rat_eval(e: &Expr) -> Expr {
    meval(e, &mut crate::Environment::new())
}

/// ∫ R(x, √(ax²+bx+c)) dx for the common rational forms R, via
/// completing-the-square (→ asinh/asin/log) plus the Euler reduction
/// u = 1/(x+r) for ∫ 1/((x+r)√Q). Every result is differentiated back and
/// checked numerically before it is returned (noun form beats a wrong answer).
fn try_quadratic_radical_integrate(f: &Expr, var: &Expr) -> Option<Expr> {
    let var_id = match var {
        Expr::Symbol(id) => *id,
        _ => return None,
    };

    // Locate the √(quadratic) factor and its sign (+1 = numerator, -1 = denominator).
    let mut factors = Vec::new();
    collect_mult_factors(f, &mut factors);
    let mut radicand: Option<Expr> = None;
    let mut rad_sign = 0i32;
    let mut rest_factors: Vec<Expr> = Vec::new();
    for fac in &factors {
        if radicand.is_none() {
            if let Some((q, s)) = match_sqrt_factor(fac) {
                radicand = Some(q);
                rad_sign = s;
                continue;
            }
        }
        rest_factors.push(fac.clone());
    }
    let q = radicand?;
    let qpoly = maxima_poly::expr_to_poly(&q, var_id)?;
    if qpoly.degree()? != 2 {
        return None;
    }
    let a = coeff_to_expr(&get_coeff(&qpoly, 2));
    let b = coeff_to_expr(&get_coeff(&qpoly, 1));
    let c = coeff_to_expr(&get_coeff(&qpoly, 0));

    let rest = if rest_factors.is_empty() {
        Expr::int(1)
    } else if rest_factors.len() == 1 {
        simplify(&rest_factors[0])
    } else {
        simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: rest_factors })
    };

    let candidate = if rad_sign == 1 {
        // √Q in the numerator: only the constant-multiple case ∫ k·√Q.
        if contains_var(&rest, var) {
            return None;
        }
        simplify(&Expr::mul(rest, integrate_sqrt_quadratic(&a, &b, &c, var)?))
    } else {
        // 1/√Q times a rational rest.
        if !contains_var(&rest, var) {
            // ∫ k/√Q
            simplify(&Expr::mul(rest, integrate_inv_sqrt_quadratic(&a, &b, &c, var)?))
        } else if let Some(rp) = maxima_poly::expr_to_poly(&rest, var_id) {
            // ∫ P(x)/√Q for a polynomial numerator P (any degree ≥ 1).
            let d = rp.degree()? as usize;
            let p_coeffs: Vec<Expr> = (0..=d).map(|k| coeff_to_expr(&get_coeff(&rp, k as u32))).collect();
            integrate_poly_over_sqrt(&a, &b, &c, &p_coeffs, &q, var)?
        } else {
            // ∫ 1/((x+r)√Q) (Euler substitution) deferred — returns noun.
            return None;
        }
    };

    // Accept the candidate or its negation (the Euler reduction leaves a sign
    // ambiguity), whichever differentiates back to the integrand.
    if verify_antiderivative(&candidate, f, var) {
        Some(candidate)
    } else {
        let neg = simplify(&Expr::neg(candidate));
        if verify_antiderivative(&neg, f, var) {
            Some(neg)
        } else {
            None
        }
    }
}

/// Match a √(quadratic) factor: sqrt(Q), Q^(1/2), 1/sqrt(Q), Q^(-1/2).
/// Returns (radicand, +1 for numerator / -1 for denominator).
fn match_sqrt_factor(fac: &Expr) -> Option<(Expr, i32)> {
    match fac {
        Expr::List { op: Operator::Named(id), args, .. } if args.len() == 1 && resolve(*id) == "sqrt" => {
            Some((args[0].clone(), 1))
        }
        Expr::List { op: Operator::MExpt, args, .. } if args.len() == 2 => {
            // sqrt(Q)^(-1)
            if args[1] == Expr::int(-1) {
                if let Expr::List { op: Operator::Named(id), args: inner, .. } = &args[0] {
                    if inner.len() == 1 && resolve(*id) == "sqrt" {
                        return Some((inner[0].clone(), -1));
                    }
                }
            }
            // Q^(1/2) or Q^(-1/2)
            match &args[1] {
                Expr::Rational { num: 1, den: 2 } => Some((args[0].clone(), 1)),
                Expr::Rational { num: -1, den: 2 } => Some((args[0].clone(), -1)),
                _ => None,
            }
        }
        _ => None,
    }
}

/// ∫ 1/√(a x² + b x + c) dx using the discriminant form
///   a>0, disc>0: (1/√a)·asinh((2ax+b)/√disc)
///   a>0, disc<0: (1/√a)·log(2ax+b + 2√a·√Q)
///   a<0, disc<0: (1/√(-a))·asin((2ax+b)/√(-disc))
/// where disc = 4ac − b² (an integer for integer coefficients, so √disc is
/// clean — avoiding the sqrt-of-fraction pitfall of the completing-square form).
fn integrate_inv_sqrt_quadratic(a: &Expr, b: &Expr, c: &Expr, var: &Expr) -> Option<Expr> {
    let af = to_f64(a)?;
    if af == 0.0 {
        return None;
    }
    // disc = 4ac − b²
    let disc = rat_eval(&Expr::sub(
        Expr::mul(Expr::mul(Expr::int(4), a.clone()), c.clone()),
        Expr::pow(b.clone(), Expr::int(2)),
    ));
    let discf = to_f64(&disc)?;
    // lin = 2a·x + b
    let lin = simplify(&Expr::add(Expr::mul(Expr::mul(Expr::int(2), a.clone()), var.clone()), b.clone()));
    let q = simplify(&Expr::add(
        Expr::add(Expr::mul(a.clone(), Expr::pow(var.clone(), Expr::int(2))), Expr::mul(b.clone(), var.clone())),
        c.clone(),
    ));

    if af > 0.0 {
        let sqrt_a = simplify(&Expr::call("sqrt", vec![a.clone()]));
        let inner = if discf > 0.0 {
            let arg = rat_eval(&Expr::div(lin, Expr::call("sqrt", vec![disc])));
            Expr::call("asinh", vec![arg])
        } else if discf < 0.0 {
            // log(2ax+b + 2√a·√Q)
            let radit = simplify(&Expr::mul(
                Expr::mul(Expr::int(2), sqrt_a.clone()),
                Expr::call("sqrt", vec![q]),
            ));
            Expr::call("log", vec![simplify(&Expr::add(lin, radit))])
        } else {
            Expr::call("log", vec![lin])
        };
        Some(simplify(&Expr::div(inner, sqrt_a)))
    } else {
        // a<0 needs disc<0 (i.e. b²−4ac > 0) for a real integral.
        if discf >= 0.0 {
            return None;
        }
        // ∫1/√(ax²+bx+c) = (1/√(-a))·asin(-(2ax+b)/√(b²-4ac)) for a<0.
        let sqrt_na = simplify(&Expr::call("sqrt", vec![Expr::neg(a.clone())]));
        let neg_disc = simplify(&Expr::neg(disc));
        let arg = rat_eval(&Expr::div(Expr::neg(lin), Expr::call("sqrt", vec![neg_disc])));
        Some(simplify(&Expr::div(Expr::call("asin", vec![arg]), sqrt_na)))
    }
}

/// ∫ P(x)/√(a x² + b x + c) dx for a polynomial numerator P, via the reduction
/// ∫P/√Q = R(x)·√Q + λ·∫1/√Q, where deg R = deg P − 1. Matching coefficients of
/// R'·Q + ½·R·Q' + λ = P gives the top-down recurrence
///   [x^k]:  a·k·r_{k-1} + b·(k+½)·r_k + c·(k+1)·r_{k+1} = p_k
/// solved from k = d (r_{d-1} = p_d/(a·d)) down to k = 1, then λ from [x^0].
/// `p` holds the numerator coefficients, p[i] = coefficient of x^i.
fn integrate_poly_over_sqrt(
    a: &Expr, b: &Expr, c: &Expr, p: &[Expr], q_expr: &Expr, var: &Expr,
) -> Option<Expr> {
    let d = p.len().checked_sub(1)?;
    let inv = integrate_inv_sqrt_quadratic(a, b, c, var)?;

    // r[j] = coefficient of x^j in R, for j = 0..d-1 (empty if d == 0).
    let mut r = vec![Expr::int(0); d];
    let get_r = |r: &Vec<Expr>, idx: usize| -> Expr { r.get(idx).cloned().unwrap_or(Expr::int(0)) };
    // Solve r_{k-1} from the x^k equation, k = d .. 1.
    for k in (1..=d).rev() {
        let rk = get_r(&r, k);
        let rk1 = get_r(&r, k + 1);
        // half = b·(2k+1)/2
        let bterm = Expr::mul(Expr::mul(b.clone(), Expr::Rational { num: (2 * k as i64 + 1), den: 2 }), rk);
        let cterm = Expr::mul(Expr::mul(c.clone(), Expr::int(k as i64 + 1)), rk1);
        let numer = Expr::sub(Expr::sub(p[k].clone(), bterm), cterm);
        let rkm1 = rat_eval(&Expr::div(numer, Expr::mul(a.clone(), Expr::int(k as i64))));
        r[k - 1] = rkm1;
    }
    // λ = p_0 − (b/2)·r_0 − c·r_1
    let lambda = rat_eval(&Expr::sub(
        Expr::sub(
            p[0].clone(),
            Expr::mul(Expr::div(b.clone(), Expr::int(2)), get_r(&r, 0)),
        ),
        Expr::mul(c.clone(), get_r(&r, 1)),
    ));

    // R(x) = Σ r_j x^j
    let mut r_terms = Vec::new();
    for (j, rj) in r.iter().enumerate() {
        if *rj == Expr::int(0) { continue; }
        let term = if j == 0 { rj.clone() } else { Expr::mul(rj.clone(), Expr::pow(var.clone(), Expr::int(j as i64))) };
        r_terms.push(term);
    }
    let r_poly = if r_terms.is_empty() {
        Expr::int(0)
    } else {
        simplify(&Expr::List { op: Operator::MPlus, simplified: false, args: r_terms })
    };

    let sqrt_q = simplify(&Expr::call("sqrt", vec![q_expr.clone()]));
    let rational_part = simplify(&Expr::mul(r_poly, sqrt_q));
    Some(simplify(&Expr::add(rational_part, Expr::mul(lambda, inv))))
}

/// ∫ √(a x² + b x + c) dx
/// = (2a x + b)/(4a)·√Q + ((4ac − b²)/(8a))·∫ 1/√Q dx.
fn integrate_sqrt_quadratic(a: &Expr, b: &Expr, c: &Expr, var: &Expr) -> Option<Expr> {
    let inv = integrate_inv_sqrt_quadratic(a, b, c, var)?;
    let q_expr = simplify(&Expr::add(
        Expr::add(Expr::mul(a.clone(), Expr::pow(var.clone(), Expr::int(2))), Expr::mul(b.clone(), var.clone())),
        c.clone(),
    ));
    let sqrt_q = simplify(&Expr::call("sqrt", vec![q_expr]));
    let lin = simplify(&Expr::div(
        Expr::add(Expr::mul(Expr::mul(Expr::int(2), a.clone()), var.clone()), b.clone()),
        rat_eval(&Expr::mul(Expr::int(4), a.clone())),
    ));
    let term1 = simplify(&Expr::mul(lin, sqrt_q));
    let k = rat_eval(&Expr::div(
        Expr::sub(Expr::mul(Expr::mul(Expr::int(4), a.clone()), c.clone()), Expr::pow(b.clone(), Expr::int(2))),
        Expr::mul(Expr::int(8), a.clone()),
    ));
    Some(simplify(&Expr::add(term1, Expr::mul(k, inv))))
}

