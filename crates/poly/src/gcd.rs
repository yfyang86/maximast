use crate::coeff::Coeff;
use crate::poly::Poly;

/// Compute GCD of two polynomials using the Euclidean algorithm
/// with primitive part normalization (subresultant-like).
pub fn poly_gcd(a: &Poly, b: &Poly) -> Poly {
    assert_eq!(a.var, b.var);

    if a.is_zero() { return primitive(b); }
    if b.is_zero() { return primitive(a); }

    let mut r0 = primitive(a);
    let mut r1 = primitive(b);

    // Ensure deg(r0) >= deg(r1)
    if r0.degree().unwrap_or(0) < r1.degree().unwrap_or(0) {
        std::mem::swap(&mut r0, &mut r1);
    }

    while !r1.is_zero() {
        let (_, rem) = match r0.divmod(&r1) {
            Some(qr) => qr,
            None => {
                // Can't divide exactly — use pseudo-remainder
                let rem = pseudo_remainder(&r0, &r1);
                (Poly::zero(a.var), rem)
            }
        };
        r0 = r1;
        r1 = primitive(&rem);
    }

    primitive(&r0)
}

/// Compute GCD and cofactors: returns (g, a/g, b/g)
pub fn poly_gcd_cofactors(a: &Poly, b: &Poly) -> (Poly, Poly, Poly) {
    let g = poly_gcd(a, b);
    let ca = a.exact_div(&g).unwrap_or_else(|| a.clone());
    let cb = b.exact_div(&g).unwrap_or_else(|| b.clone());
    (g, ca, cb)
}

/// Pseudo-remainder: rem = lc(b)^(deg(a)-deg(b)+1) * a mod b
fn pseudo_remainder(a: &Poly, b: &Poly) -> Poly {
    if b.is_zero() { return a.clone(); }

    let deg_a = a.degree().unwrap_or(0);
    let deg_b = b.degree().unwrap_or(0);
    if deg_a < deg_b { return a.clone(); }

    let lc_b = b.leading_coeff();
    let delta = deg_a - deg_b;

    // Scale a by lc(b)^(delta+1)
    let mut scale = Coeff::one();
    for _ in 0..=delta {
        scale = scale.mul(&lc_b);
    }
    let mut r = a.scale(&scale);

    // Now do polynomial division
    for _ in 0..=delta {
        if r.is_zero() { break; }
        let deg_r = match r.degree() {
            Some(d) => d,
            None => break,
        };
        if deg_r < deg_b { break; }

        let lc_r = r.leading_coeff();
        let coeff = match lc_r.div(&b.leading_coeff()) {
            Some(c) => c,
            None => break,
        };
        let shift = deg_r - deg_b;
        let term = Poly::monomial(a.var, shift, coeff);
        let sub = term.mul(b);
        r = r.sub(&sub);
    }
    r
}

/// Normalize polynomial to primitive part (positive leading coefficient).
/// For polynomials with rational coefficients, makes the polynomial monic
/// and then clears denominators.
fn primitive(p: &Poly) -> Poly {
    if p.is_zero() { return p.clone(); }

    let has_rats = p.terms.iter().any(|(_, c)| matches!(c, Coeff::Rat(_, _)));

    if has_rats {
        // Make monic: divide all coefficients by the leading coefficient
        let lc = p.leading_coeff();
        if lc.is_zero() { return p.clone(); }
        let mut terms: Vec<(u32, Coeff)> = Vec::new();
        for (e, coeff) in &p.terms {
            match coeff.div(&lc) {
                Some(q) if !q.is_zero() => terms.push((*e, q)),
                Some(_) => {} // zero coefficient, skip
                None => return p.clone(), // division failed, return original
            }
        }
        let mut poly = Poly { var: p.var, terms };
        // Now clear any remaining rational denominators
        let mut lcd = 1i64;
        for (_, c) in &poly.terms {
            if let Coeff::Rat(_, d) = c {
                lcd = lcm_abs(lcd, d.abs());
            }
        }
        if lcd > 1 {
            poly = poly.scale(&Coeff::Int(lcd));
        }
        // Extract integer content
        let c = poly.content();
        if !c.is_one() && !c.is_zero() {
            let mut result = Vec::new();
            for (e, coeff) in &poly.terms {
                if let Some(q) = coeff.div(&c) {
                    if !q.is_zero() {
                        result.push((*e, q));
                    }
                }
            }
            poly = Poly { var: p.var, terms: result };
        }
        // Ensure positive leading coefficient
        if matches!(poly.leading_coeff(), Coeff::Int(n) if n < 0) {
            poly.neg()
        } else {
            poly
        }
    } else {
        let c = p.content();
        if c.is_one() {
            let lc = p.leading_coeff();
            if matches!(lc, Coeff::Int(n) if n < 0) {
                return p.neg();
            }
            return p.clone();
        }
        let mut result: Vec<(u32, Coeff)> = Vec::new();
        for (e, coeff) in &p.terms {
            if let Some(q) = coeff.div(&c) {
                if !q.is_zero() {
                    result.push((*e, q));
                }
            }
        }
        let poly = Poly { var: p.var, terms: result };
        if matches!(poly.leading_coeff(), Coeff::Int(n) if n < 0) {
            poly.neg()
        } else {
            poly
        }
    }
}

fn lcm_abs(a: i64, b: i64) -> i64 {
    if a == 0 || b == 0 { return 0; }
    let g = gcd_coeff(a.unsigned_abs(), b.unsigned_abs());
    (a.abs() / g as i64) * b.abs()
}

