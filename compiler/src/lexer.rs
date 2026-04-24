use crate::error::{Error, Result};
use crate::token::{StringPart, Token, TokenKind};

/// Does this token kind "complete" a statement? If so, a newline immediately
/// after it (at bracket depth 0) is treated as a statement terminator and a
/// synthetic `Semicolon` is emitted. Tokens NOT in this set leave the
/// statement open across the newline — e.g. after a binary operator or open
/// paren, the expression continues.
fn is_stmt_end(k: &TokenKind) -> bool {
    matches!(
        k,
        TokenKind::Integer(_)
            | TokenKind::Float(_)
            | TokenKind::String(_)
            | TokenKind::Ident(_)
            | TokenKind::Baqderg
            | TokenKind::Xarco
            | TokenKind::RParen
            | TokenKind::RBrace
            | TokenKind::RBracket
            | TokenKind::Sac
            | TokenKind::Khida
            | TokenKind::Yuxadalo
    )
}

pub struct Lexer<'a> {
    source: &'a str,
    pos: usize,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        // Mott uses newlines as statement terminators (with explicit `;` still
        // accepted as a synonym). We synthesize `Semicolon` tokens on newlines
        // at bracket-depth 0 when the previous token can end a statement —
        // a simplified Go/Swift ASI rule. Inside `(` or `[`, newlines are
        // whitespace, so multi-line expressions work naturally when wrapped
        // in parens. `{` and `}` do NOT affect bracket depth: we want
        // newlines inside blocks to terminate statements.
        let mut tokens: Vec<Token> = Vec::new();
        let mut paren_depth: i32 = 0;
        loop {
            let saw_newline = self.skip_trivia();
            let line = self.line;
            let col = self.col;

            // Synthesize Semicolon on newline at depth 0 if the last real
            // token can end a statement. Deduplicates against explicit `;`.
            if saw_newline && paren_depth == 0 {
                if let Some(last) = tokens.last() {
                    if is_stmt_end(&last.kind) {
                        tokens.push(Token {
                            kind: TokenKind::Semicolon,
                            line,
                            col,
                        });
                    }
                }
            }

            let Some(ch) = self.peek() else {
                // Before EOF, synthesize a final `;` if needed — handles
                // files that don't end with a trailing newline.
                if let Some(last) = tokens.last() {
                    if is_stmt_end(&last.kind) {
                        tokens.push(Token {
                            kind: TokenKind::Semicolon,
                            line,
                            col,
                        });
                    }
                }
                tokens.push(Token {
                    kind: TokenKind::Eof,
                    line,
                    col,
                });
                return Ok(tokens);
            };
            let kind = self.scan_token(ch, line, col)?;
            match &kind {
                TokenKind::LParen | TokenKind::LBracket => paren_depth += 1,
                TokenKind::RParen | TokenKind::RBracket => {
                    paren_depth = paren_depth.saturating_sub(1);
                }
                _ => {}
            }
            tokens.push(Token { kind, line, col });
        }
    }

    fn scan_token(&mut self, ch: char, line: usize, col: usize) -> Result<TokenKind> {
        match ch {
            '(' => {
                self.advance();
                Ok(TokenKind::LParen)
            }
            ')' => {
                self.advance();
                Ok(TokenKind::RParen)
            }
            '{' => {
                self.advance();
                Ok(TokenKind::LBrace)
            }
            '}' => {
                self.advance();
                Ok(TokenKind::RBrace)
            }
            '[' => {
                self.advance();
                Ok(TokenKind::LBracket)
            }
            ']' => {
                self.advance();
                Ok(TokenKind::RBracket)
            }
            '.' => {
                self.advance();
                if self.peek() == Some('.') {
                    self.advance();
                    Ok(TokenKind::DotDot)
                } else {
                    Err(Error::Lex {
                        line,
                        col,
                        message: "unexpected single `.` (did you mean `..`?)".into(),
                    })
                }
            }
            ';' => {
                self.advance();
                Ok(TokenKind::Semicolon)
            }
            ',' => {
                self.advance();
                Ok(TokenKind::Comma)
            }
            ':' => {
                self.advance();
                Ok(TokenKind::Colon)
            }
            '+' => {
                self.advance();
                Ok(TokenKind::Plus)
            }
            '*' => {
                self.advance();
                Ok(TokenKind::Star)
            }
            '/' => {
                self.advance();
                Ok(TokenKind::Slash)
            }
            '%' => {
                self.advance();
                Ok(TokenKind::Percent)
            }
            '-' => {
                self.advance();
                if self.peek() == Some('>') {
                    self.advance();
                    Ok(TokenKind::Arrow)
                } else {
                    Ok(TokenKind::Minus)
                }
            }
            '=' => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(TokenKind::Eq)
                } else {
                    Ok(TokenKind::Assign)
                }
            }
            '!' => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(TokenKind::NotEq)
                } else {
                    Ok(TokenKind::Not)
                }
            }
            '<' => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(TokenKind::Le)
                } else {
                    Ok(TokenKind::Lt)
                }
            }
            '>' => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    Ok(TokenKind::Ge)
                } else {
                    Ok(TokenKind::Gt)
                }
            }
            '"' => self.scan_string(line, col),
            c if c.is_ascii_digit() => self.scan_number(line, col),
            c if c.is_ascii_alphabetic() || c == '_' => self.scan_ident_or_keyword(line, col),
            _ => Err(Error::Lex {
                line,
                col,
                message: format!("unexpected character '{}'", ch),
            }),
        }
    }

    fn scan_number(&mut self, line: usize, col: usize) -> Result<TokenKind> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }
        // Float if we see '.' followed by a digit (to avoid consuming `.` meant for something else)
        let is_float = self.peek() == Some('.')
            && self.peek_nth(1).map_or(false, |c| c.is_ascii_digit());
        if is_float {
            self.advance(); // consume '.'
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.advance();
                } else {
                    break;
                }
            }
            let text = &self.source[start..self.pos];
            text.parse::<f64>()
                .map(TokenKind::Float)
                .map_err(|_| Error::Lex {
                    line,
                    col,
                    message: format!("invalid float literal: {}", text),
                })
        } else {
            let text = &self.source[start..self.pos];
            text.parse::<i64>()
                .map(TokenKind::Integer)
                .map_err(|_| Error::Lex {
                    line,
                    col,
                    message: format!("invalid integer literal: {}", text),
                })
        }
    }

    fn scan_ident_or_keyword(&mut self, line: usize, col: usize) -> Result<TokenKind> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '_' {
                self.advance();
            } else {
                break;
            }
        }
        let text = &self.source[start..self.pos];
        let kind = match text {
            "fnc" => TokenKind::Fnc,
            "xilit" => TokenKind::Xilit,
            "yuxadalo" => TokenKind::Yuxadalo,
            "yazde" => TokenKind::Yazde,
            "esha" => TokenKind::Esha,
            "khi" => TokenKind::Khi,
            "cqachunna" => TokenKind::Cqachunna,
            "yallalc" => TokenKind::Yallalc,
            "chu" => TokenKind::Chu,
            "sac" => TokenKind::Sac,
            "khida" => TokenKind::Khida,
            "baram" => TokenKind::Baram,
            "parse_terah" => TokenKind::ParseTerah,
            "parse_daqosh" => TokenKind::ParseDaqosh,
            "baqderg" => TokenKind::Baqderg,
            "xarco" => TokenKind::Xarco,
            "a" => TokenKind::A,
            "ya" => TokenKind::Ya,
            "terah" => TokenKind::Terah,
            "bool" => TokenKind::Bool,
            "deshnash" => TokenKind::Deshnash,
            "daqosh" => TokenKind::Daqosh,
            "nagah" => {
                return self.consume_sanna_after_nagah(line, col);
            }
            "sanna" => {
                return Err(Error::Lex {
                    line,
                    col,
                    message: "`sanna` must follow `nagah`".into(),
                });
            }
            _ => TokenKind::Ident(text.to_string()),
        };
        Ok(kind)
    }

    /// After lexing the word `nagah`, require `sanna` to follow (separated only by whitespace).
    /// Emits a single `NagahSanna` token. Comments between the halves are not allowed.
    fn consume_sanna_after_nagah(&mut self, line: usize, col: usize) -> Result<TokenKind> {
        let saved = self.snapshot();
        self.skip_whitespace_only();
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '_' {
                self.advance();
            } else {
                break;
            }
        }
        let word = &self.source[start..self.pos];
        if word == "sanna" {
            Ok(TokenKind::NagahSanna)
        } else {
            self.restore(saved);
            Err(Error::Lex {
                line,
                col,
                message: "expected `sanna` after `nagah`".into(),
            })
        }
    }

    fn scan_string(&mut self, line: usize, col: usize) -> Result<TokenKind> {
        self.advance(); // consume opening "
        let mut parts: Vec<StringPart> = Vec::new();
        let mut current = String::new();
        loop {
            match self.peek() {
                None => {
                    return Err(Error::Lex {
                        line,
                        col,
                        message: "unterminated string literal".into(),
                    });
                }
                Some('"') => {
                    self.advance();
                    if !current.is_empty() {
                        parts.push(StringPart::Literal(current));
                    }
                    return Ok(TokenKind::String(parts));
                }
                Some('\\') => {
                    self.advance();
                    let el = self.line;
                    let ec = self.col;
                    match self.peek() {
                        Some('n') => {
                            self.advance();
                            current.push('\n');
                        }
                        Some('t') => {
                            self.advance();
                            current.push('\t');
                        }
                        Some('r') => {
                            self.advance();
                            current.push('\r');
                        }
                        Some('\\') => {
                            self.advance();
                            current.push('\\');
                        }
                        Some('"') => {
                            self.advance();
                            current.push('"');
                        }
                        Some('{') => {
                            self.advance();
                            current.push('{');
                        }
                        Some('}') => {
                            self.advance();
                            current.push('}');
                        }
                        Some(c) => {
                            return Err(Error::Lex {
                                line: el,
                                col: ec,
                                message: format!("unknown escape: \\{}", c),
                            });
                        }
                        None => {
                            return Err(Error::Lex {
                                line: el,
                                col: ec,
                                message: "unterminated escape in string".into(),
                            });
                        }
                    }
                }
                Some('{') => {
                    self.advance();
                    if !current.is_empty() {
                        parts.push(StringPart::Literal(std::mem::take(&mut current)));
                    }
                    let id_line = self.line;
                    let id_col = self.col;
                    let id_start = self.pos;
                    match self.peek() {
                        Some(c) if c.is_ascii_alphabetic() || c == '_' => {
                            self.advance();
                        }
                        _ => {
                            return Err(Error::Lex {
                                line: id_line,
                                col: id_col,
                                message: "expected identifier inside `{...}`".into(),
                            });
                        }
                    }
                    while let Some(c) = self.peek() {
                        if c.is_ascii_alphanumeric() || c == '_' {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    let ident = self.source[id_start..self.pos].to_string();
                    if self.peek() != Some('}') {
                        return Err(Error::Lex {
                            line: self.line,
                            col: self.col,
                            message: "expected `}` after identifier".into(),
                        });
                    }
                    self.advance(); // consume '}'
                    parts.push(StringPart::Interpolation(ident));
                }
                Some(c) => {
                    self.advance();
                    current.push(c);
                }
            }
        }
    }

    /// Skip whitespace and line comments. Returns `true` if at least one
    /// newline was consumed — the tokenizer uses this to decide whether to
    /// synthesize a `Semicolon` (newline = statement terminator at depth 0).
    fn skip_trivia(&mut self) -> bool {
        let mut saw_newline = false;
        loop {
            match self.peek() {
                Some(' ') | Some('\t') | Some('\r') => {
                    self.advance();
                }
                Some('\n') => {
                    self.advance();
                    saw_newline = true;
                }
                Some('/') if self.peek_nth(1) == Some('/') => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                _ => return saw_newline,
            }
        }
    }

    fn skip_whitespace_only(&mut self) {
        while let Some(c) = self.peek() {
            if c == ' ' || c == '\t' || c == '\r' || c == '\n' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> Option<char> {
        self.source[self.pos..].chars().next()
    }

    fn peek_nth(&self, n: usize) -> Option<char> {
        self.source[self.pos..].chars().nth(n)
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    fn snapshot(&self) -> (usize, usize, usize) {
        (self.pos, self.line, self.col)
    }

    fn restore(&mut self, s: (usize, usize, usize)) {
        self.pos = s.0;
        self.line = s.1;
        self.col = s.2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        Lexer::new(src)
            .tokenize()
            .unwrap()
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    #[test]
    fn empty_input_yields_eof() {
        assert_eq!(kinds(""), vec![TokenKind::Eof]);
    }

    #[test]
    fn single_keywords() {
        assert_eq!(
            kinds(
                "fnc xilit yuxadalo yazde esha khi cqachunna yallalc chu \
                 sac khida baram parse_terah parse_daqosh baqderg xarco a ya"
            ),
            vec![
                TokenKind::Fnc,
                TokenKind::Xilit,
                TokenKind::Yuxadalo,
                TokenKind::Yazde,
                TokenKind::Esha,
                TokenKind::Khi,
                TokenKind::Cqachunna,
                TokenKind::Yallalc,
                TokenKind::Chu,
                TokenKind::Sac,
                TokenKind::Khida,
                TokenKind::Baram,
                TokenKind::ParseTerah,
                TokenKind::ParseDaqosh,
                TokenKind::Baqderg,
                TokenKind::Xarco,
                TokenKind::A,
                TokenKind::Ya,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn array_brackets_and_dotdot() {
        assert_eq!(
            kinds("[1, 2] 0..10"),
            vec![
                TokenKind::LBracket,
                TokenKind::Integer(1),
                TokenKind::Comma,
                TokenKind::Integer(2),
                TokenKind::RBracket,
                TokenKind::Integer(0),
                TokenKind::DotDot,
                TokenKind::Integer(10),
                TokenKind::Semicolon,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn type_keywords() {
        assert_eq!(
            kinds("terah bool deshnash daqosh"),
            vec![
                TokenKind::Terah,
                TokenKind::Bool,
                TokenKind::Deshnash,
                TokenKind::Daqosh,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn nagah_sanna_merges_into_one_token() {
        assert_eq!(
            kinds("nagah sanna"),
            vec![TokenKind::NagahSanna, TokenKind::Eof]
        );
        // extra whitespace/newlines between halves is fine
        assert_eq!(
            kinds("nagah\n   sanna"),
            vec![TokenKind::NagahSanna, TokenKind::Eof]
        );
    }

    #[test]
    fn nagah_without_sanna_errors() {
        let err = Lexer::new("nagah foo").tokenize().unwrap_err();
        assert!(format!("{}", err).contains("expected `sanna`"));
    }

    #[test]
    fn sanna_alone_errors() {
        let err = Lexer::new("sanna").tokenize().unwrap_err();
        assert!(format!("{}", err).contains("`sanna` must follow `nagah`"));
    }

    #[test]
    fn identifiers_and_numbers() {
        assert_eq!(
            kinds("foo bar_baz x1 42 3.14"),
            vec![
                TokenKind::Ident("foo".into()),
                TokenKind::Ident("bar_baz".into()),
                TokenKind::Ident("x1".into()),
                TokenKind::Integer(42),
                TokenKind::Float(3.14),
                TokenKind::Semicolon, // synthetic EOF terminator
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn operators_and_punctuation() {
        assert_eq!(
            kinds("+ - * / % == != < <= > >= ! = -> ( ) { } ; , :"),
            vec![
                TokenKind::Plus,
                TokenKind::Minus,
                TokenKind::Star,
                TokenKind::Slash,
                TokenKind::Percent,
                TokenKind::Eq,
                TokenKind::NotEq,
                TokenKind::Lt,
                TokenKind::Le,
                TokenKind::Gt,
                TokenKind::Ge,
                TokenKind::Not,
                TokenKind::Assign,
                TokenKind::Arrow,
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::LBrace,
                TokenKind::RBrace,
                TokenKind::Semicolon,
                TokenKind::Comma,
                TokenKind::Colon,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn string_without_interpolation() {
        // The trailing Semicolon before Eof is the lexer's ASI rule: a
        // string literal ends a statement, so the implicit EOF terminator
        // fires. All subsequent fixture tests follow the same pattern.
        assert_eq!(
            kinds(r#""salam""#),
            vec![
                TokenKind::String(vec![StringPart::Literal("salam".into())]),
                TokenKind::Semicolon,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn string_with_interpolation() {
        assert_eq!(
            kinds(r#""x = {x}, y = {y}""#),
            vec![
                TokenKind::String(vec![
                    StringPart::Literal("x = ".into()),
                    StringPart::Interpolation("x".into()),
                    StringPart::Literal(", y = ".into()),
                    StringPart::Interpolation("y".into()),
                ]),
                TokenKind::Semicolon,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn string_escape_of_brace_is_literal() {
        assert_eq!(
            kinds(r#""\{x}""#),
            vec![
                TokenKind::String(vec![StringPart::Literal("{x}".into())]),
                TokenKind::Semicolon,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn empty_string_produces_empty_parts() {
        assert_eq!(
            kinds(r#""""#),
            vec![
                TokenKind::String(vec![]),
                TokenKind::Semicolon,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn unterminated_string_errors() {
        let err = Lexer::new(r#""oops"#).tokenize().unwrap_err();
        assert!(format!("{}", err).contains("unterminated"));
    }

    #[test]
    fn line_comment_is_skipped() {
        assert_eq!(
            kinds("fnc // this is ignored\nxilit"),
            vec![TokenKind::Fnc, TokenKind::Xilit, TokenKind::Eof]
        );
    }

    #[test]
    fn line_col_tracking() {
        let tokens = Lexer::new("fnc\n  xilit").tokenize().unwrap();
        assert_eq!((tokens[0].line, tokens[0].col), (1, 1));
        assert_eq!((tokens[1].line, tokens[1].col), (2, 3));
    }

    #[test]
    fn tokenize_hello_example() {
        let source = r#"fnc kort() {
    yazde("Salam, mott!");
}
"#;
        let ks = kinds(source);
        assert_eq!(
            ks,
            vec![
                TokenKind::Fnc,
                TokenKind::Ident("kort".into()),
                TokenKind::LParen,
                TokenKind::RParen,
                TokenKind::LBrace,
                TokenKind::Yazde,
                TokenKind::LParen,
                TokenKind::String(vec![StringPart::Literal("Salam, mott!".into())]),
                TokenKind::RParen,
                TokenKind::Semicolon,
                TokenKind::RBrace,
                TokenKind::Semicolon, // synthetic after closing `}` + newline
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn newline_synthesizes_semicolon_after_expression() {
        // Two `xilit` statements separated only by newline — no explicit `;`.
        let ks = kinds("xilit x = 1\nxilit y = 2\n");
        assert_eq!(
            ks,
            vec![
                TokenKind::Xilit,
                TokenKind::Ident("x".into()),
                TokenKind::Assign,
                TokenKind::Integer(1),
                TokenKind::Semicolon, // synthetic
                TokenKind::Xilit,
                TokenKind::Ident("y".into()),
                TokenKind::Assign,
                TokenKind::Integer(2),
                TokenKind::Semicolon, // synthetic
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn no_synthetic_semicolon_after_binary_operator() {
        // Multi-line expressions continue across newlines when the line ends
        // on a token that can't terminate a statement.
        let ks = kinds("xilit x = 1 +\n    2\n");
        assert_eq!(
            ks,
            vec![
                TokenKind::Xilit,
                TokenKind::Ident("x".into()),
                TokenKind::Assign,
                TokenKind::Integer(1),
                TokenKind::Plus,
                TokenKind::Integer(2),
                TokenKind::Semicolon,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn no_synthetic_semicolon_inside_parens() {
        // Inside `(`, newlines are pure whitespace.
        let ks = kinds("foo(1,\n    2,\n    3)\n");
        assert_eq!(
            ks,
            vec![
                TokenKind::Ident("foo".into()),
                TokenKind::LParen,
                TokenKind::Integer(1),
                TokenKind::Comma,
                TokenKind::Integer(2),
                TokenKind::Comma,
                TokenKind::Integer(3),
                TokenKind::RParen,
                TokenKind::Semicolon,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn blank_lines_do_not_produce_duplicate_semicolons() {
        // Consecutive newlines only emit a single terminator — the second
        // newline sees `Semicolon` as the last token and skips.
        let ks = kinds("xilit x = 1\n\n\nxilit y = 2\n");
        let semis = ks
            .iter()
            .filter(|k| matches!(k, TokenKind::Semicolon))
            .count();
        assert_eq!(semis, 2, "expected two Semicolons, got: {:?}", ks);
    }

    #[test]
    fn explicit_semicolon_still_works() {
        // Users who want C-style can still write `;` — lexer produces a
        // single Semicolon, no duplicate from the following newline.
        let ks = kinds("xilit x = 1;\n");
        let semis = ks
            .iter()
            .filter(|k| matches!(k, TokenKind::Semicolon))
            .count();
        assert_eq!(semis, 1, "expected single Semicolon, got: {:?}", ks);
    }
}
