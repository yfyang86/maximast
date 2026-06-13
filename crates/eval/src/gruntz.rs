use maxima_core::{Expr, Operator, resolve};
use crate::helpers::{to_f64, contains_var, subst};
use crate::simp::simplify;

/// Gruntz algorithm: compute limit as var → +∞.
/// Tries proper MRV-based algorithm first, falls back to growth-order heuristic.
pub fn gruntz_limit(expr: &Expr, var: &Expr) -> Option<Expr> {
    if let Some(result) = gruntz_mrv(expr, var, 0) {
        return Some(result);
    }
    // Fallback to heuristic
    limitinf(expr, var, 0)
}

/// Proper Gruntz algorithm using MRV (Most Rapidly Varying) sets.
/// Algorithm from Gruntz PhD 1996, Chapter 4.
fn gruntz_mrv(expr: &Expr, var: &Expr, depth: u32) -> Option<Expr> {
    if depth > 15 { return None; }
    if !contains_var(expr, var) { return Some(expr.clone()); }

    // Step 1: Compute the MRV set
    let mrv = compute_mrv(expr, var);
    if mrv.is_empty() {
        return limitinf(expr, var, 0);
    }

    // Step 2: Choose ω — the representative from MRV with largest growth
    // ω should be of the form exp(g(x)) where g → +∞
    let omega = choose_omega(&mrv, var)?;

    let rewritten = match rewrite_in_omega(expr, &omega, var) {
        Some(r) => r,
        None => {
            return None;
        }
    };

    // Step 4: Extract leading term (after combining and cancellation)
    let combined = combine_terms(rewritten);
    if let Some((exp, coeff)) = leading_term_of_rewrite(&combined) {
        // Step 5: Determine limit from leading term
        if exp > 0.0 {
            return Some(Expr::int(0));
        } else if exp < 0.0 {
            if let Some(sign) = sign_of_limit(&coeff, var, depth) {
                return Some(if sign > 0 { Expr::sym("inf") } else { Expr::sym("minf") });
            }
        } else {
            return gruntz_mrv(&coeff, var, depth + 1);
        }
    }

    None
}

/// Compute the MRV set: subexpressions that grow most rapidly as var → ∞.
/// Returns expressions of the form exp(g(x)) that are in the same comparability class.
fn compute_mrv(expr: &Expr, var: &Expr) -> Vec<Expr> {
    if !contains_var(expr, var) { return vec![]; }

    match expr {
        Expr::Symbol(_) if expr == var => vec![var.clone()],
        Expr::List { op: Operator::Named(id), args, .. } if args.len() == 1 => {
            let fname = resolve(*id);
            if fname == "exp" {
                let inner_mrv = compute_mrv(&args[0], var);
                // Check if the argument actually → +∞ (not just has growth > 0)
                if let Some(lim) = limitinf(&args[0], var, 0) {
                    if is_pos_inf(&lim) {
                        // exp(g) where g → +∞: this is in the MRV set
                        let mut result = vec![expr.clone()];
                        result.extend(inner_mrv);
                        return result;
                    }
                }
                // exp(g) where g → -∞ or finite: not in MRV
                return inner_mrv;
            } else {
                compute_mrv(&args[0], var)
            }
        }
        Expr::List { args, .. } => {
            let mut result = vec![];
            for arg in args {
                let sub = compute_mrv(arg, var);
                result = mrv_max(result, sub, var);
            }
            result
        }
        _ => vec![],
    }
}

/// Merge two MRV sets: keep only the fastest-growing set.
fn mrv_max(a: Vec<Expr>, b: Vec<Expr>, var: &Expr) -> Vec<Expr> {
    if a.is_empty() { return b; }
    if b.is_empty() { return a; }
    let ga = a.iter().map(|e| growth_order(e, var)).max().unwrap_or(0);
    let gb = b.iter().map(|e| growth_order(e, var)).max().unwrap_or(0);
    if ga > gb { a }
    else if gb > ga { b }
    else {
        let mut merged = a;
        for e in b { if !merged.contains(&e) { merged.push(e); } }
        merged
    }
}

/// Choose the representative ω from the MRV set.
/// Prefers exp(g) forms; falls back to var itself.
fn choose_omega(mrv: &[Expr], var: &Expr) -> Option<Expr> {
    // Pick the exp(...) with SIMPLEST argument (fewest nodes) — this ensures
    // the rewrite step can express other exp terms as powers of ω.
    let mut best: Option<(Expr, usize)> = None;
    for e in mrv {
        if let Expr::List { op: Operator::Named(id), args, .. } = e {
            if resolve(*id) == "exp" && args.len() == 1 {
                if let Some(lim) = limitinf(&args[0], var, 0) {
                    if is_pos_inf(&lim) {
                        let size = expr_size(&args[0]);
                        if best.as_ref().map(|(_, s)| size < *s).unwrap_or(true) {
                            best = Some((e.clone(), size));
                        }
                    }
                }
            }
        }
    }
    if let Some((omega, _)) = best { return Some(omega); }
    if mrv.contains(var) { return Some(var.clone()); }
    mrv.first().cloned()
}

