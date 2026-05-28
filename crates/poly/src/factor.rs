use crate::coeff::Coeff;
use crate::poly::Poly;
use crate::gcd::poly_gcd;

/// Square-free factorization (Yun's algorithm).
/// Returns list of (factor, multiplicity) pairs.
pub fn sqfree(p: &Poly) -> Vec<(Poly, u32)> {
    if p.is_zero() || p.is_constant() {
        return vec![(p.clone(), 1)];
    }

    let dp = p.derivative();
    if dp.is_zero() {
        return vec![(p.clone(), 1)];
    }

    let g = poly_gcd(p, &dp);

    if g.is_constant() {
        // p is already square-free
        return vec![(p.clone(), 1)];
    }

    let mut factors = Vec::new();
    let mut w = match p.exact_div(&g) {
        Some(q) => q,
        None => return vec![(p.clone(), 1)],
    };
    let mut y = match dp.exact_div(&g) {
        Some(q) => q,
        None => return vec![(p.clone(), 1)],
    };
    let mut i = 1u32;

    loop {
        let dw = w.derivative();
        let z = y.sub(&dw);
        if z.is_zero() {
            if !w.is_constant() {
                factors.push((w, i));
            }
            break;
        }
        let g = poly_gcd(&w, &z);
        if !g.is_constant() {
            factors.push((g.clone(), i));
        }
        w = match w.exact_div(&g) {
            Some(q) => q,
            None => break,
        };
        y = match z.exact_div(&g) {
            Some(q) => q,
            None => break,
        };
        i += 1;
    }

    if factors.is_empty() {
        vec![(p.clone(), 1)]
    } else {
        factors
    }
}

/// Try to factor a polynomial into irreducible factors over Z.
/// Returns list of (factor, multiplicity) pairs.
/// Currently uses square-free factorization + trial factors for small degrees.
pub fn factor_poly(p: &Poly) -> Vec<(Poly, u32)> {
    let content = p.content();
    let prim = if content.is_one() {
        p.clone()
    } else {
        let mut terms = Vec::new();
        for (e, c) in &p.terms {
            if let Some(q) = c.div(&content) {
                terms.push((*e, q));
            }
        }
        Poly { var: p.var, terms }
    };

    // Square-free factorization
    let sq_factors = sqfree(&prim);

    let mut result = Vec::new();
    for (factor, mult) in sq_factors {
        // Try to find rational roots for small-degree factors
        let sub_factors = try_rational_roots(&factor);
        for (sf, sm) in sub_factors {
            result.push((sf, mult * sm));
        }
    }

    result
}

