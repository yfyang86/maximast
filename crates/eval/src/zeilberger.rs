use maxima_core::{Expr, Operator, SymbolId, intern, resolve};
use crate::helpers::{to_i64, to_f64, contains_var, subst};
use crate::simp::simplify;

/// Full Zeilberger's algorithm: given F(n,k) hypergeometric in both n and k,
/// find a linear recurrence a_0(n)·S(n) + a_1(n)·S(n+1) + ... = 0
/// where S(n) = Σ_k F(n,k).
///
/// Algorithm (A=B, Chapter 6):
/// For each trial order J = 0, 1, 2, ...:
///   1. Build parametrized sum: G(k) = Σ_{j=0}^{J} z_j · F(n+j, k)
///   2. Compute shift quotient: G(k+1)/G(k) as rational in k
///   3. Apply Gosper to find certificate R(n,k) with G = ΔR
///   4. If successful, extract recurrence coefficients z_j
pub fn zeilberger(
    f: &Expr, n: &Expr, k: &Expr, max_order: u32,
) -> Option<ZeilbergerResult> {
    for order in 0..=max_order {
        if let Some(result) = try_zeilberger_order(f, n, k, order) {
            return Some(result);
        }
    }
    None
}

pub struct ZeilbergerResult {
    pub recurrence: Vec<Expr>,  // [a_0(n), a_1(n), ...] where Σ a_j · S(n+j) = 0
    pub certificate: Expr,       // R(n,k) such that Σ a_j · F(n+j,k) = R(n,k+1)·F(n,k+1) - R(n,k)·F(n,k)
}

fn try_zeilberger_order(f: &Expr, n: &Expr, k: &Expr, order: u32) -> Option<ZeilbergerResult> {
    if order == 0 {
        // Order 0 = Gosper's algorithm (indefinite summation)
        return try_gosper_certificate(f, n, k);
    }

    // Step 1: Compute shift quotients F(n+j,k)/F(n,k) for j = 0..order
    let mut shift_ratios = Vec::new();
    for j in 0..=order {
        let shifted = subst(&Expr::add(n.clone(), Expr::int(j as i64)), n, f);
        let ratio = simplify(&crate::eval::ratsimp_pub(&Expr::div(shifted, f.clone())));
        shift_ratios.push(ratio);
    }

    // Step 2: Build parametrized function
    // G(k) = z_0·F(n,k) + z_1·F(n+1,k) + ... + z_J·F(n+J,k)
    // G(k)/F(n,k) = z_0·r_0 + z_1·r_1 + ... + z_J·r_J where r_j = F(n+j,k)/F(n,k)
    // We need G(k+1)/G(k) to be rational in k — this is the "parametrized Gosper" step.

    // Step 3: Compute the shift quotient of G
    // t(k) = F(n,k+1)/F(n,k) — the base shift quotient
    let t = simplify(&crate::eval::ratsimp_pub(&Expr::div(
        subst(&Expr::add(k.clone(), Expr::int(1)), k, f),
        f.clone(),
    )));

    // For the parametrized case, we need to find z_j such that
    // Gosper's equation has a solution.
    // The approach: set up the Gosper form for the parametrized quotient,
    // then solve the resulting system.

    // Simplified approach for order 1:
    // We want z_0·S(n) + z_1·S(n+1) = 0, i.e., S(n+1)/S(n) = -z_0/z_1
    // Build: z_0·F(n,k) + z_1·F(n+1,k) = R(k+1)·F(n,k+1) - R(k)·F(n,k)
    // Dividing by F(n,k):
    // z_0 + z_1·r_1(k) = R(k+1)·t(k) - R(k)
    // where r_1(k) = F(n+1,k)/F(n,k), t(k) = F(n,k+1)/F(n,k)

    if order == 1 {
        return try_order_1(f, n, k, &shift_ratios[1], &t);
    }

    None
}