/// Rewrite expr in terms of ω, returning (exponent_mapping, rewritten_expr).
/// For ω = exp(g): replace exp(c*g) with ω^c, and ω → 0.
/// For ω = x: replace x with 1/ω, and ω → 0.
fn rewrite_in_omega(expr: &Expr, omega: &Expr, var: &Expr) -> Option<Vec<(f64, Expr)>> {
    if let Expr::List { op: Operator::Named(id), args, .. } = omega {
        if resolve(*id) == "exp" && args.len() == 1 {
            let g = &args[0]; // omega = exp(g)
            return rewrite_exp_omega(expr, g, omega, var);
        }
    }
    // omega = var: substitute x = 1/ω
    if omega == var {
        return rewrite_var_omega(expr, var);
    }
    None
}

/// Rewrite for ω = exp(g(x)). Replace occurrences of exp(c*g) with ω^c.
fn rewrite_exp_omega(expr: &Expr, g: &Expr, omega: &Expr, var: &Expr) -> Option<Vec<(f64, Expr)>> {
    if !contains_var(expr, var) {
        return Some(vec![(0.0, expr.clone())]);
    }
    // If expr IS omega = exp(g): ω = exp(-g), so exp(g) = ω^(-1)
    if expr == omega {
        return Some(vec![(-1.0, Expr::int(1))]);
    }
    match expr {
        Expr::List { op: Operator::Named(id), args, .. } if resolve(*id) == "exp" && args.len() == 1 => {
            let h = &args[0];
            let ratio = simplify(&Expr::div(h.clone(), g.clone()));
            let c_val = if let Some(c) = to_f64(&ratio) {
                if !contains_var(&ratio, var) { Some(c) } else { None }
            } else {
                // Numeric fallback: evaluate ratio at a test point
                let mut env = crate::Environment::new();
                let test = Expr::Float(2.7);
                let r_at = crate::eval::meval(&subst(&test, var, &ratio), &mut env);
                if let Some(v) = to_f64(&r_at) {
                    let rounded = v.round();
                    if (v - rounded).abs() < 1e-10 { Some(rounded) } else { None }
                } else { None }
            };
            if let Some(c) = c_val {
                return Some(vec![(-c, Expr::int(1))]);
            }
            // Check if h = c*g + small(x) where small → 0 as x → ∞
            // Then exp(h) = exp(c*g)*exp(small) = ω^(-c) * (1 + small + small²/2 + ...)
            // Try to decompose h = ratio_part*g + remainder
            if let Expr::List { op: Operator::MPlus, args: sum_args, .. } = h {
                let mut g_coeff = 0.0f64;
                let mut remainder = Vec::new();
                for term in sum_args {
                    let r = simplify(&Expr::div(term.clone(), g.clone()));
                    if let Some(c) = to_f64(&r) {
                        if !contains_var(&r, var) {
                            g_coeff += c;
                            continue;
                        }
                    }
                    remainder.push(term.clone());
                }
                if g_coeff != 0.0 && !remainder.is_empty() {
                    let rest = if remainder.len() == 1 { remainder[0].clone() }
                        else { Expr::List { op: Operator::MPlus, simplified: false, args: remainder } };
                    // Check if rest → 0 (so we can Taylor-expand exp(rest))
                    if let Some(rest_lim) = limitinf(&rest, var, 0) {
                        if rest_lim == Expr::int(0) {
                            // exp(h) = ω^(-g_coeff) * exp(rest)
                            // ≈ ω^(-g_coeff) * (1 + rest + rest²/2 + ...)
                            let rest_rewritten = rewrite_exp_omega(&rest, g, omega, var);
                            if let Some(rest_terms) = rest_rewritten {
                                // Build exp(rest) ≈ 1 + rest + rest²/2
                                let mut exp_terms = vec![(0.0, Expr::int(1))]; // constant term 1
                                // Add rest terms (first order)
                                for (e, c) in &rest_terms {
                                    exp_terms.push((*e, c.clone()));
                                }
                                // Add rest²/2 terms (second order)
                                let rest_sq = convolve_terms(&rest_terms, &rest_terms);
                                for (e, c) in rest_sq {
                                    exp_terms.push((e, simplify(&Expr::div(c, Expr::int(2)))));
                                }
                                let exp_series = combine_terms(exp_terms);
                                // Multiply by ω^(-g_coeff)
                                let result: Vec<(f64, Expr)> = exp_series.into_iter()
                                    .map(|(e, c)| (e - g_coeff, c))
                                    .collect();
                                return Some(result);
                            }
                        }
                    }
                }
            }
            // exp(something_small): expand as 1 + something + something²/2
            if let Some(h_lim) = limitinf(h, var, 0) {
                if h_lim == Expr::int(0) {
                    if let Some(h_terms) = rewrite_exp_omega(h, g, omega, var) {
                        let mut exp_terms = vec![(0.0, Expr::int(1))];
                        for (e, c) in &h_terms { exp_terms.push((*e, c.clone())); }
                        let h_sq = convolve_terms(&h_terms, &h_terms);
                        for (e, c) in h_sq { exp_terms.push((e, simplify(&Expr::div(c, Expr::int(2))))); }
                        return Some(combine_terms(exp_terms));
                    }
                }
            }
            None
        }
        Expr::List { op: Operator::MTimes, args, .. } => {
            // Multiply: convolve rewritings
            let mut result = vec![(0.0, Expr::int(1))];
            for arg in args {
                let sub = rewrite_exp_omega(arg, g, omega, var)?;
                result = convolve_terms(&result, &sub);
            }
            Some(result)
        }
        Expr::List { op: Operator::MPlus, args, .. } => {
            let mut all_terms = Vec::new();
            for arg in args {
                let sub = rewrite_exp_omega(arg, g, omega, var)?;
                all_terms.extend(sub);
            }
            Some(combine_terms(all_terms))
        }
        Expr::List { op: Operator::MExpt, args, .. } if args.len() == 2 => {
            if let Some(n) = to_f64(&args[1]) {
                let base_terms = rewrite_exp_omega(&args[0], g, omega, var)?;
                let result: Vec<(f64, Expr)> = base_terms.into_iter()
                    .map(|(e, c)| (e * n, simplify(&Expr::pow(c, args[1].clone()))))
                    .collect();
                return Some(result);
            }
            None
        }
        _ => {
            if !contains_var(expr, var) {
                Some(vec![(0.0, expr.clone())])
            } else {
                None
            }
        }
    }
}

