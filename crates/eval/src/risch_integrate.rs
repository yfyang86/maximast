use maxima_core::{Expr, Operator, SymbolId, resolve};
use crate::helpers::{contains_var, subst};
use crate::simp::simplify;
use crate::risch_tower::{Tower, Extension, build_tower};

/// Try Risch integration for transcendental integrands.
/// Builds a tower, then dispatches to the appropriate case.
pub fn risch_integrate(f: &Expr, var: &Expr) -> Option<Expr> {
    let tower = build_tower(f, var);
    if tower.is_empty() { return None; }

    // Try integrating in the tower, starting from the outermost extension
    integrate_in_tower(f, var, &tower)
}

fn integrate_in_tower(f: &Expr, var: &Expr, tower: &Tower) -> Option<Expr> {
    if tower.extensions.is_empty() { return None; }

    let (tid, ext, deriv) = tower.extensions.last()?;
    let t_var = Expr::Symbol(*tid);

    // Rewrite f in terms of tower variables
    let f_rewritten = tower.rewrite(f);

    match ext {
        Extension::Primitive { log_arg } => {
            integrate_primitive(&f_rewritten, var, &t_var, *tid, log_arg, deriv, tower)
        }
        Extension::Exponential { exp_arg } => {
            integrate_exponential(&f_rewritten, var, &t_var, *tid, exp_arg, deriv, tower)
        }
    }
}

/// Integrate in Q(x, t) where t = log(u(x)).
/// The integrand f should be rewritten in terms of t.
fn integrate_primitive(
    f: &Expr, var: &Expr, t_var: &Expr, tid: SymbolId,
    log_arg: &Expr, t_deriv: &Expr, tower: &Tower,
) -> Option<Expr> {
    // In Q(x)[t] where t = log(u), t' = u'/u:
    // Express f as polynomial in t with coefficients in Q(x)
    // f = a_n(x)*t^n + ... + a_1(x)*t + a_0(x)

    // Extract polynomial degree in t
    let coeffs = extract_poly_coeffs(f, t_var);
    if coeffs.is_empty() { return None; }

    let deg = coeffs.len() - 1;

    // For degree 0: f = a_0(x), no t involvement — shouldn't reach here
    if deg == 0 && !contains_var(&coeffs[0], t_var) {
        return None;
    }

    // The integral must have the form: g(x, t) where g is polynomial in t
    // of degree ≤ deg(f) in t.
    // From the structure theorem: ∫ f dx = q(x,t) + Σ c_i * log(v_i)
    // where q is rational in x and polynomial in t.

    // Simple case: f = a(x) * t^n
    // ∫ a(x) * log(u)^n dx — integration by parts
    // For n=1: ∫ a(x)*log(u) dx — try ansatz F = A(x)*t + B(x)
    //   F' = A'(x)*t + A(x)*t' + B'(x) = A'(x)*t + A(x)*u'/u + B'(x) = f
    //   Matching t coefficients: A'(x) = coefficient of t in f
    //   Constant term: A(x)*u'/u + B'(x) = constant term of f

    // General degree: F = Σ A_i(x)*t^i, solve A_i top-down
    // F' = Σ (A_i'*t^i + i*A_i*t'*t^(i-1))
    // Matching t^k: A_k' + (k+1)*A_{k+1}*t' = a_k
    if deg >= 1 {
        let log_expr = Expr::call("log", vec![log_arg.clone()]);
        let mut capital_a = vec![Expr::int(0); deg + 1];

        // Solve from highest degree down
        for k in (0..=deg).rev() {
            let a_k = if k < coeffs.len() { coeffs[k].clone() } else { Expr::int(0) };
            // rhs = a_k - (k+1)*A_{k+1}*t'
            let rhs = if k < deg {
                simplify(&Expr::sub(a_k, Expr::mul(
                    Expr::int((k + 1) as i64),
                    Expr::mul(capital_a[k + 1].clone(), t_deriv.clone()),
                )))
            } else {
                a_k
            };
            // A_k = ∫ rhs dx
            match try_rational_integrate(&rhs, var) {
                Some(ak) => { capital_a[k] = ak; }
                None => return None,
            }
        }

        // Build result: Σ A_i * log(u)^i
        let mut terms = Vec::new();
        for (i, ai) in capital_a.iter().enumerate() {
            if *ai != Expr::int(0) {
                if i == 0 {
                    terms.push(ai.clone());
                } else if i == 1 {
                    terms.push(simplify(&Expr::mul(ai.clone(), log_expr.clone())));
                } else {
                    terms.push(simplify(&Expr::mul(ai.clone(),
                        Expr::pow(log_expr.clone(), Expr::int(i as i64)))));
                }
            }
        }
        if terms.is_empty() { return Some(Expr::int(0)); }
        if terms.len() == 1 { return Some(terms.remove(0)); }
        return Some(simplify(&Expr::List {
            op: Operator::MPlus, simplified: false, args: terms }));
    }

    // For constant in t but involving 1/t: e.g., 1/(x*log(x)^2)
    // This is a rational function in t — the Hermite approach should handle it
    // For now, handle the special case f = c(x) * t^(-n)
    if coeffs.len() == 1 && !contains_var(&coeffs[0], t_var) {
        // f doesn't actually depend on t — shouldn't be here
        return None;
    }

    // Try to handle rational functions in t
    // f = P(t) / Q(t) where P, Q have coefficients in Q(x)
    // For 1/t: ∫ c(x)/log(u) dx — generally non-elementary
    // For 1/t^n: ∫ c(x)/log(u)^n — integrable when c is proportional to u'/u
    //   ∫ (u'/u) / log(u)^n dx = -1/((n-1)*log(u)^(n-1))

    // Check if f = c * t' * t^(-n) for some constant c and integer n ≥ 2
    if let Some((c, neg_n)) = match_t_prime_power(f, t_var, t_deriv) {
        if neg_n <= -2 {
            // ∫ c * t' * t^n dt = c * t^(n+1)/(n+1) ... but in terms of log:
            // ∫ c * (u'/u) * log(u)^n dx = c * log(u)^(n+1)/(n+1)
            let new_power = neg_n + 1;
            let log_expr = Expr::call("log", vec![log_arg.clone()]);
            return Some(simplify(&Expr::div(
                Expr::mul(c, Expr::pow(log_expr, Expr::int(new_power))),
                Expr::int(new_power),
            )));
        }
    }

    None
}

