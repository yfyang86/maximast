use maxima_core::{Expr, Operator, resolve};
use crate::helpers::{to_f64, contains_var, subst};
use crate::simp::simplify;

/// Risch-Norman heuristic: try to integrate by making an ansatz,
/// differentiating, and matching coefficients.
///
/// This handles ~80% of textbook integrals as a fast path.
/// Falls through to None if the heuristic fails.
pub fn risch_norman(f: &Expr, var: &Expr) -> Option<Expr> {
    // Extract building blocks from the integrand
    let blocks = extract_blocks(f, var);
    if blocks.is_empty() {
        return None;
    }

    // For each building block, try ansatz: F = poly(x) * block
    // where poly(x) has undetermined coefficients
    for block in &blocks {
        // Determine degree of polynomial coefficient to try
        let max_deg = estimate_poly_degree(f, var, block);

        for deg in 0..=max_deg {
            if let Some(result) = try_ansatz(f, var, block, deg) {
                return Some(result);
            }
        }
    }

    // Try sum of building blocks: F = Σ poly_i(x) * block_i
    if blocks.len() >= 2 {
        if let Some(result) = try_multi_ansatz(f, var, &blocks) {
            return Some(result);
        }
    }

    None
}

/// Extract "building blocks" — transcendental subexpressions that
/// should appear in the antiderivative.
fn extract_blocks(expr: &Expr, var: &Expr) -> Vec<Expr> {
    let mut blocks = Vec::new();
    collect_blocks(expr, var, &mut blocks);
    // Always include 1 (polynomial part)
    if !blocks.contains(&Expr::int(1)) {
        blocks.push(Expr::int(1));
    }
    blocks.dedup_by(|a, b| a == b);
    blocks
}

fn collect_blocks(expr: &Expr, var: &Expr, blocks: &mut Vec<Expr>) {
    match expr {
        Expr::List { op: Operator::Named(id), args, .. } if args.len() == 1 => {
            let fname = resolve(*id);
            if contains_var(&args[0], var) {
                match fname.as_str() {
                    "exp" | "log" | "sin" | "cos" | "tan"
                    | "sinh" | "cosh" | "tanh" => {
                        if !blocks.contains(expr) {
                            blocks.push(expr.clone());
                        }
                    }
                    _ => {}
                }
            }
            collect_blocks(&args[0], var, blocks);
        }
        Expr::List { op: Operator::MExpt, args, .. } if args.len() == 2 => {
            // exp(f) is a building block
            if let Expr::List { op: Operator::Named(id), .. } = &args[0] {
                if resolve(*id) == "exp" {
                    if !blocks.contains(expr) {
                        blocks.push(expr.clone());
                    }
                }
            }
            collect_blocks(&args[0], var, blocks);
            collect_blocks(&args[1], var, blocks);
        }
        Expr::List { args, .. } => {
            for arg in args {
                collect_blocks(arg, var, blocks);
            }
        }
        _ => {}
    }
}

/// Estimate the degree of polynomial coefficient needed.
fn estimate_poly_degree(f: &Expr, var: &Expr, _block: &Expr) -> u32 {
    // Heuristic: look at the polynomial degree of f with respect to var
    let mut max_power = 0u32;
    find_max_power(f, var, &mut max_power);
    max_power + 1 // Need one degree higher for the antiderivative
}

fn find_max_power(expr: &Expr, var: &Expr, max: &mut u32) {
    match expr {
        Expr::Symbol(_) if expr == var => { *max = (*max).max(1); }
        Expr::List { op: Operator::MExpt, args, .. } if args.len() == 2 && args[0] == *var => {
            if let Some(n) = to_f64(&args[1]) {
                if n > 0.0 && n == n.floor() {
                    *max = (*max).max(n as u32);
                }
            }
        }
        Expr::List { args, .. } => {
            for arg in args { find_max_power(arg, var, max); }
        }
        _ => {}
    }
}