/// Rewrite for ω = var: x = 1/ω, so x^n = ω^(-n).
fn rewrite_var_omega(expr: &Expr, var: &Expr) -> Option<Vec<(f64, Expr)>> {
    if !contains_var(expr, var) {
        return Some(vec![(0.0, expr.clone())]);
    }
    if expr == var {
        return Some(vec![(-1.0, Expr::int(1))]);
    }
    match expr {
        Expr::List { op: Operator::MExpt, args, .. } if args.len() == 2 && args[0] == *var => {
            if let Some(n) = to_f64(&args[1]) {
                return Some(vec![(-n, Expr::int(1))]);
            }
            None
        }
        Expr::List { op: Operator::MExpt, args, .. } if args.len() == 2 => {
            // Handle log(x)^n etc.
            if let Some(n) = to_f64(&args[1]) {
                let base = rewrite_var_omega(&args[0], var)?;
                return Some(base.into_iter()
                    .map(|(e, c)| (e * n, simplify(&Expr::pow(c, args[1].clone()))))
                    .collect());
            }
            None
        }
        Expr::List { op: Operator::Named(id), args: fargs, .. }
            if fargs.len() == 1 && resolve(*id) == "log" =>
        {
            // log(x) as x→∞: grows to ∞ but slower than any power of x.
            // In the ω-series where ω=x, x=1/ω: log(x) = -log(ω).
            // As ω→0, -log(ω) → +∞. This is not a finite-order ω term.
            // For expressions like log(log(x))/log(x), we need to handle
            // the ratio directly. Return log(x) as a coefficient at exponent 0
            // (since it doesn't affect the leading ω-power).
            if fargs[0] == *var {
                // log(x): grows, but at exponent 0 in the ω-expansion
                return Some(vec![(0.0, Expr::call("log", vec![var.clone()]))]);
            }
            None
        }
        Expr::List { op: Operator::MTimes, args, .. } => {
            let mut result = vec![(0.0, Expr::int(1))];
            for arg in args {
                let sub = rewrite_var_omega(arg, var)?;
                result = convolve_terms(&result, &sub);
            }
            Some(result)
        }
        Expr::List { op: Operator::MPlus, args, .. } => {
            let mut all = Vec::new();
            for arg in args {
                let sub = rewrite_var_omega(arg, var)?;
                all.extend(sub);
            }
            Some(combine_terms(all))
        }
        _ => None,
    }
}