/// Integrate in Q(x, t) where t = exp(v(x)).
fn integrate_exponential(
    f: &Expr, var: &Expr, t_var: &Expr, tid: SymbolId,
    exp_arg: &Expr, t_deriv: &Expr, tower: &Tower,
) -> Option<Expr> {
    // t = exp(v), t' = v' * t
    // Express f as Laurent polynomial in t: f = Σ a_i(x) * t^i

    let coeffs = extract_laurent_coeffs(f, t_var);
    if coeffs.is_empty() { return None; }

    // For each term a_i(x) * t^i:
    // ∫ a_i(x) * exp(i*v(x)) dx
    // If i=0: standard rational integration
    // If i≠0: need Risch DE solver or reduction

    // Simple case: f = a(x) * t = a(x) * exp(v)
    // Ansatz: F = B(x) * exp(v)
    // F' = B'(x)*exp(v) + B(x)*v'*exp(v) = (B' + B*v')*exp(v)
    // Match: B' + B*v' = a(x) — this is the Risch differential equation

    let mut terms = Vec::new();
    let exp_expr = Expr::call("exp", vec![exp_arg.clone()]);

    for (power, coeff) in &coeffs {
        if *power == 0 {
            // Rational part: ∫ a_0(x) dx
            if let Some(int) = try_rational_integrate(coeff, var) {
                terms.push(int);
            } else {
                return None;
            }
        } else {
            // ∫ a_i(x) * exp(i*v(x)) dx
            // Risch DE: B' + i*v'*B = a_i
            let v_prime = crate::eval::diff_once_pub(exp_arg, var);
            let iv_prime = if *power == 1 { v_prime.clone() }
                else { simplify(&Expr::mul(Expr::int(*power), v_prime)) };

            if let Some(b) = solve_risch_de_simple(coeff, &iv_prime, var) {
                let exp_term = if *power == 1 { exp_expr.clone() }
                    else { Expr::pow(exp_expr.clone(), Expr::int(*power)) };
                terms.push(simplify(&Expr::mul(b, exp_term)));
            } else {
                return None;
            }
        }
    }

    if terms.is_empty() { return None; }
    if terms.len() == 1 { return Some(terms.remove(0)); }
    Some(simplify(&Expr::List { op: Operator::MPlus, simplified: false, args: terms }))
}

