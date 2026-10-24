use crate::ast::*;
use crate::error::LexError;
use crate::token::{Delim, Keyword, Op, Token, TokenKind};

pub fn parse(tokens: Vec<Token>) -> Result<Program, LexError> {
    let mut p = Parser { tokens, pos: 0 };
    p.parse_program()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn advance(&mut self) -> Token {
        let t = self.tokens[self.pos].clone();
        self.pos += 1;
        t
    }

    fn line(&self) -> usize {
        self.peek().line
    }

    fn err<T>(&self, message: impl Into<String>) -> Result<T, LexError> {
        Err(LexError {
            line: self.line(),
            message: message.into(),
        })
    }

    fn at_keyword(&self, kw: Keyword) -> bool {
        matches!(self.peek_kind(), TokenKind::Keyword(k) if *k == kw)
    }

    fn eat_keyword(&mut self, kw: Keyword) -> bool {
        if self.at_keyword(kw) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn at_op(&self, op: Op) -> bool {
        matches!(self.peek_kind(), TokenKind::Op(o) if *o == op)
    }

    fn eat_op(&mut self, op: Op) -> bool {
        if self.at_op(op) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn at_delim(&self, d: Delim) -> bool {
        matches!(self.peek_kind(), TokenKind::Delim(dd) if *dd == d)
    }

    fn eat_delim(&mut self, d: Delim) -> bool {
        if self.at_delim(d) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect_delim(&mut self, d: Delim, what: &str) -> Result<Token, LexError> {
        if self.at_delim(d) {
            Ok(self.advance())
        } else {
            self.err(format!("expected {}", what))
        }
    }

    fn expect_op(&mut self, op: Op, what: &str) -> Result<Token, LexError> {
        if self.at_op(op) {
            Ok(self.advance())
        } else {
            self.err(format!("expected {}", what))
        }
    }

    fn expect_ident(&mut self) -> Result<(String, usize), LexError> {
        if let TokenKind::Ident(name) = self.peek_kind() {
            let name = name.clone();
            let line = self.line();
            self.advance();
            Ok((name, line))
        } else {
            self.err("expected identifier")
        }
    }

    fn at_newline(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Newline)
    }

    fn eat_newline(&mut self) -> bool {
        if self.at_newline() {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect_newline(&mut self) -> Result<(), LexError> {
        if self.at_newline() {
            self.advance();
            Ok(())
        } else if matches!(self.peek_kind(), TokenKind::Eof | TokenKind::Dedent) {
            Ok(())
        } else {
            self.err("expected newline")
        }
    }

    fn at_indent(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Indent)
    }

    fn eat_indent(&mut self) -> bool {
        if self.at_indent() {
            self.advance();
            true
        } else {
            false
        }
    }

    fn at_dedent(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Dedent)
    }

    fn eat_dedent(&mut self) -> bool {
        if self.at_dedent() {
            self.advance();
            true
        } else {
            false
        }
    }

    fn parse_program(&mut self) -> Result<Program, LexError> {
        let mut stmts = Vec::new();
        self.skip_newlines();
        while !matches!(self.peek_kind(), TokenKind::Eof) {
            if self.at_dedent() {
                break;
            }
            let stmt = self.parse_stmt()?;
            stmts.push(stmt);
            self.skip_newlines();
        }
        Ok(Program { stmts })
    }

    fn skip_newlines(&mut self) {
        while self.eat_newline() {}
    }

    fn parse_block(&mut self) -> Result<Block, LexError> {
        self.expect_newline()?;
        if !self.eat_indent() {
            return self.err("expected indented block");
        }
        let mut stmts = Vec::new();
        self.skip_newlines();
        while !self.at_dedent() && !matches!(self.peek_kind(), TokenKind::Eof) {
            let stmt = self.parse_stmt()?;
            stmts.push(stmt);
            self.skip_newlines();
        }
        self.eat_dedent();
        Ok(Block { stmts })
    }

    fn parse_stmt(&mut self) -> Result<Stmt, LexError> {
        if self.at_keyword(Keyword::If) {
            return self.parse_if().map(Stmt::If);
        }
        if self.at_keyword(Keyword::While) {
            return self.parse_while().map(Stmt::While);
        }
        if self.at_keyword(Keyword::For) {
            return self.parse_for().map(Stmt::For);
        }
        if self.at_keyword(Keyword::Def) {
            return self.parse_def().map(Stmt::Def);
        }
        if self.at_keyword(Keyword::Return) {
            return self.parse_return().map(Stmt::Return);
        }
        if self.at_keyword(Keyword::Break) {
            let line = self.line();
            self.advance();
            self.expect_newline()?;
            return Ok(Stmt::Break(line));
        }
        if self.at_keyword(Keyword::Continue) {
            let line = self.line();
            self.advance();
            self.expect_newline()?;
            return Ok(Stmt::Continue(line));
        }
        self.parse_simple_stmt()
    }

    fn parse_if(&mut self) -> Result<IfStmt, LexError> {
        let line = self.line();
        self.advance();
        let cond = self.parse_cond()?;
        self.expect_delim(Delim::Colon, "':'")?;
        let body = self.parse_block()?;
        let mut branches = vec![(cond, body)];
        let mut else_block = None;
        loop {
            self.skip_newlines();
            if self.at_keyword(Keyword::Elif) {
                self.advance();
                let cond = self.parse_cond()?;
                self.expect_delim(Delim::Colon, "':'")?;
                let body = self.parse_block()?;
                branches.push((cond, body));
            } else if self.at_keyword(Keyword::Else) {
                self.advance();
                self.expect_delim(Delim::Colon, "':'")?;
                else_block = Some(self.parse_block()?);
                break;
            } else {
                break;
            }
        }
        Ok(IfStmt {
            branches,
            else_block,
            line,
        })
    }

    fn parse_cond(&mut self) -> Result<Expr, LexError> {
        self.expect_delim(Delim::LParen, "'(' before condition")?;
        let cond = self.parse_expr()?;
        self.expect_delim(Delim::RParen, "')' after condition")?;
        Ok(cond)
    }

    fn parse_while(&mut self) -> Result<WhileStmt, LexError> {
        let line = self.line();
        self.advance();
        let cond = self.parse_cond()?;
        self.expect_delim(Delim::Colon, "':'")?;
        let body = self.parse_block()?;
        Ok(WhileStmt { cond, body, line })
    }

    fn parse_for(&mut self) -> Result<ForStmt, LexError> {
        let line = self.line();
        self.advance();
        self.expect_delim(Delim::LParen, "'(' after for")?;
        let (first, _) = self.expect_ident()?;
        let mut vars = vec![first];
        if self.eat_delim(Delim::Comma) {
            let (second, _) = self.expect_ident()?;
            vars.push(second);
        }
        if !self.eat_keyword(Keyword::In) {
            return self.err("expected 'in' in for loop");
        }
        let iter = self.parse_expr()?;
        self.expect_delim(Delim::RParen, "')' after for header")?;
        self.expect_delim(Delim::Colon, "':'")?;
        let body = self.parse_block()?;
        Ok(ForStmt {
            vars,
            iter,
            body,
            line,
        })
    }

    fn parse_def(&mut self) -> Result<DefStmt, LexError> {
        let line = self.line();
        self.advance();
        let (name, _) = self.expect_ident()?;
        self.expect_delim(Delim::LParen, "'(' after function name")?;
        let mut params = Vec::new();
        if !self.at_delim(Delim::RParen) {
            loop {
                let (type_name, _) = self.expect_ident()?;
                let (pname, pline) = self.expect_ident()?;
                params.push(Param {
                    type_name,
                    name: pname,
                    line: pline,
                });
                if !self.eat_delim(Delim::Comma) {
                    break;
                }
            }
        }
        self.expect_delim(Delim::RParen, "')' after parameters")?;
        self.expect_op(Op::Arrow, "'->' before return type")?;
        let (return_type, _) = self.expect_ident()?;
        self.expect_delim(Delim::Colon, "':'")?;
        let body = self.parse_block()?;
        Ok(DefStmt {
            name,
            params,
            return_type,
            body,
            line,
        })
    }

    fn parse_return(&mut self) -> Result<ReturnStmt, LexError> {
        let line = self.line();
        self.advance();
        if self.at_newline() || self.at_dedent() || matches!(self.peek_kind(), TokenKind::Eof) {
            self.expect_newline()?;
            return Ok(ReturnStmt {
                value: Expr::None,
                line,
            });
        }
        let value = self.parse_expr()?;
        self.expect_newline()?;
        Ok(ReturnStmt { value, line })
    }

    fn parse_simple_stmt(&mut self) -> Result<Stmt, LexError> {
        if let TokenKind::Ident(type_name) = self.peek_kind().clone() {
            if let Some(TokenKind::Ident(var_name_raw)) = self.tokens.get(self.pos + 1).map(|t| &t.kind) {
                if let Some(TokenKind::Op(Op::Assign)) = self.tokens.get(self.pos + 2).map(|t| &t.kind) {
                    let var_name = var_name_raw.clone();
                    let line = self.line();
                    self.advance();
                    self.advance();
                    self.advance();
                    let init = self.parse_expr()?;
                    self.expect_newline()?;
                    return Ok(Stmt::VarDecl(VarDecl {
                        type_name,
                        name: var_name,
                        init,
                        line,
                    }));
                }
            }
        }
        let line = self.line();
        let expr = self.parse_expr()?;
        if self.at_op(Op::Assign)
            || self.at_op(Op::PlusAssign)
            || self.at_op(Op::MinusAssign)
            || self.at_op(Op::StarAssign)
            || self.at_op(Op::SlashAssign)
        {
            let op = match self.advance().kind {
                TokenKind::Op(Op::Assign) => AssignOp::Assign,
                TokenKind::Op(Op::PlusAssign) => AssignOp::AddAssign,
                TokenKind::Op(Op::MinusAssign) => AssignOp::SubAssign,
                TokenKind::Op(Op::StarAssign) => AssignOp::MulAssign,
                TokenKind::Op(Op::SlashAssign) => AssignOp::DivAssign,
                _ => unreachable!(),
            };
            let value = self.parse_expr()?;
            self.expect_newline()?;
            return Ok(Stmt::Assign(Assign {
                target: expr,
                op,
                value,
                line,
            }));
        }
        self.expect_newline()?;
        Ok(Stmt::ExprStmt(ExprStmt { expr, line }))
    }

    fn parse_expr(&mut self) -> Result<Expr, LexError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, LexError> {
        let mut left = self.parse_and()?;
        while self.at_keyword(Keyword::Or) {
            let line = self.line();
            self.advance();
            let right = self.parse_and()?;
            left = Expr::BinOp(BinOpExpr {
                op: BinOp::Or,
                left: Box::new(left),
                right: Box::new(right),
                line,
            });
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, LexError> {
        let mut left = self.parse_not()?;
        while self.at_keyword(Keyword::And) {
            let line = self.line();
            self.advance();
            let right = self.parse_not()?;
            left = Expr::BinOp(BinOpExpr {
                op: BinOp::And,
                left: Box::new(left),
                right: Box::new(right),
                line,
            });
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> Result<Expr, LexError> {
        if self.at_keyword(Keyword::Not) {
            let line = self.line();
            self.advance();
            let operand = self.parse_not()?;
            return Ok(Expr::UnOp(UnOpExpr {
                op: UnOp::Not,
                operand: Box::new(operand),
                line,
            }));
        }
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<Expr, LexError> {
        let mut left = self.parse_additive()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Op(Op::EqEq) => BinOp::Eq,
                TokenKind::Op(Op::NotEq) => BinOp::Ne,
                TokenKind::Op(Op::Lt) => BinOp::Lt,
                TokenKind::Op(Op::Gt) => BinOp::Gt,
                TokenKind::Op(Op::Le) => BinOp::Le,
                TokenKind::Op(Op::Ge) => BinOp::Ge,
                _ => break,
            };
            let line = self.line();
            self.advance();
            let right = self.parse_additive()?;
            left = Expr::BinOp(BinOpExpr {
                op,
                left: Box::new(left),
                right: Box::new(right),
                line,
            });
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, LexError> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Op(Op::Plus) => BinOp::Add,
                TokenKind::Op(Op::Minus) => BinOp::Sub,
                _ => break,
            };
            let line = self.line();
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::BinOp(BinOpExpr {
                op,
                left: Box::new(left),
                right: Box::new(right),
                line,
            });
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, LexError> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Op(Op::Star) => BinOp::Mul,
                TokenKind::Op(Op::Slash) => BinOp::Div,
                TokenKind::Op(Op::FloorDiv) => BinOp::FloorDiv,
                TokenKind::Op(Op::Percent) => BinOp::Mod,
                _ => break,
            };
            let line = self.line();
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::BinOp(BinOpExpr {
                op,
                left: Box::new(left),
                right: Box::new(right),
                line,
            });
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, LexError> {
        if self.at_op(Op::Minus) {
            let line = self.line();
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expr::UnOp(UnOpExpr {
                op: UnOp::Neg,
                operand: Box::new(operand),
                line,
            }));
        }
        if self.at_op(Op::Plus) {
            self.advance();
            return self.parse_unary();
        }
        self.parse_power()
    }

    fn parse_power(&mut self) -> Result<Expr, LexError> {
        let base = self.parse_postfix()?;
        if self.at_op(Op::Pow) {
            let line = self.line();
            self.advance();
            let exp = self.parse_unary()?;
            return Ok(Expr::BinOp(BinOpExpr {
                op: BinOp::Pow,
                left: Box::new(base),
                right: Box::new(exp),
                line,
            }));
        }
        Ok(base)
    }

    fn parse_postfix(&mut self) -> Result<Expr, LexError> {
        let mut expr = self.parse_atom()?;
        loop {
            if self.at_delim(Delim::LParen) {
                let line = self.line();
                self.advance();
                let mut args = Vec::new();
                if !self.at_delim(Delim::RParen) {
                    loop {
                        args.push(self.parse_expr()?);
                        if !self.eat_delim(Delim::Comma) {
                            break;
                        }
                    }
                }
                self.expect_delim(Delim::RParen, "')' after arguments")?;
                expr = Expr::Call(CallExpr {
                    callee: Box::new(expr),
                    args,
                    line,
                });
            } else if self.at_delim(Delim::LBracket) {
                let line = self.line();
                self.advance();
                let index = self.parse_expr()?;
                self.expect_delim(Delim::RBracket, "']' after index")?;
                expr = Expr::Index(IndexExpr {
                    object: Box::new(expr),
                    index: Box::new(index),
                    line,
                });
            } else if self.at_delim(Delim::Dot) {
                let line = self.line();
                self.advance();
                let (method, _) = self.expect_ident()?;
                self.expect_delim(Delim::LParen, "'(' after method name")?;
                let mut args = Vec::new();
                if !self.at_delim(Delim::RParen) {
                    loop {
                        args.push(self.parse_expr()?);
                        if !self.eat_delim(Delim::Comma) {
                            break;
                        }
                    }
                }
                self.expect_delim(Delim::RParen, "')' after method arguments")?;
                expr = Expr::MethodCall(MethodCallExpr {
                    object: Box::new(expr),
                    method,
                    args,
                    line,
                });
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_atom(&mut self) -> Result<Expr, LexError> {
        let line = self.line();
        match self.peek_kind().clone() {
            TokenKind::Int(n) => {
                self.advance();
                Ok(Expr::Int(n))
            }
            TokenKind::Float(f) => {
                self.advance();
                Ok(Expr::Float(f))
            }
            TokenKind::Str(s) => {
                self.advance();
                Ok(Expr::Str(s))
            }
            TokenKind::Keyword(Keyword::True) => {
                self.advance();
                Ok(Expr::Bool(true))
            }
            TokenKind::Keyword(Keyword::False) => {
                self.advance();
                Ok(Expr::Bool(false))
            }
            TokenKind::Keyword(Keyword::None) => {
                self.advance();
                Ok(Expr::None)
            }
            TokenKind::Ident(name) => {
                self.advance();
                Ok(Expr::Var(name))
            }
            TokenKind::Delim(Delim::LParen) => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect_delim(Delim::RParen, "')'")?;
                Ok(expr)
            }
            TokenKind::Delim(Delim::LBracket) => {
                self.advance();
                let mut elems = Vec::new();
                if !self.at_delim(Delim::RBracket) {
                    loop {
                        elems.push(self.parse_expr()?);
                        if !self.eat_delim(Delim::Comma) {
                            break;
                        }
                    }
                }
                self.expect_delim(Delim::RBracket, "']'")?;
                Ok(Expr::ListLit(elems, line))
            }
            TokenKind::Delim(Delim::LBrace) => {
                self.advance();
                let mut pairs = Vec::new();
                if !self.at_delim(Delim::RBrace) {
                    loop {
                        let k = self.parse_expr()?;
                        self.expect_delim(Delim::Colon, "':' in dict literal")?;
                        let v = self.parse_expr()?;
                        pairs.push((k, v));
                        if !self.eat_delim(Delim::Comma) {
                            break;
                        }
                    }
                }
                self.expect_delim(Delim::RBrace, "'}'")?;
                Ok(Expr::DictLit(pairs, line))
            }
            _ => self.err("unexpected token in expression"),
        }
    }
}