fn convolve_terms(a: &[(f64, Expr)], b: &[(f64, Expr)]) -> Vec<(f64, Expr)> {
    let mut result = Vec::new();
    for (ea, ca) in a {
        for (eb, cb) in b {
            result.push((ea + eb, simplify(&Expr::mul(ca.clone(), cb.clone()))));
        }
    }
    combine_terms(result)
}

fn combine_terms(mut terms: Vec<(f64, Expr)>) -> Vec<(f64, Expr)> {
    terms.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut combined: Vec<(f64, Expr)> = Vec::new();
    for (e, c) in terms {
        if let Some(last) = combined.last_mut() {
            if (last.0 - e).abs() < 1e-12 {
                last.1 = simplify(&Expr::add(last.1.clone(), c));
                continue;
            }
        }
        combined.push((e, c));
    }
    combined.into_iter().filter(|(_, c)| *c != Expr::int(0)).collect()
}

fn leading_term_of_rewrite(terms: &[(f64, Expr)]) -> Option<(f64, Expr)> {
    terms.first().map(|(e, c)| (*e, c.clone()))
}

fn sign_of_limit(expr: &Expr, var: &Expr, depth: u32) -> Option<i32> {
    if let Some(v) = to_f64(expr) {
        return Some(if v > 0.0 { 1 } else if v < 0.0 { -1 } else { 0 });
    }
    if let Some(lim) = gruntz_mrv(expr, var, depth + 1) {
        if let Some(v) = to_f64(&lim) {
            return Some(if v > 0.0 { 1 } else if v < 0.0 { -1 } else { 0 });
        }
    }
    None
}

