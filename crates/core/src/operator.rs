use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operator {
    MPlus,
    MTimes,
    MExpt,
    MMinus,
    MQuotient,
    MEqual,
    MNotEqual,
    MLessThan,
    MGreaterThan,
    MLessEqual,
    MGreaterEqual,
    MAnd,
    MOr,
    MNot,
    MList,
    MSet,
    MMatrix,
    MLambda,
    MDefine,
    MAssign,
    MSetq,
    MIf,
    MDo,
    MBlock,
    MReturn,
    MQuote,
    /// User-defined or unrecognized operator, referenced by interned name
    Named(crate::SymbolId),
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operator::MPlus => write!(f, "+"),
            Operator::MTimes => write!(f, "*"),
            Operator::MExpt => write!(f, "^"),
            Operator::MMinus => write!(f, "-"),
            Operator::MQuotient => write!(f, "/"),
            Operator::MEqual => write!(f, "="),
            Operator::MNotEqual => write!(f, "#"),
            Operator::MLessThan => write!(f, "<"),
            Operator::MGreaterThan => write!(f, ">"),
            Operator::MLessEqual => write!(f, "<="),
            Operator::MGreaterEqual => write!(f, ">="),
            Operator::MAnd => write!(f, "and"),
            Operator::MOr => write!(f, "or"),
            Operator::MNot => write!(f, "not"),
            Operator::MList => write!(f, "list"),
            Operator::MSet => write!(f, "set"),
            Operator::MMatrix => write!(f, "matrix"),
            Operator::MLambda => write!(f, "lambda"),
            Operator::MDefine => write!(f, ":="),
            Operator::MAssign => write!(f, ":"),
            Operator::MSetq => write!(f, "::"),
            Operator::MIf => write!(f, "if"),
            Operator::MDo => write!(f, "do"),
            Operator::MBlock => write!(f, "block"),
            Operator::MReturn => write!(f, "return"),
            Operator::MQuote => write!(f, "'"),
            Operator::Named(id) => write!(f, "{}", crate::resolve(*id)),
        }
    }
}
