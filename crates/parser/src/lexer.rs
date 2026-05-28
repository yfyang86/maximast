use crate::token::Token;

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.input.get(self.pos).copied();
        self.pos += 1;
        ch
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() || ch == '\u{200B}' || ch == '\u{FEFF}' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) -> bool {
        if self.pos + 1 < self.input.len()
            && self.input[self.pos] == '/'
            && self.input[self.pos + 1] == '*'
        {
            self.pos += 2;
            let mut depth = 1;
            while depth > 0 && self.pos < self.input.len() {
                if self.pos + 1 < self.input.len()
                    && self.input[self.pos] == '/'
                    && self.input[self.pos + 1] == '*'
                {
                    depth += 1;
                    self.pos += 2;
                } else if self.pos + 1 < self.input.len()
                    && self.input[self.pos] == '*'
                    && self.input[self.pos + 1] == '/'
                {
                    depth -= 1;
                    self.pos += 2;
                } else {
                    self.pos += 1;
                }
            }
            return true;
        }
        false
    }

    fn read_number(&mut self) -> Token {
        let start = self.pos;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }

        // Check for digit-letter combination like 0a000, 0xyz, 1abc
        // These are potential base-N numbers; lex as Ident so the evaluator can decide
        if self.peek().is_some_and(|c| c.is_alphabetic()) {
            // Read rest as alphanumeric identifier
            while let Some(ch) = self.peek() {
                if ch.is_alphanumeric() || ch == '_' {
                    self.advance();
                } else {
                    break;
                }
            }
            let s: String = self.input[start..self.pos].iter().collect();
            return Token::Ident(s);
        }

        // Check for trailing dot: "2." is an integer in Maxima when followed by non-digit
        if self.peek() == Some('.') {
            let next_after_dot = self.input.get(self.pos + 1);
            if next_after_dot.is_none() || !next_after_dot.unwrap().is_ascii_digit() {
                self.advance(); // consume the dot
                let s: String = self.input[start..self.pos - 1].iter().collect();
                return Token::DotInteger(s.parse().unwrap());
            }
        }

        if self.peek() == Some('.') && self.input.get(self.pos + 1).is_some_and(|c| c.is_ascii_digit()) {
            self.advance(); // consume '.'
            while let Some(ch) = self.peek() {
                if ch.is_ascii_digit() {
                    self.advance();
                } else {
                    break;
                }
            }
            // Scientific notation: e, E, d, D, f, F, s, S, b, B
            // Maxima uses d0 for double, f0 for float, s0 for short, b0 for bigfloat
            let exp_char = self.peek();
            if matches!(exp_char, Some('e' | 'E' | 'd' | 'D' | 'f' | 'F' | 's' | 'S')) {
                self.advance();
                if self.peek() == Some('+') || self.peek() == Some('-') {
                    self.advance();
                }
                while let Some(ch) = self.peek() {
                    if ch.is_ascii_digit() {
                        self.advance();
                    } else {
                        break;
                    }
                }
            } else if self.peek() == Some('b') || self.peek() == Some('B') {
                // bigfloat: 1.0b0 — treat as float for now
                self.advance();
                if self.peek() == Some('+') || self.peek() == Some('-') {
                    self.advance();
                }
                while let Some(ch) = self.peek() {
                    if ch.is_ascii_digit() {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
            // Normalize: replace d/f/s/b exponent markers with e for parsing
            let s: String = self.input[start..self.pos].iter().collect();
            let normalized = s.replace(['d', 'D', 'f', 'F', 's', 'S', 'b', 'B'], "e");
            Token::Float(normalized.parse().unwrap_or(0.0))
        } else {
            let s: String = self.input[start..self.pos].iter().collect();
            Token::Integer(s.parse().unwrap())
        }
    }

    fn read_ident(&mut self) -> Token {
        let start = self.pos;
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' || ch == '%' || ch == '?' {
                self.advance();
            } else {
                break;
            }
        }
        let s: String = self.input[start..self.pos].iter().collect();
        Token::Ident(s)
    }

    fn read_string(&mut self) -> Token {
        self.advance(); // consume opening quote
        let mut s = String::new();
        while let Some(ch) = self.advance() {
            if ch == '"' {
                break;
            }
            if ch == '\\' {
                if let Some(escaped) = self.advance() {
                    match escaped {
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        '\\' => s.push('\\'),
                        '"' => s.push('"'),
                        other => {
                            s.push('\\');
                            s.push(other);
                        }
                    }
                }
            } else {
                s.push(ch);
            }
        }
        Token::String(s)
    }

    pub fn next_token(&mut self) -> Token {
        loop {
            self.skip_whitespace();
            if !self.skip_comment() {
                break;
            }
        }

        let ch = match self.peek() {
            Some(ch) => ch,
            None => return Token::Eof,
        };

        if ch.is_ascii_digit() {
            return self.read_number();
        }

        if ch.is_alphabetic() || ch == '%' || ch == '_' || ch == '?' {
            return self.read_ident();
        }

        // Backslash-escaped symbol: \name
        if ch == '\\' {
            self.advance(); // consume backslash
            let start = self.pos;
            while let Some(c) = self.peek() {
                if c.is_alphanumeric() || c == '_' {
                    self.advance();
                } else {
                    break;
                }
            }
            let s: String = self.input[start..self.pos].iter().collect();
            return Token::Ident(format!("\\{}", s));
        }

        if ch == '"' {
            return self.read_string();
        }

        self.advance();
        match ch {
            '+' => Token::Plus,
            '-' => Token::Minus,
            '*' => Token::Star,
            '/' => Token::Slash,
            '^' => {
                if self.peek() == Some('^') {
                    self.advance();
                    Token::Ident("^^".into())
                } else {
                    Token::Caret
                }
            }
            '!' => {
                if self.peek() == Some('!') {
                    self.advance();
                    Token::Ident("!!".into())
                } else {
                    Token::Excl
                }
            }
            '.' => Token::Dot,
            '\'' => {
                if self.peek() == Some('\'') {
                    self.advance();
                    Token::DoubleQuote
                } else {
                    Token::SingleQuote
                }
            }
            '(' => Token::LParen,
            ')' => Token::RParen,
            '[' => Token::LBracket,
            ']' => Token::RBracket,
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            ',' => Token::Comma,
            ';' => Token::Semicolon,
            '$' => Token::Dollar,
            '#' => Token::Hash,
            '=' => Token::Equal,
            ':' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Token::ColonEqual
                } else if self.peek() == Some(':') {
                    self.advance();
                    Token::ColonColon
                } else {
                    Token::Colon
                }
            }
            '<' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Token::LessEqual
                } else {
                    Token::LessThan
                }
            }
            '>' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Token::GreaterEqual
                } else {
                    Token::GreaterThan
                }
            }
            _ => panic!("unexpected character: '{}' (U+{:04X})", ch, ch as u32),
        }
    }

    pub fn tokenize(input: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(input);
        let mut tokens = Vec::new();
        loop {
            let tok = lexer.next_token();
            if tok == Token::Eof {
                break;
            }
            tokens.push(tok);
        }
        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_integer() {
        assert_eq!(Lexer::tokenize("42"), vec![Token::Integer(42)]);
    }

    #[test]
    fn lex_float() {
        assert_eq!(Lexer::tokenize("3.14"), vec![Token::Float(3.14)]);
    }

    #[test]
    fn lex_arithmetic() {
        assert_eq!(
            Lexer::tokenize("1+2*3"),
            vec![
                Token::Integer(1),
                Token::Plus,
                Token::Integer(2),
                Token::Star,
                Token::Integer(3),
            ]
        );
    }

    #[test]
    fn lex_parens() {
        assert_eq!(
            Lexer::tokenize("(1+2)"),
            vec![
                Token::LParen,
                Token::Integer(1),
                Token::Plus,
                Token::Integer(2),
                Token::RParen,
            ]
        );
    }

    #[test]
    fn lex_ident() {
        assert_eq!(
            Lexer::tokenize("x + %pi"),
            vec![
                Token::Ident("x".into()),
                Token::Plus,
                Token::Ident("%pi".into()),
            ]
        );
    }

    #[test]
    fn lex_funcdef() {
        assert_eq!(
            Lexer::tokenize("f(x):=x^2"),
            vec![
                Token::Ident("f".into()),
                Token::LParen,
                Token::Ident("x".into()),
                Token::RParen,
                Token::ColonEqual,
                Token::Ident("x".into()),
                Token::Caret,
                Token::Integer(2),
            ]
        );
    }

    #[test]
    fn lex_string() {
        assert_eq!(
            Lexer::tokenize("\"hello\""),
            vec![Token::String("hello".into())]
        );
    }

    #[test]
    fn lex_comment() {
        assert_eq!(
            Lexer::tokenize("1 + /* comment */ 2"),
            vec![Token::Integer(1), Token::Plus, Token::Integer(2)]
        );
    }

    #[test]
    fn lex_nested_comment() {
        assert_eq!(
            Lexer::tokenize("1 + /* outer /* inner */ still comment */ 2"),
            vec![Token::Integer(1), Token::Plus, Token::Integer(2)]
        );
    }

    #[test]
    fn lex_comparison() {
        assert_eq!(
            Lexer::tokenize("x >= 0"),
            vec![
                Token::Ident("x".into()),
                Token::GreaterEqual,
                Token::Integer(0),
            ]
        );
    }

    #[test]
    fn lex_scientific() {
        assert_eq!(Lexer::tokenize("1.5e-3"), vec![Token::Float(0.0015)]);
    }

    #[test]
    fn lex_list() {
        assert_eq!(
            Lexer::tokenize("[1,2,3]"),
            vec![
                Token::LBracket,
                Token::Integer(1),
                Token::Comma,
                Token::Integer(2),
                Token::Comma,
                Token::Integer(3),
                Token::RBracket,
            ]
        );
    }

    // --- Trailing dot integers ---

    #[test]
    fn lex_trailing_dot_integer() {
        assert_eq!(Lexer::tokenize("2."), vec![Token::DotInteger(2)]);
    }

    #[test]
    fn lex_trailing_dot_in_expr() {
        assert_eq!(
            Lexer::tokenize("10.+5"),
            vec![Token::DotInteger(10), Token::Plus, Token::Integer(5)]
        );
    }

    // --- Float formats ---

    #[test]
    fn lex_float_d_exponent() {
        let tokens = Lexer::tokenize("1.5d0");
        assert_eq!(tokens.len(), 1);
        if let Token::Float(f) = &tokens[0] {
            assert!((*f - 1.5).abs() < 1e-10);
        } else {
            panic!("expected Float, got {:?}", tokens[0]);
        }
    }

    #[test]
    fn lex_float_b_exponent() {
        let tokens = Lexer::tokenize("3.14b0");
        assert_eq!(tokens.len(), 1);
        if let Token::Float(f) = &tokens[0] {
            assert!((*f - 3.14).abs() < 1e-10);
        } else {
            panic!("expected Float, got {:?}", tokens[0]);
        }
    }

    #[test]
    fn lex_float_negative_exponent() {
        let tokens = Lexer::tokenize("1.0e-2");
        assert_eq!(tokens.len(), 1);
        if let Token::Float(f) = &tokens[0] {
            assert!((*f - 0.01).abs() < 1e-10);
        } else {
            panic!("expected Float, got {:?}", tokens[0]);
        }
    }

    // --- Operators ---

    #[test]
    fn lex_colon_variants() {
        assert_eq!(Lexer::tokenize(":"), vec![Token::Colon]);
        assert_eq!(Lexer::tokenize(":="), vec![Token::ColonEqual]);
        assert_eq!(Lexer::tokenize("::"), vec![Token::ColonColon]);
    }

    #[test]
    fn lex_comparison_ops() {
        assert_eq!(Lexer::tokenize("<"), vec![Token::LessThan]);
        assert_eq!(Lexer::tokenize(">"), vec![Token::GreaterThan]);
        assert_eq!(Lexer::tokenize("<="), vec![Token::LessEqual]);
        assert_eq!(Lexer::tokenize(">="), vec![Token::GreaterEqual]);
        assert_eq!(Lexer::tokenize("="), vec![Token::Equal]);
        assert_eq!(Lexer::tokenize("#"), vec![Token::Hash]);
    }

    #[test]
    fn lex_factorial_and_dot() {
        assert_eq!(Lexer::tokenize("5!"), vec![Token::Integer(5), Token::Excl]);
        assert_eq!(Lexer::tokenize("a.b"), vec![
            Token::Ident("a".into()), Token::Dot, Token::Ident("b".into())
        ]);
    }

    #[test]
    fn lex_quote() {
        assert_eq!(Lexer::tokenize("'x"), vec![Token::SingleQuote, Token::Ident("x".into())]);
        assert_eq!(Lexer::tokenize("''x"), vec![Token::DoubleQuote, Token::Ident("x".into())]);
    }

    // --- Strings ---

    #[test]
    fn lex_string_with_escapes() {
        let tokens = Lexer::tokenize(r#""hello\nworld""#);
        assert_eq!(tokens, vec![Token::String("hello\nworld".into())]);
    }

    #[test]
    fn lex_empty_string() {
        assert_eq!(Lexer::tokenize(r#""""#), vec![Token::String("".into())]);
    }

    // --- Identifiers ---

    #[test]
    fn lex_percent_ident() {
        assert_eq!(Lexer::tokenize("%e"), vec![Token::Ident("%e".into())]);
        assert_eq!(Lexer::tokenize("%pi"), vec![Token::Ident("%pi".into())]);
        assert_eq!(Lexer::tokenize("%i"), vec![Token::Ident("%i".into())]);
    }

    #[test]
    fn lex_underscore_ident() {
        assert_eq!(Lexer::tokenize("my_var"), vec![Token::Ident("my_var".into())]);
    }

    #[test]
    fn lex_question_mark_ident() {
        assert_eq!(Lexer::tokenize("?foo"), vec![Token::Ident("?foo".into())]);
    }

    #[test]
    fn lex_backslash_symbol() {
        let tokens = Lexer::tokenize("\\abc");
        assert_eq!(tokens, vec![Token::Ident("\\abc".into())]);
    }

    // --- Semicolons and dollar ---

    #[test]
    fn lex_terminators() {
        assert_eq!(
            Lexer::tokenize("x; y$"),
            vec![
                Token::Ident("x".into()),
                Token::Semicolon,
                Token::Ident("y".into()),
                Token::Dollar,
            ]
        );
    }

    // --- Whitespace ---

    #[test]
    fn lex_only_whitespace() {
        assert_eq!(Lexer::tokenize("   "), Vec::<Token>::new());
    }

    #[test]
    fn lex_mixed_whitespace() {
        assert_eq!(
            Lexer::tokenize("  1  +  2  "),
            vec![Token::Integer(1), Token::Plus, Token::Integer(2)]
        );
    }

    // --- Large numbers ---

    #[test]
    fn lex_large_integer() {
        let tokens = Lexer::tokenize("99999999999");
        assert_eq!(tokens, vec![Token::Integer(99999999999)]);
    }
}
