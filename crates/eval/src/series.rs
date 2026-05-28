use maxima_core::{Expr, Operator};
use crate::helpers::{to_f64, contains_var};
use crate::simp::simplify;

/// Truncated power series: Σ c_i * (x - a)^e_i + O((x-a)^N)
/// Exponents can be rational (for Laurent/Puiseux series).
#[derive(Debug, Clone)]
pub struct Series {
    pub terms: Vec<(i64, i64, Expr)>, // (num_exp, den_exp, coefficient) — exponent = num/den
    pub var: Expr,
    pub center: Expr,
    pub order: i64, // truncation: all terms with exponent >= order are dropped
}

impl Series {
    pub fn new(var: Expr, center: Expr, order: i64) -> Self {
        Series { terms: Vec::new(), var, center, order }
    }

    pub fn add_term(&mut self, exp_num: i64, exp_den: i64, coeff: Expr) {
        // Truncate: drop terms with exponent >= order
        // Compare exp_num/exp_den >= order by cross-multiplying (same sign denominators)
        if exp_den > 0 && exp_num >= self.order * exp_den {
            return;
        }
        if exp_den < 0 && exp_num <= self.order * exp_den {
            return;
        }
        if coeff == Expr::int(0) { return; }
        self.terms.push((exp_num, exp_den, coeff));
        self.terms.sort_by(|a, b| {
            let ea = a.0 as f64 / a.1 as f64;
            let eb = b.0 as f64 / b.1 as f64;
            ea.partial_cmp(&eb).unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    pub fn from_terms(var: Expr, center: Expr, order: i64, terms: Vec<(i64, Expr)>) -> Self {
        let mut s = Series::new(var, center, order);
        for (e, c) in terms {
            s.terms.push((e, 1, c));
        }
        s
    }

    pub fn leading_term(&self) -> Option<(f64, &Expr)> {
        self.terms.first().map(|(n, d, c)| (*n as f64 / *d as f64, c))
    }

    pub fn leading_exponent(&self) -> Option<f64> {
        self.leading_term().map(|(e, _)| e)
    }

    pub fn leading_coeff(&self) -> Option<&Expr> {
        self.leading_term().map(|(_, c)| c)
    }

    pub fn is_zero(&self) -> bool {
        self.terms.is_empty()
    }

    /// Multiply two series (convolve coefficients), truncating at order.
    pub fn mul(&self, other: &Series) -> Series {
        let mut result = Series::new(self.var.clone(), self.center.clone(), self.order);
        for (en, ed, ec) in &self.terms {
            for (fn_, fd, fc) in &other.terms {
                let rn = en * fd + fn_ * ed;
                let rd = ed * fd;
                let exp_f = rn as f64 / rd as f64;
                if exp_f >= self.order as f64 { continue; }
                let coeff = simplify(&Expr::mul(ec.clone(), fc.clone()));
                if coeff != Expr::int(0) {
                    result.terms.push((rn, rd, coeff));
                }
            }
        }
        result.combine_like_terms();
        result
    }

    /// Add two series.
    pub fn add(&self, other: &Series) -> Series {
        let mut result = Series::new(self.var.clone(), self.center.clone(), self.order);
        result.terms.extend(self.terms.clone());
        result.terms.extend(other.terms.clone());
        result.combine_like_terms();
        result
    }

    fn combine_like_terms(&mut self) {
        self.terms.sort_by(|a, b| {
            let ea = a.0 as f64 / a.1 as f64;
            let eb = b.0 as f64 / b.1 as f64;
            ea.partial_cmp(&eb).unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut combined: Vec<(i64, i64, Expr)> = Vec::new();
        for (n, d, c) in &self.terms {
            if let Some(last) = combined.last_mut() {
                let e1 = last.0 as f64 / last.1 as f64;
                let e2 = *n as f64 / *d as f64;
                if (e1 - e2).abs() < 1e-15 {
                    last.2 = simplify(&Expr::add(last.2.clone(), c.clone()));
                    continue;
                }
            }
            combined.push((*n, *d, c.clone()));
        }
        self.terms = combined.into_iter().filter(|(_, _, c)| *c != Expr::int(0)).collect();
    }

    /// Convert back to expression: Σ c_i * (x - center)^(n_i/d_i)
    pub fn to_expr(&self) -> Expr {
        if self.terms.is_empty() { return Expr::int(0); }
        let dx = if self.center == Expr::int(0) {
            self.var.clone()
        } else {
            Expr::sub(self.var.clone(), self.center.clone())
        };
        let mut parts = Vec::new();
        for (n, d, c) in &self.terms {
            let exp = if *d == 1 {
                Expr::int(*n)
            } else {
                Expr::Rational { num: *n, den: *d }
            };
            let term = if *n == 0 && *d == 1 {
                c.clone()
            } else if *n == 1 && *d == 1 {
                simplify(&Expr::mul(c.clone(), dx.clone()))
            } else {
                simplify(&Expr::mul(c.clone(), Expr::pow(dx.clone(), exp)))
            };
            parts.push(term);
        }
        if parts.len() == 1 { return parts.remove(0); }
        simplify(&Expr::List { op: Operator::MPlus, simplified: false, args: parts })
    }
}

/// Compute Taylor series of expr around center to given order.
pub fn taylor(expr: &Expr, var: &Expr, center: &Expr, order: u32) -> Option<Series> {
    use crate::eval::diff_once_pub;

    let mut result = Series::new(var.clone(), center.clone(), order as i64);

    let mut current = expr.clone();
    let mut factorial = 1i64;

    for k in 0..=order {
        let val = eval_at(&current, var, center);
        if let Some(v) = val {
            if v != Expr::int(0) {
                let coeff = if factorial == 1 { v }
                    else { simplify(&Expr::div(v, Expr::int(factorial))) };
                result.terms.push((k as i64, 1, coeff));
            }
        } else {
            return None;
        }
        if k < order {
            current = diff_once_pub(&current, var);
            factorial *= (k + 1) as i64;
        }
    }
    Some(result)
}

/// Build series for known functions around 0.
pub fn series_at_zero(fname: &str, var: &Expr, order: u32) -> Option<Series> {
    let mut s = Series::new(var.clone(), Expr::int(0), order as i64);
    match fname {
        "exp" => {
            // exp(x) = 1 + x + x²/2 + x³/6 + ...
            let mut fact = 1i64;
            for k in 0..=order {
                s.terms.push((k as i64, 1, Expr::Rational { num: 1, den: fact }));
                fact *= (k + 1) as i64;
            }
        }
        "sin" => {
            // sin(x) = x - x³/6 + x⁵/120 - ...
            let mut fact = 1i64;
            for k in 0..=order {
                if k % 2 == 1 {
                    let sign = if (k / 2) % 2 == 0 { 1i64 } else { -1 };
                    s.terms.push((k as i64, 1, Expr::Rational { num: sign, den: fact }));
                }
                fact *= (k + 1) as i64;
            }
        }
        "cos" => {
            let mut fact = 1i64;
            for k in 0..=order {
                if k % 2 == 0 {
                    let sign = if (k / 2) % 2 == 0 { 1i64 } else { -1 };
                    s.terms.push((k as i64, 1, Expr::Rational { num: sign, den: fact }));
                }
                fact *= (k + 1) as i64;
            }
        }
        "log1p" => {
            // log(1+x) = x - x²/2 + x³/3 - ...
            for k in 1..=order {
                let sign = if k % 2 == 1 { 1i64 } else { -1 };
                s.terms.push((k as i64, 1, Expr::Rational { num: sign, den: k as i64 }));
            }
        }
        _ => return None,
    }
    Some(s)
}

/// Evaluate expression at var = value by substitution.
fn eval_at(expr: &Expr, var: &Expr, value: &Expr) -> Option<Expr> {
    let substituted = crate::helpers::subst(value, var, expr);
    Some(simplify(&substituted))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn series_exp() {
        let x = Expr::sym("x");
        let s = series_at_zero("exp", &x, 4).unwrap();
        assert_eq!(s.terms.len(), 5);
        assert_eq!(s.terms[0].2, Expr::Rational { num: 1, den: 1 });
    }

    #[test]
    fn series_sin() {
        let x = Expr::sym("x");
        let s = series_at_zero("sin", &x, 5).unwrap();
        // sin(x) = x - x³/6 + x⁵/120
        assert_eq!(s.terms.len(), 3);
    }

    #[test]
    fn series_mul() {
        let x = Expr::sym("x");
        // (1 + x) * (1 + x) = 1 + 2x + x²
        let mut a = Series::new(x.clone(), Expr::int(0), 3);
        a.terms = vec![(0, 1, Expr::int(1)), (1, 1, Expr::int(1))];
        let prod = a.mul(&a);
        let expr = prod.to_expr();
        let s = expr.to_string();
        assert!(s.contains("2"), "expected 2x term, got {}", s);
    }

    #[test]
    fn series_to_expr() {
        let x = Expr::sym("x");
        let s = Series::from_terms(x.clone(), Expr::int(0), 3, vec![
            (0, Expr::int(1)),
            (1, Expr::int(2)),
            (2, Expr::int(3)),
        ]);
        let e = s.to_expr();
        assert!(!e.to_string().is_empty());
    }
}