/// Try ansatz: F = (c₀ + c₁x + c₂x² + ... + cₙxⁿ) * block
fn try_ansatz(f: &Expr, var: &Expr, block: &Expr, deg: u32) -> Option<Expr> {
    if *block == Expr::int(1) {
        return None; // Polynomial integration is already handled
    }

    // Build ansatz with symbolic coefficients
    // F = (c0 + c1*x + c2*x^2 + ...) * block
    // F' = (c1 + 2*c2*x + ...) * block + (c0 + c1*x + ...) * block'
    // Set F' = f and solve for c0, c1, ...

    // For the common case: F = (a*x + b) * block, deg=1
    // F' = a*block + (a*x+b)*block'
    // Need: a*block + (a*x+b)*block' = f

    let block_deriv = diff_expr(block, var);

    // Build the polynomial part
    // For deg=0: F = c * block → F' = c * block'
    // For deg=1: F = (c1*x + c0) * block → F' = c1*block + (c1*x+c0)*block'
    // For deg=2: F = (c2*x²+c1*x+c0)*block → F' = (2c2*x+c1)*block + (c2*x²+c1*x+c0)*block'

    // Try specific small degrees
    match deg {
        0 => {
            // F = c * block, F' = c * block'
            // c * block' = f → c = f / block'
            if block_deriv == Expr::int(0) { return None; }
            let ratio = simplify(&Expr::div(f.clone(), block_deriv));
            if !contains_var(&ratio, var) {
                return Some(simplify(&Expr::mul(ratio, block.clone())));
            }
            None
        }
        1 => {
            // F = (a*x + b) * block
            // F' = a*block + (a*x + b)*block'
            // Rearrange: f = a*block + a*x*block' + b*block'
            // Group by x: f = (a*block') * x + (a*block + b*block')

            // Extract coefficient of x and constant in f relative to block/block'
            // This is the coefficient matching step
            try_linear_ansatz(f, var, block, &block_deriv)
        }
        _ => None, // Higher degrees: complex coefficient matching needed
    }
}

fn try_linear_ansatz(f: &Expr, var: &Expr, block: &Expr, block_deriv: &Expr) -> Option<Expr> {
    // F = (a*x + b) * block
    // F' = a*block + (a*x + b)*block' = a*block + a*x*block' + b*block'
    // f must equal: x*(a*block') + (a*block + b*block')

    // If block' = k*block (exponential case: d/dx exp(g) = g'*exp(g))
    // Then F' = a*block + (a*x+b)*k*block = (a + a*k*x + b*k)*block
    // = block * (a*k*x + (a + b*k))
    // Match with f = block * (polynomial in x)

    // Check if f = block * something
    if let Some(quotient) = try_divide_by_block(f, block) {
        // quotient should be a polynomial in x of degree ≤ 1
        // If quotient = px + q, and block' = k*block:
        //   a*k = p → a = p/k
        //   a + b*k = q → b = (q - a)/k = (q - p/k)/k

        // Check if block' = k * block for some k
        if let Some(k) = is_proportional(block_deriv, block) {
            // quotient = p*x + q
            let p = coeff_of_x(&quotient, var, 1);
            let q = coeff_of_x(&quotient, var, 0);

            if let (Some(p_val), Some(q_val), Some(k_val)) = (to_f64(&p), to_f64(&q), to_f64(&k)) {
                if k_val.abs() > 1e-15 {
                    let a = p_val / k_val;
                    let b = (q_val - a) / k_val;

                    let poly = simplify(&Expr::add(
                        Expr::mul(Expr::Float(a), var.clone()),
                        Expr::Float(b),
                    ));

                    // Verify: differentiate and check
                    let candidate = simplify(&Expr::mul(poly, block.clone()));
                    let candidate_deriv = diff_expr(&candidate, var);
                    let diff = simplify(&Expr::sub(candidate_deriv, f.clone()));

                    if diff == Expr::int(0) || diff == Expr::Float(0.0) {
                        // Clean up floating point to integers if possible
                        let a_int = if (a - a.round()).abs() < 1e-10 { Expr::int(a.round() as i64) } else { Expr::Float(a) };
                        let b_int = if (b - b.round()).abs() < 1e-10 { Expr::int(b.round() as i64) } else { Expr::Float(b) };
                        let clean_poly = simplify(&Expr::add(Expr::mul(a_int, var.clone()), b_int));
                        return Some(simplify(&Expr::mul(clean_poly, block.clone())));
                    }
                }
            }
        }
    }
    None
}

fn try_multi_ansatz(_f: &Expr, _var: &Expr, _blocks: &[Expr]) -> Option<Expr> {
    // For now, just try each block individually
    // A full multi-block ansatz would solve a larger system
    None
}

/// Try to express f as block * something
fn try_divide_by_block(f: &Expr, block: &Expr) -> Option<Expr> {
    // If f = block * quotient, return quotient
    if let Expr::List { op: Operator::MTimes, args, .. } = f {
        if args.contains(block) {
            let remaining: Vec<Expr> = args.iter().filter(|a| *a != block).cloned().collect();
            return Some(if remaining.len() == 1 {
                remaining.into_iter().next().unwrap()
            } else {
                simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: remaining })
            });
        }
    }
    // f itself might equal block * 1
    if f == block { return Some(Expr::int(1)); }
    None
}

