use maxima_core::{Expr, Operator, resolve};
use super::database::AssumptionDB;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sign {
    Pos,
    Neg,
    Zero,
    /// Positive or zero
    Poz,
    /// Negative or zero
    Noz,
    /// Positive, negative, or zero (unknown)
    Pnz,
}

impl Sign {
    pub fn is_known_positive(self) -> bool {
        self == Sign::Pos
    }

    pub fn is_known_negative(self) -> bool {
        self == Sign::Neg
    }

    pub fn is_known_zero(self) -> bool {
        self == Sign::Zero
    }

    pub fn is_known_nonneg(self) -> bool {
        matches!(self, Sign::Pos | Sign::Zero | Sign::Poz)
    }

    pub fn multiply(self, other: Sign) -> Sign {
        use Sign::*;
        match (self, other) {
            (Zero, _) | (_, Zero) => Zero,
            (Pos, Pos) | (Neg, Neg) => Pos,
            (Pos, Neg) | (Neg, Pos) => Neg,
            (Pos, Poz) | (Poz, Pos) => Poz,
            (Neg, Poz) | (Poz, Neg) => Noz,
            (Neg, Noz) | (Noz, Neg) => Poz,
            (Pos, Noz) | (Noz, Pos) => Noz,
            _ => Pnz,
        }
    }

    pub fn add(self, other: Sign) -> Sign {
        use Sign::*;
        match (self, other) {
            (Zero, x) | (x, Zero) => x,
            (Pos, Pos) => Pos,
            (Neg, Neg) => Neg,
            (Pos, Poz) | (Poz, Pos) => Pos,
            (Neg, Noz) | (Noz, Neg) => Neg,
            _ => Pnz,
        }
    }

    pub fn negate(self) -> Sign {
        use Sign::*;
        match self {
            Pos => Neg,
            Neg => Pos,
            Zero => Zero,
            Poz => Noz,
            Noz => Poz,
            Pnz => Pnz,
        }
    }

    pub fn power(self, _exp_sign: Sign, exp_even: Option<bool>) -> Sign {
        use Sign::*;
        match (self, exp_even) {
            (Zero, _) => Zero,
            (Pos, _) => Pos,
            (Neg, Some(true)) => Pos,
            (Neg, Some(false)) => Neg,
            _ => Pnz,
        }
    }

    pub fn to_maxima_str(self) -> &'static str {
        match self {
            Sign::Pos => "pos",
            Sign::Neg => "neg",
            Sign::Zero => "zero",
            Sign::Poz => "pz",
            Sign::Noz => "nz",
            Sign::Pnz => "pnz",
        }
    }
}

fn expr_to_f64(expr: &Expr) -> Option<f64> {
    match expr {
        Expr::Integer(n) => Some(*n as f64),
        Expr::Float(f) => Some(*f),
        Expr::Rational { num, den } => Some(*num as f64 / *den as f64),
        _ => None,
    }
}

