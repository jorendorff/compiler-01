use crate::ast::{BinOp, Expr, Stmt};
use crate::lexer::{SpannedToken, Token};

pub struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<SpannedToken>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos].token
    }

    fn current_span(&self) -> (usize, usize) {
        let t = &self.tokens[self.pos];
        (t.line, t.col)
    }

    fn advance(&mut self) -> &SpannedToken {
        let t = &self.tokens[self.pos];
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    fn expect(&mut self, expected: &Token) -> Result<(), String> {
        let (line, col) = self.current_span();
        if self.peek() == expected {
            self.advance();
            Ok(())
        } else {
            Err(format!(
                "{}:{}: expected {:?}, found {:?}",
                line,
                col,
                expected,
                self.peek()
            ))
        }
    }

    pub fn parse_program(&mut self) -> Result<Vec<Stmt>, String> {
        let mut stmts = Vec::new();
        while *self.peek() != Token::Eof {
            stmts.push(self.parse_stmt()?);
        }
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        match self.peek().clone() {
            Token::Let => self.parse_let(),
            Token::Print => self.parse_print(),
            Token::Ident(_) => self.parse_assign(),
            _ => {
                let (line, col) = self.current_span();
                Err(format!(
                    "{}:{}: expected statement, found {:?}",
                    line,
                    col,
                    self.peek()
                ))
            }
        }
    }

    fn parse_let(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'let'
        let (line, col) = self.current_span();
        let name = match self.peek().clone() {
            Token::Ident(name) => {
                self.advance();
                name
            }
            _ => {
                return Err(format!(
                    "{}:{}: expected identifier after 'let'",
                    line, col
                ));
            }
        };
        self.expect(&Token::Eq)?;
        let expr = self.parse_expr()?;
        self.expect(&Token::Semi)?;
        Ok(Stmt::Let { name, expr })
    }

    fn parse_assign(&mut self) -> Result<Stmt, String> {
        let name = match self.peek().clone() {
            Token::Ident(name) => {
                self.advance();
                name
            }
            _ => unreachable!(),
        };
        self.expect(&Token::Eq)?;
        let expr = self.parse_expr()?;
        self.expect(&Token::Semi)?;
        Ok(Stmt::Assign { name, expr })
    }

    fn parse_print(&mut self) -> Result<Stmt, String> {
        self.advance(); // consume 'print'
        let expr = self.parse_expr()?;
        self.expect(&Token::Semi)?;
        Ok(Stmt::Print { expr })
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_term()?;
        loop {
            let op = match self.peek() {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_term()?;
            left = Expr::BinOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                Token::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::BinOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        if *self.peek() == Token::Minus {
            self.advance();
            let expr = self.parse_unary()?;
            Ok(Expr::UnaryMinus(Box::new(expr)))
        } else {
            self.parse_atom()
        }
    }

    fn parse_atom(&mut self) -> Result<Expr, String> {
        let (line, col) = self.current_span();
        match self.peek().clone() {
            Token::IntLit(s) => {
                self.advance();
                // Parse as u64 first to handle the full range of i64 values
                // (the value 9223372036854775808 can appear as the operand of unary minus)
                let val: i64 = s.parse().map_err(|e| {
                    format!("{}:{}: invalid integer literal '{}': {}", line, col, s, e)
                })?;
                Ok(Expr::IntLit(val))
            }
            Token::Ident(name) => {
                self.advance();
                Ok(Expr::Var(name))
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(expr)
            }
            _ => Err(format!(
                "{}:{}: expected expression, found {:?}",
                line,
                col,
                self.peek()
            )),
        }
    }
}
