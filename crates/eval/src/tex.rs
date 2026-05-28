use maxima_core::{Expr, Operator, resolve};

pub fn expr_to_tex(expr: &Expr) -> String {
    match expr {
        Expr::Integer(n) => n.to_string(),
        Expr::BigInt(n) => n.to_string(),
        Expr::Float(f) => format!("{}", f),
        Expr::Rational { num, den } => format!("\\frac{{{}}}{{{}}}", num, den),
        Expr::Symbol(id) => {
            let name = resolve(*id);
            match name.as_str() {
                "%pi" => "\\pi".to_string(),
                "%e" => "e".to_string(),
                "%i" => "i".to_string(),
                "%phi" => "\\phi".to_string(),
                _ if name.len() > 1 => format!("\\mathrm{{{}}}", name),
                _ => name,
            }
        }
        Expr::String(s) => format!("\\text{{{}}}", s),
        Expr::List { op, args, .. } => match op {
            Operator::MPlus => {
                if args.is_empty() { return "0".to_string(); }
                let mut s = expr_to_tex(&args[0]);
                for arg in &args[1..] {
                    let t = expr_to_tex(arg);
                    if t.starts_with('-') {
                        s.push_str(&t);
                    } else {
                        s.push('+');
                        s.push_str(&t);
                    }
                }
                s
            }
            Operator::MTimes => {
                args.iter().map(|a| {
                    if matches!(a, Expr::List { op: Operator::MPlus, .. }) {
                        format!("\\left({}\\right)", expr_to_tex(a))
                    } else {
                        expr_to_tex(a)
                    }
                }).collect::<Vec<_>>().join(" \\, ")
            }
            Operator::MExpt if args.len() == 2 => {
                let base = if matches!(&args[0], Expr::List { .. }) && !matches!(&args[0], Expr::List { op: Operator::Named(_), .. }) {
                    format!("\\left({}\\right)", expr_to_tex(&args[0]))
                } else {
                    expr_to_tex(&args[0])
                };
                format!("{}^{{{}}}", base, expr_to_tex(&args[1]))
            }
            Operator::MList => {
                let items: Vec<String> = args.iter().map(|a| expr_to_tex(a)).collect();
                format!("\\left[{}\\right]", items.join(" , "))
            }
            Operator::MMatrix => {
                let rows: Vec<String> = args.iter().map(|row| {
                    if let Expr::List { args: cols, .. } = row {
                        cols.iter().map(|c| expr_to_tex(c)).collect::<Vec<_>>().join(" & ")
                    } else {
                        expr_to_tex(row)
                    }
                }).collect();
                format!("\\begin{{pmatrix}} {} \\end{{pmatrix}}", rows.join(" \\\\ "))
            }
            Operator::Named(id) => {
                let fname = resolve(*id);
                match fname.as_str() {
                    "sqrt" if args.len() == 1 => format!("\\sqrt{{{}}}", expr_to_tex(&args[0])),
                    "sin" | "cos" | "tan" | "log" | "exp" if args.len() == 1 => {
                        format!("\\{}\\left({}\\right)", fname, expr_to_tex(&args[0]))
                    }
                    "abs" if args.len() == 1 => format!("\\left|{}\\right|", expr_to_tex(&args[0])),
                    "factorial" if args.len() == 1 => format!("{}!", expr_to_tex(&args[0])),
                    _ => {
                        let fargs: Vec<String> = args.iter().map(|a| expr_to_tex(a)).collect();
                        format!("\\mathrm{{{}}}\\left({}\\right)", fname, fargs.join(","))
                    }
                }
            }
            Operator::MEqual => format!("{} = {}", expr_to_tex(&args[0]), expr_to_tex(&args[1])),
            _ => expr.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tex_integer() {
        assert_eq!(expr_to_tex(&Expr::int(42)), "42");
    }

    #[test]
    fn tex_rational() {
        let e = Expr::Rational { num: 3, den: 4 };
        assert_eq!(expr_to_tex(&e), "\\frac{3}{4}");
    }

    #[test]
    fn tex_symbol_pi() {
        assert_eq!(expr_to_tex(&Expr::sym("%pi")), "\\pi");
    }

    #[test]
    fn tex_power() {
        let e = Expr::pow(Expr::sym("x"), Expr::int(2));
        assert_eq!(expr_to_tex(&e), "x^{2}");
    }

    #[test]
    fn tex_sqrt() {
        let e = Expr::call("sqrt", vec![Expr::sym("x")]);
        assert_eq!(expr_to_tex(&e), "\\sqrt{x}");
    }

    #[test]
    fn tex_sin() {
        let e = Expr::call("sin", vec![Expr::sym("x")]);
        assert_eq!(expr_to_tex(&e), "\\sin\\left(x\\right)");
    }

    #[test]
    fn tex_abs() {
        let e = Expr::call("abs", vec![Expr::sym("x")]);
        assert_eq!(expr_to_tex(&e), "\\left|x\\right|");
    }

    #[test]
    fn tex_list() {
        let e = Expr::list(vec![Expr::int(1), Expr::int(2)]);
        assert_eq!(expr_to_tex(&e), "\\left[1 , 2\\right]");
    }

    #[test]
    fn tex_equation() {
        let e = Expr::List {
            op: Operator::MEqual,
            simplified: false,
            args: vec![Expr::sym("x"), Expr::int(1)],
        };
        assert_eq!(expr_to_tex(&e), "x = 1");
    }
}