fn gcd_coeff(a: u64, b: u64) -> u64 {
    if b == 0 { return a; }
    gcd_coeff(b, a % b)
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
    fn gcd_trivial() {
        let a = p(&[(2, 1), (0, -1)]); // x^2-1
        let g = poly_gcd(&a, &Poly::zero(x()));
        assert_eq!(g, a);
    }

    #[test]
    fn gcd_coprime() {
        let a = p(&[(2, 1), (0, 1)]); // x^2+1
        let b = p(&[(1, 1), (0, 1)]); // x+1
        let g = poly_gcd(&a, &b);
        assert!(g.is_constant(), "expected constant gcd, got {}", g);
    }

    #[test]
    fn gcd_common_factor() {
        let a = p(&[(2, 1), (0, -1)]); // x^2-1 = (x+1)(x-1)
        let b = p(&[(2, 1), (1, 2), (0, 1)]); // x^2+2x+1 = (x+1)^2
        let g = poly_gcd(&a, &b);
        // GCD should be x+1
        assert_eq!(g, p(&[(1, 1), (0, 1)]));
    }

    #[test]
    fn gcd_with_content() {
        let a = p(&[(2, 4), (1, 8), (0, 4)]); // 4x^2+8x+4 = 4*(x+1)^2
        let b = p(&[(1, 2), (0, 2)]); // 2x+2 = 2*(x+1)
        let g = poly_gcd(&a, &b);
        assert_eq!(g, p(&[(1, 1), (0, 1)])); // x+1 (primitive)
    }

    #[test]
    fn gcd_x3_minus_1() {
        let a = p(&[(3, 1), (0, -1)]); // x^3-1
        let b = p(&[(6, 1), (0, -1)]); // x^6-1
        let g = poly_gcd(&a, &b);
        assert_eq!(g, p(&[(3, 1), (0, -1)])); // x^3-1
    }

    #[test]
    fn gcd_cofactors() {
        let a = p(&[(2, 1), (0, -1)]); // x^2-1
        let b = p(&[(2, 1), (1, 2), (0, 1)]); // x^2+2x+1
        let (g, ca, cb) = poly_gcd_cofactors(&a, &b);
        assert_eq!(g, p(&[(1, 1), (0, 1)])); // x+1
        assert_eq!(ca, p(&[(1, 1), (0, -1)])); // x-1
        assert_eq!(cb, p(&[(1, 1), (0, 1)])); // x+1
    }

    // --- Comprehensive GCD tests ---

    #[test]
    fn gcd_same_polynomial() {
        let a = p(&[(2, 1), (1, 1), (0, 1)]); // x^2+x+1
        let g = poly_gcd(&a, &a);
        assert_eq!(g, a);
    }

    #[test]
    fn gcd_one_is_constant() {
        let a = p(&[(3, 1), (0, -1)]); // x^3-1
        let b = p(&[(0, 5)]); // 5
        let g = poly_gcd(&a, &b);
        assert!(g.is_constant());
    }

    #[test]
    fn gcd_both_zero() {
        let g = poly_gcd(&Poly::zero(x()), &Poly::zero(x()));
        assert!(g.is_zero());
    }

    #[test]
    fn gcd_x4_minus_1() {
        // x^4-1 = (x-1)(x+1)(x^2+1)
        // x^2-1 = (x-1)(x+1)
        let a = p(&[(4, 1), (0, -1)]); // x^4-1
        let b = p(&[(2, 1), (0, -1)]); // x^2-1
        let g = poly_gcd(&a, &b);
        assert_eq!(g, p(&[(2, 1), (0, -1)])); // x^2-1
    }

    #[test]
    fn gcd_preserves_sign() {
        // GCD should have positive leading coefficient
        let a = p(&[(2, 1), (0, -1)]); // x^2-1
        let b = p(&[(1, 1), (0, 1)]); // x+1
        let g = poly_gcd(&a, &b);
        match g.leading_coeff() {
            Coeff::Int(n) => assert!(n > 0, "leading coeff should be positive"),
            _ => {}
        }
    }

    #[test]
    fn gcd_linear_factors() {
        // (x+1)(x+2) and (x+2)(x+3) → gcd = x+2
        let a = p(&[(1, 1), (0, 1)]).mul(&p(&[(1, 1), (0, 2)])); // (x+1)(x+2)
        let b = p(&[(1, 1), (0, 2)]).mul(&p(&[(1, 1), (0, 3)])); // (x+2)(x+3)
        let g = poly_gcd(&a, &b);
        assert_eq!(g, p(&[(1, 1), (0, 2)])); // x+2
    }

    #[test]
    fn gcd_cofactors_verify() {
        // Verify a = g * ca, b = g * cb
        let a = p(&[(3, 1), (0, -1)]); // x^3-1
        let b = p(&[(2, 1), (0, -1)]); // x^2-1
        let (g, ca, cb) = poly_gcd_cofactors(&a, &b);
        assert_eq!(g.mul(&ca), a);
        assert_eq!(g.mul(&cb), b);
    }

    #[test]
    fn gcd_swapped_order() {
        let a = p(&[(2, 1), (0, -1)]); // x^2-1
        let b = p(&[(2, 1), (1, 2), (0, 1)]); // x^2+2x+1
        // GCD should be the same regardless of argument order
        assert_eq!(poly_gcd(&a, &b), poly_gcd(&b, &a));
    }

    #[test]
    fn gcd_sqfree_derivative() {
        // gcd(x^3+2x^2+x, 3x^2+4x+1) = x+1
        // This tests the case where intermediate remainders have rational coefficients
        let a = p(&[(3, 1), (2, 2), (1, 1)]); // x^3+2x^2+x = x(x+1)^2
        let b = p(&[(2, 3), (1, 4), (0, 1)]); // 3x^2+4x+1 = (3x+1)(x+1)
        let g = poly_gcd(&a, &b);
        assert_eq!(g, p(&[(1, 1), (0, 1)]), "gcd should be x+1, got {}", g);
    }
}
