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
            // matchdeclare(var, pred [, var2, pred2, ...]) — pairs of (var, predicate).
            // var may also be a list [v1,v2] sharing one predicate.
            if args.len() >= 2 && args.len() % 2 == 0 {
                let mut i = 0;
                while i + 1 < args.len() {
                    let pred = args[i + 1].clone();
                    match &args[i] {
                        Expr::Symbol(id) => {
                            env.pattern_state.match_vars.insert(
                                *id, PatternVar { name: *id, predicate: pred });
                        }
                        Expr::List { op: Operator::MList, args: vars, .. } => {
                            for v in vars {
                                if let Expr::Symbol(id) = v {
                                    env.pattern_state.match_vars.insert(
                                        *id, PatternVar { name: *id, predicate: pred.clone() });
                                }
                            }
                        }
                        _ => return None,
                    }
                    i += 2;
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

/// Resource caps for AC matching (prevent combinatorial abuse / DoS).
/// Legitimate patterns resolve in well under 1000 nodes thanks to
/// selectivity ordering; the budget only bounds adversarial/pathological
/// inputs, which fail gracefully (return no match) when the budget is hit.
const AC_NODE_BUDGET: u64 = 50_000;
const AC_MAX_SUBJECT: usize = 24;

/// Is `pattern` a bare top-level pattern variable (a declared match var)?
fn as_match_var(pattern: &Expr, match_vars: &HashMap<SymbolId, PatternVar>) -> Option<SymbolId> {
    if let Expr::Symbol(id) = pattern {
        if match_vars.contains_key(id) { return Some(*id); }
    }
    None
}

/// Count pattern variables appearing in an expression (for selectivity ordering).
fn count_pat_vars(expr: &Expr, match_vars: &HashMap<SymbolId, PatternVar>) -> usize {
    match expr {
        Expr::Symbol(id) if match_vars.contains_key(id) => 1,
        Expr::List { args, .. } => args.iter().map(|a| count_pat_vars(a, match_vars)).sum(),
        _ => 0,
    }
}

/// Rebuild an AC operator's result from a list of terms, handling identities.
fn rebuild_op(op: Operator, mut terms: Vec<Expr>) -> Expr {
    match terms.len() {
        0 => if op == Operator::MTimes { Expr::int(1) } else { Expr::int(0) },
        1 => terms.pop().unwrap(),
        _ => simplify(&Expr::List { op, simplified: false, args: terms }),
    }
}

/// Try to match `expr` against `pattern`, binding pattern variables.
/// Requires a complete match (no leftover) — used for predicate/defmatch contexts.
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
        (Expr::Rational { num: n1, den: d1 }, Expr::Rational { num: n2, den: d2 }) =>
            return n1 == n2 && d1 == d2,
        (Expr::String(a), Expr::String(b)) => return a == b,
        _ => {}
    }

    // List match: AC for +/*, structural otherwise
    if let (
        Expr::List { op: op1, args: args1, .. },
        Expr::List { op: op2, args: args2, .. },
    ) = (expr, pattern) {
        if op1 != op2 { return false; }
        if matches!(op1, Operator::MPlus | Operator::MTimes) {
            let mut budget = AC_NODE_BUDGET;
            return ac_match(args1, args2, *op1, match_vars, bindings, env,
                            &mut budget, true).is_some();
        }
        // Non-AC operator: ordered structural match
        if args1.len() != args2.len() { return false; }
        for (a, p) in args1.iter().zip(args2.iter()) {
            if !pattern_match(a, p, match_vars, bindings, env) { return false; }
        }
        return true;
    }

    false
}

