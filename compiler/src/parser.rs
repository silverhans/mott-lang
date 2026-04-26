use crate::ast::{
    self, BinOp, Block, Expr, Field, Function, Item, IterSource, Param, Program, Stmt, StructDef,
    Type, UnOp,
};
use crate::error::{Error, Result};
use crate::lexer::Lexer;
use crate::token::{self, Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    /// Whether `Ident { ... }` should be parsed as a struct literal here.
    /// Set to false during the condition of `nagah sanna` / `cqachunna` /
    /// `yallalc` because the trailing `{` belongs to the body block —
    /// otherwise we'd misparse `nagah sanna p { ... }` as a struct lit.
    /// Same trick Rust uses; users wanting a struct literal in a
    /// condition wrap it in parens.
    allow_struct_lit: bool,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            allow_struct_lit: true,
        }
    }

    pub fn parse(&mut self) -> Result<Program> {
        let mut items = Vec::new();
        loop {
            // Skip stray `;` between items — the lexer synthesizes them
            // after each function's closing `}`, and users may write blank
            // lines between top-level declarations.
            while self.matches(&TokenKind::Semicolon) {}
            if self.at_end() {
                break;
            }
            items.push(self.parse_item()?);
        }
        Ok(Program { items })
    }

    // ---- top-level items ----

    fn parse_item(&mut self) -> Result<Item> {
        match self.peek() {
            TokenKind::Fnc => Ok(Item::Function(self.parse_function()?)),
            TokenKind::Kep => Ok(Item::Struct(self.parse_kep()?)),
            TokenKind::Eca => self.parse_import(),
            _ => {
                let (line, col) = self.peek_pos();
                Err(Error::Parse {
                    line,
                    col,
                    message: format!(
                        "expected `fnc`, `kep`, or `eca` at top level, got {:?}",
                        self.peek()
                    ),
                })
            }
        }
    }

    /// `eca module_name` — import directive. Only valid at top level.
    /// We don't enforce ordering (imports-before-decls) at parse time —
    /// the loader runs imports anyway, so it's consistent regardless.
    fn parse_import(&mut self) -> Result<Item> {
        self.expect(&TokenKind::Eca, "expected `eca`")?;
        let module = self.expect_ident("expected module name after `eca`")?;
        self.expect(&TokenKind::Semicolon, "expected `;` after import")?;
        Ok(Item::Import { module })
    }

    /// `kep Name { f1: T1, f2: T2 }` — top-level struct declaration.
    /// Trailing comma allowed. Empty `kep Empty {}` is also valid.
    fn parse_kep(&mut self) -> Result<StructDef> {
        self.expect(&TokenKind::Kep, "expected `kep`")?;
        let name = self.expect_ident("expected struct name after `kep`")?;
        self.expect(&TokenKind::LBrace, "expected `{` after struct name")?;
        let mut fields = Vec::new();
        // Skip leading newlines (synthesized `;`s) inside the braces.
        while self.matches(&TokenKind::Semicolon) {}
        if !self.check(&TokenKind::RBrace) {
            loop {
                let fname = self.expect_ident("expected field name")?;
                self.expect(&TokenKind::Colon, "expected `:` after field name")?;
                let ty = self.parse_type()?;
                fields.push(Field { name: fname, ty });
                // Separator: comma, semicolon (newline), or both.
                let saw_comma = self.matches(&TokenKind::Comma);
                while self.matches(&TokenKind::Semicolon) {}
                if !saw_comma && !self.check(&TokenKind::RBrace) {
                    // Need either a comma or end-of-list; otherwise the user
                    // probably forgot a separator between fields.
                    let (line, col) = self.peek_pos();
                    return Err(Error::Parse {
                        line,
                        col,
                        message: "expected `,` or newline between struct fields".into(),
                    });
                }
                if self.check(&TokenKind::RBrace) {
                    break;
                }
            }
        }
        self.expect(&TokenKind::RBrace, "expected `}` to close struct")?;
        Ok(StructDef {
            name,
            fields,
            module: None,
        })
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
        // Extern declaration: signature without body. The lexer
        // synthesizes a `;` after the closing paren / type, so we look
        // for that. Body-bearing functions have `{` here instead.
        let body = if self.check(&TokenKind::LBrace) {
            Some(self.parse_block()?)
        } else {
            // Eat the synthesized terminator; if it's missing, expect()
            // gives a clearer error than letting the next stmt parse fail.
            self.expect(
                &TokenKind::Semicolon,
                "extern function declaration: expected `;` or `{` after signature",
            )?;
            None
        };
        Ok(Function {
            name,
            params,
            return_type,
            body,
            module: None, // user-level by default; loader sets it for imports
        })
    }

    // ---- statements ----

    fn parse_block(&mut self) -> Result<Block> {
        self.expect(&TokenKind::LBrace, "expected `{`")?;
        let mut stmts = Vec::new();
        loop {
            // Skip stray terminators: the lexer synthesizes `;` on newlines,
            // and users may also write explicit `;`. Blank lines and
            // redundant `;` both collapse into multiple consecutive Semicolon
            // tokens — we treat them as a single boundary.
            while self.matches(&TokenKind::Semicolon) {}
            if self.check(&TokenKind::RBrace) || self.at_end() {
                break;
            }
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
            TokenKind::Yallalc => self.parse_for_each(),
            TokenKind::Sac => self.parse_break(),
            TokenKind::Khida => self.parse_continue(),
            TokenKind::Yuxadalo => self.parse_return(),
            TokenKind::Yazde => self.parse_print(),
            TokenKind::Push => self.parse_push(),
            TokenKind::Ident(_) if self.peek_kind_at(1) == Some(&TokenKind::Assign) => {
                self.parse_assign()
            }
            // `ident[...] = ...` — index-assignment. We need to look past
            // the bracketed expression for `=`. Rather than full lookahead,
            // detect the `ident [` start and try index-assign first; fall
            // back to expr-stmt if what follows isn't an assignment shape.
            TokenKind::Ident(_) if self.peek_kind_at(1) == Some(&TokenKind::LBracket) => {
                self.parse_maybe_index_assign()
            }
            // `ident.field = expr` — field assignment. Same shape as index
            // assignment: try the assign path, fall back to expr-stmt if
            // we don't actually see `=` after `.field`.
            TokenKind::Ident(_) if self.peek_kind_at(1) == Some(&TokenKind::Dot) => {
                self.parse_maybe_field_assign()
            }
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_for_each(&mut self) -> Result<Stmt> {
        self.expect(&TokenKind::Yallalc, "expected `yallalc`")?;
        let var = self.expect_ident("expected loop variable after `yallalc`")?;
        self.expect(&TokenKind::Chu, "expected `chu` after loop variable")?;
        // Same struct-lit restriction as `if` / `while`: the trailing `{`
        // is the loop body, not a struct literal payload.
        let first = self.parse_expr_no_struct_lit()?;
        let iter = if self.matches(&TokenKind::DotDot) {
            let end = self.parse_expr_no_struct_lit()?;
            IterSource::Range { start: first, end }
        } else {
            IterSource::Array(first)
        };
        let body = self.parse_block()?;
        Ok(Stmt::ForEach { var, iter, body })
    }

    /// Called when `ident . ` is at the start of a statement. If it
    /// matches `ident.field = expr` shape, produce a FieldAssign;
    /// otherwise rewind and parse as an expression statement (the user
    /// wrote something like `point.x` as a side-effect-free expression,
    /// which is weird but legal).
    fn parse_maybe_field_assign(&mut self) -> Result<Stmt> {
        let start_pos = self.pos;
        let target = self.expect_ident("expected identifier")?;
        self.expect(&TokenKind::Dot, "expected `.`")?;
        // Only a single-field chain in v0.3 — `a.b.c = ...` rejected here
        // with a hint, since the codegen path doesn't support it yet.
        let field = self.expect_ident("expected field name after `.`")?;
        if self.check(&TokenKind::Dot) {
            return Err(Error::Parse {
                line: self.tokens[self.pos].line,
                col: self.tokens[self.pos].col,
                message: "chained field assignment (`a.b.c = ...`) isn't supported yet — \
                          assign to a local copy and write it back"
                    .into(),
            });
        }
        if self.matches(&TokenKind::Assign) {
            let value = self.parse_expr()?;
            self.expect(&TokenKind::Semicolon, "expected `;` after assignment")?;
            Ok(Stmt::FieldAssign {
                target,
                field,
                value,
            })
        } else {
            // Not an assignment — rewind and parse as expression statement.
            self.pos = start_pos;
            self.parse_expr_stmt()
        }
    }

    /// Called when `ident [` is at the start of a statement. The expression
    /// `ident[idx]` followed by `=` is an IndexAssign; anything else is an
    /// expression statement (a read-only indexing). We commit to one path
    /// by peeking past the closing `]`.
    fn parse_maybe_index_assign(&mut self) -> Result<Stmt> {
        // Snapshot the parse position so we can fall back if it turns out
        // to not be an assignment.
        let start_pos = self.pos;
        let name = self.expect_ident("expected identifier")?;
        self.expect(&TokenKind::LBracket, "expected `[`")?;
        let index = self.parse_expr()?;
        self.expect(&TokenKind::RBracket, "expected `]`")?;
        if self.matches(&TokenKind::Assign) {
            let value = self.parse_expr()?;
            self.expect(&TokenKind::Semicolon, "expected `;` after assignment")?;
            Ok(Stmt::IndexAssign { name, index, value })
        } else {
            // Not an assignment — rewind and reparse as an expression stmt.
            self.pos = start_pos;
            self.parse_expr_stmt()
        }
    }

    fn parse_break(&mut self) -> Result<Stmt> {
        self.expect(&TokenKind::Sac, "expected `sac`")?;
        self.expect(&TokenKind::Semicolon, "expected `;` after `sac`")?;
        Ok(Stmt::Break)
    }

    fn parse_continue(&mut self) -> Result<Stmt> {
        self.expect(&TokenKind::Khida, "expected `khida`")?;
        self.expect(&TokenKind::Semicolon, "expected `;` after `khida`")?;
        Ok(Stmt::Continue)
    }

    fn parse_let(&mut self) -> Result<Stmt> {
        self.expect(&TokenKind::Xilit, "expected `xilit`")?;
        let (name_line, name_col) = self.peek_pos();
        let name = self.expect_ident("expected variable name after `xilit`")?;
        let ty = if self.matches(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        // Three shapes: `xilit x = e`, `xilit x: T = e`, `xilit x: T`.
        // Without a type annotation, we need an initializer to infer the
        // type — so the fourth shape (`xilit x`) is rejected here with a
        // targeted error.
        let value = if self.matches(&TokenKind::Assign) {
            Some(self.parse_expr()?)
        } else if ty.is_some() {
            None
        } else {
            return Err(Error::Parse {
                line: name_line,
                col: name_col,
                message: format!(
                    "variable `{}` needs either a type annotation \
                     (`xilit {}: terah`) or an initializer (`xilit {} = 0`) — \
                     the compiler can't guess the type from nothing",
                    name, name, name
                ),
            });
        };
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
        // No required `(...)` around the condition: `{` delimits its end.
        // Grouping `(...)` inside the expression still works as a primary.
        // We disable struct literals in the condition so `nagah sanna p {`
        // is parsed as condition+block, not as `Ident { ... }`.
        let cond = self.parse_expr_no_struct_lit()?;
        let then_block = self.parse_block()?;

        // The lexer synthesizes `;` on newlines after `}`. Skip it so that
        // `} vusht {` on separate lines still joins as a single if/else.
        // This is the mott equivalent of Go forbidding `}\nelse`: we just
        // don't force the user to put it on the same line.
        while self.matches(&TokenKind::Semicolon) {}

        // `vusht` may be followed by either `{...}` (classic else) or
        // `nagah sanna (...) {...}` (else-if sugar). In the sugar case we
        // desugar into a nested `If` wrapped in a single-stmt Block so the
        // rest of the pipeline (typecheck, codegen) doesn't need to know
        // about chains at all. The C backend then re-detects this pattern
        // to emit idiomatic `else if`.
        let else_block = if self.matches(&TokenKind::Vusht) {
            if self.check(&TokenKind::NagahSanna) {
                let nested_if = self.parse_if()?;
                Some(Block {
                    stmts: vec![nested_if],
                })
            } else {
                Some(self.parse_block()?)
            }
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
        let cond = self.parse_expr_no_struct_lit()?;
        let body = self.parse_block()?;
        Ok(Stmt::While { cond, body })
    }

    /// `parse_expr` with the struct-literal restriction enabled. Used in
    /// condition contexts where `Ident { ... }` would be ambiguous with
    /// the trailing block. Saves/restores the flag so nested expressions
    /// (parens, function args) get their normal behavior.
    fn parse_expr_no_struct_lit(&mut self) -> Result<Expr> {
        let saved = self.allow_struct_lit;
        self.allow_struct_lit = false;
        let e = self.parse_expr();
        self.allow_struct_lit = saved;
        e
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

    /// `push(IDENT, expr)` — statement form. First arg must be a bare
    /// identifier so we can take its address in codegen. We reject
    /// `push(arr[0], x)` and `push(some_expr(), x)` here at parse time
    /// rather than in codegen for a clearer error.
    fn parse_push(&mut self) -> Result<Stmt> {
        self.expect(&TokenKind::Push, "expected `push`")?;
        self.expect(&TokenKind::LParen, "expected `(` after `push`")?;
        let name = self.expect_ident(
            "first argument of `push` must be a variable name (got something else)",
        )?;
        // After the ident, anything other than `,` means the user wrote a
        // complex l-value like `push(nums[0], ...)` or `push(f(), ...)`.
        // Give a pointed error rather than letting expect(Comma) complain
        // about the mysterious next token.
        if !self.check(&TokenKind::Comma) {
            let (line, col) = self.peek_pos();
            return Err(Error::Parse {
                line,
                col,
                message: format!(
                    "first argument of `push` must be a bare variable name — \
                     got `{}` followed by {:?}",
                    name,
                    self.peek()
                ),
            });
        }
        self.advance(); // consume the comma
        let value = self.parse_expr()?;
        self.expect(&TokenKind::RParen, "expected `)` after push arguments")?;
        self.expect(&TokenKind::Semicolon, "expected `;` after push")?;
        Ok(Stmt::Push { name, value })
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
                // Convert lex-time parts (raw strings) to AST parts (with
                // parsed Expr for interpolations). We re-lex + re-parse
                // each interpolation substring here.
                let ast_parts = parts
                    .into_iter()
                    .map(|p| self.lift_string_part(p))
                    .collect::<Result<Vec<_>>>()?;
                Ok(Expr::String(ast_parts))
            }
            TokenKind::Ident(name) => {
                self.advance();
                let expr = if self.matches(&TokenKind::LParen) {
                    let mut args = Vec::new();
                    if !self.check(&TokenKind::RParen) {
                        args.push(self.parse_expr()?);
                        while self.matches(&TokenKind::Comma) {
                            args.push(self.parse_expr()?);
                        }
                    }
                    self.expect(&TokenKind::RParen, "expected `)` after arguments")?;
                    Expr::Call {
                        module: None,
                        callee: name,
                        args,
                    }
                } else if self.allow_struct_lit && self.check(&TokenKind::LBrace) {
                    // `Name { f1: e1, f2: e2 }` — struct literal. Disabled
                    // when we're parsing a condition because `{` there
                    // belongs to the block body.
                    self.parse_struct_lit_body(name)?
                } else if self.check(&TokenKind::Dot)
                    && matches!(self.peek_kind_at(1), Some(TokenKind::Ident(_)))
                    && matches!(self.peek_kind_at(2), Some(TokenKind::LParen))
                {
                    // Module-qualified call: `name . field (args)`. Three
                    // tokens of lookahead disambiguate from postfix field
                    // access (`name.field` without trailing `(`). When we
                    // get methods someday this branch becomes the natural
                    // dispatch site for them too.
                    self.advance(); // '.'
                    let func = self.expect_ident("expected function name after `.`")?;
                    self.expect(&TokenKind::LParen, "expected `(` after qualified name")?;
                    let mut args = Vec::new();
                    if !self.check(&TokenKind::RParen) {
                        args.push(self.parse_expr()?);
                        while self.matches(&TokenKind::Comma) {
                            args.push(self.parse_expr()?);
                        }
                    }
                    self.expect(&TokenKind::RParen, "expected `)` after arguments")?;
                    Expr::Call {
                        module: Some(name),
                        callee: func,
                        args,
                    }
                } else {
                    Expr::Ident(name)
                };
                self.apply_postfix(expr)
            }
            TokenKind::LParen => {
                self.advance();
                // Parens reset the struct-literal restriction: an
                // explicit grouping says "this is an expression" and
                // un-shadows whatever the outer context disabled.
                let saved = self.allow_struct_lit;
                self.allow_struct_lit = true;
                let e = self.parse_expr()?;
                self.allow_struct_lit = saved;
                self.expect(&TokenKind::RParen, "expected `)`")?;
                // Postfix chains apply after `(...)` too: `(p).x`,
                // `(arr)[0]`, `(get())[0].field` all need to work.
                self.apply_postfix(e)
            }
            TokenKind::LBracket => {
                // Array literal `[e1, e2, ...]`. Empty literals `[]` now
                // parse — the codegen accepts them when the surrounding
                // context provides a type (e.g. `xilit x: [terah] = []`);
                // without a type hint, emission errors with a clear message.
                self.advance();
                let mut elems = Vec::new();
                if !self.check(&TokenKind::RBracket) {
                    elems.push(self.parse_expr()?);
                    while self.matches(&TokenKind::Comma) {
                        if self.check(&TokenKind::RBracket) {
                            break; // trailing comma allowed
                        }
                        elems.push(self.parse_expr()?);
                    }
                }
                self.expect(&TokenKind::RBracket, "expected `]` in array literal")?;
                Ok(Expr::ArrayLit(elems))
            }
            TokenKind::Esha => {
                // `esha()` — built-in read-line expression. Always zero args;
                // the parens are mandatory to keep the grammar simple and
                // to match the rest of the call syntax.
                self.advance();
                self.expect(&TokenKind::LParen, "expected `(` after `esha`")?;
                self.expect(&TokenKind::RParen, "`esha` takes no arguments")?;
                Ok(Expr::Input)
            }
            TokenKind::Baram => {
                // `baram(x)` — built-in "size/length". Works on arrays and
                // strings; the codegen picks `.len` off the right struct.
                self.advance();
                self.expect(&TokenKind::LParen, "expected `(` after `baram`")?;
                let inner = self.parse_expr()?;
                self.expect(&TokenKind::RParen, "expected `)` after `baram(...)`")?;
                Ok(Expr::Baram(Box::new(inner)))
            }
            TokenKind::ParseTerah => {
                // `parse_terah(s)` — string -> terah. Arg must be deshnash;
                // fail-fast at runtime on bad input. Structure mirrors
                // `baram` above — same primary-call shape.
                self.advance();
                self.expect(&TokenKind::LParen, "expected `(` after `parse_terah`")?;
                let inner = self.parse_expr()?;
                self.expect(
                    &TokenKind::RParen,
                    "expected `)` after `parse_terah(...)`",
                )?;
                Ok(Expr::ParseTerah(Box::new(inner)))
            }
            TokenKind::ParseDaqosh => {
                self.advance();
                self.expect(
                    &TokenKind::LParen,
                    "expected `(` after `parse_daqosh`",
                )?;
                let inner = self.parse_expr()?;
                self.expect(
                    &TokenKind::RParen,
                    "expected `)` after `parse_daqosh(...)`",
                )?;
                Ok(Expr::ParseDaqosh(Box::new(inner)))
            }
            TokenKind::ToTerah => {
                // `to_terah(x)` — numeric conversion, call-shaped like the
                // parse_* and baram primaries above. Codegen lowers it to
                // a C cast; no runtime function involved.
                self.advance();
                self.expect(&TokenKind::LParen, "expected `(` after `to_terah`")?;
                let inner = self.parse_expr()?;
                self.expect(&TokenKind::RParen, "expected `)` after `to_terah(...)`")?;
                Ok(Expr::ToTerah(Box::new(inner)))
            }
            TokenKind::ToDaqosh => {
                self.advance();
                self.expect(&TokenKind::LParen, "expected `(` after `to_daqosh`")?;
                let inner = self.parse_expr()?;
                self.expect(&TokenKind::RParen, "expected `)` after `to_daqosh(...)`")?;
                Ok(Expr::ToDaqosh(Box::new(inner)))
            }
            TokenKind::Pop => {
                // `pop(IDENT)` — expression form. Same l-value restriction
                // as push: arg must be a bare identifier. We could
                // theoretically accept `pop(arr[i])` later (remove by
                // index) but that's a different operation.
                self.advance();
                self.expect(&TokenKind::LParen, "expected `(` after `pop`")?;
                let name = self.expect_ident(
                    "argument of `pop` must be a variable name (got something else)",
                )?;
                self.expect(&TokenKind::RParen, "expected `)` after `pop(...)`")?;
                Ok(Expr::Pop(name))
            }
            other => Err(Error::Parse {
                line,
                col,
                message: format!("expected expression, got {:?}", other),
            }),
        }
    }

    /// Apply postfix `.field` / `[i]` chains to a primary expression.
    /// Either can repeat — `arr[0].field.next[i]` is valid.
    fn apply_postfix(&mut self, mut expr: Expr) -> Result<Expr> {
        loop {
            if self.matches(&TokenKind::LBracket) {
                let index = self.parse_expr()?;
                self.expect(&TokenKind::RBracket, "expected `]` after index")?;
                expr = Expr::Index {
                    target: Box::new(expr),
                    index: Box::new(index),
                };
            } else if self.matches(&TokenKind::Dot) {
                let field = self.expect_ident("expected field name after `.`")?;
                expr = Expr::FieldAccess {
                    target: Box::new(expr),
                    field,
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    /// Parse the body of a struct literal — caller has already consumed
    /// the type name and the `{`. Wait, that's wrong — caller has only
    /// the name; we still need to eat the `{` ourselves. Field/value
    /// pairs separated by `,` (newlines also accepted, like the decl).
    fn parse_struct_lit_body(&mut self, name: String) -> Result<Expr> {
        self.expect(&TokenKind::LBrace, "expected `{` for struct literal")?;
        let mut fields = Vec::new();
        while self.matches(&TokenKind::Semicolon) {}
        if !self.check(&TokenKind::RBrace) {
            loop {
                let fname = self.expect_ident("expected field name in struct literal")?;
                self.expect(&TokenKind::Colon, "expected `:` after field name")?;
                let value = self.parse_expr()?;
                fields.push((fname, value));
                let saw_comma = self.matches(&TokenKind::Comma);
                while self.matches(&TokenKind::Semicolon) {}
                if !saw_comma && !self.check(&TokenKind::RBrace) {
                    let (line, col) = self.peek_pos();
                    return Err(Error::Parse {
                        line,
                        col,
                        message: "expected `,` or newline between struct literal fields".into(),
                    });
                }
                if self.check(&TokenKind::RBrace) {
                    break;
                }
            }
        }
        self.expect(&TokenKind::RBrace, "expected `}` to close struct literal")?;
        Ok(Expr::StructLit { name, fields })
    }

    /// Convert a lex-time string part into an AST string part.
    /// Literals pass through; interpolations get their captured source
    /// re-lexed and re-parsed as a single expression.
    fn lift_string_part(&self, p: token::StringPart) -> Result<ast::StringPart> {
        match p {
            token::StringPart::Literal(s) => Ok(ast::StringPart::Literal(s)),
            token::StringPart::Interpolation(src) => {
                // Re-lex + parse the captured source as a standalone
                // expression. Errors bubble up with their own positions
                // (relative to the interpolation source, not the outer
                // file — good enough for now; we can plumb absolute
                // positions later if it gets annoying).
                let tokens = Lexer::new(&src).tokenize()?;
                let mut sub = Parser::new(tokens);
                let expr = sub.parse_expr()?;
                // Must have consumed everything except a trailing
                // synthesized `;` / EOF.
                while sub.matches(&TokenKind::Semicolon) {}
                if !sub.at_end() {
                    let (line, col) = sub.peek_pos();
                    return Err(Error::Parse {
                        line,
                        col,
                        message: format!(
                            "interpolation must be a single expression \
                             (stray tokens after `{}`...)",
                            src.trim()
                        ),
                    });
                }
                Ok(ast::StringPart::Interpolation(Box::new(expr)))
            }
        }
    }

    // ---- types ----

    fn parse_type(&mut self) -> Result<Type> {
        let (line, col) = self.peek_pos();
        // Array type: `[T]`. Recurse so `[[terah]]` would parse — though the
        // rest of the pipeline rejects nested arrays in v0.2.
        if self.matches(&TokenKind::LBracket) {
            let inner = self.parse_type()?;
            self.expect(&TokenKind::RBracket, "expected `]` in array type")?;
            return Ok(Type::Array(Box::new(inner)));
        }
        let ty = match self.peek() {
            TokenKind::Terah => Type::Terah,
            TokenKind::Bool => Type::Bool,
            TokenKind::Deshnash => Type::Deshnash,
            TokenKind::Daqosh => Type::Daqosh,
            // Identifier in a type position is a user-defined struct
            // reference. Sema validates the name actually exists.
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                return Ok(Type::Struct(name));
            }
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
    use crate::ast::StringPart;
    use crate::lexer::Lexer;

    fn parse_source(src: &str) -> Result<Program> {
        let tokens = Lexer::new(src).tokenize()?;
        Parser::new(tokens).parse()
    }

    fn parse_ok(src: &str) -> Program {
        parse_source(src).expect("parse should succeed")
    }

    fn only_function(p: &Program) -> &Function {
        assert_eq!(p.items.len(), 1);
        match &p.items[0] {
            Item::Function(f) => f,
            other => panic!("expected single function item, got {:?}", other),
        }
    }

    /// Helper for the common pattern: get the body of the only-function in
    /// a one-function program. Tests that previously did
    /// `only_function(&p).body.stmts` now do `only_body(&p).stmts`. Saves
    /// each test from peeling the new `Option<Block>` themselves.
    fn body(f: &Function) -> &Block {
        f.body.as_ref().expect("function has body")
    }

    #[test]
    fn empty_main_function() {
        let p = parse_ok("fnc kort() {}");
        let f = only_function(&p);
        assert_eq!(f.name, "kort");
        assert!(f.params.is_empty());
        assert!(f.return_type.is_none());
        assert!(body(f).stmts.is_empty());
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
        assert_eq!(body(f).stmts.len(), 1);
        assert!(matches!(body(f).stmts[0], Stmt::Return(Some(_))));
    }

    #[test]
    fn let_with_type_but_no_init_parses() {
        // `xilit x: terah` — declaration without initializer.
        // Codegen zero-inits; parser just stores None. The trailing newline
        // matters: the lexer synthesizes `;` from it (type keywords are
        // stmt-enders).
        let p = parse_ok("fnc kort() {\n    xilit x: terah\n}\n");
        let f = only_function(&p);
        match &body(f).stmts[0] {
            Stmt::Let { name, ty, value } => {
                assert_eq!(name, "x");
                assert_eq!(*ty, Some(Type::Terah));
                assert!(value.is_none());
            }
            other => panic!("expected Let, got {:?}", other),
        }
    }

    #[test]
    fn let_with_neither_type_nor_init_errors() {
        // `xilit x` — nothing to infer from; give a targeted error.
        let err = parse_source("fnc kort() {\n    xilit x\n}\n").unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("type annotation"), "got: {}", msg);
        assert!(msg.contains("initializer"), "got: {}", msg);
    }

    #[test]
    fn import_parses_at_top_level() {
        let p = parse_ok(
            r#"
eca math
fnc kort() {}
"#,
        );
        assert_eq!(p.items.len(), 2);
        match &p.items[0] {
            Item::Import { module } => assert_eq!(module, "math"),
            other => panic!("expected import, got {:?}", other),
        }
    }

    #[test]
    fn extern_function_parses_without_body() {
        let p = parse_ok(
            r#"
fnc sqrt(x: daqosh) -> daqosh
fnc kort() {}
"#,
        );
        match &p.items[0] {
            Item::Function(f) => {
                assert_eq!(f.name, "sqrt");
                assert!(f.body.is_none());
            }
            other => panic!("expected function, got {:?}", other),
        }
    }

    #[test]
    fn qualified_call_parses_as_call_with_module() {
        let p = parse_ok(
            r#"
fnc kort() {
    xilit r: daqosh = math.sqrt(2.0)
}
"#,
        );
        let f = only_function(&p);
        match &body(f).stmts[0] {
            Stmt::Let { value, .. } => match value {
                Some(Expr::Call {
                    module: Some(m),
                    callee,
                    args,
                }) => {
                    assert_eq!(m, "math");
                    assert_eq!(callee, "sqrt");
                    assert_eq!(args.len(), 1);
                }
                other => panic!("expected qualified Call, got {:?}", other),
            },
            other => panic!("expected Let, got {:?}", other),
        }
    }

    #[test]
    fn kep_declaration_parses() {
        let p = parse_ok(
            r#"
kep Point {
    x: terah,
    y: terah,
}
fnc kort() {}
"#,
        );
        assert_eq!(p.items.len(), 2);
        match &p.items[0] {
            Item::Struct(s) => {
                assert_eq!(s.name, "Point");
                assert_eq!(s.fields.len(), 2);
                assert_eq!(s.fields[0].name, "x");
                assert_eq!(s.fields[1].name, "y");
                assert_eq!(s.fields[0].ty, Type::Terah);
            }
            other => panic!("expected struct, got {:?}", other),
        }
    }

    #[test]
    fn kep_with_trailing_comma_parses() {
        // Trailing comma allowed (matches Rust convention).
        let p = parse_ok(
            r#"
kep T { foo: terah, bar: bool, }
fnc kort() {}
"#,
        );
        match &p.items[0] {
            Item::Struct(s) => assert_eq!(s.fields.len(), 2),
            _ => panic!(),
        }
    }

    #[test]
    fn struct_literal_in_let_parses() {
        let p = parse_ok(
            r#"
kep Point { x: terah, y: terah }
fnc kort() {
    xilit p: Point = Point { x: 3, y: 5 }
}
"#,
        );
        // Find the kort function, check it has Stmt::Let with StructLit value.
        match &p.items[1] {
            Item::Function(f) => match &body(f).stmts[0] {
                Stmt::Let { value, .. } => {
                    assert!(matches!(value, Some(Expr::StructLit { .. })));
                }
                other => panic!("expected Let, got {:?}", other),
            },
            _ => panic!(),
        }
    }

    #[test]
    fn struct_literal_disabled_in_if_condition() {
        // `nagah sanna p { ... }` parses as cond=Ident("p"), body={...},
        // not as `Ident("p") { ... }` struct literal. Otherwise we'd
        // misparse simple ident conditions.
        let p = parse_ok(
            r#"
kep T { x: terah }
fnc kort() {
    xilit b = baqderg
    nagah sanna b {
        yazde("yes")
    }
}
"#,
        );
        match &p.items[1] {
            Item::Function(f) => match &body(f).stmts[1] {
                Stmt::If { cond, .. } => {
                    assert!(matches!(cond, Expr::Ident(_)));
                }
                other => panic!("expected If, got {:?}", other),
            },
            _ => panic!(),
        }
    }

    #[test]
    fn struct_literal_in_paren_works_in_condition() {
        // Wrapping in parens un-shadows the struct-literal restriction.
        // (Practically silly here since == on structs is rejected by sema,
        // but the parser doesn't know that — this test is purely about
        // grammar.)
        parse_ok(
            r#"
kep T { x: terah }
fnc kort() {
    xilit p: T = T { x: 1 }
    nagah sanna (T { x: 1 }).x == p.x {
        yazde("ok")
    }
}
"#,
        );
    }

    #[test]
    fn field_assignment_parses() {
        let p = parse_ok(
            r#"
kep T { x: terah }
fnc kort() {
    xilit p: T = T { x: 1 }
    p.x = 5
}
"#,
        );
        match &p.items[1] {
            Item::Function(f) => match &body(f).stmts[1] {
                Stmt::FieldAssign {
                    target,
                    field,
                    value,
                } => {
                    assert_eq!(target, "p");
                    assert_eq!(field, "x");
                    assert!(matches!(value, Expr::Integer(5)));
                }
                other => panic!("expected FieldAssign, got {:?}", other),
            },
            _ => panic!(),
        }
    }

    #[test]
    fn chained_field_assignment_rejected() {
        // `a.b.c = ...` not supported in v0.3 — give a targeted error.
        let err = parse_source(
            r#"
kep Inner { x: terah }
kep Outer { i: Inner }
fnc kort() {
    xilit o: Outer = Outer { i: Inner { x: 1 } }
    o.i.x = 5
}
"#,
        )
        .unwrap_err();
        assert!(format!("{}", err).contains("chained field"));
    }

    #[test]
    fn field_access_chain_in_expression_works() {
        // Reading is fine — `o.i.x` parses as nested FieldAccess.
        // Only assignment is restricted.
        parse_ok(
            r#"
kep Inner { x: terah }
kep Outer { i: Inner }
fnc kort() {
    xilit o: Outer = Outer { i: Inner { x: 1 } }
    yazde("{o.i.x}")
}
"#,
        );
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
        match &body(f).stmts[0] {
            Stmt::Let { name, ty, value } => {
                assert_eq!(name, "x");
                assert!(ty.is_none());
                assert!(matches!(value, Some(Expr::Integer(5))));
            }
            other => panic!("expected Let, got {:?}", other),
        }
    }

    #[test]
    fn let_statement_explicit_type() {
        let p = parse_ok("fnc kort() { xilit x: terah = 5; }");
        let f = only_function(&p);
        match &body(f).stmts[0] {
            Stmt::Let { ty, .. } => assert_eq!(*ty, Some(Type::Terah)),
            other => panic!("expected Let, got {:?}", other),
        }
    }

    #[test]
    fn assignment_vs_equality() {
        // `x = 5;` is assignment, not the comparison `x == 5`
        let p = parse_ok("fnc kort() { xilit x = 0; x = 5; }");
        let f = only_function(&p);
        assert!(matches!(body(f).stmts[1], Stmt::Assign { .. }));
    }

    #[test]
    fn arithmetic_precedence_mul_binds_tighter_than_add() {
        let p = parse_ok("fnc kort() { xilit x = 1 + 2 * 3; }");
        let f = only_function(&p);
        // Should be: Add(1, Mul(2, 3))
        if let Stmt::Let {
            value: Some(Expr::Binary { op, left, right }),
            ..
        } = &body(f).stmts[0]
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
                } vusht {
                    yazde("big");
                }
            }"#,
        );
        let f = only_function(&p);
        match &body(f).stmts[0] {
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
        assert!(matches!(body(f).stmts[0], Stmt::While { .. }));
    }

    #[test]
    fn break_and_continue_parse_as_statements() {
        let p = parse_ok(
            r#"fnc kort() {
                cqachunna (baqderg) {
                    sac;
                    khida;
                }
            }"#,
        );
        let f = only_function(&p);
        let Stmt::While { body, .. } = &body(f).stmts[0] else {
            panic!("expected While");
        };
        assert!(matches!(body.stmts[0], Stmt::Break));
        assert!(matches!(body.stmts[1], Stmt::Continue));
    }

    #[test]
    fn sac_requires_semicolon() {
        let err = parse_source("fnc kort() { cqachunna (baqderg) { sac } }").unwrap_err();
        assert!(format!("{}", err).contains("expected `;`"));
    }

    #[test]
    fn vusht_nagah_sanna_chain_desugars_to_nested_if() {
        let p = parse_ok(
            r#"fnc kort() {
                xilit x: terah = 1;
                nagah sanna (x == 1) {
                    yazde("one");
                } vusht nagah sanna (x == 2) {
                    yazde("two");
                } vusht {
                    yazde("other");
                }
            }"#,
        );
        let f = only_function(&p);
        // body: [let, if]. The outer If's else should be a single-stmt Block
        // whose sole stmt is another If with its own else.
        let Stmt::If { else_block, .. } = &body(f).stmts[1] else {
            panic!("expected If at body[1]");
        };
        let else_block = else_block.as_ref().expect("outer if must have else");
        assert_eq!(else_block.stmts.len(), 1, "else-if should desugar to one-stmt block");
        let Stmt::If {
            else_block: inner_else,
            ..
        } = &else_block.stmts[0]
        else {
            panic!("expected nested If inside else block");
        };
        let inner_else = inner_else.as_ref().expect("inner if must have final else");
        assert!(matches!(inner_else.stmts[0], Stmt::Print(_)));
    }

    #[test]
    fn vusht_nagah_sanna_without_final_else_is_ok() {
        // Chain without a terminal `vusht { ... }` should still parse.
        let p = parse_ok(
            r#"fnc kort() {
                xilit x: terah = 1;
                nagah sanna (x == 1) {
                    yazde("one");
                } vusht nagah sanna (x == 2) {
                    yazde("two");
                }
            }"#,
        );
        let f = only_function(&p);
        let Stmt::If { else_block, .. } = &body(f).stmts[1] else {
            panic!("expected If");
        };
        let else_block = else_block.as_ref().unwrap();
        let Stmt::If {
            else_block: inner,
            ..
        } = &else_block.stmts[0]
        else {
            panic!("expected nested If");
        };
        assert!(inner.is_none(), "innermost else should be absent");
    }

    #[test]
    fn logical_and_two_conjuncts() {
        let p = parse_ok("fnc kort() { nagah sanna (x > 0 a, x < 10 a) { yazde(\"ok\"); } }");
        let f = only_function(&p);
        if let Stmt::If { cond, .. } = &body(f).stmts[0] {
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
        } = &body(f).stmts[0]
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
        } = &body(f).stmts[0]
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
            &body(f).stmts[0],
            Stmt::Let {
                value: Some(Expr::Unary { op: UnOp::Neg, .. }),
                ..
            }
        ));
        assert!(matches!(
            &body(f).stmts[1],
            Stmt::Let {
                value: Some(Expr::Unary { op: UnOp::Not, .. }),
                ..
            }
        ));
    }

    #[test]
    fn call_expression() {
        let p = parse_ok("fnc kort() { add(1, 2); }");
        let f = only_function(&p);
        if let Stmt::ExprStmt(Expr::Call { callee, args, .. }) = &body(f).stmts[0] {
            assert_eq!(callee, "add");
            assert_eq!(args.len(), 2);
        } else {
            panic!("expected call expr");
        }
    }

    #[test]
    fn esha_parses_as_input_expression() {
        let p = parse_ok("fnc kort() { xilit s: deshnash = esha(); }");
        let f = only_function(&p);
        let Stmt::Let { value, .. } = &body(f).stmts[0] else {
            panic!("expected let");
        };
        assert!(matches!(value, Some(Expr::Input)));
    }

    #[test]
    fn esha_with_args_is_rejected() {
        // `esha` takes no arguments — C/Rust-style, no Python-style prompt.
        let err =
            parse_source("fnc kort() { xilit s: deshnash = esha(\"prompt\"); }").unwrap_err();
        assert!(format!("{}", err).contains("no arguments"));
    }

    #[test]
    fn parenthesized_expression_overrides_precedence() {
        let p = parse_ok("fnc kort() { xilit x = (1 + 2) * 3; }");
        let f = only_function(&p);
        if let Stmt::Let {
            value:
                Some(Expr::Binary {
                    op: BinOp::Mul,
                    left,
                    ..
                }),
            ..
        } = &body(f).stmts[0]
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
        if let Stmt::Print(Expr::String(parts)) = &body(f).stmts[0] {
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
        assert_eq!(body(f).stmts.len(), 2);
        assert!(matches!(body(f).stmts[0], Stmt::Let { .. }));
        assert!(matches!(body(f).stmts[1], Stmt::While { .. }));
    }

    #[test]
    fn parses_hello_example() {
        let source = include_str!("../../examples/hello.mott");
        let p = parse_ok(source);
        let f = only_function(&p);
        assert_eq!(f.name, "kort");
        assert!(matches!(body(f).stmts[0], Stmt::Print(_)));
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

    #[test]
    fn parses_without_semicolons_or_condition_parens() {
        // No explicit `;` anywhere, no `(...)` around the `if` or `while`
        // conditions. The lexer synthesizes terminators and the parser now
        // ends conditions at `{`.
        let p = parse_ok(
            "fnc kort() {\n    \
                 xilit x: terah = 5\n    \
                 cqachunna x < 10 {\n        \
                     nagah sanna x == 7 {\n            \
                         yazde(\"seven\")\n        \
                     } vusht {\n            \
                         yazde(\"{x}\")\n        \
                     }\n        \
                     x = x + 1\n    \
                 }\n\
             }\n",
        );
        let f = only_function(&p);
        assert!(matches!(body(f).stmts[0], Stmt::Let { .. }));
        assert!(matches!(body(f).stmts[1], Stmt::While { .. }));
    }

    #[test]
    fn else_chain_across_newlines() {
        // `}` on one line, `vusht nagah sanna` on the next — must still parse
        // as an else-if chain (lexer's synthetic `;` gets skipped before vusht).
        let p = parse_ok(
            "fnc kort() {\n    \
                 xilit x: terah = 1\n    \
                 nagah sanna x == 1 {\n        \
                     yazde(\"one\")\n    \
                 }\n    \
                 vusht nagah sanna x == 2 {\n        \
                     yazde(\"two\")\n    \
                 }\n    \
                 vusht {\n        \
                     yazde(\"other\")\n    \
                 }\n\
             }\n",
        );
        let f = only_function(&p);
        let Stmt::If { else_block, .. } = &body(f).stmts[1] else {
            panic!("expected If");
        };
        let eb = else_block.as_ref().unwrap();
        // else_block is a 1-stmt block wrapping the nested If — the
        // else-if chain shape.
        assert_eq!(eb.stmts.len(), 1);
        assert!(matches!(eb.stmts[0], Stmt::If { .. }));
    }

    #[test]
    fn multiline_expression_via_parens() {
        // Inside `(...)`, newlines are whitespace — multi-line expressions
        // work without extra ceremony.
        let p = parse_ok(
            "fnc kort() {\n    \
                 xilit x: terah = (1 +\n        2 +\n        3)\n    \
                 yazde(x)\n\
             }\n",
        );
        let f = only_function(&p);
        assert!(matches!(body(f).stmts[0], Stmt::Let { .. }));
    }

    #[test]
    fn explicit_semicolons_still_allowed() {
        // Backward compatibility: users may still write C-style `;`.
        let p = parse_ok("fnc kort() { xilit x: terah = 1; yazde(x); }");
        let f = only_function(&p);
        assert_eq!(body(f).stmts.len(), 2);
    }

    #[test]
    fn array_literal_and_type_parse() {
        let p = parse_ok(
            r#"fnc kort() {
                xilit nums: [terah] = [1, 2, 3]
            }"#,
        );
        let f = only_function(&p);
        let Stmt::Let { ty, value, .. } = &body(f).stmts[0] else {
            panic!("expected Let");
        };
        assert_eq!(ty, &Some(Type::Array(Box::new(Type::Terah))));
        assert!(matches!(value, Some(Expr::ArrayLit(elems)) if elems.len() == 3));
    }

    #[test]
    fn indexing_parses_as_index_expr() {
        let p = parse_ok(
            r#"fnc kort() {
                xilit nums: [terah] = [1, 2, 3]
                xilit first: terah = nums[0]
            }"#,
        );
        let f = only_function(&p);
        let Stmt::Let { value, .. } = &body(f).stmts[1] else {
            panic!("expected Let");
        };
        assert!(matches!(value, Some(Expr::Index { .. })));
    }

    #[test]
    fn index_assign_stmt() {
        let p = parse_ok(
            r#"fnc kort() {
                xilit nums: [terah] = [1, 2, 3]
                nums[0] = 42
            }"#,
        );
        let f = only_function(&p);
        assert!(matches!(body(f).stmts[1], Stmt::IndexAssign { .. }));
    }

    #[test]
    fn for_each_over_array() {
        let p = parse_ok(
            r#"fnc kort() {
                xilit nums: [terah] = [1, 2, 3]
                yallalc x chu nums {
                    yazde(x)
                }
            }"#,
        );
        let f = only_function(&p);
        let Stmt::ForEach { var, iter, .. } = &body(f).stmts[1] else {
            panic!("expected ForEach");
        };
        assert_eq!(var, "x");
        assert!(matches!(iter, IterSource::Array(_)));
    }

    #[test]
    fn for_each_over_range() {
        let p = parse_ok(
            r#"fnc kort() {
                yallalc i chu 0..10 {
                    yazde("{i}")
                }
            }"#,
        );
        let f = only_function(&p);
        let Stmt::ForEach { iter, .. } = &body(f).stmts[0] else {
            panic!("expected ForEach");
        };
        assert!(matches!(iter, IterSource::Range { .. }));
    }

    #[test]
    fn parse_terah_parses_as_parse_terah_expr() {
        let p = parse_ok(
            r#"fnc kort() {
                xilit n: terah = parse_terah("42")
            }"#,
        );
        let f = only_function(&p);
        let Stmt::Let { value, .. } = &body(f).stmts[0] else {
            panic!("expected Let");
        };
        assert!(matches!(value, Some(Expr::ParseTerah(_))));
    }

    #[test]
    fn parse_daqosh_parses_as_parse_daqosh_expr() {
        let p = parse_ok(
            r#"fnc kort() {
                xilit x: daqosh = parse_daqosh("3.14")
            }"#,
        );
        let f = only_function(&p);
        let Stmt::Let { value, .. } = &body(f).stmts[0] else {
            panic!("expected Let");
        };
        assert!(matches!(value, Some(Expr::ParseDaqosh(_))));
    }

    #[test]
    fn parse_terah_requires_parens() {
        // Bare `parse_terah` with no `(` is a parse error — it's not a
        // usable identifier, only a call-shaped primary.
        let err = parse_source("fnc kort() { xilit n: terah = parse_terah }").unwrap_err();
        assert!(format!("{}", err).contains("`(`"));
    }

    #[test]
    fn to_terah_parses_as_to_terah_expr() {
        let p = parse_ok(
            r#"fnc kort() {
                xilit n: terah = to_terah(3.7)
            }"#,
        );
        let f = only_function(&p);
        let Stmt::Let { value, .. } = &body(f).stmts[0] else {
            panic!("expected Let");
        };
        assert!(matches!(value, Some(Expr::ToTerah(_))));
    }

    #[test]
    fn push_parses_as_stmt_with_ident_target() {
        let p = parse_ok(
            r#"fnc kort() {
                xilit nums: [terah] = [1]
                push(nums, 42)
            }"#,
        );
        let f = only_function(&p);
        let Stmt::Push { name, value } = &body(f).stmts[1] else {
            panic!("expected Push stmt");
        };
        assert_eq!(name, "nums");
        assert!(matches!(value, Expr::Integer(42)));
    }

    #[test]
    fn push_with_non_identifier_target_errors() {
        // `push(arr[i], x)` — complex l-value — should give a targeted
        // error rather than a confusing "expected comma" complaint.
        let err = parse_source(
            r#"fnc kort() {
                xilit nums: [terah] = [1]
                push(nums[0], 2)
            }"#,
        )
        .unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("bare variable name"), "got: {}", msg);
    }

    #[test]
    fn pop_parses_as_expression() {
        let p = parse_ok(
            r#"fnc kort() {
                xilit nums: [terah] = [1, 2, 3]
                xilit last: terah = pop(nums)
            }"#,
        );
        let f = only_function(&p);
        let Stmt::Let { value, .. } = &body(f).stmts[1] else {
            panic!("expected Let");
        };
        assert!(matches!(value, Some(Expr::Pop(n)) if n == "nums"));
    }

    #[test]
    fn empty_array_literal_parses() {
        // [] is now valid at parse time; the codegen only accepts it in
        // typed-annotation contexts, but parsing it succeeds unconditionally.
        let p = parse_ok(
            r#"fnc kort() {
                xilit nums: [terah] = []
            }"#,
        );
        let f = only_function(&p);
        let Stmt::Let { value, .. } = &body(f).stmts[0] else {
            panic!("expected Let");
        };
        assert!(matches!(value, Some(Expr::ArrayLit(elems)) if elems.is_empty()));
    }

    #[test]
    fn to_daqosh_parses_as_to_daqosh_expr() {
        let p = parse_ok(
            r#"fnc kort() {
                xilit x: daqosh = to_daqosh(42)
            }"#,
        );
        let f = only_function(&p);
        let Stmt::Let { value, .. } = &body(f).stmts[0] else {
            panic!("expected Let");
        };
        assert!(matches!(value, Some(Expr::ToDaqosh(_))));
    }

    #[test]
    fn baram_parses_as_baram_expr() {
        let p = parse_ok(
            r#"fnc kort() {
                xilit nums: [terah] = [1, 2, 3]
                xilit n: terah = baram(nums)
            }"#,
        );
        let f = only_function(&p);
        let Stmt::Let { value, .. } = &body(f).stmts[1] else {
            panic!("expected Let");
        };
        assert!(matches!(value, Some(Expr::Baram(_))));
    }
}
