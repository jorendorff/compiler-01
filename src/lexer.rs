#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Let,
    Print,
    Ident(String),
    IntLit(String), // Store as string to defer parsing to later stage
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    Semi,
    LParen,
    RParen,
    Eof,
}

#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub token: Token,
    pub line: usize,
    pub col: usize,
}

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.input.get(self.pos).copied()?;
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            // Skip whitespace
            while let Some(ch) = self.peek() {
                if ch.is_ascii_whitespace() {
                    self.advance();
                } else {
                    break;
                }
            }
            // Skip // comments
            if self.pos + 1 < self.input.len()
                && self.input[self.pos] == '/'
                && self.input[self.pos + 1] == '/'
            {
                while let Some(ch) = self.peek() {
                    if ch == '\n' {
                        break;
                    }
                    self.advance();
                }
                continue; // After comment, skip more whitespace
            }
            break;
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<SpannedToken>, String> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            let line = self.line;
            let col = self.col;
            let ch = match self.peek() {
                Some(ch) => ch,
                None => {
                    tokens.push(SpannedToken {
                        token: Token::Eof,
                        line,
                        col,
                    });
                    return Ok(tokens);
                }
            };

            let token = match ch {
                '+' => {
                    self.advance();
                    Token::Plus
                }
                '-' => {
                    self.advance();
                    Token::Minus
                }
                '*' => {
                    self.advance();
                    Token::Star
                }
                '/' => {
                    self.advance();
                    Token::Slash
                }
                '%' => {
                    self.advance();
                    Token::Percent
                }
                '=' => {
                    self.advance();
                    Token::Eq
                }
                ';' => {
                    self.advance();
                    Token::Semi
                }
                '(' => {
                    self.advance();
                    Token::LParen
                }
                ')' => {
                    self.advance();
                    Token::RParen
                }
                c if c.is_ascii_digit() => {
                    let mut num = String::new();
                    while let Some(c) = self.peek() {
                        if c.is_ascii_digit() {
                            num.push(c);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    Token::IntLit(num)
                }
                c if c.is_ascii_alphabetic() || c == '_' => {
                    let mut ident = String::new();
                    while let Some(c) = self.peek() {
                        if c.is_ascii_alphanumeric() || c == '_' {
                            ident.push(c);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    match ident.as_str() {
                        "let" => Token::Let,
                        "print" => Token::Print,
                        _ => Token::Ident(ident),
                    }
                }
                _ => {
                    return Err(format!(
                        "{}:{}: unexpected character '{}'",
                        line, col, ch
                    ));
                }
            };

            tokens.push(SpannedToken { token, line, col });
        }
    }
}