/// Associative-commutative match of subject terms against pattern terms.
///
/// Strategy (constrained-first):
///   1. Classify pattern terms into consuming terms (each takes exactly one
///      subject term) and at most one "rest" variable (the last bare pattern
///      variable, which absorbs ≥0 leftover terms).
///   2. Assign consuming terms to subject terms via backtracking, most
///      constrained (fewest pattern vars) first.
///   3. The rest variable (if any) absorbs unused subject terms; otherwise
///      `require_complete` decides whether leftover is allowed.
///
/// On success, mutates `bindings` and returns the leftover subject terms
/// (empty unless `require_complete == false` and there is no rest variable).
fn ac_match(
    subj: &[Expr], pat: &[Expr], op: Operator,
    match_vars: &HashMap<SymbolId, PatternVar>,
    bindings: &mut HashMap<SymbolId, Expr>,
    env: &mut crate::env::Environment,
    budget: &mut u64,
    require_complete: bool,
) -> Option<Vec<Expr>> {
    if subj.len() > AC_MAX_SUBJECT { return None; }

    // Classify: consuming terms vs the rest variable.
    // The LAST bare pattern variable becomes the rest variable.
    let mut last_var_idx: Option<usize> = None;
    for (i, p) in pat.iter().enumerate() {
        if as_match_var(p, match_vars).is_some() {
            last_var_idx = Some(i);
        }
    }
    let mut consuming: Vec<&Expr> = Vec::new();
    let mut rest_var: Option<SymbolId> = None;
    for (i, p) in pat.iter().enumerate() {
        if Some(i) == last_var_idx {
            rest_var = as_match_var(p, match_vars);
        } else {
            consuming.push(p);
        }
    }

    // A rest variable can absorb leftover, so completeness is satisfied by it.
    if consuming.len() > subj.len() { return None; }

    // Selectivity: match the most-constrained pattern terms first.
    consuming.sort_by_key(|p| count_pat_vars(p, match_vars));

    let mut used = vec![false; subj.len()];
    assign_consuming(
        &consuming, 0, subj, &mut used, op, rest_var,
        match_vars, bindings, env, budget, require_complete,
    )
}

#[allow(clippy::too_many_arguments)]
fn assign_consuming(
    consuming: &[&Expr], ci: usize, subj: &[Expr], used: &mut Vec<bool>,
    op: Operator, rest_var: Option<SymbolId>,
    match_vars: &HashMap<SymbolId, PatternVar>,
    bindings: &mut HashMap<SymbolId, Expr>,
    env: &mut crate::env::Environment,
    budget: &mut u64,
    require_complete: bool,
) -> Option<Vec<Expr>> {
    if *budget == 0 { return None; }
    *budget -= 1;

    if ci == consuming.len() {
        // All consuming terms matched. Handle leftover subject terms.
        let leftover: Vec<Expr> = subj.iter().enumerate()
            .filter(|(i, _)| !used[*i]).map(|(_, e)| e.clone()).collect();
        match rest_var {
            Some(id) => {
                let cand = rebuild_op(op, leftover);
                if let Some(existing) = bindings.get(&id) {
                    return if *existing == cand { Some(Vec::new()) } else { None };
                }
                let pred = match_vars.get(&id).map(|v| v.predicate.clone())
                    .unwrap_or_else(|| Expr::sym("true"));
                if check_predicate(&cand, &pred, env) {
                    bindings.insert(id, cand);
                    Some(Vec::new())
                } else {
                    None
                }
            }
            None => {
                if require_complete && !leftover.is_empty() {
                    None
                } else {
                    Some(leftover)
                }
            }
        }
    } else {
        let p = consuming[ci];
        for si in 0..subj.len() {
            if used[si] { continue; }
            let snapshot = bindings.clone();
            used[si] = true;
            if pattern_match(&subj[si], p, match_vars, bindings, env) {
                if let Some(lo) = assign_consuming(
                    consuming, ci + 1, subj, used, op, rest_var,
                    match_vars, bindings, env, budget, require_complete,
                ) {
                    return Some(lo);
                }
            }
            used[si] = false;
            *bindings = snapshot;
        }
        None
    }
}