/// Core: compute limit of expr as var → +∞.
fn limitinf(expr: &Expr, var: &Expr, depth: u32) -> Option<Expr> {
    if depth > 10 { return None; }
    if !contains_var(expr, var) { return Some(expr.clone()); }

    match expr {
        Expr::Symbol(_) if expr == var => Some(Expr::sym("inf")),

        Expr::List { op: Operator::MPlus, args, .. } => {
            let mut terms: Vec<(Expr, i32)> = args.iter()
                .map(|a| (a.clone(), growth_order(a, var)))
                .collect();
            terms.sort_by(|a, b| b.1.cmp(&a.1));
            if terms.is_empty() { return Some(Expr::int(0)); }
            // Dominant term determines limit
            let max_growth = terms[0].1;
            if max_growth > 0 {
                // Collect all terms with max growth, take their limit
                let dominant: Vec<&Expr> = terms.iter()
                    .filter(|(_, g)| *g == max_growth)
                    .map(|(t, _)| t)
                    .collect();
                if dominant.len() == 1 {
                    return limitinf(dominant[0], var, depth + 1);
                }
            }
            // When multiple dominant terms exist (potential ∞-∞ cancellation),
            // try rationalization via ratsimp
            if max_growth > 0 {
                let dominant: Vec<&Expr> = terms.iter()
                    .filter(|(_, g)| *g == max_growth)
                    .map(|(t, _)| t)
                    .collect();
                if dominant.len() > 1 {
                    let dom_args: Vec<Expr> = dominant.into_iter().cloned().collect();
                    let sum_expr = Expr::List {
                        op: Operator::MPlus, simplified: false, args: dom_args.clone(),
                    };
                    let rationalized = simplify(&crate::simp::simplify(&sum_expr));
                    if growth_order(&rationalized, var) < max_growth {
                        return limitinf(&rationalized, var, depth + 1);
                    }
                    // Try conjugate for sqrt(f) ± g: multiply by (sqrt(f) ∓ g)/(sqrt(f) ∓ g)
                    if dom_args.len() == 2 {
                        if let Some(conj) = try_conjugate(&dom_args[0], &dom_args[1]) {
                            if growth_order(&conj, var) < max_growth {
                                return limitinf(&conj, var, depth + 1);
                            }
                        }
                    }
                }
            }
            // All same growth or all constant: sum limits
            let mut sum = Expr::int(0);
            for (t, _) in &terms {
                if let Some(lim) = limitinf(t, var, depth + 1) {
                    sum = add_with_inf(sum, lim);
                } else {
                    return None;
                }
            }
            Some(sum)
        }

        Expr::List { op: Operator::MTimes, args, .. } => {
            // Check growth orders: exponential growth dominates polynomial
            let orders: Vec<i32> = args.iter().map(|a| growth_order(a, var)).collect();
            let has_exp_growth = orders.iter().any(|&g| g >= 2);
            let has_exp_decay = orders.iter().any(|&g| g <= -2);
            let overall_growth: i32 = orders.iter().sum();
            if has_exp_growth && !has_exp_decay {
                // Exponential growth dominates: determine sign
                let mut sign_positive = true;
                for a in args {
                    if let Some(lim) = limitinf(a, var, depth + 1) {
                        if is_neg_inf(&lim) { sign_positive = !sign_positive; }
                        else if let Some(v) = to_f64(&lim) { if v < 0.0 { sign_positive = !sign_positive; } }
                    }
                }
                return Some(if sign_positive { Expr::sym("inf") } else { Expr::sym("minf") });
            }
            if overall_growth <= -2 {
                return Some(Expr::int(0));
            }
            // Handle 0*∞ indeterminate forms
            if args.len() == 2 {
                let l0 = limitinf(&args[0], var, depth + 1);
                let l1 = limitinf(&args[1], var, depth + 1);
                if let (Some(ref a), Some(ref b)) = (&l0, &l1) {
                    let a_zero = a == &Expr::int(0);
                    let b_zero = b == &Expr::int(0);
                    let a_inf = is_inf(a);
                    let b_inf = is_inf(b);
                    if (a_zero && b_inf) || (a_inf && b_zero) {
                        // 0*∞: rewrite as f/g and try L'Hôpital via substitution
                        let _combined = simplify(&Expr::mul(args[0].clone(), args[1].clone()));
                        // Try Taylor-like: if one factor is sin(g)/g → 1 type
                        if let Some(result) = try_zero_inf_limit(&args[0], &args[1], var, depth) {
                            return Some(result);
                        }
                        if let Some(result) = try_zero_inf_limit(&args[1], &args[0], var, depth) {
                            return Some(result);
                        }
                        // For log-dominated 0*∞: log(f)/g or log(f)*g^(-n)
                        // If one factor has log growth and the other decays polynomially,
                        // the product → 0 (log grows slower than any positive power)
                        let (g0, g1) = (growth_order(&args[0], var), growth_order(&args[1], var));
                        // log^n * x^(-ε) → 0 for any ε > 0
                        if (g0 == 0 && g1 < 0) || (g0 < 0 && g1 == 0) {
                            // One is log-growth (order 0 but → ∞), other decays
                            return Some(Expr::int(0));
                        }
                    }
                }
            }
            let mut result = Expr::int(1);
            for arg in args {
                let lim = limitinf(arg, var, depth + 1)?;
                result = mul_with_inf(result, lim);
            }
            Some(result)
        }

        Expr::List { op: Operator::MExpt, args, .. } if args.len() == 2 => {
            let base = &args[0];
            let exp = &args[1];
            if base == var {
                if let Some(n) = to_f64(exp) {
                    return if n > 0.0 { Some(Expr::sym("inf")) }
                    else if n < 0.0 { Some(Expr::int(0)) }
                    else { Some(Expr::int(1)) };
                }
            }
            let base_lim = limitinf(base, var, depth + 1);
            let exp_lim = limitinf(exp, var, depth + 1);
            match (base_lim, exp_lim) {
                (Some(b), Some(e)) => {
                    if b == Expr::int(1) && is_inf(&e) {
                        // 1^∞: try log(1+f) ≈ f for small f
                        let f = simplify(&Expr::sub(base.clone(), Expr::int(1)));
                        let f_lim = limitinf(&f, var, depth + 1)?;
                        if f_lim == Expr::int(0) {
                            let product = simplify(&Expr::mul(exp.clone(), f));
                            if let Some(pl) = limitinf(&product, var, depth + 1) {
                                if !is_inf(&pl) {
                                    return Some(simplify(&Expr::call("exp", vec![pl])));
                                }
                            }
                        }
                        return None;
                    }
                    if is_pos_inf(&e) {
                        if let Some(bv) = to_f64(&b) {
                            if bv > 1.0 { return Some(Expr::sym("inf")); }
                            if (bv - 1.0).abs() < 1e-15 { return Some(Expr::int(1)); }
                            if bv > 0.0 { return Some(Expr::int(0)); }
                        }
                    }
                    if is_neg_inf(&e) {
                        if let Some(bv) = to_f64(&b) {
                            if bv > 1.0 { return Some(Expr::int(0)); }
                            if bv > 0.0 && bv < 1.0 { return Some(Expr::sym("inf")); }
                        }
                    }
                    // Handle inf^n for constant n
                    if is_pos_inf(&b) {
                        if let Some(n) = to_f64(&e) {
                            if n < 0.0 { return Some(Expr::int(0)); }
                            if n > 0.0 { return Some(Expr::sym("inf")); }
                            return Some(Expr::int(1));
                        }
                    }
                    Some(simplify(&Expr::pow(b, e)))
                }
                _ => None,
            }
        }

        Expr::List { op: Operator::Named(id), args, .. } if args.len() == 1 => {
            let fname = resolve(*id);
            let inner_lim = limitinf(&args[0], var, depth + 1)?;
            match fname.as_str() {
                "exp" => {
                    if is_pos_inf(&inner_lim) { Some(Expr::sym("inf")) }
                    else if is_neg_inf(&inner_lim) { Some(Expr::int(0)) }
                    else { Some(Expr::call("exp", vec![inner_lim])) }
                }
                "log" => {
                    if is_pos_inf(&inner_lim) { Some(Expr::sym("inf")) }
                    else if inner_lim == Expr::int(0) { Some(Expr::sym("minf")) }
                    else { Some(Expr::call("log", vec![inner_lim])) }
                }
                "sin" if is_inf(&inner_lim) => Some(Expr::sym("ind")),
                "sin" => {
                    if inner_lim == Expr::int(0) { Some(Expr::int(0)) }
                    else { Some(simplify(&Expr::call("sin", vec![inner_lim]))) }
                }
                "cos" if is_inf(&inner_lim) => Some(Expr::sym("ind")),
                "cos" => {
                    if inner_lim == Expr::int(0) { Some(Expr::int(1)) }
                    else { Some(simplify(&Expr::call("cos", vec![inner_lim]))) }
                }
                "tan" => {
                    if inner_lim == Expr::int(0) { Some(Expr::int(0)) }
                    else { Some(simplify(&Expr::call("tan", vec![inner_lim]))) }
                }
                "sqrt" => {
                    if is_pos_inf(&inner_lim) { Some(Expr::sym("inf")) }
                    else if inner_lim == Expr::int(0) { Some(Expr::int(0)) }
                    else { Some(simplify(&Expr::call("sqrt", vec![inner_lim]))) }
                }
                "atan" => {
                    if is_pos_inf(&inner_lim) {
                        Some(Expr::div(Expr::sym("%pi"), Expr::int(2)))
                    } else if is_neg_inf(&inner_lim) {
                        Some(Expr::neg(Expr::div(Expr::sym("%pi"), Expr::int(2))))
                    } else {
                        Some(simplify(&Expr::call("atan", vec![inner_lim])))
                    }
                }
                "abs" if is_inf(&inner_lim) => Some(Expr::sym("inf")),
                _ => Some(simplify(&Expr::call(&fname, vec![inner_lim]))),
            }
        }
        _ => None,
    }
}

