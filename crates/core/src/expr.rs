use std::fmt;

use num::BigInt;

use crate::{Operator, SymbolId, resolve};

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Integer(i64),
    BigInt(Box<BigInt>),
    Rational { num: i64, den: i64 },
    Float(f64),
    Symbol(SymbolId),
    String(Box<str>),
    List {
        op: Operator,
        simplified: bool,
        args: Vec<Expr>,
    },
}

impl Expr {
    pub fn int(n: i64) -> Self {
        Expr::Integer(n)
    }

    pub fn sym(name: &str) -> Self {
        Expr::Symbol(crate::intern(name))
    }

    pub fn add(a: Expr, b: Expr) -> Self {
        Expr::List {
            op: Operator::MPlus,
            simplified: false,
            args: vec![a, b],
        }
    }

    pub fn mul(a: Expr, b: Expr) -> Self {
        Expr::List {
            op: Operator::MTimes,
            simplified: false,
            args: vec![a, b],
        }
    }

    pub fn pow(base: Expr, exponent: Expr) -> Self {
        Expr::List {
            op: Operator::MExpt,
            simplified: false,
            args: vec![base, exponent],
        }
    }

    pub fn neg(a: Expr) -> Self {
        Expr::mul(Expr::int(-1), a)
    }

    pub fn sub(a: Expr, b: Expr) -> Self {
        Expr::add(a, Expr::neg(b))
    }

    pub fn div(a: Expr, b: Expr) -> Self {
        Expr::mul(a, Expr::pow(b, Expr::int(-1)))
    }

    pub fn list(items: Vec<Expr>) -> Self {
        Expr::List {
            op: Operator::MList,
            simplified: false,
            args: items,
        }
    }

    pub fn set(items: Vec<Expr>) -> Self {
        Expr::List {
            op: Operator::MSet,
            simplified: false,
            args: items,
        }
    }

    pub fn call(name: &str, args: Vec<Expr>) -> Self {
        Expr::List {
            op: Operator::Named(crate::intern(name)),
            simplified: false,
            args,
        }
    }

    pub fn is_zero(&self) -> bool {
        matches!(self, Expr::Integer(0))
    }

    pub fn is_one(&self) -> bool {
        matches!(self, Expr::Integer(1))
    }

    pub fn is_atom(&self) -> bool {
        !matches!(self, Expr::List { .. })
    }
}

fn format_float_maxima(x: f64) -> String {
    // Format with enough digits for round-trip, then trim trailing zeros
    let s = format!("{:.15}", x);
    let s = s.trim_end_matches('0');
    let s = s.trim_end_matches('.');
    // Ensure at least one decimal place if it's a non-integer float
    if !s.contains('.') {
        format!("{}.0", s)
    } else {
        s.to_string()
    }
}

fn needs_parens_in_product(expr: &Expr) -> bool {
    match expr {
        Expr::List { op: Operator::MPlus, .. } => true,
        Expr::Rational { den, .. } if *den != 1 && *den != -1 => true,
        _ => false,
    }
}

fn needs_parens_after_star(expr: &Expr) -> bool {
    match expr {
        Expr::Integer(n) if *n < 0 => true,
        Expr::Float(f) if *f < 0.0 => true,
        Expr::Rational { num, den } => (*num < 0) != (*den < 0) || (*den != 1 && *den != -1),
        Expr::List { op: Operator::MPlus, .. } => true,
        _ => false,
    }
}

fn needs_parens_in_power(expr: &Expr) -> bool {
    // A base prints with parens when leaving it bare would re-parse to a
    // different value: negative numerics ((-1)^n ≠ -1^n) and any rational
    // ((1/2)^n ≠ 1/2^n), plus sums/products.
    match expr {
        Expr::Integer(n) => *n < 0,
        Expr::Float(f) => *f < 0.0,
        Expr::Rational { .. } => true,
        Expr::List { op: Operator::MPlus | Operator::MTimes, .. } => true,
        _ => false,
    }
}

fn needs_parens_as_exponent(expr: &Expr) -> bool {
    match expr {
        Expr::Integer(n) if *n < 0 => true,
        Expr::Float(f) if *f < 0.0 => true,
        Expr::Rational { .. } => true,
        Expr::List { op: Operator::MPlus | Operator::MTimes | Operator::MExpt, .. } => true,
        _ => false,
    }
}

fn is_negative_term(expr: &Expr) -> bool {
    match expr {
        Expr::Integer(n) => *n < 0,
        Expr::Float(f) => *f < 0.0,
        Expr::Rational { num, den } => (*num < 0) != (*den < 0),
        Expr::List { op: Operator::MTimes, args, .. } => {
            if let Some(first) = args.first() {
                is_negative_term(first)
            } else {
                false
            }
        }
        _ => false,
    }
}

