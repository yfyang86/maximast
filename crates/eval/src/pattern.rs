use std::collections::HashMap;
use maxima_core::{Expr, Operator, SymbolId, resolve, intern};
use crate::simp::simplify;
use crate::eval::meval;

/// A pattern variable declared via matchdeclare(var, predicate).
#[derive(Clone)]
pub struct PatternVar {
    pub name: SymbolId,
    pub predicate: Expr, // predicate function or `true` (match anything)
}

/// A rewrite rule: defrule(name, pattern, replacement).
#[derive(Clone)]
pub struct Rule {
    pub name: SymbolId,
    pub pattern: Expr,
    pub replacement: Expr,
}

/// Pattern matching state stored in Environment.
#[derive(Default, Clone)]
pub struct PatternState {
    pub match_vars: HashMap<SymbolId, PatternVar>,
    pub rules: HashMap<SymbolId, Rule>,
    pub tellsimp_rules: Vec<(SymbolId, Rule)>,  // (operator, rule)
}

pub(crate) fn eval_pattern_func(
    name: &str, args: &[Expr], env: &mut crate::env::Environment,
) -> Option<Expr> {
    match name {
        "matchdeclare" => {
            // matchdeclare(var, predicate) or matchdeclare([v1,v2], pred)
            if args.len() >= 2 {
                let pred = args[1].clone();
                match &args[0] {
                    Expr::Symbol(id) => {
                        env.pattern_state.match_vars.insert(*id, PatternVar { name: *id, predicate: pred });
                    }
                    Expr::List { op: Operator::MList, args: vars, .. } => {
                        for v in vars {
                            if let Expr::Symbol(id) = v {
                                env.pattern_state.match_vars.insert(*id, PatternVar { name: *id, predicate: pred.clone() });
                            }
                        }
                    }
                    _ => return None,
                }
                Some(Expr::sym("done"))
            } else { None }
        }
        "defrule" => {
            // defrule(name, pattern, replacement)
            if args.len() == 3 {
                if let Expr::Symbol(rule_name) = &args[0] {
                    let rule = Rule {
                        name: *rule_name,
                        pattern: args[1].clone(),
                        replacement: args[2].clone(),
                    };
                    env.pattern_state.rules.insert(*rule_name, rule);
                    return Some(Expr::sym("done"));
                }
            }
            None
        }
        "apply1" | "applyb1" => {
            if args.len() >= 2 {
                let mut expr = args[0].clone();
                let bottom_up = name == "applyb1";
                for rule_arg in &args[1..] {
                    if let Expr::Symbol(rule_id) = rule_arg {
                        let rule = env.pattern_state.rules.get(rule_id).cloned();
                        let mvars = env.pattern_state.match_vars.clone();
                        if let Some(rule) = rule {
                            expr = apply_rule(&expr, &rule, &mvars, env, bottom_up);
                        }
                    }
                }
                return Some(expr);
            }
            None
        }
        "tellsimp" => {
            // tellsimp(pattern, replacement) — add simplification rule
            if args.len() == 2 {
                let op_id = get_main_op(&args[0]).unwrap_or(intern("_unknown_"));
                let rule = Rule {
                    name: intern("_tellsimp_"),
                    pattern: args[0].clone(),
                    replacement: args[1].clone(),
                };
                env.pattern_state.tellsimp_rules.push((op_id, rule));
                return Some(Expr::sym("done"));
            }
            None
        }
        "tellsimpafter" => {
            // Same as tellsimp for now (simplified implementation)
            if args.len() == 2 {
                let op_id = get_main_op(&args[0]).unwrap_or(intern("_unknown_"));
                let rule = Rule {
                    name: intern("_tellsimpafter_"),
                    pattern: args[0].clone(),
                    replacement: args[1].clone(),
                };
                env.pattern_state.tellsimp_rules.push((op_id, rule));
                return Some(Expr::sym("done"));
            }
            None
        }
        _ => None,
    }
}