/// Growth order: 2=exp, 1=polynomial/log, 0=constant, -1=decay, -2=exp decay
fn growth_order(expr: &Expr, var: &Expr) -> i32 {
    if !contains_var(expr, var) { return 0; }
    match expr {
        Expr::Symbol(_) if expr == var => 1,
        Expr::List { op: Operator::MExpt, args, .. } if args.len() == 2 => {
            if args[0] == *var {
                if let Some(n) = to_f64(&args[1]) {
                    return if n > 0.0 { 1 } else if n < 0.0 { -1 } else { 0 };
                }
            }
            // For f^n: if n is constant and f grows, growth depends on sign of n
            if let Some(n) = to_f64(&args[1]) {
                if !contains_var(&args[1], var) {
                    let bg = growth_order(&args[0], var);
                    if bg > 0 && n < 0.0 { return -1; } // f→∞, f^(-n) → 0
                    if bg > 0 && n > 0.0 { return bg; }
                    // f has log growth (bg=0 but f→∞) and n<0: decays
                    if bg == 0 && n < 0.0 {
                        if let Some(lim) = limitinf(&args[0], var, 0) {
                            if is_pos_inf(&lim) { return -1; }
                        }
                    }
                }
            }
            let bg = growth_order(&args[0], var);
            let eg = growth_order(&args[1], var);
            bg.max(eg)
        }
        Expr::List { op: Operator::Named(id), args, .. } if args.len() == 1 => {
            let fname = resolve(*id);
            match fname.as_str() {
                "exp" => {
                    let ig = growth_order(&args[0], var);
                    if ig > 0 {
                        // Check sign of the argument at ∞
                        if let Some(lim) = limitinf(&args[0], var, 0) {
                            if is_neg_inf(&lim) { -2 } else { 2 }
                        } else { 2 }
                    } else if ig < 0 { 0 } // exp(decay) → constant
                    else { 0 }
                }
                "log" => 0,
                _ => growth_order(&args[0], var),
            }
        }
        Expr::List { op: Operator::MTimes, args, .. } => {
            // Product: sum of growth orders, but check for exp cancellation
            let orders: Vec<i32> = args.iter().map(|a| growth_order(a, var)).collect();
            let has_pos_exp = orders.iter().any(|&g| g >= 2);
            let has_neg_exp = orders.iter().any(|&g| g <= -2);
            if has_pos_exp && !has_neg_exp { return 2; }
            if has_neg_exp && !has_pos_exp { return -2; }
            // Both exp growth and decay cancel — fall through to polynomial sum
            let total: i32 = orders.iter().filter(|&&g| g.abs() < 2).sum();
            total.clamp(-1, 1)
        }
        Expr::List { op: Operator::MPlus, args, .. } => {
            args.iter().map(|a| growth_order(a, var)).max().unwrap_or(0)
        }
        _ => 0,
    }
}

