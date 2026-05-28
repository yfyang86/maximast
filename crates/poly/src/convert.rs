use maxima_core::{Expr, Operator, SymbolId, resolve};
use crate::coeff::Coeff;
use crate::poly::Poly;

/// Convert an expression to a univariate polynomial in the given variable.
/// Returns None if the expression contains the variable in a non-polynomial way.
pub fn expr_to_poly(expr: &Expr, var: SymbolId) -> Option<Poly> {
    match expr {
        Expr::Integer(n) => Some(Poly::constant(var, Coeff::Int(*n))),
        Expr::Rational { num, den } => Some(Poly::constant(var, Coeff::Rat(*num, *den))),
        Expr::Float(_) => None, // Don't mix floats into polynomial arithmetic
        Expr::Symbol(id) => {
            if *id == var {
                Some(Poly::var_poly(var))
            } else {
                None // Other symbols are not constants in this context
            }
        }
        Expr::List { op: Operator::MPlus, args, .. } => {
            let mut result = Poly::zero(var);
            for arg in args {
                let p = expr_to_poly(arg, var)?;
                result = result.add(&p);
            }
            Some(result)
        }
        Expr::List { op: Operator::MTimes, args, .. } => {
            let mut result = Poly::constant(var, Coeff::one());
            for arg in args {
                match arg {
                    Expr::Integer(n) => {
                        result = result.scale(&Coeff::Int(*n));
                    }
                    Expr::Rational { num, den } => {
                        result = result.scale(&Coeff::Rat(*num, *den));
                    }
                    Expr::Symbol(id) if *id == var => {
                        result = result.mul(&Poly::var_poly(var));
                    }
                    Expr::List { op: Operator::MExpt, args: pow_args, .. }
                        if pow_args.len() == 2 => {
                        if let (Expr::Symbol(base_id), Expr::Integer(exp)) = (&pow_args[0], &pow_args[1]) {
                            if *base_id == var && *exp >= 0 {
                                result = result.mul(&Poly::monomial(var, *exp as u32, Coeff::one()));
                                continue;
                            }
                        }
                        return None;
                    }
                    other => {
                        let p = expr_to_poly(other, var)?;
                        result = result.mul(&p);
                    }
                }
            }
            Some(result)
        }
        Expr::List { op: Operator::MExpt, args, .. } if args.len() == 2 => {
            if let (Expr::Symbol(id), Expr::Integer(exp)) = (&args[0], &args[1]) {
                if *id == var && *exp >= 0 {
                    return Some(Poly::monomial(var, *exp as u32, Coeff::one()));
                }
            }
            None
        }
        _ => None,
    }
}

/// Convert a polynomial back to an expression.
pub fn poly_to_expr(poly: &Poly) -> Expr {
    if poly.is_zero() {
        return Expr::int(0);
    }

    let var_name = resolve(poly.var);
    let var_expr = Expr::sym(&var_name);

    let terms: Vec<Expr> = poly.terms.iter().map(|(e, c)| {
        let coeff_expr = coeff_to_expr(c);
        if *e == 0 {
            coeff_expr
        } else {
            let var_part = if *e == 1 {
                var_expr.clone()
            } else {
                Expr::pow(var_expr.clone(), Expr::int(*e as i64))
            };
            if c.is_one() {
                var_part
            } else if *c == Coeff::Int(-1) {
                Expr::neg(var_part)
            } else {
                Expr::mul(coeff_expr, var_part)
            }
        }
    }).collect();

    if terms.len() == 1 {
        terms.into_iter().next().unwrap()
    } else {
        Expr::List {
            op: Operator::MPlus,
            simplified: true,
            args: terms,
        }
    }
}