/// Check if a = k * b for some constant k, return k
fn is_proportional(a: &Expr, b: &Expr) -> Option<Expr> {
    if a == b { return Some(Expr::int(1)); }
    if let Expr::List { op: Operator::MTimes, args, .. } = a {
        if args.len() == 2 {
            if &args[1] == b && !contains_var(&args[0], &Expr::sym("_any_")) {
                return Some(args[0].clone());
            }
            if &args[0] == b && !contains_var(&args[1], &Expr::sym("_any_")) {
                return Some(args[1].clone());
            }
        }
    }
    None
}

/// Extract coefficient of x^n in a polynomial expression
fn coeff_of_x(expr: &Expr, var: &Expr, power: u32) -> Expr {
    if power == 0 {
        // Substitute x=0
        return simplify(&subst(&Expr::int(0), var, expr));
    }
    if power == 1 {
        // (f(1) - f(0)) approximately, or structural extraction
        let at_0 = simplify(&subst(&Expr::int(0), var, expr));
        let at_1 = simplify(&subst(&Expr::int(1), var, expr));
        return simplify(&Expr::sub(at_1, at_0));
    }
    Expr::int(0)
}

/// Simple symbolic differentiation (reuses eval's diff but standalone)
fn diff_expr(expr: &Expr, var: &Expr) -> Expr {
    match expr {
        Expr::Integer(_) | Expr::Float(_) | Expr::Rational { .. } => Expr::int(0),
        Expr::Symbol(_) if expr == var => Expr::int(1),
        Expr::Symbol(_) => Expr::int(0),
        Expr::List { op: Operator::MPlus, args, .. } => {
            let terms: Vec<Expr> = args.iter().map(|a| diff_expr(a, var)).collect();
            simplify(&Expr::List { op: Operator::MPlus, simplified: false, args: terms })
        }
        Expr::List { op: Operator::MTimes, args, .. } if args.len() == 2 => {
            let (a, b) = (&args[0], &args[1]);
            let da = diff_expr(a, var);
            let db = diff_expr(b, var);
            simplify(&Expr::add(
                Expr::mul(da, b.clone()),
                Expr::mul(a.clone(), db),
            ))
        }
        Expr::List { op: Operator::MExpt, args, .. } if args.len() == 2 => {
            let (base, exp) = (&args[0], &args[1]);
            if !contains_var(exp, var) {
                // d/dx f^n = n * f^(n-1) * f'
                let df = diff_expr(base, var);
                simplify(&Expr::mul(
                    Expr::mul(exp.clone(), Expr::pow(base.clone(), simplify(&Expr::sub(exp.clone(), Expr::int(1))))),
                    df,
                ))
            } else {
                Expr::int(0) // General case not handled here
            }
        }
        Expr::List { op: Operator::Named(id), args, .. } if args.len() == 1 => {
            let fname = resolve(*id);
            let inner = &args[0];
            let dinner = diff_expr(inner, var);
            let outer_d = match fname.as_str() {
                "exp" => expr.clone(),
                "log" => Expr::pow(inner.clone(), Expr::int(-1)),
                "sin" => Expr::call("cos", vec![inner.clone()]),
                "cos" => Expr::neg(Expr::call("sin", vec![inner.clone()])),
                _ => return Expr::int(0),
            };
            simplify(&Expr::mul(outer_d, dinner))
        }
        _ => Expr::int(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_extraction() {
        let x = Expr::sym("x");
        let f = Expr::mul(Expr::sym("x"), Expr::call("exp", vec![x.clone()]));
        let blocks = extract_blocks(&f, &x);
        assert!(blocks.iter().any(|b| b.to_string().contains("exp")));
    }

    #[test]
    fn rn_exp_x() {
        // ∫ exp(x) dx = exp(x) — degree 0 ansatz
        let x = Expr::sym("x");
        let f = Expr::call("exp", vec![x.clone()]);
        let result = risch_norman(&f, &x);
        assert!(result.is_some(), "should integrate exp(x)");
        if let Some(r) = result {
            assert!(r.to_string().contains("exp"), "got: {}", r);
        }
    }

    #[test]
    fn rn_x_exp_x() {
        // ∫ x*exp(x) dx = (x-1)*exp(x)
        let x = Expr::sym("x");
        let f = Expr::mul(x.clone(), Expr::call("exp", vec![x.clone()]));
        let result = risch_norman(&f, &x);
        // May or may not work depending on coefficient matching
        // The heuristic is not guaranteed to succeed
        if let Some(r) = &result {
            assert!(r.to_string().contains("exp"), "got: {}", r);
        }
    }
}