/// Compute the sign of an expression given the assumption database.
pub fn compute_sign(expr: &Expr, db: &AssumptionDB) -> Sign {
    match expr {
        Expr::Integer(n) => {
            if *n > 0 { Sign::Pos }
            else if *n < 0 { Sign::Neg }
            else { Sign::Zero }
        }
        Expr::Float(f) => {
            if *f > 0.0 { Sign::Pos }
            else if *f < 0.0 { Sign::Neg }
            else { Sign::Zero }
        }
        Expr::Rational { num, den } => {
            let sign = (*num > 0) == (*den > 0);
            if *num == 0 { Sign::Zero }
            else if sign { Sign::Pos }
            else { Sign::Neg }
        }
        Expr::Symbol(id) => {
            let name = resolve(*id);
            match name.as_str() {
                "%pi" | "%e" | "%phi" => Sign::Pos,
                "%i" => Sign::Pnz,
                _ => db.get_sign(expr),
            }
        }
        Expr::List { op, args, .. } => match op {
            Operator::MPlus => {
                let mut result = Sign::Zero;
                for arg in args {
                    result = result.add(compute_sign(arg, db));
                }
                result
            }
            Operator::MTimes => {
                let mut result = Sign::Pos;
                for arg in args {
                    result = result.multiply(compute_sign(arg, db));
                }
                result
            }
            Operator::MExpt if args.len() == 2 => {
                let base_sign = compute_sign(&args[0], db);
                let exp_sign = compute_sign(&args[1], db);
                let exp_even = match &args[1] {
                    Expr::Integer(n) => Some(n % 2 == 0),
                    _ => None,
                };
                base_sign.power(exp_sign, exp_even)
            }
            Operator::Named(id) => {
                let fname = resolve(*id);
                match fname.as_str() {
                    "abs" | "cabs" => {
                        let inner = compute_sign(&args[0], db);
                        if inner == Sign::Zero { Sign::Zero } else { Sign::Poz }
                    }
                    "exp" | "cosh" => Sign::Pos,
                    "cos" => {
                        // cos(n) for small numeric n: evaluate sign
                        if args.len() == 1 {
                            if let Some(x) = expr_to_f64(&args[0]) {
                                let v = x.cos();
                                if v > 0.0 { return Sign::Pos; }
                                if v < 0.0 { return Sign::Neg; }
                                return Sign::Zero;
                            }
                        }
                        Sign::Pnz
                    }
                    "sin" => {
                        if args.len() == 1 {
                            if let Some(x) = expr_to_f64(&args[0]) {
                                let v = x.sin();
                                if v > 0.0 { return Sign::Pos; }
                                if v < 0.0 { return Sign::Neg; }
                                return Sign::Zero;
                            }
                        }
                        Sign::Pnz
                    }
                    "sqrt" => {
                        let inner = compute_sign(&args[0], db);
                        if inner == Sign::Zero { Sign::Zero }
                        else if inner.is_known_nonneg() { Sign::Poz }
                        else { Sign::Pnz }
                    }
                    _ => Sign::Pnz,
                }
            }
            _ => Sign::Pnz,
        },
        _ => Sign::Pnz,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_multiply_table() {
        assert_eq!(Sign::Pos.multiply(Sign::Pos), Sign::Pos);
        assert_eq!(Sign::Pos.multiply(Sign::Neg), Sign::Neg);
        assert_eq!(Sign::Neg.multiply(Sign::Neg), Sign::Pos);
        assert_eq!(Sign::Pos.multiply(Sign::Zero), Sign::Zero);
        assert_eq!(Sign::Neg.multiply(Sign::Zero), Sign::Zero);
    }

    #[test]
    fn sign_add_table() {
        assert_eq!(Sign::Pos.add(Sign::Pos), Sign::Pos);
        assert_eq!(Sign::Neg.add(Sign::Neg), Sign::Neg);
        assert_eq!(Sign::Pos.add(Sign::Zero), Sign::Pos);
        assert_eq!(Sign::Pos.add(Sign::Neg), Sign::Pnz);
    }

    #[test]
    fn sign_negate() {
        assert_eq!(Sign::Pos.negate(), Sign::Neg);
        assert_eq!(Sign::Neg.negate(), Sign::Pos);
        assert_eq!(Sign::Zero.negate(), Sign::Zero);
    }

    #[test]
    fn sign_power() {
        assert_eq!(Sign::Neg.power(Sign::Pos, Some(true)), Sign::Pos);
        assert_eq!(Sign::Neg.power(Sign::Pos, Some(false)), Sign::Neg);
        assert_eq!(Sign::Pos.power(Sign::Pos, None), Sign::Pos);
        assert_eq!(Sign::Zero.power(Sign::Pos, None), Sign::Zero);
    }

    #[test]
    fn compute_sign_numeric() {
        let db = AssumptionDB::new();
        assert_eq!(compute_sign(&Expr::int(5), &db), Sign::Pos);
        assert_eq!(compute_sign(&Expr::int(-3), &db), Sign::Neg);
        assert_eq!(compute_sign(&Expr::int(0), &db), Sign::Zero);
    }

    #[test]
    fn compute_sign_product() {
        let db = AssumptionDB::new();
        let e = Expr::mul(Expr::int(-1), Expr::int(5));
        assert_eq!(compute_sign(&e, &db), Sign::Neg);
    }

    #[test]
    fn compute_sign_exp() {
        let db = AssumptionDB::new();
        let e = Expr::call("exp", vec![Expr::sym("x")]);
        assert_eq!(compute_sign(&e, &db), Sign::Pos);
    }
}