fn add_with_inf(a: Expr, b: Expr) -> Expr {
    if is_pos_inf(&a) && is_neg_inf(&b) { return Expr::sym("und"); }
    if is_neg_inf(&a) && is_pos_inf(&b) { return Expr::sym("und"); }
    if is_pos_inf(&a) || is_pos_inf(&b) { return Expr::sym("inf"); }
    if is_neg_inf(&a) || is_neg_inf(&b) { return Expr::sym("minf"); }
    simplify(&Expr::add(a, b))
}

fn mul_with_inf(a: Expr, b: Expr) -> Expr {
    if a == Expr::int(0) || b == Expr::int(0) { return Expr::int(0); }
    let (ap, an) = (is_pos_inf(&a), is_neg_inf(&a));
    let (bp, bn) = (is_pos_inf(&b), is_neg_inf(&b));
    if ap && bp || an && bn { return Expr::sym("inf"); }
    if ap && bn || an && bp { return Expr::sym("minf"); }
    if bp || bn {
        if let Some(n) = to_f64(&a) {
            return if n > 0.0 { b } else if n < 0.0 { if bp { Expr::sym("minf") } else { Expr::sym("inf") } } else { Expr::int(0) };
        }
    }
    if ap || an {
        if let Some(n) = to_f64(&b) {
            return if n > 0.0 { a } else if n < 0.0 { if ap { Expr::sym("minf") } else { Expr::sym("inf") } } else { Expr::int(0) };
        }
    }
    simplify(&Expr::mul(a, b))
}

/// Try to resolve 0*∞: given f→0 and g→∞, compute limit(f*g).
/// Handles cases like sin(1/x)*x by recognizing sin(t)/t → 1 as t → 0.
fn try_zero_inf_limit(f_zero: &Expr, g_inf: &Expr, var: &Expr, depth: u32) -> Option<Expr> {
    // Case: f = h(t(x)) where t(x) → 0, and g = c/t(x), so f*g = c*h(t)/t
    // Check if f is a named function applied to some argument that → 0
    if let Expr::List { op: Operator::Named(id), args, .. } = f_zero {
        if args.len() == 1 {
            let fname = resolve(*id);
            let t = &args[0];
            let t_lim = limitinf(t, var, depth + 1)?;
            if t_lim == Expr::int(0) {
                // f(t) → 0, g → ∞.
                // Check if g*t → finite (i.e., g ≈ c/t)
                let gt = simplify(&Expr::mul(g_inf.clone(), t.clone()));
                if let Some(gt_lim) = limitinf(&gt, var, depth + 1) {
                    if !is_inf(&gt_lim) {
                        // limit = gt_lim * lim_{t→0} f(t)/t
                        let ft_ratio = match fname.as_str() {
                            "sin" | "tan" | "sinh" | "tanh" | "asin" | "atan" => Some(Expr::int(1)),
                            "exp" => None, // exp(t)/t → ∞ as t → 0, not useful
                            "log" => None,
                            "cos" | "cosh" => None, // cos(t)/t → ∞
                            _ => None,
                        };
                        if let Some(ratio) = ft_ratio {
                            return Some(simplify(&Expr::mul(gt_lim, ratio)));
                        }
                    }
                }
            }
        }
    }
    None
}

/// Try conjugate rationalization: (√f - g) → (f - g²)/(√f + g).
fn try_conjugate(a: &Expr, b: &Expr) -> Option<Expr> {
    // Check if a = sqrt(something) and b = -something_else (or vice versa)
    // Detect sqrt(f) - g or -sqrt(f) + g patterns
    let (sqrt_expr, other_expr) = if is_sqrt(a) && is_negated_expr(b) {
        (a.clone(), negate_simple(b))
    } else if is_sqrt(b) && is_negated_expr(a) {
        (b.clone(), negate_simple(a))
    } else if is_neg_sqrt(a) {
        // -sqrt(f) + g → -(sqrt(f) - g)
        (negate_simple(a), b.clone())
    } else if is_neg_sqrt(b) {
        (negate_simple(b), a.clone())
    } else {
        return None;
    };

    let f_inner = get_sqrt_arg(&sqrt_expr)?;
    let num = simplify(&Expr::sub(f_inner, Expr::pow(other_expr.clone(), Expr::int(2))));
    let den = simplify(&Expr::add(sqrt_expr, other_expr));
    Some(simplify(&Expr::div(num, den)))
}