fn negate_term(expr: &Expr) -> Expr {
    match expr {
        Expr::Integer(n) => Expr::Integer(-n),
        Expr::Float(f) => Expr::Float(-f),
        Expr::Rational { num, den } => Expr::Rational { num: -num, den: *den },
        Expr::List { op: Operator::MTimes, args, simplified } => {
            if let Some(first) = args.first() {
                let mut new_args = vec![negate_term(first)];
                new_args.extend(args[1..].iter().cloned());
                // If the negated first element is 1, drop it
                if new_args[0] == Expr::Integer(1) && new_args.len() > 1 {
                    new_args.remove(0);
                    if new_args.len() == 1 {
                        return new_args.pop().unwrap();
                    }
                }
                Expr::List {
                    op: Operator::MTimes,
                    simplified: *simplified,
                    args: new_args,
                }
            } else {
                expr.clone()
            }
        }
        _ => expr.clone(),
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Integer(n) => write!(f, "{}", n),
            Expr::BigInt(n) => write!(f, "{}", n),
            Expr::Rational { num, den } => {
                if *den == 1 { write!(f, "{}", num) }
                else if *den == -1 { write!(f, "{}", -num) }
                else { write!(f, "{}/{}", num, den) }
            }
            Expr::Float(x) => {
                if *x == x.floor() && x.abs() < 1e15 {
                    write!(f, "{}", *x as i64)
                } else {
                    write!(f, "{}", format_float_maxima(*x))
                }
            }
            Expr::Symbol(id) => write!(f, "{}", resolve(*id)),
            Expr::String(s) => write!(f, "\"{}\"", s),
            Expr::List { op, args, .. } => match op {
                Operator::MPlus => {
                    if args.is_empty() {
                        return write!(f, "0");
                    }
                    write!(f, "{}", args[0])?;
                    for arg in &args[1..] {
                        if is_negative_term(arg) {
                            write!(f, "-{}", negate_term(arg))?;
                        } else {
                            write!(f, "+{}", arg)?;
                        }
                    }
                    Ok(())
                }
                Operator::MTimes => {
                    if args.is_empty() {
                        return write!(f, "1");
                    }
                    // -1*expr → -expr (or -(expr) if needed)
                    let (start, prefix) = if args.len() >= 2 && args[0] == Expr::Integer(-1) {
                        (1usize, true)
                    } else {
                        (0, false)
                    };
                    // Separate numerator and denominator factors
                    let mut num_parts: Vec<&Expr> = Vec::new();
                    let mut den_parts: Vec<String> = Vec::new();
                    for arg in &args[start..] {
                        if let Expr::List { op: Operator::MExpt, args: pa, .. } = arg {
                            if pa.len() == 2 {
                                if let Expr::Integer(e) = &pa[1] {
                                    let base_needs_parens = matches!(&pa[0], Expr::List { op: Operator::MPlus | Operator::MTimes, .. });
                                    if *e == -1 {
                                        if base_needs_parens {
                                            den_parts.push(format!("({})", pa[0]));
                                        } else {
                                            den_parts.push(format!("{}", pa[0]));
                                        }
                                        continue;
                                    } else if *e < -1 {
                                        if base_needs_parens {
                                            den_parts.push(format!("({})^{}", pa[0], -e));
                                        } else {
                                            den_parts.push(format!("{}^{}", pa[0], -e));
                                        }
                                        continue;
                                    }
                                }
                            }
                        }
                        num_parts.push(arg);
                    }
                    if !den_parts.is_empty() {
                        let num_str = if num_parts.is_empty() {
                            "1".to_string()
                        } else {
                            let parts: Vec<String> = num_parts.iter().enumerate().map(|(i, a)| {
                                if i > 0 && needs_parens_after_star(a) { format!("({})", a) }
                                else if needs_parens_in_product(a) { format!("({})", a) }
                                else { format!("{}", a) }
                            }).collect();
                            parts.join("*")
                        };
                        let den_str = if den_parts.len() == 1 {
                            den_parts[0].clone()
                        } else {
                            format!("({})", den_parts.join("*"))
                        };
                        if prefix {
                            return write!(f, "-{}/{}", num_str, den_str);
                        }
                        return write!(f, "{}/{}", num_str, den_str);
                    }
                    if prefix {
                        let rest = &args[start..];
                        if rest.len() == 1 {
                            let arg = &rest[0];
                            if needs_parens_in_product(arg) {
                                return write!(f, "-({})", arg);
                            } else {
                                return write!(f, "-{}", arg);
                            }
                        }
                        write!(f, "-")?;
                    }
                    for (i, arg) in args[start..].iter().enumerate() {
                        if i > 0 {
                            write!(f, "*")?;
                            if needs_parens_after_star(arg) {
                                write!(f, "({})", arg)?;
                            } else {
                                write!(f, "{}", arg)?;
                            }
                        } else if needs_parens_in_product(arg) {
                            write!(f, "({})", arg)?;
                        } else {
                            write!(f, "{}", arg)?;
                        }
                    }
                    Ok(())
                }
                Operator::MExpt => {
                    let base = &args[0];
                    let exp = &args[1];
                    // x^(-1) → 1/x, x^(-n) → 1/x^n
                    if let Expr::Integer(e) = exp {
                        if *e == -1 {
                            if needs_parens_in_power(base) {
                                return write!(f, "1/({})", base);
                            }
                            return write!(f, "1/{}", base);
                        } else if *e < -1 {
                            if needs_parens_in_power(base) {
                                return write!(f, "1/({})^{}", base, -e);
                            }
                            return write!(f, "1/{}^{}", base, -e);
                        }
                    }
                    if needs_parens_in_power(base) {
                        write!(f, "({})", base)?;
                    } else {
                        write!(f, "{}", base)?;
                    }
                    if needs_parens_as_exponent(exp) {
                        write!(f, "^({})", exp)
                    } else {
                        write!(f, "^{}", exp)
                    }
                }
                Operator::MList => {
                    write!(f, "[")?;
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ",")?;
                        }
                        write!(f, "{}", arg)?;
                    }
                    write!(f, "]")
                }
                Operator::MSet => {
                    write!(f, "{{")?;
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 { write!(f, ",")?; }
                        write!(f, "{}", arg)?;
                    }
                    write!(f, "}}")
                }
                Operator::MMatrix => {
                    write!(f, "matrix(")?;
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 { write!(f, ",")?; }
                        write!(f, "{}", arg)?;
                    }
                    write!(f, ")")
                }
                Operator::MEqual => write!(f, "{} = {}", args[0], args[1]),
                Operator::MNotEqual => write!(f, "{} # {}", args[0], args[1]),
                Operator::MLessThan => write!(f, "{} < {}", args[0], args[1]),
                Operator::MGreaterThan => write!(f, "{} > {}", args[0], args[1]),
                Operator::MLessEqual => write!(f, "{} <= {}", args[0], args[1]),
                Operator::MGreaterEqual => write!(f, "{} >= {}", args[0], args[1]),
                Operator::MDefine if args.len() == 2 => {
                    write!(f, "{}:={}", args[0], args[1])
                }
                Operator::MAssign if args.len() == 2 => {
                    write!(f, "{}:{}", args[0], args[1])
                }
                Operator::MQuote if args.len() == 1 => {
                    write!(f, "'{}", args[0])
                }
                Operator::MNot if args.len() == 1 => {
                    write!(f, "not {}", args[0])
                }
                Operator::MAnd if args.len() == 2 => {
                    write!(f, "{} and {}", args[0], args[1])
                }
                Operator::MOr if args.len() == 2 => {
                    write!(f, "{} or {}", args[0], args[1])
                }
                Operator::Named(id) => {
                    write!(f, "{}(", resolve(*id))?;
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ",")?;
                        }
                        write!(f, "{}", arg)?;
                    }
                    write!(f, ")")
                }
                _ => {
                    write!(f, "{}(", op)?;
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", arg)?;
                    }
                    write!(f, ")")
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_display() {
        assert_eq!(Expr::int(42).to_string(), "42");
    }

    #[test]
    fn negative_integer_display() {
        assert_eq!(Expr::int(-7).to_string(), "-7");
    }

    #[test]
    fn float_display() {
        assert_eq!(Expr::Float(3.14).to_string(), "3.14");
    }

    #[test]
    fn symbol_display() {
        assert_eq!(Expr::sym("x").to_string(), "x");
    }

    #[test]
    fn string_display() {
        assert_eq!(
            Expr::String("hello".into()).to_string(),
            "\"hello\""
        );
    }

    #[test]
    fn add_display() {
        let e = Expr::add(Expr::int(1), Expr::int(2));
        assert_eq!(e.to_string(), "1+2");
    }

    #[test]
    fn mul_display() {
        let e = Expr::mul(Expr::sym("a"), Expr::sym("b"));
        assert_eq!(e.to_string(), "a*b");
    }

    #[test]
    fn pow_display() {
        let e = Expr::pow(Expr::sym("x"), Expr::int(2));
        assert_eq!(e.to_string(), "x^2");
    }

    #[test]
    fn compound_display() {
        // (x+1)^2
        let e = Expr::pow(Expr::add(Expr::sym("x"), Expr::int(1)), Expr::int(2));
        assert_eq!(e.to_string(), "(x+1)^2");
    }

    #[test]
    fn sub_display() {
        let e = Expr::sub(Expr::sym("a"), Expr::sym("b"));
        assert_eq!(e.to_string(), "a-b");
    }

    #[test]
    fn list_display() {
        let e = Expr::list(vec![Expr::int(1), Expr::int(2), Expr::int(3)]);
        assert_eq!(e.to_string(), "[1,2,3]");
    }

    #[test]
    fn call_display() {
        let e = Expr::call("sin", vec![Expr::sym("x")]);
        assert_eq!(e.to_string(), "sin(x)");
    }

    #[test]
    fn power_base_needs_parens() {
        // Negative numeric and rational bases must print parenthesized so they
        // round-trip: (-1)^n ≠ -1^n, (1/2)^n ≠ 1/2^n.
        let neg1_n = Expr::pow(Expr::int(-1), Expr::sym("n"));
        assert_eq!(neg1_n.to_string(), "(-1)^n");
        let half_n = Expr::pow(Expr::Rational { num: 1, den: 2 }, Expr::sym("n"));
        assert_eq!(half_n.to_string(), "(1/2)^n");
        // Positive integer / symbol bases stay bare.
        assert_eq!(Expr::pow(Expr::int(2), Expr::sym("n")).to_string(), "2^n");
    }

    #[test]
    fn rational_display() {
        let e = Expr::Rational { num: 3, den: 4 };
        assert_eq!(e.to_string(), "3/4");
    }

    #[test]
    fn nested_arithmetic() {
        // (a+b)*c
        let e = Expr::mul(
            Expr::add(Expr::sym("a"), Expr::sym("b")),
            Expr::sym("c"),
        );
        assert_eq!(e.to_string(), "(a+b)*c");
    }

    #[test]
    fn is_zero() {
        assert!(Expr::int(0).is_zero());
        assert!(!Expr::int(1).is_zero());
    }

    #[test]
    fn is_one() {
        assert!(Expr::int(1).is_one());
        assert!(!Expr::int(0).is_one());
    }

    #[test]
    fn is_atom() {
        assert!(Expr::int(42).is_atom());
        assert!(Expr::sym("x").is_atom());
        assert!(!Expr::add(Expr::int(1), Expr::int(2)).is_atom());
    }

    // --- Display: negative terms ---

    #[test]
    fn display_negative_coefficient() {
        // -3*x should display as -3*x
        let e = Expr::mul(Expr::int(-3), Expr::sym("x"));
        assert_eq!(e.to_string(), "-3*x");
    }

    #[test]
    fn display_sum_with_negative_term() {
        // a + (-2)*b should display as a-2*b
        let e = Expr::List {
            op: Operator::MPlus,
            simplified: true,
            args: vec![
                Expr::sym("a"),
                Expr::mul(Expr::int(-2), Expr::sym("b")),
            ],
        };
        assert_eq!(e.to_string(), "a-2*b");
    }

    #[test]
    fn display_sum_with_negative_integer() {
        let e = Expr::List {
            op: Operator::MPlus,
            simplified: true,
            args: vec![Expr::sym("x"), Expr::int(-5)],
        };
        assert_eq!(e.to_string(), "x-5");
    }

    // --- Display: operators ---

    #[test]
    fn display_equal() {
        let e = Expr::List {
            op: Operator::MEqual,
            simplified: false,
            args: vec![Expr::sym("x"), Expr::int(1)],
        };
        assert_eq!(e.to_string(), "x = 1");
    }

    #[test]
    fn display_not_equal() {
        let e = Expr::List {
            op: Operator::MNotEqual,
            simplified: false,
            args: vec![Expr::sym("x"), Expr::int(0)],
        };
        assert_eq!(e.to_string(), "x # 0");
    }

    #[test]
    fn display_less_than() {
        let e = Expr::List {
            op: Operator::MLessThan,
            simplified: false,
            args: vec![Expr::sym("a"), Expr::sym("b")],
        };
        assert_eq!(e.to_string(), "a < b");
    }

    #[test]
    fn display_greater_equal() {
        let e = Expr::List {
            op: Operator::MGreaterEqual,
            simplified: false,
            args: vec![Expr::sym("x"), Expr::int(0)],
        };
        assert_eq!(e.to_string(), "x >= 0");
    }

    #[test]
    fn display_define() {
        let e = Expr::List {
            op: Operator::MDefine,
            simplified: false,
            args: vec![
                Expr::call("f", vec![Expr::sym("x")]),
                Expr::pow(Expr::sym("x"), Expr::int(2)),
            ],
        };
        assert_eq!(e.to_string(), "f(x):=x^2");
    }

    #[test]
    fn display_assign() {
        let e = Expr::List {
            op: Operator::MAssign,
            simplified: false,
            args: vec![Expr::sym("x"), Expr::int(5)],
        };
        assert_eq!(e.to_string(), "x:5");
    }

    #[test]
    fn display_quote() {
        let e = Expr::List {
            op: Operator::MQuote,
            simplified: false,
            args: vec![Expr::call("f", vec![Expr::sym("x")])],
        };
        assert_eq!(e.to_string(), "'f(x)");
    }

    #[test]
    fn display_and_or_not() {
        let and_e = Expr::List {
            op: Operator::MAnd,
            simplified: false,
            args: vec![Expr::sym("a"), Expr::sym("b")],
        };
        assert_eq!(and_e.to_string(), "a and b");

        let or_e = Expr::List {
            op: Operator::MOr,
            simplified: false,
            args: vec![Expr::sym("a"), Expr::sym("b")],
        };
        assert_eq!(or_e.to_string(), "a or b");

        let not_e = Expr::List {
            op: Operator::MNot,
            simplified: false,
            args: vec![Expr::sym("p")],
        };
        assert_eq!(not_e.to_string(), "not p");
    }

    // --- Display: edge cases ---

    #[test]
    fn display_empty_sum() {
        let e = Expr::List {
            op: Operator::MPlus,
            simplified: false,
            args: vec![],
        };
        assert_eq!(e.to_string(), "0");
    }

    #[test]
    fn display_empty_product() {
        let e = Expr::List {
            op: Operator::MTimes,
            simplified: false,
            args: vec![],
        };
        assert_eq!(e.to_string(), "1");
    }

    #[test]
    fn display_empty_list() {
        assert_eq!(Expr::list(vec![]).to_string(), "[]");
    }

    #[test]
    fn display_nested_list() {
        let e = Expr::list(vec![
            Expr::list(vec![Expr::int(1), Expr::int(2)]),
            Expr::list(vec![Expr::int(3), Expr::int(4)]),
        ]);
        assert_eq!(e.to_string(), "[[1,2],[3,4]]");
    }

    #[test]
    fn display_bigint() {
        let big = num::BigInt::from(123456789012345i64);
        let e = Expr::BigInt(Box::new(big));
        assert_eq!(e.to_string(), "123456789012345");
    }

    #[test]
    fn display_negative_rational() {
        let e = Expr::Rational { num: -3, den: 4 };
        assert_eq!(e.to_string(), "-3/4");
    }

    // --- Constructors ---

    #[test]
    fn div_constructor() {
        let e = Expr::div(Expr::sym("a"), Expr::sym("b"));
        // a/b = a * b^(-1)
        assert!(matches!(e, Expr::List { op: Operator::MTimes, .. }));
    }

    #[test]
    fn neg_constructor() {
        let e = Expr::neg(Expr::sym("x"));
        // -x = (-1)*x
        assert!(matches!(e, Expr::List { op: Operator::MTimes, .. }));
    }

    #[test]
    fn call_zero_args() {
        let e = Expr::call("foo", vec![]);
        assert_eq!(e.to_string(), "foo()");
    }

    #[test]
    fn call_multi_args() {
        let e = Expr::call("f", vec![Expr::int(1), Expr::int(2), Expr::int(3)]);
        assert_eq!(e.to_string(), "f(1,2,3)");
    }

    // --- Equality ---

    #[test]
    fn expr_equality() {
        assert_eq!(Expr::int(42), Expr::int(42));
        assert_ne!(Expr::int(1), Expr::int(2));
        assert_eq!(Expr::sym("x"), Expr::sym("x"));
        assert_ne!(Expr::sym("x"), Expr::sym("y"));
        assert_eq!(Expr::Float(3.14), Expr::Float(3.14));
    }

    #[test]
    fn list_equality() {
        let a = Expr::list(vec![Expr::int(1), Expr::int(2)]);
        let b = Expr::list(vec![Expr::int(1), Expr::int(2)]);
        assert_eq!(a, b);

        let c = Expr::list(vec![Expr::int(1), Expr::int(3)]);
        assert_ne!(a, c);
    }
}