/// Convert an expression to a CRE (rational function) in the given variable.
pub fn expr_to_cre(expr: &Expr, var: SymbolId) -> Option<crate::CRE> {
    // Try as polynomial first
    if let Some(p) = expr_to_poly(expr, var) {
        return Some(crate::CRE::from_poly(p));
    }
    // Try as fraction num/den
    match expr {
        Expr::List { op: Operator::MTimes, args, .. } => {
            // Check for negative exponent factors (x^-1, etc.)
            let mut num = Poly::constant(var, Coeff::one());
            let mut den = Poly::constant(var, Coeff::one());
            for arg in args {
                match arg {
                    Expr::List { op: Operator::MExpt, args: pa, .. }
                        if pa.len() == 2 =>
                    {
                        if let Expr::Integer(n) = &pa[1] {
                            if *n < 0 {
                                let base_poly = expr_to_poly(&pa[0], var)?;
                                let pow = (-*n) as u32;
                                let mut p = Poly::constant(var, Coeff::one());
                                for _ in 0..pow { p = p.mul(&base_poly); }
                                den = den.mul(&p);
                                continue;
                            }
                        }
                        let p = expr_to_poly(arg, var)?;
                        num = num.mul(&p);
                    }
                    _ => {
                        if let Some(p) = expr_to_poly(arg, var) {
                            num = num.mul(&p);
                        } else {
                            return None;
                        }
                    }
                }
            }
            Some(crate::CRE::new(num, den))
        }
        _ => None,
    }
}

/// Convert a CRE back to an expression.
pub fn cre_to_expr(cre: &crate::CRE) -> Expr {
    let num = poly_to_expr(&cre.num);
    if cre.den.is_constant() && cre.den.leading_coeff().is_one() {
        num
    } else {
        let den = poly_to_expr(&cre.den);
        Expr::List {
            op: Operator::MTimes,
            simplified: false,
            args: vec![num, Expr::pow(den, Expr::int(-1))],
        }
    }
}

fn coeff_to_expr(c: &Coeff) -> Expr {
    match c {
        Coeff::Int(n) => Expr::int(*n),
        Coeff::Rat(n, d) => Expr::Rational { num: *n, den: *d },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maxima_core::intern;

    fn x() -> SymbolId { intern("x") }

    #[test]
    fn convert_integer() {
        let e = Expr::int(42);
        let p = expr_to_poly(&e, x()).unwrap();
        assert!(p.is_constant());
        assert_eq!(p.constant_term(), Coeff::Int(42));
    }

    #[test]
    fn convert_variable() {
        let e = Expr::sym("x");
        let p = expr_to_poly(&e, x()).unwrap();
        assert_eq!(p.degree(), Some(1));
        assert_eq!(p.leading_coeff(), Coeff::one());
    }

    #[test]
    fn convert_sum() {
        // x^2 + 2*x + 1
        let e = Expr::List {
            op: Operator::MPlus,
            simplified: true,
            args: vec![
                Expr::pow(Expr::sym("x"), Expr::int(2)),
                Expr::mul(Expr::int(2), Expr::sym("x")),
                Expr::int(1),
            ],
        };
        let p = expr_to_poly(&e, x()).unwrap();
        assert_eq!(p.degree(), Some(2));
        assert_eq!(p.eval_at(&Coeff::Int(1)), Coeff::Int(4));
    }

    #[test]
    fn convert_roundtrip() {
        let p = Poly {
            var: x(),
            terms: vec![(2, Coeff::Int(1)), (1, Coeff::Int(2)), (0, Coeff::Int(1))],
        };
        let e = poly_to_expr(&p);
        let p2 = expr_to_poly(&e, x()).unwrap();
        assert_eq!(p, p2);
    }

    #[test]
    fn convert_product() {
        // 3*x^2
        let e = Expr::mul(Expr::int(3), Expr::pow(Expr::sym("x"), Expr::int(2)));
        let p = expr_to_poly(&e, x()).unwrap();
        assert_eq!(p.degree(), Some(2));
        assert_eq!(p.leading_coeff(), Coeff::Int(3));
    }
}
