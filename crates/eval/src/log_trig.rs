use maxima_core::{Expr, Operator, resolve};
use crate::simp::simplify;
use crate::helpers::contains_var;

pub(crate) fn eval_log_trig(name: &str, args: &[Expr]) -> Option<Expr> {
    match name {
        "logcontract" => args.first().map(|a| logcontract(a)),
        "logexpand" => args.first().map(|a| logexpand(a)),
        _ => None,
    }
}

fn logcontract(expr: &Expr) -> Expr {
    match expr {
        Expr::List { op: Operator::MPlus, args, .. } => {
            // Collect log terms and non-log terms
            let mut log_args = Vec::new();
            let mut other = Vec::new();
            for a in args {
                let contracted = logcontract(a);
                if let Some(inner) = extract_log_term(&contracted) {
                    log_args.push(inner);
                } else {
                    other.push(contracted);
                }
            }
            if log_args.len() <= 1 && other.is_empty() && log_args.len() + args.len() == args.len() {
                return expr.clone();
            }
            if !log_args.is_empty() {
                let product = if log_args.len() == 1 {
                    log_args.pop().unwrap()
                } else {
                    simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: log_args })
                };
                other.push(Expr::call("log", vec![product]));
            }
            if other.len() == 1 { other.pop().unwrap() }
            else { simplify(&Expr::List { op: Operator::MPlus, simplified: false, args: other }) }
        }
        // n*log(x) → log(x^n)
        Expr::List { op: Operator::MTimes, args, .. } => {
            let mut log_idx = None;
            let mut coeff_parts = Vec::new();
            for (i, a) in args.iter().enumerate() {
                if log_idx.is_none() {
                    if let Expr::List { op: Operator::Named(id), args: la, .. } = a {
                        if resolve(*id) == "log" && la.len() == 1 {
                            log_idx = Some(i);
                            continue;
                        }
                    }
                }
                coeff_parts.push(logcontract(a));
            }
            if let Some(idx) = log_idx {
                if let Expr::List { op: Operator::Named(_), args: la, .. } = &args[idx] {
                    let inner = &la[0];
                    let coeff = if coeff_parts.len() == 1 {
                        coeff_parts[0].clone()
                    } else {
                        simplify(&Expr::List { op: Operator::MTimes, simplified: false, args: coeff_parts })
                    };
                    return Expr::call("log", vec![simplify(&Expr::pow(inner.clone(), coeff))]);
                }
            }
            expr.clone()
        }
        _ => expr.clone(),
    }
}

fn extract_log_term(expr: &Expr) -> Option<Expr> {
    if let Expr::List { op: Operator::Named(id), args, .. } = expr {
        if resolve(*id) == "log" && args.len() == 1 {
            return Some(args[0].clone());
        }
    }
    None
}

fn logexpand(expr: &Expr) -> Expr {
    match expr {
        Expr::List { op: Operator::Named(id), args, .. }
            if resolve(*id) == "log" && args.len() == 1 =>
        {
            let inner = &args[0];
            match inner {
                // log(a*b) → log(a) + log(b)
                Expr::List { op: Operator::MTimes, args: factors, .. } => {
                    let terms: Vec<Expr> = factors.iter()
                        .map(|f| logexpand(&Expr::call("log", vec![f.clone()])))
                        .collect();
                    simplify(&Expr::List { op: Operator::MPlus, simplified: false, args: terms })
                }
                // log(a/b) → log(a) - log(b) — handled via a*b^(-1)
                // log(a^n) → n*log(a)
                Expr::List { op: Operator::MExpt, args: pa, .. } if pa.len() == 2 => {
                    let base_log = logexpand(&Expr::call("log", vec![pa[0].clone()]));
                    simplify(&Expr::mul(pa[1].clone(), base_log))
                }
                _ => expr.clone(),
            }
        }
        Expr::List { op, args, simplified } => {
            let new_args: Vec<Expr> = args.iter().map(|a| logexpand(a)).collect();
            Expr::List { op: *op, simplified: *simplified, args: new_args }
        }
        _ => expr.clone(),
    }
}