/// Try to match `expr` against `pattern`, binding pattern variables.
pub(crate) fn pattern_match(
    expr: &Expr, pattern: &Expr,
    match_vars: &HashMap<SymbolId, PatternVar>,
    bindings: &mut HashMap<SymbolId, Expr>,
    env: &mut crate::env::Environment,
) -> bool {
    // Pattern variable: matches anything satisfying the predicate
    if let Expr::Symbol(id) = pattern {
        if let Some(pv) = match_vars.get(id) {
            if let Some(existing) = bindings.get(id) {
                return *existing == *expr;
            }
            if check_predicate(expr, &pv.predicate, env) {
                bindings.insert(*id, expr.clone());
                return true;
            }
            return false;
        }
    }

    // Exact match for atoms
    match (expr, pattern) {
        (Expr::Integer(a), Expr::Integer(b)) => return *a == *b,
        (Expr::Float(a), Expr::Float(b)) => return *a == *b,
        (Expr::Symbol(a), Expr::Symbol(b)) => return *a == *b,
        _ => {}
    }

    // Structural match for lists
    if let (
        Expr::List { op: op1, args: args1, .. },
        Expr::List { op: op2, args: args2, .. },
    ) = (expr, pattern) {
        if op1 != op2 || args1.len() != args2.len() { return false; }
        for (a, p) in args1.iter().zip(args2.iter()) {
            if !pattern_match(a, p, match_vars, bindings, env) { return false; }
        }
        return true;
    }

    false
}

fn check_predicate(expr: &Expr, pred: &Expr, env: &mut crate::env::Environment) -> bool {
    if *pred == Expr::sym("true") { return true; }
    if let Expr::Symbol(id) = pred {
        let fname = resolve(*id);
        match fname.as_str() {
            "integerp" => return matches!(expr, Expr::Integer(_)),
            "floatnump" => return matches!(expr, Expr::Float(_)),
            "numberp" => return matches!(expr, Expr::Integer(_) | Expr::Float(_) | Expr::Rational{..}),
            "atom" | "atomp" => return !matches!(expr, Expr::List{..}),
            _ => {}
        }
    }
    // Try calling predicate as function
    let test = meval(&Expr::List {
        op: Operator::Named(intern("funapply")),
        simplified: false,
        args: vec![pred.clone(), expr.clone()],
    }, env);
    test == Expr::sym("true")
}

fn apply_rule(
    expr: &Expr, rule: &Rule,
    match_vars: &HashMap<SymbolId, PatternVar>,
    env: &mut crate::env::Environment,
    bottom_up: bool,
) -> Expr {
    if bottom_up {
        // Bottom-up: recurse into children first
        let processed = match expr {
            Expr::List { op, args, simplified } => {
                let new_args: Vec<Expr> = args.iter()
                    .map(|a| apply_rule(a, rule, match_vars, env, true))
                    .collect();
                Expr::List { op: *op, simplified: *simplified, args: new_args }
            }
            _ => expr.clone(),
        };
        try_match_and_replace(&processed, rule, match_vars, env)
    } else {
        // Top-down: try at current node first
        let result = try_match_and_replace(expr, rule, match_vars, env);
        match &result {
            Expr::List { op, args, simplified } => {
                let new_args: Vec<Expr> = args.iter()
                    .map(|a| apply_rule(a, rule, match_vars, env, false))
                    .collect();
                Expr::List { op: *op, simplified: *simplified, args: new_args }
            }
            _ => result,
        }
    }
}

fn try_match_and_replace(
    expr: &Expr, rule: &Rule,
    match_vars: &HashMap<SymbolId, PatternVar>,
    env: &mut crate::env::Environment,
) -> Expr {
    let mut bindings = HashMap::new();
    if pattern_match(expr, &rule.pattern, match_vars, &mut bindings, env) {
        substitute_bindings(&rule.replacement, &bindings)
    } else {
        expr.clone()
    }
}

fn substitute_bindings(template: &Expr, bindings: &HashMap<SymbolId, Expr>) -> Expr {
    match template {
        Expr::Symbol(id) => {
            if let Some(val) = bindings.get(id) {
                val.clone()
            } else {
                template.clone()
            }
        }
        Expr::List { op, args, simplified } => {
            let new_args: Vec<Expr> = args.iter()
                .map(|a| substitute_bindings(a, bindings))
                .collect();
            simplify(&Expr::List { op: *op, simplified: false, args: new_args })
        }
        _ => template.clone(),
    }
}

fn get_main_op(expr: &Expr) -> Option<SymbolId> {
    match expr {
        Expr::List { op: Operator::Named(id), .. } => Some(*id),
        Expr::List { op: Operator::MPlus, .. } => Some(intern("+")),
        Expr::List { op: Operator::MTimes, .. } => Some(intern("*")),
        _ => None,
    }
}