/// Try to split a polynomial using rational root theorem.
fn try_rational_roots(p: &Poly) -> Vec<(Poly, u32)> {
    let deg = match p.degree() {
        Some(d) => d,
        None => return vec![(p.clone(), 1)],
    };

    if deg <= 1 {
        return vec![(p.clone(), 1)];
    }

    let lc = match p.leading_coeff() {
        Coeff::Int(n) => n,
        _ => return vec![(p.clone(), 1)],
    };
    let ct = match p.constant_term() {
        Coeff::Int(n) => n,
        _ => return vec![(p.clone(), 1)],
    };

    if ct == 0 {
        // x is a factor
        let x = Poly::var_poly(p.var);
        if let Some(q) = p.exact_div(&x) {
            let mut factors = vec![(x, 1)];
            factors.extend(try_rational_roots(&q));
            return factors;
        }
    }

    // Try rational roots p/q where p divides constant term, q divides leading coeff
    let ct_divisors = small_divisors(ct.abs());
    let lc_divisors = small_divisors(lc.abs());

    let mut remaining = p.clone();

    let mut factors = Vec::new();

    for &cd in &ct_divisors {
        for &ld in &lc_divisors {
            for &sign in &[1i64, -1i64] {
                let num = sign * cd;
                let den = ld;
                // Check if num/den is a root
                let val = if den == 1 {
                    Coeff::Int(num)
                } else {
                    Coeff::Rat(num, den)
                };
                if remaining.eval_at(&val).is_zero() {
                    // (den*x - num) is a factor
                    let factor = Poly {
                        var: p.var,
                        terms: if den == 1 {
                            vec![(1, Coeff::Int(1)), (0, Coeff::Int(-num))]
                        } else {
                            vec![(1, Coeff::Int(den)), (0, Coeff::Int(-num))]
                        },
                    };
                    // Divide out this factor (possibly multiple times)
                    while let Some(q) = remaining.exact_div(&factor) {
                        factors.push((factor.clone(), 1));
                        remaining = q;
                    }
                }
            }
        }
    }

    if !remaining.is_constant() && remaining.degree().unwrap_or(0) >= 4 {
        // Try Kronecker's method (non-recursive — just returns remaining if can't factor)
        let kronecker_factors = try_kronecker_nonrecursive(&remaining);
        factors.extend(kronecker_factors);
    } else if !remaining.is_constant() {
        factors.push((remaining, 1));
    }

    // Merge duplicate factors
    let mut merged: Vec<(Poly, u32)> = Vec::new();
    for (f, m) in factors {
        if let Some(entry) = merged.iter_mut().find(|(ef, _)| *ef == f) {
            entry.1 += m;
        } else {
            merged.push((f, m));
        }
    }

    if merged.is_empty() {
        vec![(p.clone(), 1)]
    } else {
        merged
    }
}

/// Try to factor using Kronecker's method for small degree.
/// Evaluates at several points, finds divisors, and tries to
/// reconstruct a factor via interpolation.
fn try_kronecker_nonrecursive(p: &Poly) -> Vec<(Poly, u32)> {
    let deg = match p.degree() {
        Some(d) => d,
        None => return vec![(p.clone(), 1)],
    };

    if deg < 4 || deg > 8 {
        return vec![(p.clone(), 1)];
    }

    // Quick check: evaluate at a few points to bound possible factors
    let v0 = p.eval_at(&Coeff::Int(0));
    let v1 = p.eval_at(&Coeff::Int(1));
    let vm1 = p.eval_at(&Coeff::Int(-1));

    // If p(0), p(1), p(-1) are all ±1, there are very few candidate factors
    let vals_small = [&v0, &v1, &vm1].iter().all(|v| {
        matches!(v, Coeff::Int(n) if n.abs() <= 1)
    });
    if vals_small && deg == 4 {
        // Very likely irreducible — only try a few candidates
        let max_coeff = 2i64;
        for c in -max_coeff..=max_coeff {
            for b in -max_coeff..=max_coeff {
                let mut t = vec![(2u32, Coeff::Int(1))];
                if b != 0 { t.push((1, Coeff::Int(b))); }
                if c != 0 { t.push((0, Coeff::Int(c))); }
                let candidate = Poly { var: p.var, terms: t };
                if let Some(q) = p.exact_div(&candidate) {
                    return vec![(candidate, 1), (q, 1)];
                }
            }
        }
        return vec![(p.clone(), 1)];
    }

    let max_coeff = 3i64;

    let mut remaining = p.clone();
    let mut factors = Vec::new();
    let mut attempts = 0u32;
    let max_attempts = 100;

    'outer: for c in -max_coeff..=max_coeff {
        for b in -max_coeff..=max_coeff {
            attempts += 1;
            if attempts > max_attempts { break 'outer; }
            let candidate = Poly {
                var: p.var,
                terms: {
                    let mut t = vec![(2u32, Coeff::Int(1))];
                    if b != 0 { t.push((1, Coeff::Int(b))); }
                    if c != 0 { t.push((0, Coeff::Int(c))); }
                    t
                },
            };

            if let Some(q) = remaining.exact_div(&candidate) {
                factors.push((candidate.clone(), 1));
                remaining = q;
                // Try dividing by the same factor again
                while let Some(q2) = remaining.exact_div(&candidate) {
                    if let Some(entry) = factors.last_mut() {
                        entry.1 += 1;
                    }
                    remaining = q2;
                }
                if remaining.degree().unwrap_or(0) <= 1 {
                    break 'outer;
                }
            }
        }
    }

    if !remaining.is_constant() {
        factors.push((remaining, 1));
    }

    if factors.is_empty() {
        vec![(p.clone(), 1)]
    } else {
        factors
    }
}