fn is_sqrt(e: &Expr) -> bool {
    matches!(e, Expr::List { op: Operator::Named(id), args, .. }
        if args.len() == 1 && resolve(*id) == "sqrt")
    || matches!(e, Expr::List { op: Operator::MExpt, args, .. }
        if args.len() == 2 && args[1] == (Expr::Rational { num: 1, den: 2 }))
}

fn is_neg_sqrt(e: &Expr) -> bool {
    if let Expr::List { op: Operator::MTimes, args, .. } = e {
        args.iter().any(|a| matches!(a, Expr::Integer(n) if *n == -1))
        && args.iter().any(|a| is_sqrt(a))
    } else { false }
}

fn is_negated_expr(e: &Expr) -> bool {
    matches!(e, Expr::List { op: Operator::MTimes, args, .. }
        if args.iter().any(|a| matches!(a, Expr::Integer(n) if *n == -1)))
    || matches!(e, Expr::Integer(n) if *n < 0)
}

fn negate_simple(e: &Expr) -> Expr {
    simplify(&Expr::neg(e.clone()))
}

fn get_sqrt_arg(e: &Expr) -> Option<Expr> {
    if let Expr::List { op: Operator::Named(id), args, .. } = e {
        if resolve(*id) == "sqrt" && args.len() == 1 { return Some(args[0].clone()); }
    }
    if let Expr::List { op: Operator::MExpt, args, .. } = e {
        if args.len() == 2 && args[1] == (Expr::Rational { num: 1, den: 2 }) {
            return Some(args[0].clone());
        }
    }
    None
}

fn expr_size(e: &Expr) -> usize {
    match e {
        Expr::List { args, .. } => 1 + args.iter().map(expr_size).sum::<usize>(),
        _ => 1,
    }
}

fn is_inf(e: &Expr) -> bool { is_pos_inf(e) || is_neg_inf(e) }
fn is_pos_inf(e: &Expr) -> bool { matches!(e, Expr::Symbol(id) if { let n = resolve(*id); n == "inf" || n == "infinity" }) }
fn is_neg_inf(e: &Expr) -> bool { matches!(e, Expr::Symbol(id) if resolve(*id) == "minf") }

#[cfg(test)]
mod tests {
    use super::*;

    #[test] fn const_lim() { assert_eq!(gruntz_limit(&Expr::int(5), &Expr::sym("x")), Some(Expr::int(5))); }
    #[test] fn x_lim() { assert_eq!(gruntz_limit(&Expr::sym("x"), &Expr::sym("x")), Some(Expr::sym("inf"))); }
    #[test] fn exp_lim() { assert_eq!(gruntz_limit(&Expr::call("exp", vec![Expr::sym("x")]), &Expr::sym("x")), Some(Expr::sym("inf"))); }
    #[test] fn exp_neg() { assert_eq!(gruntz_limit(&Expr::call("exp", vec![Expr::neg(Expr::sym("x"))]), &Expr::sym("x")), Some(Expr::int(0))); }
    #[test] fn log_lim() { assert_eq!(gruntz_limit(&Expr::call("log", vec![Expr::sym("x")]), &Expr::sym("x")), Some(Expr::sym("inf"))); }
    #[test] fn x_inv() { assert_eq!(gruntz_limit(&Expr::pow(Expr::sym("x"), Expr::int(-1)), &Expr::sym("x")), Some(Expr::int(0))); }
    #[test] fn decay_product() {
        let e = Expr::mul(Expr::int(3), Expr::call("exp", vec![Expr::neg(Expr::sym("x"))]));
        assert_eq!(gruntz_limit(&e, &Expr::sym("x")), Some(Expr::int(0)));
    }
    #[test] fn log_over_x() {
        // log(x) * x^(-1): growth_order = 0+(-1) = -1 → limit = 0
        let e = Expr::mul(Expr::call("log", vec![Expr::sym("x")]), Expr::pow(Expr::sym("x"), Expr::int(-1)));
        let g = growth_order(&e, &Expr::sym("x"));
        assert!(g <= 0, "growth_order of log(x)/x should be <= 0, got {}", g);
    }
}