/// Solve the simple Risch differential equation: B' + f'*B = g
/// where B is a polynomial in x of bounded degree.
fn solve_risch_de_simple(g: &Expr, f_prime: &Expr, var: &Expr) -> Option<Expr> {
    // If f' is constant and g is polynomial: try polynomial B of degree ≤ deg(g)
    // B' + c*B = g
    // For constant c and g = a_n*x^n + ... + a_0:
    // B = b_n*x^n + ... + b_0 where:
    // n*b_n*x^{n-1} + ... + b_1 + c*(b_n*x^n + ... + b_0) = g
    // Matching x^n: c*b_n = a_n → b_n = a_n/c
    // Matching x^{n-1}: (n)*b_n + c*b_{n-1} = a_{n-1} → b_{n-1} = (a_{n-1} - n*b_n)/c

    if let Expr::Symbol(var_id) = var {
        let c = f_prime;
        if !contains_var(c, var) {
            if let Some(c_val) = crate::helpers::to_f64(c) {
                if c_val.abs() < 1e-15 { return None; }

                if let Some(g_poly) = maxima_poly::expr_to_poly(g, *var_id) {
                    let deg = g_poly.degree().unwrap_or(0);
                    let mut b_coeffs = vec![maxima_poly::Coeff::zero(); deg as usize + 1];

                    // Solve from highest degree down
                    for k in (0..=deg).rev() {
                        let g_k = g_poly.terms.iter()
                            .find(|(e, _)| *e == k)
                            .map(|(_, c)| c.clone())
                            .unwrap_or(maxima_poly::Coeff::zero());

                        // c*b_k + (k+1)*b_{k+1} = g_k (if k < deg)
                        let deriv_contrib = if (k as usize + 1) <= deg as usize {
                            b_coeffs[k as usize + 1].mul(&maxima_poly::Coeff::Int((k + 1) as i64))
                        } else {
                            maxima_poly::Coeff::zero()
                        };

                        let rhs = g_k.sub(&deriv_contrib);
                        let c_coeff = maxima_poly::Coeff::Int(c_val.round() as i64);
                        if c_coeff.is_zero() { return None; }
                        b_coeffs[k as usize] = rhs.div(&c_coeff)?;
                    }

                    let b_poly = maxima_poly::Poly {
                        var: *var_id,
                        terms: b_coeffs.into_iter().enumerate()
                            .filter(|(_, c)| !c.is_zero())
                            .map(|(e, c)| (e as u32, c))
                            .collect(),
                    };
                    return Some(maxima_poly::poly_to_expr(&b_poly));
                }
            }
        }
    }

    // For non-constant f': try B = constant
    // B' + f'*B = g → 0 + f'*B = g → B = g/f'
    let ratio = simplify(&Expr::div(g.clone(), f_prime.clone()));
    if !contains_var(&ratio, var) {
        return Some(ratio);
    }

    None
}

/// Try to integrate a rational function (no transcendentals).
fn try_rational_integrate(f: &Expr, var: &Expr) -> Option<Expr> {
    let result = crate::integrate::table_integrate_pub(f, var);
    if result.to_string().starts_with("integrate") {
        None
    } else {
        Some(result)
    }
}

