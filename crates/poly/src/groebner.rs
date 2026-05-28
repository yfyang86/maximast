use crate::coeff::Coeff;
use crate::poly::Poly;
use crate::gcd::poly_gcd;

/// Compute a Gröbner basis for a set of univariate polynomials.
/// For univariate case, this reduces to computing the GCD.
/// Returns a minimal generating set.
pub fn groebner_univariate(polys: &[Poly]) -> Vec<Poly> {
    if polys.is_empty() {
        return vec![];
    }
    let _var = polys[0].var;

    // For univariate polynomials, the Gröbner basis is just the GCD
    let mut g = polys[0].clone();
    for p in &polys[1..] {
        g = poly_gcd(&g, p);
    }

    if g.is_zero() {
        vec![]
    } else {
        // Normalize to monic (positive leading coefficient)
        let lc = g.leading_coeff();
        if matches!(&lc, Coeff::Int(n) if *n < 0) {
            vec![g.neg()]
        } else {
            vec![g]
        }
    }
}

/// Reduce a polynomial modulo a set of polynomials (polynomial division by a set).
/// Returns the remainder after dividing by each polynomial in the basis.
pub fn poly_reduce(f: &Poly, basis: &[Poly]) -> Poly {
    let mut r = f.clone();

    loop {
        let mut made_progress = false;
        for g in basis {
            if g.is_zero() { continue; }
            let g_deg = match g.degree() {
                Some(d) => d,
                None => continue,
            };
            let r_deg = match r.degree() {
                Some(d) => d,
                None => return r,
            };
            if r_deg >= g_deg {
                let lc_r = r.leading_coeff();
                let lc_g = g.leading_coeff();
                if let Some(coeff) = lc_r.div(&lc_g) {
                    let shift = r_deg - g_deg;
                    let term = Poly::monomial(f.var, shift, coeff);
                    let sub = term.mul(g);
                    r = r.sub(&sub);
                    made_progress = true;
                    break;
                }
            }
        }
        if !made_progress { break; }
    }
    r
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
    fn groebner_single() {
        let basis = groebner_univariate(&[p(&[(2, 1), (0, -1)])]);
        assert_eq!(basis.len(), 1);
        assert_eq!(basis[0], p(&[(2, 1), (0, -1)]));
    }

    #[test]
    fn groebner_gcd() {
        // GCD of x^2-1 and x^2+2x+1 is x+1
        let basis = groebner_univariate(&[
            p(&[(2, 1), (0, -1)]),
            p(&[(2, 1), (1, 2), (0, 1)]),
        ]);
        assert_eq!(basis.len(), 1);
        assert_eq!(basis[0], p(&[(1, 1), (0, 1)]));
    }

    #[test]
    fn groebner_coprime() {
        // GCD of x^2+1 and x+1 is 1
        let basis = groebner_univariate(&[
            p(&[(2, 1), (0, 1)]),
            p(&[(1, 1), (0, 1)]),
        ]);
        assert_eq!(basis.len(), 1);
        assert!(basis[0].is_constant());
    }

    #[test]
    fn reduce_simple() {
        let f = p(&[(3, 1), (0, -1)]); // x^3-1
        let basis = vec![p(&[(1, 1), (0, -1)])]; // x-1
        let r = poly_reduce(&f, &basis);
        assert!(r.is_zero()); // x^3-1 ≡ 0 mod (x-1)
    }

    #[test]
    fn reduce_remainder() {
        let f = p(&[(2, 1), (0, 1)]); // x^2+1
        let basis = vec![p(&[(1, 1), (0, -1)])]; // x-1
        let r = poly_reduce(&f, &basis);
        assert_eq!(r, p(&[(0, 2)])); // remainder is 2
    }
}
