use maxima_core::{Expr, Operator, SymbolId, intern, resolve};
use crate::helpers::{contains_var, subst};
use crate::simp::simplify;

/// A transcendental extension in the differential field tower.
#[derive(Debug, Clone)]
pub enum Extension {
    Primitive { log_arg: Expr },
    Exponential { exp_arg: Expr },
}

/// The differential field tower: Q(x) ⊂ Q(x, t1) ⊂ Q(x, t1, t2) ⊂ ...
#[derive(Debug, Clone)]
pub struct Tower {
    pub var: Expr,
    pub extensions: Vec<(SymbolId, Extension, Expr)>, // (tower_var, type, derivative)
}

impl Tower {
    pub fn new(var: Expr) -> Self {
        Tower { var, extensions: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.extensions.is_empty()
    }

    /// Rewrite an expression by replacing log/exp subexpressions with tower variables.
    pub fn rewrite(&self, expr: &Expr) -> Expr {
        let mut result = expr.clone();
        // Apply substitutions in reverse order (innermost first)
        for (tid, ext, _) in self.extensions.iter().rev() {
            let original = match ext {
                Extension::Primitive { log_arg } => Expr::call("log", vec![log_arg.clone()]),
                Extension::Exponential { exp_arg } => Expr::call("exp", vec![exp_arg.clone()]),
            };
            result = subst(&Expr::Symbol(*tid), &original, &result);
        }
        result
    }

    /// Compute the derivative of a tower variable.
    pub fn deriv_of(&self, tid: SymbolId) -> Option<&Expr> {
        self.extensions.iter()
            .find(|(id, _, _)| *id == tid)
            .map(|(_, _, d)| d)
    }
}

/// Build a differential field tower from an expression.
/// Scans for log/exp subexpressions and orders them by dependency.
pub fn build_tower(expr: &Expr, var: &Expr) -> Tower {
    let mut tower = Tower::new(var.clone());
    let mut log_exps = Vec::new();
    collect_transcendentals(expr, var, &mut log_exps);
    log_exps.dedup_by(|a, b| a == b);

    // Sort: simpler (fewer nested log/exp) first
    log_exps.sort_by_key(|e| transcendental_depth(e));

    let mut counter = 0;
    for te in &log_exps {
        match te {
            Expr::List { op: Operator::Named(id), args, .. } if args.len() == 1 => {
                let fname = resolve(*id);
                let arg = &args[0];

                // Check that the argument only depends on previously defined tower vars + base var
                let tid = intern(&format!("_t{}_", counter));
                counter += 1;

                match fname.as_str() {
                    "log" => {
                        // t = log(arg), t' = arg'/arg
                        let arg_deriv = crate::eval::diff_once_pub(arg, var);
                        let deriv = simplify(&Expr::div(arg_deriv, arg.clone()));
                        // Rewrite the derivative in terms of tower vars
                        let deriv_rewritten = tower.rewrite(&deriv);
                        tower.extensions.push((tid, Extension::Primitive { log_arg: arg.clone() }, deriv_rewritten));
                    }
                    "exp" => {
                        // t = exp(arg), t' = arg' * t
                        let arg_deriv = crate::eval::diff_once_pub(arg, var);
                        let deriv = simplify(&Expr::mul(arg_deriv, te.clone()));
                        let deriv_rewritten = tower.rewrite(&deriv);
                        tower.extensions.push((tid, Extension::Exponential { exp_arg: arg.clone() }, deriv_rewritten));
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    tower
}

/// Collect all log/exp subexpressions that depend on var.
fn collect_transcendentals(expr: &Expr, var: &Expr, out: &mut Vec<Expr>) {
    match expr {
        Expr::List { op: Operator::Named(id), args, .. } if args.len() == 1 => {
            let fname = resolve(*id);
            if (fname == "log" || fname == "exp") && contains_var(&args[0], var) {
                // First collect from arguments (inner transcendentals)
                collect_transcendentals(&args[0], var, out);
                // Then add this expression
                if !out.contains(expr) {
                    out.push(expr.clone());
                }
            } else {
                collect_transcendentals(&args[0], var, out);
            }
        }
        Expr::List { args, .. } => {
            for arg in args {
                collect_transcendentals(arg, var, out);
            }
        }
        _ => {}
    }
}

/// Count the nesting depth of transcendental functions.
fn transcendental_depth(expr: &Expr) -> usize {
    match expr {
        Expr::List { op: Operator::Named(id), args, .. } if args.len() == 1 => {
            let fname = resolve(*id);
            if fname == "log" || fname == "exp" {
                1 + transcendental_depth(&args[0])
            } else {
                transcendental_depth(&args[0])
            }
        }
        Expr::List { args, .. } => args.iter().map(|a| transcendental_depth(a)).max().unwrap_or(0),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tower_log_x() {
        let x = Expr::sym("x");
        let expr = Expr::call("log", vec![x.clone()]);
        let tower = build_tower(&expr, &x);
        assert_eq!(tower.extensions.len(), 1);
        assert!(matches!(&tower.extensions[0].1, Extension::Primitive { .. }));
    }

    #[test]
    fn tower_exp_x() {
        let x = Expr::sym("x");
        let expr = Expr::call("exp", vec![x.clone()]);
        let tower = build_tower(&expr, &x);
        assert_eq!(tower.extensions.len(), 1);
        assert!(matches!(&tower.extensions[0].1, Extension::Exponential { .. }));
    }

    #[test]
    fn tower_mixed() {
        let x = Expr::sym("x");
        let expr = Expr::mul(
            Expr::call("log", vec![x.clone()]),
            Expr::call("exp", vec![x.clone()]),
        );
        let tower = build_tower(&expr, &x);
        assert_eq!(tower.extensions.len(), 2);
    }

    #[test]
    fn tower_nested_log() {
        let x = Expr::sym("x");
        let expr = Expr::call("log", vec![Expr::call("log", vec![x.clone()])]);
        let tower = build_tower(&expr, &x);
        assert_eq!(tower.extensions.len(), 2, "log(log(x)) needs 2 extensions");
        // First extension should be log(x), second log(t0)
        assert!(matches!(&tower.extensions[0].1, Extension::Primitive { .. }));
        assert!(matches!(&tower.extensions[1].1, Extension::Primitive { .. }));
    }

    #[test]
    fn tower_rewrite() {
        let x = Expr::sym("x");
        let expr = Expr::mul(x.clone(), Expr::call("log", vec![x.clone()]));
        let tower = build_tower(&expr, &x);
        let rewritten = tower.rewrite(&expr);
        let s = rewritten.to_string();
        assert!(s.contains("_t0_"), "should contain tower var, got: {}", s);
        assert!(!s.contains("log"), "should not contain log, got: {}", s);
    }
}
