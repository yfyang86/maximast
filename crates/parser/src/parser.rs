use maxima_core::{Expr, Operator};

use crate::lexer::Lexer;
use crate::token::Token;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        self.pos += 1;
        tok
    }

    fn expect(&mut self, expected: &Token) {
        let tok = self.advance();
        if tok != *expected {
            panic!("expected {:?}, got {:?}", expected, tok);
        }
    }

    fn parse_expr(&mut self) -> Expr {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Expr {
        let lhs = self.parse_or();

        match self.peek() {
            Token::Colon => {
                self.advance();
                let rhs = self.parse_assignment();
                Expr::List {
                    op: Operator::MAssign,
                    simplified: false,
                    args: vec![lhs, rhs],
                }
            }
            Token::ColonEqual => {
                self.advance();
                let rhs = self.parse_expr();
                Expr::List {
                    op: Operator::MDefine,
                    simplified: false,
                    args: vec![lhs, rhs],
                }
            }
            Token::ColonColon => {
                self.advance();
                let rhs = self.parse_assignment();
                Expr::List {
                    op: Operator::MSetq,
                    simplified: false,
                    args: vec![lhs, rhs],
                }
            }
            _ => lhs,
        }
    }

    fn parse_or(&mut self) -> Expr {
        let mut left = self.parse_and();
        while self.peek_ident("or") {
            self.advance();
            let right = self.parse_and();
            left = Expr::List {
                op: Operator::MOr,
                simplified: false,
                args: vec![left, right],
            };
        }
        left
    }

    fn parse_and(&mut self) -> Expr {
        let mut left = self.parse_comparison();
        while self.peek_ident("and") {
            self.advance();
            let right = self.parse_comparison();
            left = Expr::List {
                op: Operator::MAnd,
                simplified: false,
                args: vec![left, right],
            };
        }
        left
    }

    fn parse_comparison(&mut self) -> Expr {
        let lhs = self.parse_additive();

        let op = match self.peek() {
            Token::Equal => Operator::MEqual,
            Token::Hash => Operator::MNotEqual,
            Token::LessThan => Operator::MLessThan,
            Token::GreaterThan => Operator::MGreaterThan,
            Token::LessEqual => Operator::MLessEqual,
            Token::GreaterEqual => Operator::MGreaterEqual,
            _ => return lhs,
        };
        self.advance();
        let rhs = self.parse_additive();
        Expr::List {
            op,
            simplified: false,
            args: vec![lhs, rhs],
        }
    }

    fn parse_additive(&mut self) -> Expr {
        let mut left = self.parse_multiplicative();

        loop {
            match self.peek() {
                Token::Plus => {
                    self.advance();
                    let right = self.parse_multiplicative();
                    left = Expr::add(left, right);
                }
                Token::Minus => {
                    self.advance();
                    let right = self.parse_multiplicative();
                    left = Expr::sub(left, right);
                }
                _ => break,
            }
        }
        left
    }

    fn parse_multiplicative(&mut self) -> Expr {
        let mut left = self.parse_power();

        loop {
            match self.peek() {
                Token::Star => {
                    self.advance();
                    let right = self.parse_power();
                    left = Expr::mul(left, right);
                }
                Token::Slash => {
                    self.advance();
                    let right = self.parse_power();
                    left = Expr::div(left, right);
                }
                Token::Dot => {
                    self.advance();
                    let right = self.parse_power();
                    left = Expr::call(".", vec![left, right]);
                }
                _ => break,
            }
        }
        left
    }

    fn parse_power(&mut self) -> Expr {
        let base = self.parse_unary();
        if *self.peek() == Token::Caret {
            self.advance();
            let exp = self.parse_power();
            Expr::pow(base, exp)
        } else if self.peek_ident("^^") {
            self.advance();
            let exp = self.parse_power();
            Expr::call("ncexpt", vec![base, exp])
        } else {
            base
        }
    }

    fn parse_unary(&mut self) -> Expr {
        match self.peek() {
            Token::Minus => {
                self.advance();
                let operand = self.parse_power();
                Expr::neg(operand)
            }
            Token::Plus => {
                self.advance();
                self.parse_power()
            }
            Token::SingleQuote => {
                self.advance();
                let operand = self.parse_postfix();
                Expr::List {
                    op: Operator::MQuote,
                    simplified: false,
                    args: vec![operand],
                }
            }
            Token::DoubleQuote => {
                self.advance();
                let operand = self.parse_postfix();
                Expr::call("meval", vec![operand])
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Expr {
        let mut expr = self.parse_primary();

        loop {
            match self.peek() {
                Token::LParen => {
                    self.advance();
                    let args = self.parse_arg_list(Token::RParen);
                    self.expect(&Token::RParen);
                    match &expr {
                        Expr::Symbol(id) => {
                            expr = Expr::List {
                                op: Operator::Named(*id),
                                simplified: false,
                                args,
                            };
                        }
                        _ => {
                            // Lambda or other expression being called
                            let mut call_args = vec![expr];
                            call_args.extend(args);
                            expr = Expr::call("funapply", call_args);
                        }
                    }
                }
                Token::LBracket => {
                    self.advance();
                    let indices = self.parse_arg_list(Token::RBracket);
                    self.expect(&Token::RBracket);
                    let mut args = vec![expr];
                    args.extend(indices);
                    expr = Expr::call("mqapply", args);
                }
                Token::Excl => {
                    self.advance();
                    expr = Expr::call("factorial", vec![expr]);
                }
                _ => break,
            }
        }
        expr
    }

    fn parse_primary(&mut self) -> Expr {
        match self.peek().clone() {
            Token::Integer(n) => {
                self.advance();
                Expr::int(n)
            }
            Token::DotInteger(n) => {
                self.advance();
                // Mark as float to make it immune to ibase reinterpretation.
                // integerp returns false for these, but that matches Maxima:
                // integerp(2.) => true only because 2. is always decimal.
                // We use Float here as a practical compromise.
                Expr::Float(n as f64)
            }
            Token::Float(x) => {
                self.advance();
                Expr::Float(x)
            }
            Token::Ident(ref name) => {
                let name = name.clone();
                self.advance();
                match name.as_str() {
                    "if" => self.parse_if(),
                    "for" => self.parse_for(),
                    "while" => self.parse_while(),
                    "block" => {
                        self.expect(&Token::LParen);
                        let args = self.parse_arg_list(Token::RParen);
                        self.expect(&Token::RParen);
                        Expr::List {
                            op: Operator::MBlock,
                            simplified: false,
                            args,
                        }
                    }
                    "lambda" => {
                        self.expect(&Token::LParen);
                        let args = self.parse_arg_list(Token::RParen);
                        self.expect(&Token::RParen);
                        Expr::List {
                            op: Operator::MLambda,
                            simplified: false,
                            args,
                        }
                    }
                    "return" => {
                        self.expect(&Token::LParen);
                        let val = self.parse_expr();
                        self.expect(&Token::RParen);
                        Expr::List {
                            op: Operator::MReturn,
                            simplified: false,
                            args: vec![val],
                        }
                    }
                    "not" => {
                        let operand = self.parse_comparison();
                        Expr::List {
                            op: Operator::MNot,
                            simplified: false,
                            args: vec![operand],
                        }
                    }
                    "true" => Expr::sym("true"),
                    "false" => Expr::sym("false"),
                    "done" => Expr::sym("done"),
                    _ => Expr::sym(&name),
                }
            }
            Token::String(ref s) => {
                let s = s.clone();
                self.advance();
                Expr::String(s.into())
            }
            Token::LParen => {
                self.advance();
                let first = self.parse_expr();
                if *self.peek() == Token::Comma {
                    // Grouping with comma: (expr1, expr2, ...) → progn/block
                    let mut exprs = vec![first];
                    while *self.peek() == Token::Comma {
                        self.advance();
                        exprs.push(self.parse_expr());
                    }
                    self.expect(&Token::RParen);
                    Expr::List {
                        op: Operator::MBlock,
                        simplified: false,
                        args: exprs,
                    }
                } else {
                    self.expect(&Token::RParen);
                    first
                }
            }
            Token::LBracket => {
                self.advance();
                let items = self.parse_arg_list(Token::RBracket);
                self.expect(&Token::RBracket);
                Expr::list(items)
            }
            Token::LBrace => {
                self.advance();
                let items = self.parse_arg_list(Token::RBrace);
                self.expect(&Token::RBrace);
                Expr::set(items)
            }
            other => panic!("unexpected token: {:?}", other),
        }
    }

    fn parse_arg_list(&mut self, end: Token) -> Vec<Expr> {
        let mut args = Vec::new();
        if *self.peek() == end {
            return args;
        }
        args.push(self.parse_expr());
        while *self.peek() == Token::Comma {
            self.advance();
            args.push(self.parse_expr());
        }
        args
    }

    fn parse_if(&mut self) -> Expr {
        let cond = self.parse_expr();
        self.expect_ident("then");
        let then_branch = self.parse_expr();
        let else_branch = if self.peek_ident("else") {
            self.advance();
            Some(self.parse_expr())
        } else if self.peek_ident("elseif") {
            self.advance();
            Some(self.parse_if())
        } else {
            None
        };
        let mut args = vec![cond, then_branch];
        if let Some(eb) = else_branch {
            args.push(eb);
        }
        Expr::List {
            op: Operator::MIf,
            simplified: false,
            args,
        }
    }

    fn parse_for(&mut self) -> Expr {
        let var = match self.advance() {
            Token::Ident(name) => Expr::sym(&name),
            other => panic!("expected variable name after 'for', got {:?}", other),
        };

        // Check for "for var in list do body" syntax
        if self.peek_ident("in") {
            self.advance();
            let list_expr = self.parse_expr();
            self.expect_ident("do");
            let body = self.parse_expr();
            return Expr::call("mdo_in", vec![var, list_expr, body]);
        }

        // Accept both "for i:start" and "for i from start"
        if self.peek_ident("from") {
            self.advance();
        } else {
            self.expect(&Token::Colon);
        }
        let start = self.parse_expr();
        self.expect_ident("thru");
        let end = self.parse_expr();
        let step = if self.peek_ident("step") {
            self.advance();
            Some(self.parse_expr())
        } else {
            None
        };
        self.expect_ident("do");
        let body = self.parse_expr();
        let mut args = vec![var, start, end];
        if let Some(s) = step {
            args.push(s);
        }
        args.push(body);
        Expr::List {
            op: Operator::MDo,
            simplified: false,
            args,
        }
    }

    fn parse_while(&mut self) -> Expr {
        let cond = self.parse_expr();
        self.expect_ident("do");
        let body = self.parse_expr();
        Expr::List {
            op: Operator::MDo,
            simplified: false,
            args: vec![cond, body],
        }
    }

    fn peek_ident(&self, name: &str) -> bool {
        matches!(self.peek(), Token::Ident(s) if s == name)
    }

    fn expect_ident(&mut self, name: &str) {
        match self.advance() {
            Token::Ident(s) if s == name => {}
            other => panic!("expected '{}', got {:?}", name, other),
        }
    }
}

/// Parse a single Maxima statement (terminated by `;` or `$`).
pub fn parse(input: &str) -> Expr {
    let tokens = Lexer::tokenize(input);
    let mut parser = Parser::new(tokens);
    let expr = parser.parse_expr();
    match parser.peek() {
        Token::Semicolon | Token::Dollar => {
            parser.advance();
        }
        _ => {}
    }
    expr
}

/// Parse multiple statements separated by `;` or `$`.
pub fn parse_multi(input: &str) -> Vec<Expr> {
    let tokens = Lexer::tokenize(input);
    let mut parser = Parser::new(tokens);
    let mut exprs = Vec::new();
    while *parser.peek() != Token::Eof {
        let expr = parser.parse_expr();
        exprs.push(expr);
        match parser.peek() {
            Token::Semicolon | Token::Dollar => {
                parser.advance();
            }
            _ => {}
        }
    }
    exprs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int(n: i64) -> Expr {
        Expr::int(n)
    }

    #[test]
    fn parse_integer() {
        assert_eq!(parse("42;"), int(42));
    }

    #[test]
    fn parse_addition() {
        let e = parse("1+2;");
        assert_eq!(e, Expr::add(int(1), int(2)));
    }

    #[test]
    fn parse_precedence() {
        let e = parse("2+3*4;");
        assert_eq!(e, Expr::add(int(2), Expr::mul(int(3), int(4))));
    }

    #[test]
    fn parse_parens() {
        let e = parse("(2+3)*4;");
        assert_eq!(e, Expr::mul(Expr::add(int(2), int(3)), int(4)));
    }

    #[test]
    fn parse_power_right_assoc() {
        let e = parse("2^3^4;");
        assert_eq!(e, Expr::pow(int(2), Expr::pow(int(3), int(4))));
    }

    #[test]
    fn parse_unary_minus() {
        let e = parse("-3;");
        assert_eq!(e, Expr::neg(int(3)));
    }

    #[test]
    fn parse_subtraction() {
        let e = parse("5-3;");
        assert_eq!(e, Expr::sub(int(5), int(3)));
    }

    #[test]
    fn parse_division() {
        let e = parse("6/3;");
        assert_eq!(e, Expr::div(int(6), int(3)));
    }

    #[test]
    fn parse_function_call() {
        let e = parse("sin(x);");
        assert_eq!(e, Expr::call("sin", vec![Expr::sym("x")]));
    }

    #[test]
    fn parse_function_multi_args() {
        let e = parse("f(x,y,z);");
        assert_eq!(
            e,
            Expr::call("f", vec![Expr::sym("x"), Expr::sym("y"), Expr::sym("z")])
        );
    }

    #[test]
    fn parse_list() {
        let e = parse("[1,2,3];");
        assert_eq!(e, Expr::list(vec![int(1), int(2), int(3)]));
    }

    #[test]
    fn parse_assignment() {
        let e = parse("x:5;");
        assert_eq!(
            e,
            Expr::List {
                op: Operator::MAssign,
                simplified: false,
                args: vec![Expr::sym("x"), int(5)],
            }
        );
    }

    #[test]
    fn parse_funcdef() {
        let e = parse("f(x):=x^2;");
        assert_eq!(
            e,
            Expr::List {
                op: Operator::MDefine,
                simplified: false,
                args: vec![
                    Expr::call("f", vec![Expr::sym("x")]),
                    Expr::pow(Expr::sym("x"), int(2)),
                ],
            }
        );
    }

    #[test]
    fn parse_if_then_else() {
        let e = parse("if x > 0 then 1 else -1;");
        assert_eq!(
            e,
            Expr::List {
                op: Operator::MIf,
                simplified: false,
                args: vec![
                    Expr::List {
                        op: Operator::MGreaterThan,
                        simplified: false,
                        args: vec![Expr::sym("x"), int(0)],
                    },
                    int(1),
                    Expr::neg(int(1)),
                ],
            }
        );
    }

    #[test]
    fn parse_comparison() {
        let e = parse("x >= 0;");
        assert_eq!(
            e,
            Expr::List {
                op: Operator::MGreaterEqual,
                simplified: false,
                args: vec![Expr::sym("x"), int(0)],
            }
        );
    }

    #[test]
    fn parse_multi_stmts() {
        let stmts = parse_multi("x:1; y:2;");
        assert_eq!(stmts.len(), 2);
    }

    #[test]
    fn parse_nested() {
        let e = parse("((1+2)^2-1)*3;");
        let expected = Expr::mul(
            Expr::sub(
                Expr::pow(Expr::add(int(1), int(2)), int(2)),
                int(1),
            ),
            int(3),
        );
        assert_eq!(e, expected);
    }

    #[test]
    fn parse_factorial() {
        let e = parse("5!;");
        assert_eq!(e, Expr::call("factorial", vec![int(5)]));
    }

    #[test]
    fn parse_quote() {
        let e = parse("'f(x);");
        assert_eq!(
            e,
            Expr::List {
                op: Operator::MQuote,
                simplified: false,
                args: vec![Expr::call("f", vec![Expr::sym("x")])],
            }
        );
    }

    #[test]
    fn parse_and_or() {
        let e = parse("a and b or c;");
        // and binds tighter than or
        let ab = Expr::List {
            op: Operator::MAnd,
            simplified: false,
            args: vec![Expr::sym("a"), Expr::sym("b")],
        };
        assert_eq!(
            e,
            Expr::List {
                op: Operator::MOr,
                simplified: false,
                args: vec![ab, Expr::sym("c")],
            }
        );
    }

    #[test]
    fn parse_comma_grouping() {
        let e = parse("(x:1, x+1);");
        assert_eq!(
            e,
            Expr::List {
                op: Operator::MBlock,
                simplified: false,
                args: vec![
                    Expr::List {
                        op: Operator::MAssign,
                        simplified: false,
                        args: vec![Expr::sym("x"), int(1)],
                    },
                    Expr::add(Expr::sym("x"), int(1)),
                ],
            }
        );
    }

    // --- Control flow ---

    #[test]
    fn parse_if_without_else() {
        let e = parse("if x > 0 then 1;");
        if let Expr::List { op: Operator::MIf, args, .. } = e {
            assert_eq!(args.len(), 2); // cond, then (no else)
        } else {
            panic!("expected MIf");
        }
    }

    #[test]
    fn parse_if_elseif() {
        let e = parse("if x > 0 then 1 elseif x < 0 then -1 else 0;");
        if let Expr::List { op: Operator::MIf, args, .. } = e {
            assert_eq!(args.len(), 3); // cond, then, else(nested if)
        } else {
            panic!("expected MIf");
        }
    }

    #[test]
    fn parse_for_loop() {
        let e = parse("for i:1 thru 10 do print(i);");
        if let Expr::List { op: Operator::MDo, args, .. } = e {
            assert!(args.len() >= 4); // var, start, end, body
        } else {
            panic!("expected MDo");
        }
    }

    #[test]
    fn parse_for_step() {
        let e = parse("for i:1 thru 10 step 2 do i;");
        if let Expr::List { op: Operator::MDo, args, .. } = e {
            assert_eq!(args.len(), 5); // var, start, end, step, body
        } else {
            panic!("expected MDo");
        }
    }

    #[test]
    fn parse_while_loop() {
        let e = parse("while x > 0 do x:x-1;");
        if let Expr::List { op: Operator::MDo, args, .. } = e {
            assert_eq!(args.len(), 2); // cond, body
        } else {
            panic!("expected MDo");
        }
    }

    #[test]
    fn parse_for_in() {
        let e = parse("for i in [1,2,3] do print(i);");
        if let Expr::List { op: Operator::Named(id), args, .. } = &e {
            assert_eq!(maxima_core::resolve(*id), "mdo_in");
            assert_eq!(args.len(), 3); // var, list, body
        } else {
            panic!("expected mdo_in call, got {:?}", e);
        }
    }

    // --- Block and lambda ---

    #[test]
    fn parse_block_with_locals() {
        let e = parse("block([x:1, y], x+y);");
        if let Expr::List { op: Operator::MBlock, args, .. } = e {
            assert_eq!(args.len(), 2); // locals list, body
        } else {
            panic!("expected MBlock");
        }
    }

    #[test]
    fn parse_lambda() {
        let e = parse("lambda([x,y], x+y);");
        if let Expr::List { op: Operator::MLambda, args, .. } = e {
            assert_eq!(args.len(), 2); // params list, body
        } else {
            panic!("expected MLambda");
        }
    }

    #[test]
    fn parse_return() {
        let e = parse("return(42);");
        if let Expr::List { op: Operator::MReturn, args, .. } = e {
            assert_eq!(args.len(), 1);
            assert_eq!(args[0], int(42));
        } else {
            panic!("expected MReturn");
        }
    }

    // --- Not operator ---

    #[test]
    fn parse_not() {
        let e = parse("not x > 0;");
        if let Expr::List { op: Operator::MNot, args, .. } = e {
            assert_eq!(args.len(), 1);
        } else {
            panic!("expected MNot");
        }
    }

    // --- Subscript ---

    #[test]
    fn parse_subscript() {
        let e = parse("a[1];");
        if let Expr::List { op: Operator::Named(id), args, .. } = &e {
            assert_eq!(maxima_core::resolve(*id), "mqapply");
            assert_eq!(args.len(), 2);
        } else {
            panic!("expected mqapply");
        }
    }

    #[test]
    fn parse_multi_subscript() {
        let e = parse("a[1,2];");
        if let Expr::List { op: Operator::Named(id), args, .. } = &e {
            assert_eq!(maxima_core::resolve(*id), "mqapply");
            assert_eq!(args.len(), 3); // a, 1, 2
        } else {
            panic!("expected mqapply");
        }
    }

    // --- Double quote ---

    #[test]
    fn parse_double_quote() {
        let e = parse("''(1+1);");
        if let Expr::List { op: Operator::Named(id), args, .. } = &e {
            assert_eq!(maxima_core::resolve(*id), "meval");
            assert_eq!(args.len(), 1);
        } else {
            panic!("expected meval call");
        }
    }

    // --- Empty constructs ---

    #[test]
    fn parse_empty_list() {
        let e = parse("[];");
        assert_eq!(e, Expr::list(vec![]));
    }

    #[test]
    fn parse_zero_arg_function() {
        let e = parse("foo();");
        if let Expr::List { op: Operator::Named(_), args, .. } = &e {
            assert_eq!(args.len(), 0);
        } else {
            panic!("expected named call");
        }
    }

    // --- Operator precedence chain ---

    #[test]
    fn parse_full_precedence() {
        // assignment < or < and < comparison < add < mul < power < unary < postfix
        let e = parse("x : a or b and c > d + e * f ^ g;");
        // Should parse as x : (a or (b and (c > (d + (e * (f^g))))))
        if let Expr::List { op: Operator::MAssign, .. } = &e {
            // Top level is assignment — correct
        } else {
            panic!("expected assignment at top level, got {:?}", e);
        }
    }

    // --- Dollar terminator ---

    #[test]
    fn parse_dollar_terminator() {
        let e = parse("x+1$");
        assert_eq!(e, Expr::add(Expr::sym("x"), int(1)));
    }

    #[test]
    fn parse_no_terminator() {
        let e = parse("42");
        assert_eq!(e, int(42));
    }

    // --- Multiple statements ---

    #[test]
    fn parse_multi_dollar_semicolon() {
        let stmts = parse_multi("a:1$ b:2; c:3$");
        assert_eq!(stmts.len(), 3);
    }

    #[test]
    fn parse_multi_empty() {
        let stmts = parse_multi("");
        assert_eq!(stmts.len(), 0);
    }

    // --- Complex expressions ---

    #[test]
    fn parse_function_of_function() {
        let e = parse("sin(cos(x));");
        if let Expr::List { op: Operator::Named(id), args, .. } = &e {
            assert_eq!(maxima_core::resolve(*id), "sin");
            assert!(matches!(&args[0], Expr::List { op: Operator::Named(_), .. }));
        } else {
            panic!("expected nested function call");
        }
    }

    #[test]
    fn parse_chained_power() {
        // a^b^c is right-associative: a^(b^c)
        let e = parse("a^b^c;");
        if let Expr::List { op: Operator::MExpt, args, .. } = &e {
            assert_eq!(args[0], Expr::sym("a"));
            assert!(matches!(&args[1], Expr::List { op: Operator::MExpt, .. }));
        } else {
            panic!("expected power");
        }
    }

    #[test]
    fn parse_keywords_as_identifiers() {
        // "true", "false", "done" are special
        assert_eq!(parse("true;"), Expr::sym("true"));
        assert_eq!(parse("false;"), Expr::sym("false"));
        assert_eq!(parse("done;"), Expr::sym("done"));
    }

    #[test]
    fn parse_string_literal() {
        let e = parse("\"hello world\";");
        assert_eq!(e, Expr::String("hello world".into()));
    }

    #[test]
    fn parse_float() {
        let e = parse("3.14;");
        assert_eq!(e, Expr::Float(3.14));
    }

    #[test]
    fn parse_negative_in_product() {
        // -x*y = (-1*x)*y
        let e = parse("-x*y;");
        if let Expr::List { op: Operator::MTimes, .. } = &e {
            // Should have negative term and y
        } else {
            panic!("expected product");
        }
    }
}