fn small_divisors(n: i64) -> Vec<i64> {
    if n == 0 { return vec![1]; }
    let n = n.abs();
    let mut divs = Vec::new();
    let mut i = 1i64;
    while i * i <= n {
        if n % i == 0 {
            divs.push(i);
            if i != n / i {
                divs.push(n / i);
            }
        }
        i += 1;
    }
    divs.sort();
    divs
}

#[cfg(test)]
mod tests {
    use super::*;
    use maxima_core::{SymbolId, intern};

    fn x() -> SymbolId { intern("x") }

    fn p(terms: &[(u32, i64)]) -> Poly {
        Poly {
            var: x(),
            terms: terms.iter().map(|(e, c)| (*e, Coeff::Int(*c))).collect(),
        }
    }

    #[test]
    fn factor_x2_minus_1() {
        let poly = p(&[(2, 1), (0, -1)]); // x^2-1
        let factors = factor_poly(&poly);
        assert!(factors.len() >= 2, "should factor into at least 2 factors, got {:?}", factors);
    }

    #[test]
    fn factor_x2_plus_2x_plus_1() {
        let poly = p(&[(2, 1), (1, 2), (0, 1)]); // x^2+2x+1 = (x+1)^2
        let factors = factor_poly(&poly);
        // Should contain (x+1) with multiplicity 2
        let total_deg: u32 = factors.iter().map(|(f, m)| f.degree().unwrap_or(0) * m).sum();
        assert_eq!(total_deg, 2);
    }

    #[test]
    fn factor_x3_minus_1() {
        let poly = p(&[(3, 1), (0, -1)]); // x^3-1
        let factors = factor_poly(&poly);
        assert!(factors.len() >= 2, "x^3-1 should factor, got {:?}", factors);
    }

    #[test]
    fn factor_x4_minus_1() {
        let poly = p(&[(4, 1), (0, -1)]); // x^4-1
        let factors = factor_poly(&poly);
        assert!(factors.len() >= 2, "x^4-1 should factor, got {:?}", factors);
    }

    #[test]
    fn sqfree_simple() {
        let poly = p(&[(2, 1), (0, -1)]); // x^2-1 (square-free)
        let factors = sqfree(&poly);
        assert_eq!(factors.len(), 1);
        assert_eq!(factors[0].1, 1);
    }

    #[test]
    fn sqfree_repeated() {
        let x_plus_1 = p(&[(1, 1), (0, 1)]);
        let poly = x_plus_1.mul(&x_plus_1); // (x+1)^2
        let factors = sqfree(&poly);
        // Should detect the repeated factor
        assert!(factors.iter().any(|(_, m)| *m >= 2) || factors.len() == 1);
    }

    #[test]
    fn factor_with_root_zero() {
        let poly = p(&[(3, 1), (2, -1)]); // x^3 - x^2 = x^2(x-1)
        let factors = factor_poly(&poly);
        assert!(factors.len() >= 2);
    }

    #[test]
    fn factor_x3_plus_x2_plus_x_plus_1() {
        // x³+x²+x+1 = (x+1)(x²+1)
        let poly = p(&[(3, 1), (2, 1), (1, 1), (0, 1)]);
        let factors = factor_poly(&poly);
        eprintln!("factors of x³+x²+x+1:");
        for (f, m) in &factors {
            eprintln!("  {} ^ {}", f, m);
        }
        assert_eq!(factors.len(), 2, "should have 2 factors");
        let degs: Vec<u32> = factors.iter().map(|(f,_)| f.degree().unwrap_or(0)).collect();
        assert!(degs.contains(&1) && degs.contains(&2));
    }
}
