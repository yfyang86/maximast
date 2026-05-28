use maxima_core::{Expr, Operator, resolve, intern};

pub(crate) fn eval_bfloat_func(name: &str, args: &[Expr], env: &mut crate::env::Environment) -> Option<Expr> {
    match name {
        "bfloat" => {
            let prec = get_fpprec_decimal(env);
            args.first().map(|a| bfloat_eval(a, prec as usize, env))
        }
        "fpprec" => {
            if args.is_empty() {
                return Some(Expr::int(get_fpprec_decimal(env)));
            }
            if let Some(Expr::Integer(n)) = args.first() {
                set_fpprec(env, *n);
                return Some(Expr::int(*n));
            }
            None
        }
        _ => None,
    }
}

fn get_fpprec_decimal(env: &crate::env::Environment) -> i64 {
    let id = intern("fpprec");
    if let Some(Expr::Integer(n)) = env.get(id) { *n } else { 16 }
}

fn set_fpprec(env: &mut crate::env::Environment, digits: i64) {
    let id = intern("fpprec");
    env.set(id, Expr::int(digits));
}

fn bfloat_eval(expr: &Expr, prec: usize, env: &mut crate::env::Environment) -> Expr {
    match expr {
        Expr::Integer(n) => Expr::Float(*n as f64),
        Expr::Rational { num, den } => Expr::Float(*num as f64 / *den as f64),
        Expr::Float(f) => Expr::Float(*f),
        Expr::Symbol(id) => {
            let name = resolve(*id);
            match name.as_str() {
                "%pi" => Expr::Float(std::f64::consts::PI),
                "%e" => Expr::Float(std::f64::consts::E),
                "%phi" => Expr::Float((1.0 + 5.0_f64.sqrt()) / 2.0),
                "%gamma" => Expr::Float(0.5772156649015329),
                _ => {
                    if let Some(val) = env.get(*id) {
                        bfloat_eval(&val.clone(), prec, env)
                    } else {
                        expr.clone()
                    }
                }
            }
        }
        Expr::List { op: Operator::MPlus, args, .. } => {
            let terms: Vec<Expr> = args.iter().map(|a| bfloat_eval(a, prec, env)).collect();
            crate::simp::simplify(&Expr::List { op: Operator::MPlus, simplified: false, args: terms })
        }
        Expr::List { op: Operator::MTimes, args, .. } => {
            let factors: Vec<Expr> = args.iter().map(|a| bfloat_eval(a, prec, env)).collect();
            crate::simp::simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: factors })
        }
        Expr::List { op: Operator::MExpt, args, .. } if args.len() == 2 => {
            let base = bfloat_eval(&args[0], prec, env);
            let exp = bfloat_eval(&args[1], prec, env);
            if let (Expr::Float(b), Expr::Float(e)) = (&base, &exp) {
                Expr::Float(b.powf(*e))
            } else if let (Expr::Float(b), Expr::Integer(e)) = (&base, &exp) {
                Expr::Float(b.powi(*e as i32))
            } else {
                crate::simp::simplify(&Expr::pow(base, exp))
            }
        }
        Expr::List { op: Operator::Named(id), args, .. } if args.len() == 1 => {
            let fname = resolve(*id);
            let inner = bfloat_eval(&args[0], prec, env);
            if let Expr::Float(x) = &inner {
                let result = match fname.as_str() {
                    "sin" => x.sin(),
                    "cos" => x.cos(),
                    "tan" => x.tan(),
                    "exp" => x.exp(),
                    "log" => x.ln(),
                    "sqrt" => x.sqrt(),
                    "asin" => x.asin(),
                    "acos" => x.acos(),
                    "atan" => x.atan(),
                    "sinh" => x.sinh(),
                    "cosh" => x.cosh(),
                    "tanh" => x.tanh(),
                    "abs" => x.abs(),
                    _ => return Expr::call(&fname, vec![inner]),
                };
                Expr::Float(result)
            } else {
                Expr::call(&fname, vec![inner])
            }
        }
        _ => expr.clone(),
    }
}