/// Extract polynomial coefficients in t_var from expr.
/// Returns vec where index i gives coefficient of t^i.
fn extract_poly_coeffs(expr: &Expr, t_var: &Expr) -> Vec<Expr> {
    if !contains_var(expr, t_var) {
        return vec![expr.clone()];
    }

    // Simple structural extraction
    match expr {
        e if e == t_var => vec![Expr::int(0), Expr::int(1)],
        Expr::List { op: Operator::MTimes, args, .. } => {
            let (t_parts, other_parts): (Vec<&Expr>, Vec<&Expr>) =
                args.iter().partition(|a| contains_var(a, t_var));
            let coeff = if other_parts.is_empty() { Expr::int(1) }
                else if other_parts.len() == 1 { other_parts[0].clone() }
                else { simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: other_parts.into_iter().cloned().collect() }) };

            if t_parts.len() == 1 {
                if t_parts[0] == t_var {
                    return vec![Expr::int(0), coeff];
                }
                if let Expr::List { op: Operator::MExpt, args: pa, .. } = t_parts[0] {
                    if pa.len() == 2 && pa[0] == *t_var {
                        if let Some(n) = crate::helpers::to_i64(&pa[1]) {
                            if n >= 0 {
                                let mut result = vec![Expr::int(0); n as usize + 1];
                                result[n as usize] = coeff;
                                return result;
                            }
                        }
                    }
                }
            }
            vec![expr.clone()]
        }
        Expr::List { op: Operator::MPlus, args, .. } => {
            let mut max_deg = 0usize;
            let mut term_coeffs: Vec<(usize, Expr)> = Vec::new();
            for arg in args {
                let sub = extract_poly_coeffs(arg, t_var);
                let deg = sub.len() - 1;
                max_deg = max_deg.max(deg);
                for (i, c) in sub.into_iter().enumerate() {
                    if c != Expr::int(0) {
                        term_coeffs.push((i, c));
                    }
                }
            }
            let mut result = vec![Expr::int(0); max_deg + 1];
            for (i, c) in term_coeffs {
                if i < result.len() {
                    result[i] = simplify(&Expr::add(result[i].clone(), c));
                }
            }
            result
        }
        _ => vec![expr.clone()],
    }
}

/// Extract Laurent polynomial coefficients: terms of the form a_i * t^i (i can be negative).
fn extract_laurent_coeffs(expr: &Expr, t_var: &Expr) -> Vec<(i64, Expr)> {
    let coeffs = extract_poly_coeffs(expr, t_var);
    coeffs.into_iter().enumerate()
        .filter(|(_, c)| *c != Expr::int(0))
        .map(|(i, c)| (i as i64, c))
        .collect()
}

/// Check if f = c * t' * t^n for constant c and integer n.
fn match_t_prime_power(f: &Expr, t_var: &Expr, t_deriv: &Expr) -> Option<(Expr, i64)> {
    // Try: f / (t' * t^n) = constant for various n
    for n in [-2i64, -3, -4, -1, 1, 2] {
        let t_pow = if n == 1 { t_var.clone() } else { Expr::pow(t_var.clone(), Expr::int(n)) };
        let divisor = simplify(&Expr::mul(t_deriv.clone(), t_pow));
        if divisor == Expr::int(0) { continue; }
        let ratio = simplify(&Expr::div(f.clone(), divisor));
        if !contains_var(&ratio, t_var) && !contains_var(&ratio, &Expr::sym("x")) {
            return Some((ratio, n));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn risch_log_x_over_x() {
        // ∫ log(x)/x dx — already works via substitution, verify tower approach
        let x = Expr::sym("x");
        let f = Expr::div(Expr::call("log", vec![x.clone()]), x.clone());
        let result = risch_integrate(&f, &x);
        if let Some(r) = result {
            eprintln!("risch log(x)/x: {}", r);
        }
    }

    #[test]
    fn extract_coeffs_linear() {
        let t = Expr::sym("t");
        let x = Expr::sym("x");
        // f = x*t + 3
        let f = Expr::add(Expr::mul(x.clone(), t.clone()), Expr::int(3));
        let coeffs = extract_poly_coeffs(&f, &t);
        assert_eq!(coeffs.len(), 2);
    }
}