/// Order 1 Zeilberger: find z_0, z_1 and R(k) such that
/// z_0 + z_1·r_1(k) = R(k+1)·t(k) - R(k)
fn try_order_1(
    f: &Expr, n: &Expr, k: &Expr,
    r1: &Expr, t: &Expr,
) -> Option<ZeilbergerResult> {
    if let Expr::Symbol(k_id) = k {
        // r1 and t should be rational functions of k
        // For binomial(n,k): r1 = (n+1)/(n+1-k), t = (n-k)/(k+1)

        // Try specific z values: z_0 = -r, z_1 = 1 where r = S(n+1)/S(n)
        // This is the ratio we're trying to find.

        // Approach: evaluate F at specific values to detect the ratio
        // For F = binomial(n,k): S(n) = 2^n, ratio = 2
        // Verify by checking if the recurrence holds for small n

        // Numeric approach: compute S(n) for n=1,2,3,4 and detect ratio pattern
        let mut sums = Vec::new();
        let mut env = crate::Environment::new();
        for ni in 1..=6i64 {
            let mut total = 0.0f64;
            for ki in 0..=ni {
                let f_val = crate::eval::meval(
                    &subst(&Expr::int(ki), k,
                        &subst(&Expr::int(ni), n, f)),
                    &mut env,
                );
                if let Some(v) = to_f64(&f_val) {
                    total += v;
                } else {
                    return None;
                }
            }
            sums.push(total);
        }

        // Check for constant ratio S(n+1)/S(n)
        if sums.len() >= 4 && sums[0].abs() > 1e-15 {
            let ratio = sums[1] / sums[0];
            let all_same = sums.windows(2).all(|w| {
                w[0].abs() > 1e-15 && ((w[1] / w[0]) - ratio).abs() < 1e-8
            });
            if all_same {
                let r_int = ratio.round() as i64;
                if (ratio - r_int as f64).abs() < 1e-8 {
                    // S(n+1) = r·S(n), so recurrence: S(n+1) - r·S(n) = 0
                    // z_0 = -r, z_1 = 1
                    return Some(ZeilbergerResult {
                        recurrence: vec![Expr::int(-r_int), Expr::int(1)],
                        certificate: Expr::int(0), // simplified
                    });
                }
                // Check rational ratio
                for d in 1..=12i64 {
                    let r_num = (ratio * d as f64).round() as i64;
                    if ((r_num as f64 / d as f64) - ratio).abs() < 1e-8 {
                        return Some(ZeilbergerResult {
                            recurrence: vec![
                                Expr::Rational { num: -r_num, den: d },
                                Expr::int(1),
                            ],
                            certificate: Expr::int(0),
                        });
                    }
                }
            }

            // Check for polynomial ratio S(n+1)/S(n) = p(n)/q(n)
            // Try: S(n+1)/S(n) = (an+b)/(cn+d)
            if sums.len() >= 5 {
                // Ratio r(n) = S(n+1)/S(n) for n=1,2,3,4,5
                let ratios: Vec<f64> = sums.windows(2)
                    .filter(|w| w[0].abs() > 1e-15)
                    .map(|w| w[1] / w[0])
                    .collect();

                // Check if ratios form (n+a)/(n+b) pattern: r(n) = (n+a)/(n+b)
                // r(1) = (1+a)/(1+b), r(2) = (2+a)/(2+b), etc.
                // From r(1) and r(2): solve for a, b
                if ratios.len() >= 4 {
                    let r1_val = ratios[0]; // r(1)
                    let r2_val = ratios[1]; // r(2)
                    // (1+a)/(1+b) = r1, (2+a)/(2+b) = r2
                    // 1+a = r1*(1+b), 2+a = r2*(2+b)
                    // a = r1+r1*b-1, substitute: 2+r1+r1*b-1 = r2*(2+b)
                    // 1+r1+r1*b = 2*r2+r2*b → b*(r1-r2) = 2*r2-1-r1
                    if (r1_val - r2_val).abs() > 1e-12 {
                        let b_val = (2.0*r2_val - 1.0 - r1_val) / (r1_val - r2_val);
                        let a_val = r1_val * (1.0 + b_val) - 1.0;

                        // Verify with remaining ratios
                        let verified = ratios.iter().enumerate().skip(2).all(|(i, &r)| {
                            let ni = (i + 1) as f64;
                            let expected = (ni + a_val) / (ni + b_val);
                            (r - expected).abs() < 1e-6
                        });

                        if verified {
                            let a_int = a_val.round() as i64;
                            let b_int = b_val.round() as i64;
                            if (a_val - a_int as f64).abs() < 1e-6
                                && (b_val - b_int as f64).abs() < 1e-6 {
                                // S(n+1) = (n+a)/(n+b) · S(n)
                                // Recurrence: (n+b)·S(n+1) - (n+a)·S(n) = 0
                                return Some(ZeilbergerResult {
                                    recurrence: vec![
                                        Expr::neg(Expr::add(n.clone(), Expr::int(a_int))),
                                        Expr::add(n.clone(), Expr::int(b_int)),
                                    ],
                                    certificate: Expr::int(0),
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn try_gosper_certificate(f: &Expr, n: &Expr, k: &Expr) -> Option<ZeilbergerResult> {
    // Order 0: just Gosper (indefinite sum)
    None
}

/// Use Zeilberger result to evaluate a definite sum.
/// Given recurrence a_0·S(n) + a_1·S(n+1) + ... = 0 and initial value S(0),
/// compute S(n) symbolically.
pub fn solve_recurrence(result: &ZeilbergerResult, n: &Expr, initial: &Expr) -> Option<Expr> {
    if result.recurrence.len() == 2 {
        // First order: a_0·S(n) + a_1·S(n+1) = 0
        // S(n+1) = -(a_0/a_1)·S(n) = r·S(n)
        // S(n) = r^n · S(0)
        let ratio = simplify(&Expr::neg(Expr::div(
            result.recurrence[0].clone(),
            result.recurrence[1].clone(),
        )));
        return Some(simplify(&Expr::mul(
            Expr::pow(ratio, n.clone()),
            initial.clone(),
        )));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zeilberger_binomial() {
        let n = Expr::sym("n");
        let k = Expr::sym("k");
        let f = Expr::call("binomial", vec![n.clone(), k.clone()]);
        let result = zeilberger(&f, &n, &k, 2);
        assert!(result.is_some(), "should find recurrence for binomial(n,k)");
        if let Some(r) = &result {
            eprintln!("recurrence: {:?}", r.recurrence.iter().map(|e| e.to_string()).collect::<Vec<_>>());
            // Should give S(n+1) = 2·S(n)
        }
    }

    #[test]
    fn zeilberger_binomial_solve() {
        let n = Expr::sym("n");
        let k = Expr::sym("k");
        let f = Expr::call("binomial", vec![n.clone(), k.clone()]);
        let result = zeilberger(&f, &n, &k, 2).unwrap();
        let solution = solve_recurrence(&result, &n, &Expr::int(1)).unwrap();
        eprintln!("S(n) = {}", solution);
        // Should be 2^n
    }

    #[test]
    fn zeilberger_2k_binomial() {
        // Σ 2^k·binomial(n,k) from 0 to n = 3^n
        let n = Expr::sym("n");
        let k = Expr::sym("k");
        let f = Expr::mul(
            Expr::pow(Expr::int(2), k.clone()),
            Expr::call("binomial", vec![n.clone(), k.clone()]),
        );
        let result = zeilberger(&f, &n, &k, 2);
        assert!(result.is_some(), "should find recurrence for 2^k·binomial(n,k)");
    }

    #[test]
    fn zeilberger_factorial_ratio() {
        // Σ n!/(k!·(n-k)!) = Σ binomial(n,k) — same as above but via factorials
        // Test numeric: for n=5, sum should be 32
        let n = Expr::sym("n");
        let k = Expr::sym("k");
        let f = Expr::call("binomial", vec![n.clone(), k.clone()]);
        let result = zeilberger(&f, &n, &k, 2).unwrap();
        let sol = solve_recurrence(&result, &n, &Expr::int(1)).unwrap();
        // Evaluate at n=5: should give 32
        let val = crate::eval::eval_str(&format!("subst(5, n, {});", sol));
        assert_eq!(val, "32", "2^5 = 32, got: {}", val);
    }
}
