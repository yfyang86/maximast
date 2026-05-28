use maxima_core::Expr;
use super::sign::Sign;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    LessThan,
    LessEqual,
    Equal,
    GreaterEqual,
    GreaterThan,
    NotEqual,
}

#[derive(Debug, Clone)]
pub struct Fact {
    pub lhs: Expr,
    pub rel: Relation,
    pub rhs: Expr,
}

#[derive(Debug, Clone)]
pub struct Context {
    pub name: String,
    pub facts: Vec<Fact>,
}

pub struct AssumptionDB {
    contexts: Vec<Context>,
    active: Vec<usize>,
    /// Property declarations: (symbol, property) pairs
    /// e.g., (x, "integer"), (n, "even")
    properties: Vec<(Expr, String)>,
}

impl AssumptionDB {
    pub fn new() -> Self {
        let initial = Context {
            name: "initial".to_string(),
            facts: Vec::new(),
        };
        AssumptionDB {
            contexts: vec![initial],
            active: vec![0],
            properties: Vec::new(),
        }
    }

    pub fn assume(&mut self, fact: Fact) -> &'static str {
        // Check for redundancy
        if self.is_known(&fact) {
            return "redundant";
        }
        // Add to the current active context
        let ctx_idx = *self.active.last().unwrap();
        self.contexts[ctx_idx].facts.push(fact);
        "done"
    }

    pub fn forget(&mut self, lhs: &Expr, rel: Relation, rhs: &Expr) {
        for ctx_idx in &self.active {
            self.contexts[*ctx_idx].facts.retain(|f| {
                !(f.lhs == *lhs && f.rel == rel && f.rhs == *rhs)
            });
        }
    }

    pub fn facts(&self) -> Vec<&Fact> {
        let mut result = Vec::new();
        for ctx_idx in &self.active {
            for fact in &self.contexts[*ctx_idx].facts {
                result.push(fact);
            }
        }
        result
    }

    pub fn is_known(&self, query: &Fact) -> bool {
        // Direct lookup
        for ctx_idx in &self.active {
            for fact in &self.contexts[*ctx_idx].facts {
                if fact.lhs == query.lhs && fact.rhs == query.rhs && fact.rel == query.rel {
                    return true;
                }
                // Derive: x > 0 implies x >= 0
                if fact.lhs == query.lhs && fact.rhs == query.rhs {
                    match (fact.rel, query.rel) {
                        (Relation::GreaterThan, Relation::GreaterEqual) => return true,
                        (Relation::LessThan, Relation::LessEqual) => return true,
                        (Relation::Equal, Relation::GreaterEqual) => return true,
                        (Relation::Equal, Relation::LessEqual) => return true,
                        _ => {}
                    }
                }
            }
        }
        false
    }

    /// Query whether a relation holds. Returns true/false/None(unknown).
    pub fn query(&self, lhs: &Expr, rel: Relation, rhs: &Expr) -> Option<bool> {
        let fact = Fact { lhs: lhs.clone(), rel, rhs: rhs.clone() };
        if self.is_known(&fact) {
            return Some(true);
        }
        // Check negation
        let neg_rel = negate_relation(rel);
        if let Some(neg) = neg_rel {
            let neg_fact = Fact { lhs: lhs.clone(), rel: neg, rhs: rhs.clone() };
            if self.is_known(&neg_fact) {
                return Some(false);
            }
        }
        // Transitive inference: a < b and b < c implies a < c
        if matches!(rel, Relation::LessThan | Relation::LessEqual
            | Relation::GreaterThan | Relation::GreaterEqual) {
            if self.transitive_check(lhs, rel, rhs) {
                return Some(true);
            }
        }
        None
    }

    fn transitive_check(&self, lhs: &Expr, rel: Relation, rhs: &Expr) -> bool {
        // For a < c, look for some b where a < b and b < c
        let all_facts = self.facts();
        for fact in &all_facts {
            match rel {
                Relation::LessThan | Relation::LessEqual => {
                    // lhs < ? and ? < rhs
                    if fact.lhs == *lhs && matches!(fact.rel, Relation::LessThan | Relation::LessEqual) {
                        let mid = &fact.rhs;
                        for f2 in &all_facts {
                            if f2.lhs == *mid && f2.rhs == *rhs
                                && matches!(f2.rel, Relation::LessThan | Relation::LessEqual) {
                                return true;
                            }
                        }
                    }
                }
                Relation::GreaterThan | Relation::GreaterEqual => {
                    if fact.lhs == *lhs && matches!(fact.rel, Relation::GreaterThan | Relation::GreaterEqual) {
                        let mid = &fact.rhs;
                        for f2 in &all_facts {
                            if f2.lhs == *mid && f2.rhs == *rhs
                                && matches!(f2.rel, Relation::GreaterThan | Relation::GreaterEqual) {
                                return true;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Get the sign of an expression from assumptions.
    pub fn get_sign(&self, expr: &Expr) -> Sign {
        let zero = Expr::int(0);
        // Check: expr > 0?
        if self.is_known(&Fact { lhs: expr.clone(), rel: Relation::GreaterThan, rhs: zero.clone() }) {
            return Sign::Pos;
        }
        // Check: expr < 0?
        if self.is_known(&Fact { lhs: expr.clone(), rel: Relation::LessThan, rhs: zero.clone() }) {
            return Sign::Neg;
        }
        // Check: expr >= 0?
        if self.is_known(&Fact { lhs: expr.clone(), rel: Relation::GreaterEqual, rhs: zero.clone() }) {
            return Sign::Poz;
        }
        // Check: expr <= 0?
        if self.is_known(&Fact { lhs: expr.clone(), rel: Relation::LessEqual, rhs: zero.clone() }) {
            return Sign::Noz;
        }
        Sign::Pnz
    }

    pub fn new_context(&mut self, name: &str) -> usize {
        let ctx = Context {
            name: name.to_string(),
            facts: Vec::new(),
        };
        let idx = self.contexts.len();
        self.contexts.push(ctx);
        self.active.push(idx);
        idx
    }

    pub fn kill_context(&mut self, name: &str) {
        if let Some(idx) = self.contexts.iter().position(|c| c.name == name) {
            if idx > 0 {
                self.active.retain(|&i| i != idx);
                self.contexts[idx].facts.clear();
            }
        }
    }

    pub fn clear(&mut self) {
        for ctx in &mut self.contexts {
            ctx.facts.clear();
        }
        self.properties.clear();
    }

    pub fn declare_property(&mut self, sym: &Expr, prop: &str) {
        if !self.properties.iter().any(|(s, p)| s == sym && p == prop) {
            self.properties.push((sym.clone(), prop.to_string()));
        }
    }

    pub fn has_property(&self, sym: &Expr, prop: &str) -> bool {
        self.properties.iter().any(|(s, p)| s == sym && p == prop)
    }

    pub fn remove_property(&mut self, sym: &Expr, prop: &str) {
        self.properties.retain(|(s, p)| !(s == sym && p == prop));
    }

    pub fn list_properties(&self, sym: &Expr) -> Vec<String> {
        self.properties.iter()
            .filter(|(s, _)| s == sym)
            .map(|(_, p)| p.clone())
            .collect()
    }

    pub fn activate_context(&mut self, name: &str) {
        if let Some(idx) = self.contexts.iter().position(|c| c.name == name) {
            if !self.active.contains(&idx) {
                self.active.push(idx);
            }
        }
    }

    pub fn deactivate_context(&mut self, name: &str) {
        if let Some(idx) = self.contexts.iter().position(|c| c.name == name) {
            if idx > 0 {
                self.active.retain(|&i| i != idx);
            }
        }
    }
}

impl Default for AssumptionDB {
    fn default() -> Self {
        Self::new()
    }
}

fn negate_relation(rel: Relation) -> Option<Relation> {
    match rel {
        Relation::GreaterThan => Some(Relation::LessEqual),
        Relation::LessThan => Some(Relation::GreaterEqual),
        Relation::GreaterEqual => Some(Relation::LessThan),
        Relation::LessEqual => Some(Relation::GreaterThan),
        Relation::Equal => Some(Relation::NotEqual),
        Relation::NotEqual => Some(Relation::Equal),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assume_and_query() {
        let mut db = AssumptionDB::new();
        let x = Expr::sym("x");
        let zero = Expr::int(0);

        db.assume(Fact { lhs: x.clone(), rel: Relation::GreaterThan, rhs: zero.clone() });

        assert_eq!(db.query(&x, Relation::GreaterThan, &zero), Some(true));
        assert_eq!(db.query(&x, Relation::GreaterEqual, &zero), Some(true));
        assert_eq!(db.get_sign(&x), Sign::Pos);
    }

    #[test]
    fn forget_removes_fact() {
        let mut db = AssumptionDB::new();
        let x = Expr::sym("x");
        let zero = Expr::int(0);

        db.assume(Fact { lhs: x.clone(), rel: Relation::GreaterThan, rhs: zero.clone() });
        assert_eq!(db.get_sign(&x), Sign::Pos);

        db.forget(&x, Relation::GreaterThan, &zero);
        assert_eq!(db.get_sign(&x), Sign::Pnz);
    }

    #[test]
    fn facts_listing() {
        let mut db = AssumptionDB::new();
        let x = Expr::sym("x");
        let y = Expr::sym("y");
        let zero = Expr::int(0);

        db.assume(Fact { lhs: x.clone(), rel: Relation::GreaterThan, rhs: zero.clone() });
        db.assume(Fact { lhs: y.clone(), rel: Relation::LessThan, rhs: zero.clone() });

        assert_eq!(db.facts().len(), 2);
    }

    #[test]
    fn context_isolation() {
        let mut db = AssumptionDB::new();
        let x = Expr::sym("x");
        let zero = Expr::int(0);

        db.new_context("test");
        db.assume(Fact { lhs: x.clone(), rel: Relation::GreaterThan, rhs: zero.clone() });
        assert_eq!(db.get_sign(&x), Sign::Pos);

        db.kill_context("test");
        assert_eq!(db.get_sign(&x), Sign::Pnz);
    }

    #[test]
    fn redundant_detection() {
        let mut db = AssumptionDB::new();
        let x = Expr::sym("x");
        let zero = Expr::int(0);

        let r1 = db.assume(Fact { lhs: x.clone(), rel: Relation::GreaterThan, rhs: zero.clone() });
        assert_eq!(r1, "done");

        let r2 = db.assume(Fact { lhs: x.clone(), rel: Relation::GreaterThan, rhs: zero.clone() });
        assert_eq!(r2, "redundant");
    }

    // --- Comprehensive database tests ---

    #[test]
    fn query_negation() {
        let mut db = AssumptionDB::new();
        let x = Expr::sym("db_neg_x");
        let zero = Expr::int(0);

        db.assume(Fact { lhs: x.clone(), rel: Relation::GreaterThan, rhs: zero.clone() });
        // x > 0 implies NOT x <= 0
        assert_eq!(db.query(&x, Relation::LessEqual, &zero), Some(false));
    }

    #[test]
    fn sign_less_than() {
        let mut db = AssumptionDB::new();
        let x = Expr::sym("db_lt_x");
        let zero = Expr::int(0);

        db.assume(Fact { lhs: x.clone(), rel: Relation::LessThan, rhs: zero.clone() });
        assert_eq!(db.get_sign(&x), Sign::Neg);
    }

    #[test]
    fn sign_geq() {
        let mut db = AssumptionDB::new();
        let x = Expr::sym("db_geq_x");
        let zero = Expr::int(0);

        db.assume(Fact { lhs: x.clone(), rel: Relation::GreaterEqual, rhs: zero.clone() });
        assert_eq!(db.get_sign(&x), Sign::Poz);
    }

    #[test]
    fn sign_leq() {
        let mut db = AssumptionDB::new();
        let x = Expr::sym("db_leq_x");
        let zero = Expr::int(0);

        db.assume(Fact { lhs: x.clone(), rel: Relation::LessEqual, rhs: zero.clone() });
        assert_eq!(db.get_sign(&x), Sign::Noz);
    }

    #[test]
    fn unknown_symbol() {
        let db = AssumptionDB::new();
        let y = Expr::sym("db_unknown");
        assert_eq!(db.get_sign(&y), Sign::Pnz);
    }

    #[test]
    fn multiple_contexts() {
        let mut db = AssumptionDB::new();
        let x = Expr::sym("db_ctx_x");
        let y = Expr::sym("db_ctx_y");
        let zero = Expr::int(0);

        db.assume(Fact { lhs: x.clone(), rel: Relation::GreaterThan, rhs: zero.clone() });
        db.new_context("c2");
        db.assume(Fact { lhs: y.clone(), rel: Relation::LessThan, rhs: zero.clone() });

        // Both should be visible
        assert_eq!(db.get_sign(&x), Sign::Pos);
        assert_eq!(db.get_sign(&y), Sign::Neg);

        db.kill_context("c2");
        assert_eq!(db.get_sign(&x), Sign::Pos); // still there
        assert_eq!(db.get_sign(&y), Sign::Pnz); // gone
    }

    #[test]
    fn clear_all() {
        let mut db = AssumptionDB::new();
        let x = Expr::sym("db_clr_x");
        let zero = Expr::int(0);
        db.assume(Fact { lhs: x.clone(), rel: Relation::GreaterThan, rhs: zero.clone() });
        db.clear();
        assert_eq!(db.get_sign(&x), Sign::Pnz);
        assert!(db.facts().is_empty());
    }

    #[test]
    fn query_unrelated() {
        let db = AssumptionDB::new();
        let x = Expr::sym("db_unrel_x");
        let y = Expr::sym("db_unrel_y");
        assert_eq!(db.query(&x, Relation::GreaterThan, &y), None);
    }

    #[test]
    fn derived_gt_implies_geq() {
        let mut db = AssumptionDB::new();
        let x = Expr::sym("db_der_x");
        let zero = Expr::int(0);
        db.assume(Fact { lhs: x.clone(), rel: Relation::GreaterThan, rhs: zero.clone() });
        assert_eq!(db.query(&x, Relation::GreaterEqual, &zero), Some(true));
    }

    #[test]
    fn derived_lt_implies_leq() {
        let mut db = AssumptionDB::new();
        let x = Expr::sym("db_der2_x");
        let zero = Expr::int(0);
        db.assume(Fact { lhs: x.clone(), rel: Relation::LessThan, rhs: zero.clone() });
        assert_eq!(db.query(&x, Relation::LessEqual, &zero), Some(true));
    }

    #[test]
    fn derived_eq_implies_geq_leq() {
        let mut db = AssumptionDB::new();
        let x = Expr::sym("db_der3_x");
        let y = Expr::sym("db_der3_y");
        db.assume(Fact { lhs: x.clone(), rel: Relation::Equal, rhs: y.clone() });
        assert_eq!(db.query(&x, Relation::GreaterEqual, &y), Some(true));
        assert_eq!(db.query(&x, Relation::LessEqual, &y), Some(true));
    }
}