/// Match a rule pattern against an expression in REWRITE mode: an AC pattern
/// may match a sub-multiset of an AC subject, leaving the rest untouched.
/// Returns the rewritten expression on success.
pub(crate) fn ac_rewrite(
    expr: &Expr, pattern: &Expr, replacement: &Expr,
    match_vars: &HashMap<SymbolId, PatternVar>,
    env: &mut crate::env::Environment,
) -> Option<Expr> {
    if let (
        Expr::List { op: op1, args: subj, .. },
        Expr::List { op: op2, args: pat, .. },
    ) = (expr, pattern) {
        if op1 == op2 && matches!(op1, Operator::MPlus | Operator::MTimes) {
            let mut bindings = HashMap::new();
            let mut budget = AC_NODE_BUDGET;
            // require_complete = false → allow leftover (subset match)
            if let Some(leftover) = ac_match(
                subj, pat, *op1, match_vars, &mut bindings, env, &mut budget, false,
            ) {
                let repl = substitute_bindings(replacement, &bindings);
                let mut terms = vec![repl];
                terms.extend(leftover);
                return Some(rebuild_op(*op1, terms));
            }
            return None;
        }
    }
    // Non-AC: fall back to exact match
    let mut bindings = HashMap::new();
    if pattern_match(expr, pattern, match_vars, &mut bindings, env) {
        Some(substitute_bindings(replacement, &bindings))
    } else {
        None
    }
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
    // Rewrite mode: AC patterns may match a sub-multiset of an AC subject.
    ac_rewrite(expr, &rule.pattern, &rule.replacement, match_vars, env)
        .unwrap_or_else(|| expr.clone())
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
        Expr::List { op, args, simplified: _ } => {
            let new_args: Vec<Expr> = args.iter()
                .map(|a| substitute_bindings(a, bindings))
                .collect();
            simplify(&Expr::List { op: *op, simplified: false, args: new_args })
        }
        _ => template.clone(),
    }
}

/// Hard cap on tellsimp rewrite iterations at a single node — prevents
/// non-terminating rules (e.g. h(a) -> h(a+1)) from hanging the evaluator.
const TELLSIMP_MAX_ITERS: usize = 100;

/// Apply registered tellsimp rules at the top level of `expr`, iterating to a
/// fixpoint (bounded by TELLSIMP_MAX_ITERS). Subexpressions are handled by the
/// natural `meval` recursion, so this only needs to drive the current node.
///
/// Loop guards:
///   - fixpoint detection: stop when a pass produces no change;
///   - hard iteration cap: stop after TELLSIMP_MAX_ITERS even if still changing.
pub(crate) fn apply_tellsimp(mut expr: Expr, env: &mut crate::env::Environment) -> Expr {
    if env.pattern_state.tellsimp_rules.is_empty() {
        return expr;
    }
    for _ in 0..TELLSIMP_MAX_ITERS {
        let next = apply_tellsimp_once(&expr, env);
        if next == expr {
            return expr; // fixpoint
        }
        expr = next;
    }
    expr // iteration cap hit; return current state rather than hang
}

/// One pass of top-level tellsimp matching: try each rule keyed to the
/// expression's main operator, returning the first result that changes it.
fn apply_tellsimp_once(expr: &Expr, env: &mut crate::env::Environment) -> Expr {
    let op = match get_main_op(expr) {
        Some(o) => o,
        None => return expr.clone(),
    };
    let rules: Vec<Rule> = env.pattern_state.tellsimp_rules.iter()
        .filter(|(rule_op, _)| *rule_op == op)
        .map(|(_, r)| r.clone())
        .collect();
    if rules.is_empty() {
        return expr.clone();
    }
    let mvars = env.pattern_state.match_vars.clone();
    for rule in &rules {
        if let Some(result) = ac_rewrite(expr, &rule.pattern, &rule.replacement, &mvars, env) {
            if result != *expr {
                return result;
            }
        }
    }
    expr.clone()
}

fn get_main_op(expr: &Expr) -> Option<SymbolId> {
    match expr {
        Expr::List { op: Operator::Named(id), .. } => Some(*id),
        Expr::List { op: Operator::MPlus, .. } => Some(intern("+")),
        Expr::List { op: Operator::MTimes, .. } => Some(intern("*")),
        _ => None,
    }
}
