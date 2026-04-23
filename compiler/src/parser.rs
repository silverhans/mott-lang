use crate::ast::{BinOp, Block, Expr, Function, Item, Param, Program, Stmt, Type, UnOp};
use crate::error::{Error, Result};
use crate::token::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse(&mut self) -> Result<Program> {
        let mut items = Vec::new();
        while !self.at_end() {
            items.push(self.parse_item()?);
        }
        Ok(Program { items })
    }

    // ---- top-level items ----

    fn parse_item(&mut self) -> Result<Item> {
        match self.peek() {
            TokenKind::Fnc => Ok(Item::Function(self.parse_function()?)),
            _ => {
                let (line, col) = self.peek_pos();
                Err(Error::Parse {
                    line,
                    col,
                    message: format!("expected `fnc` at top level, got {:?}", self.peek()),
                })
            }
        }
    }

    fn parse_function(&mut self) -> Result<Function> {
        self.expect(&TokenKind::Fnc, "expected `fnc`")?;
        let name = self.expect_ident("expected function name after `fnc`")?;
        self.expect(&TokenKind::LParen, "expected `(` after function name")?;
        let mut params = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                let pname = self.expect_ident("expected parameter name")?;
                self.expect(&TokenKind::Colon, "expected `:` after parameter name")?;
                let ty = self.parse_type()?;
                params.push(Param { name: pname, ty });
                if !self.matches(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(&TokenKind::RParen, "expected `)` after parameters")?;
        let return_type = if self.matches(&TokenKind::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };
        let body = self.parse_block()?;
        Ok(Function {
            name,
            params,
            return_type,
            body,
        })
    }

    // ---- statements ----

    fn parse_block(&mut self) -> Result<Block> {
        self.expect(&TokenKind::LBrace, "expected `{`")?;
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.at_end() {
            stmts.push(self.parse_stmt()?);
        }
        self.expect(&TokenKind::RBrace, "expected `}`")?;
        Ok(Block { stmts })
    }

    fn parse_stmt(&mut self) -> Result<Stmt> {
        match self.peek() {
            TokenKind::Xilit => self.parse_let(),
            TokenKind::NagahSanna => self.parse_if(),
            TokenKind::Cqachunna => self.parse_while(),
            TokenKind::Yuxadalo => self.parse_return(),
            TokenKind::Yazde => self.parse_print(),
            TokenKind::Ident(_) if self.peek_kind_at(1) == Some(&TokenKind::Assign) => {
                self.parse_assign()
            }
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_let(&mut self) -> Result<Stmt> {
        self.expect(&TokenKind::Xilit, "expected `xilit`")?;
        let name = self.expect_ident("expected variable name after `xilit`")?;
        let ty = if self.matches(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(&TokenKind::Assign, "expected `=` in variable declaration")?;
        let value = self.parse_expr()?;
        self.expect(&TokenKind::Semicolon, "expected `;` after declaration")?;
        Ok(Stmt::Let { name, ty, value })
    }

    fn parse_assign(&mut self) -> Result<Stmt> {
        let name = self.expect_ident("expected identifier")?;
        self.expect(&TokenKind::Assign, "expected `=`")?;
        let value = self.parse_expr()?;
        self.expect(&TokenKind::Semicolon, "expected `;` after assignment")?;
        Ok(Stmt::Assign { name, value })
    }

    fn parse_if(&mut self) -> Result<Stmt> {
        self.expect(&TokenKind::NagahSanna, "expected `nagah sanna`")?;
        self.expect(&TokenKind::LParen, "expected `(` after `nagah sanna`")?;
        let cond = self.parse_expr()?;
        self.expect(&TokenKind::RParen, "expected `)` after condition")?;
        let then_block = self.parse_block()?;
        let else_block = if self.matches(&TokenKind::Khi) {
            Some(self.parse_block()?)
        } else {
            None
        };
        Ok(Stmt::If {
            cond,
            then_block,
            else_block,
        })
    }

    fn parse_while(&mut self) -> Result<Stmt> {
        self.expect(&TokenKind::Cqachunna, "expected `cqachunna`")?;
        self.expect(&TokenKind::LParen, "expected `(` after `cqachunna`")?;
        let cond = self.parse_expr()?;
        self.expect(&TokenKind::RParen, "expected `)` after condition")?;
        let body = self.parse_block()?;
        Ok(Stmt::While { cond, body })
    }

    fn parse_return(&mut self) -> Result<Stmt> {
        self.expect(&TokenKind::Yuxadalo, "expected `yuxadalo`")?;
        let value = if self.check(&TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_expr()?)
        };
        self.expect(&TokenKind::Semicolon, "expected `;` after return")?;
        Ok(Stmt::Return(value))
    }

    fn parse_print(&mut self) -> Result<Stmt> {
        self.expect(&TokenKind::Yazde, "expected `yazde`")?;
        self.expect(&TokenKind::LParen, "expected `(` after `yazde`")?;
        let arg = self.parse_expr()?;
        self.expect(&TokenKind::RParen, "expected `)` after `yazde` argument")?;
        self.expect(&TokenKind::Semicolon, "expected `;`")?;
        Ok(Stmt::Print(arg))
    }

    fn parse_expr_stmt(&mut self) -> Result<Stmt> {
        let e = self.parse_expr()?;
        self.expect(&TokenKind::Semicolon, "expected `;` after expression")?;
        Ok(Stmt::ExprStmt(e))
    }

    // ---- expressions (precedence climbs from parse_or down to parse_primary) ----

    fn parse_expr(&mut self) -> Result<Expr> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr> {
        let first = self.parse_and()?;
        if !matches!(self.peek(), TokenKind::Ya) {
            return Ok(first);
        }
        let mut operands = vec![first];
        while self.matches(&TokenKind::Ya) {
            operands.push(self.parse_and()?);
        }
        Ok(Expr::LogicOr(operands))
    }

    /// AND with Chechen-style trailing `a` plus commas: `c1 a, c2 a [, c3 a ...]`.
    /// Once the first conjunct is followed by `a`, we commit: at least 2 conjuncts required,
    /// every conjunct must have its trailing `a`. Greedy: keeps extending while `,` follows.
    /// Use parens to delimit when adjacent context also uses `,` (e.g. call args).
    fn parse_and(&mut self) -> Result<Expr> {
        let first = self.parse_cmp()?;
        if !matches!(self.peek(), TokenKind::A) {
            return Ok(first);
        }
        self.advance(); // consume first `a`
        self.expect(&TokenKind::Comma, "expected `,` between AND conjuncts")?;
        let second = self.parse_cmp()?;
        self.expect(&TokenKind::A, "expected `a` after AND conjunct")?;
        let mut operands = vec![first, second];
        while self.matches(&TokenKind::Comma) {
            let next = self.parse_cmp()?;
            self.expect(&TokenKind::A, "expected `a` after AND conjunct")?;
            operands.push(next);
        }
        Ok(Expr::LogicAnd(operands))
    }

    fn parse_cmp(&mut self) -> Result<Expr> {
        let left = self.parse_add()?;
        let op = match self.peek() {
            TokenKind::Eq => BinOp::Eq,
            TokenKind::NotEq => BinOp::NotEq,
            TokenKind::Lt => BinOp::Lt,
            TokenKind::Le => BinOp::Le,
            TokenKind::Gt => BinOp::Gt,
            TokenKind::Ge => BinOp::Ge,
            _ => return Ok(left),
        };
        self.advance();
        let right = self.parse_add()?;
        Ok(Expr::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        })
    }

    fn parse_add(&mut self) -> Result<Expr> {
        let mut left = self.parse_mul()?;
        loop {
            let op = match self.peek() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_mul()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_mul(&mut self) -> Result<Expr> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr> {
        let op = match self.peek() {
            TokenKind::Minus => UnOp::Neg,
            TokenKind::Not => UnOp::Not,
            _ => return self.parse_primary(),
        };
        self.advance();
        let inner = self.parse_unary()?;
        Ok(Expr::Unary {
            op,
            expr: Box::new(inner),
        })
    }

    fn parse_primary(&mut self) -> Result<Expr> {
        let (line, col) = self.peek_pos();
        match self.peek().clone() {
            TokenKind::Integer(n) => {
                self.advance();
                Ok(Expr::Integer(n))
            }
            TokenKind::Float(x) => {
                self.advance();
                Ok(Expr::Float(x))
            }
            TokenKind::Baqderg => {
                self.advance();
                Ok(Expr::Bool(true))
            }
            TokenKind::Xarco => {
                self.advance();
                Ok(Expr::Bool(false))
            }
            TokenKind::String(parts) => {
                self.advance();
                Ok(Expr::String(parts))
            }
            TokenKind::Ident(name) => {
                self.advance();
                if self.matches(&TokenKind::LParen) {
                    let mut args = Vec::new();
                    if !self.check(&TokenKind::RParen) {
                        args.push(self.parse_expr()?);
                        while self.matches(&TokenKind::Comma) {
                            args.push(self.parse_expr()?);
                        }
                    }
                    self.expect(&TokenKind::RParen, "expected `)` after arguments")?;
                    Ok(Expr::Call { callee: name, args })
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            TokenKind::LParen => {
                self.advance();
                let e = self.parse_expr()?;
                self.expect(&TokenKind::RParen, "expected `)`")?;
                Ok(e)
            }
            other => Err(Error::Parse {
                line,
                col,
                message: format!("expected expression, got {:?}", other),
            }),
        }
    }

    // ---- types ----

    fn parse_type(&mut self) -> Result<Type> {
        let (line, col) = self.peek_pos();
        let ty = match self.peek() {
            TokenKind::Terah => Type::Terah,
            TokenKind::Bool => Type::Bool,
            TokenKind::Deshnash => Type::Deshnash,
            TokenKind::Daqosh => Type::Daqosh,
            other => {
                return Err(Error::Parse {
                    line,
                    col,
                    message: format!("expected type name, got {:?}", other),
                })
            }
        };
        self.advance();
        Ok(ty)
    }

    // ---- low-level helpers ----

    fn peek(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn peek_kind_at(&self, offset: usize) -> Option<&TokenKind> {
        self.tokens.get(self.pos + offset).map(|t| &t.kind)
    }

    fn peek_pos(&self) -> (usize, usize) {
        let t = &self.tokens[self.pos];
        (t.line, t.col)
    }

    fn advance(&mut self) {
        if !matches!(self.peek(), TokenKind::Eof) {
            self.pos += 1;
        }
    }

    fn at_end(&self) -> bool {
        matches!(self.peek(), TokenKind::Eof)
    }

    /// Discriminant-based kind check. Only use with fieldless variants or dummy-valued ones
    /// (we only call it with unit variants in this parser).
    fn check(&self, expected: &TokenKind) -> bool {
        std::mem::discriminant(self.peek()) == std::mem::discriminant(expected)
    }

    fn matches(&mut self, expected: &TokenKind) -> bool {
        if self.check(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, expected: &TokenKind, message: &str) -> Result<()> {
        if self.check(expected) {
            self.advance();
            Ok(())
        } else {
            let (line, col) = self.peek_pos();
            Err(Error::Parse {
                line,
                col,
                message: format!("{} (got {:?})", message, self.peek()),
            })
        }
    }

    fn expect_ident(&mut self, message: &str) -> Result<String> {
        let (line, col) = self.peek_pos();
        if let TokenKind::Ident(name) = self.peek().clone() {
            self.advance();
            Ok(name)
        } else {
            Err(Error::Parse {
                line,
                col,
                message: format!("{} (got {:?})", message, self.peek()),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::token::StringPart;

    fn parse_source(src: &str) -> Result<Program> {
        let tokens = Lexer::new(src).tokenize()?;
        Parser::new(tokens).parse()
    }

    fn parse_ok(src: &str) -> Program {
        parse_source(src).expect("parse should succeed")
    }

    fn only_function(p: &Program) -> &Function {
        assert_eq!(p.items.len(), 1);
        let Item::Function(f) = &p.items[0];
        f
    }

    #[test]
    fn empty_main_function() {
        let p = parse_ok("fnc kort() {}");
        let f = only_function(&p);
        assert_eq!(f.name, "kort");
        assert!(f.params.is_empty());
        assert!(f.return_type.is_none());
        assert!(f.body.stmts.is_empty());
    }

    #[test]
    fn function_with_params_and_return_type() {
        // Note: `a` is a reserved keyword (AND operator), so we use `x`/`y` as param names.
        let p = parse_ok("fnc add(x: terah, y: terah) -> terah { yuxadalo x + y; }");
        let f = only_function(&p);
        assert_eq!(f.name, "add");
        assert_eq!(f.params.len(), 2);
        assert_eq!(f.params[0].name, "x");
        assert_eq!(f.params[0].ty, Type::Terah);
        assert_eq!(f.return_type, Some(Type::Terah));
        assert_eq!(f.body.stmts.len(), 1);
        assert!(matches!(f.body.stmts[0], Stmt::Return(Some(_))));
    }

    #[test]
    fn reserved_word_a_cannot_be_identifier() {
        // `a` is the AND operator, so it can't be a variable/parameter name.
        let err = parse_source("fnc kort() { xilit a = 5; }").unwrap_err();
        assert!(format!("{}", err).contains("variable name"));
    }

    #[test]
    fn let_statement_inferred_type() {
        let p = parse_ok("fnc kort() { xilit x = 5; }");
        let f = only_function(&p);
        match &f.body.stmts[0] {
            Stmt::Let { name, ty, value } => {
                assert_eq!(name, "x");
                assert!(ty.is_none());
                assert!(matches!(value, Expr::Integer(5)));
            }
            other => panic!("expected Let, got {:?}", other),
        }
    }

    #[test]
    fn let_statement_explicit_type() {
        let p = parse_ok("fnc kort() { xilit x: terah = 5; }");
        let f = only_function(&p);
        match &f.body.stmts[0] {
            Stmt::Let { ty, .. } => assert_eq!(*ty, Some(Type::Terah)),
            other => panic!("expected Let, got {:?}", other),
        }
    }

    #[test]
    fn assignment_vs_equality() {
        // `x = 5;` is assignment, not the comparison `x == 5`
        let p = parse_ok("fnc kort() { xilit x = 0; x = 5; }");
        let f = only_function(&p);
        assert!(matches!(f.body.stmts[1], Stmt::Assign { .. }));
    }

    #[test]
    fn arithmetic_precedence_mul_binds_tighter_than_add() {
        let p = parse_ok("fnc kort() { xilit x = 1 + 2 * 3; }");
        let f = only_function(&p);
        // Should be: Add(1, Mul(2, 3))
        if let Stmt::Let {
            value: Expr::Binary { op, left, right },
            ..
        } = &f.body.stmts[0]
        {
            assert!(matches!(op, BinOp::Add));
            assert!(matches!(**left, Expr::Integer(1)));
            assert!(matches!(
                **right,
                Expr::Binary {
                    op: BinOp::Mul,
                    ..
                }
            ));
        } else {
            panic!("unexpected AST shape");
        }
    }

    #[test]
    fn if_with_else() {
        let p = parse_ok(
            r#"fnc kort() {
                nagah sanna (x < 5) {
                    yazde("small");
                } khi {
                    yazde("big");
                }
            }"#,
        );
        let f = only_function(&p);
        match &f.body.stmts[0] {
            Stmt::If {
                else_block: Some(_),
                ..
            } => {}
            other => panic!("expected If-with-else, got {:?}", other),
        }
    }

    #[test]
    fn while_loop() {
        let p = parse_ok("fnc kort() { cqachunna (i < 10) { i = i + 1; } }");
        let f = only_function(&p);
        assert!(matches!(f.body.stmts[0], Stmt::While { .. }));
    }

    #[test]
    fn logical_and_two_conjuncts() {
        let p = parse_ok("fnc kort() { nagah sanna (x > 0 a, x < 10 a) { yazde(\"ok\"); } }");
        let f = only_function(&p);
        if let Stmt::If { cond, .. } = &f.body.stmts[0] {
            if let Expr::LogicAnd(items) = cond {
                assert_eq!(items.len(), 2);
            } else {
                panic!("expected LogicAnd, got {:?}", cond);
            }
        } else {
            panic!("expected if");
        }
    }

    #[test]
    fn logical_and_three_conjuncts() {
        let p = parse_ok(
            "fnc kort() { nagah sanna (x > 0 a, x < 10 a, y != 0 a) { yazde(\"ok\"); } }",
        );
        let f = only_function(&p);
        if let Stmt::If {
            cond: Expr::LogicAnd(items),
            ..
        } = &f.body.stmts[0]
        {
            assert_eq!(items.len(), 3);
        } else {
            panic!("expected LogicAnd with 3 conjuncts");
        }
    }

    #[test]
    fn logical_and_requires_trailing_a() {
        // `x a,` with no second conjunct after → parse error
        let err = parse_source("fnc kort() { nagah sanna (x a) { } }").unwrap_err();
        assert!(format!("{}", err).contains("expected `,`"));
    }

    #[test]
    fn logical_or_chained() {
        let p = parse_ok("fnc kort() { nagah sanna (x < 0 ya x > 100 ya x == 5) { } }");
        let f = only_function(&p);
        if let Stmt::If {
            cond: Expr::LogicOr(items),
            ..
        } = &f.body.stmts[0]
        {
            assert_eq!(items.len(), 3);
        } else {
            panic!("expected LogicOr with 3 operands");
        }
    }

    #[test]
    fn unary_minus_and_not() {
        let p = parse_ok("fnc kort() { xilit x = -5; xilit y = !baqderg; }");
        let f = only_function(&p);
        assert!(matches!(
            &f.body.stmts[0],
            Stmt::Let {
                value: Expr::Unary { op: UnOp::Neg, .. },
                ..
            }
        ));
        assert!(matches!(
            &f.body.stmts[1],
            Stmt::Let {
                value: Expr::Unary { op: UnOp::Not, .. },
                ..
            }
        ));
    }

    #[test]
    fn call_expression() {
        let p = parse_ok("fnc kort() { add(1, 2); }");
        let f = only_function(&p);
        if let Stmt::ExprStmt(Expr::Call { callee, args }) = &f.body.stmts[0] {
            assert_eq!(callee, "add");
            assert_eq!(args.len(), 2);
        } else {
            panic!("expected call expr");
        }
    }

    #[test]
    fn parenthesized_expression_overrides_precedence() {
        let p = parse_ok("fnc kort() { xilit x = (1 + 2) * 3; }");
        let f = only_function(&p);
        if let Stmt::Let {
            value:
                Expr::Binary {
                    op: BinOp::Mul,
                    left,
                    ..
                },
            ..
        } = &f.body.stmts[0]
        {
            assert!(matches!(
                **left,
                Expr::Binary {
                    op: BinOp::Add,
                    ..
                }
            ));
        } else {
            panic!("expected (1+2)*3 -> Mul at root");
        }
    }

    #[test]
    fn string_interpolation_preserved_in_ast() {
        let p = parse_ok(r#"fnc kort() { yazde("x = {x}"); }"#);
        let f = only_function(&p);
        if let Stmt::Print(Expr::String(parts)) = &f.body.stmts[0] {
            assert_eq!(parts.len(), 2);
            assert!(matches!(parts[0], StringPart::Literal(_)));
            assert!(matches!(parts[1], StringPart::Interpolation(_)));
        } else {
            panic!("expected yazde with interpolated string");
        }
    }

    #[test]
    fn parses_fizzbuzz_example() {
        let source = include_str!("../../examples/fizzbuzz.mott");
        let p = parse_ok(source);
        let f = only_function(&p);
        assert_eq!(f.name, "kort");
        // Body: xilit i..., cqachunna(...)
        assert_eq!(f.body.stmts.len(), 2);
        assert!(matches!(f.body.stmts[0], Stmt::Let { .. }));
        assert!(matches!(f.body.stmts[1], Stmt::While { .. }));
    }

    #[test]
    fn parses_hello_example() {
        let source = include_str!("../../examples/hello.mott");
        let p = parse_ok(source);
        let f = only_function(&p);
        assert_eq!(f.name, "kort");
        assert!(matches!(f.body.stmts[0], Stmt::Print(_)));
    }

    #[test]
    fn missing_semicolon_errors() {
        let err = parse_source("fnc kort() { xilit x = 5 }").unwrap_err();
        assert!(format!("{}", err).contains("expected `;`"));
    }

    #[test]
    fn top_level_must_be_function() {
        let err = parse_source("xilit x = 5;").unwrap_err();
        assert!(format!("{}", err).contains("expected `fnc`"));
    }
}
