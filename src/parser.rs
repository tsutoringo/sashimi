use crate::{
    ast::{
        ClassDecl, Expr, Function, ImplDecl, ImportDecl, ImportKind, ImportSpecifier, Item, Param,
        Program, Stmt, TraitDecl, TraitMethod, TypeRef,
    },
    diagnostic::CompileError,
    lexer::{Token, TokenKind},
};

pub fn parse(tokens: Vec<Token>) -> Result<Program, CompileError> {
    Parser { tokens, cursor: 0 }.program()
}

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    fn program(&mut self) -> Result<Program, CompileError> {
        let mut items = Vec::new();
        while !self.at(&TokenKind::Eof) {
            items.push(self.item()?);
        }
        Ok(Program { items })
    }

    fn item(&mut self) -> Result<Item, CompileError> {
        if self.at(&TokenKind::Import) {
            return Ok(Item::Import(self.import_decl()?));
        }
        let public = self.eat(&TokenKind::Pub);
        match self.peek_kind() {
            TokenKind::Fn => Ok(Item::Function(self.function(public)?)),
            TokenKind::Trait => Ok(Item::Trait(self.trait_decl(public)?)),
            TokenKind::Impl if !public => Ok(Item::Impl(self.impl_decl()?)),
            TokenKind::Class => Ok(Item::Class(self.class_decl(public)?)),
            _ => Err(self.error_here("expected `import`, `fn`, `trait`, `impl`, or `class`")),
        }
    }

    fn import_decl(&mut self) -> Result<ImportDecl, CompileError> {
        self.expect(TokenKind::Import, "expected `import`")?;
        let mut specifiers = Vec::new();

        if self.eat(&TokenKind::LBrace) {
            self.named_imports(&mut specifiers)?;
        } else if self.eat(&TokenKind::Star) {
            self.expect(TokenKind::As, "expected `as` after `*`")?;
            let local = self.ident()?;
            specifiers.push(ImportSpecifier {
                kind: ImportKind::Namespace,
                imported: None,
                local,
            });
        } else {
            let local = self.ident()?;
            specifiers.push(ImportSpecifier {
                kind: ImportKind::Default,
                imported: Some("default".to_string()),
                local,
            });
            if self.eat(&TokenKind::Comma) {
                if self.eat(&TokenKind::LBrace) {
                    self.named_imports(&mut specifiers)?;
                } else if self.eat(&TokenKind::Star) {
                    self.expect(TokenKind::As, "expected `as` after `*`")?;
                    let local = self.ident()?;
                    specifiers.push(ImportSpecifier {
                        kind: ImportKind::Namespace,
                        imported: None,
                        local,
                    });
                } else {
                    return Err(self.error_here("expected `{` or `*` after default import"));
                }
            }
        }

        self.expect(TokenKind::From, "expected `from` in import")?;
        let source = self.string()?;
        self.expect(TokenKind::Semi, "expected `;` after import")?;
        Ok(ImportDecl { source, specifiers })
    }

    fn named_imports(&mut self, specifiers: &mut Vec<ImportSpecifier>) -> Result<(), CompileError> {
        if self.eat(&TokenKind::RBrace) {
            return Ok(());
        }
        loop {
            let imported = self.ident()?;
            let local = if self.eat(&TokenKind::As) {
                self.ident()?
            } else {
                imported.clone()
            };
            specifiers.push(ImportSpecifier {
                kind: ImportKind::Named,
                imported: Some(imported),
                local,
            });
            if self.eat(&TokenKind::RBrace) {
                break;
            }
            self.expect(TokenKind::Comma, "expected `,` or `}` in import")?;
        }
        Ok(())
    }

    fn function(&mut self, public: bool) -> Result<Function, CompileError> {
        self.expect(TokenKind::Fn, "expected `fn`")?;
        let name = self.ident()?;
        let params = self.params()?;
        let return_type = if self.eat(&TokenKind::Colon) { Some(self.type_ref()?) } else { None };
        let body = self.block()?;
        Ok(Function { public, name, params, return_type, body })
    }

    fn trait_decl(&mut self, public: bool) -> Result<TraitDecl, CompileError> {
        self.expect(TokenKind::Trait, "expected `trait`")?;
        let name = self.ident()?;
        self.expect(TokenKind::LBrace, "expected `{` after trait name")?;
        let mut methods = Vec::new();
        while !self.eat(&TokenKind::RBrace) {
            self.expect(TokenKind::Fn, "expected trait method")?;
            let name = self.ident()?;
            let params = self.params()?;
            let return_type = if self.eat(&TokenKind::Colon) { Some(self.type_ref()?) } else { None };
            self.expect(TokenKind::Semi, "trait methods must end with `;`")?;
            methods.push(TraitMethod { name, params, return_type });
        }
        Ok(TraitDecl { public, name, methods })
    }

    fn impl_decl(&mut self) -> Result<ImplDecl, CompileError> {
        self.expect(TokenKind::Impl, "expected `impl`")?;
        let generics = if self.eat(&TokenKind::Less) {
            let mut names = Vec::new();
            loop {
                names.push(self.ident()?);
                if self.eat(&TokenKind::Greater) { break; }
                self.expect(TokenKind::Comma, "expected `,` or `>` in generic parameter list")?;
            }
            names
        } else {
            Vec::new()
        };
        let trait_name = self.ident()?;
        self.expect(TokenKind::For, "expected `for` in trait implementation")?;
        let target = self.type_ref()?;
        self.expect(TokenKind::LBrace, "expected `{` after implementation target")?;
        let mut methods = Vec::new();
        while !self.eat(&TokenKind::RBrace) {
            methods.push(self.function(false)?);
        }
        Ok(ImplDecl { generics, trait_name, target, methods })
    }

    fn class_decl(&mut self, public: bool) -> Result<ClassDecl, CompileError> {
        self.expect(TokenKind::Class, "expected `class`")?;
        let name = self.ident()?;
        self.expect(TokenKind::LBrace, "expected `{` after class name")?;
        self.expect(TokenKind::RBrace, "MVP classes must have an empty body")?;
        Ok(ClassDecl { public, name })
    }

    fn params(&mut self) -> Result<Vec<Param>, CompileError> {
        self.expect(TokenKind::LParen, "expected `(`")?;
        let mut params = Vec::new();
        if self.eat(&TokenKind::RParen) { return Ok(params); }
        loop {
            let receiver = self.eat(&TokenKind::Amp);
            let name = self.ident()?;
            let ty = if receiver { None } else if self.eat(&TokenKind::Colon) { Some(self.type_ref()?) } else { None };
            params.push(Param { receiver, name, ty });
            if self.eat(&TokenKind::RParen) { break; }
            self.expect(TokenKind::Comma, "expected `,` or `)` in parameter list")?;
        }
        Ok(params)
    }

    fn type_ref(&mut self) -> Result<TypeRef, CompileError> {
        let name = self.ident()?;
        let mut args = Vec::new();
        if self.eat(&TokenKind::Less) {
            loop {
                args.push(self.type_ref()?);
                if self.eat(&TokenKind::Greater) { break; }
                self.expect(TokenKind::Comma, "expected `,` or `>` in type arguments")?;
            }
        }
        Ok(TypeRef { name, args })
    }

    fn block(&mut self) -> Result<Vec<Stmt>, CompileError> {
        self.expect(TokenKind::LBrace, "expected `{`")?;
        let mut statements = Vec::new();
        while !self.eat(&TokenKind::RBrace) {
            statements.push(self.statement()?);
        }
        Ok(statements)
    }

    fn statement(&mut self) -> Result<Stmt, CompileError> {
        if self.eat(&TokenKind::Let) {
            let name = self.ident()?;
            self.expect(TokenKind::Eq, "expected `=` in let binding")?;
            let value = self.expr()?;
            self.expect(TokenKind::Semi, "expected `;` after let binding")?;
            return Ok(Stmt::Let { name, value });
        }
        if self.eat(&TokenKind::Return) {
            if self.eat(&TokenKind::Semi) { return Ok(Stmt::Return(None)); }
            let value = self.expr()?;
            self.expect(TokenKind::Semi, "expected `;` after return")?;
            return Ok(Stmt::Return(Some(value)));
        }
        let expr = self.expr()?;
        self.expect(TokenKind::Semi, "expected `;` after expression")?;
        Ok(Stmt::Expr(expr))
    }

    fn expr(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.primary()?;
        loop {
            if self.eat(&TokenKind::Dot) {
                let property = self.ident()?;
                if self.at(&TokenKind::LParen) {
                    let args = self.arguments()?;
                    expr = Expr::MethodCall { receiver: Box::new(expr), method: property, args };
                } else {
                    expr = Expr::Member { object: Box::new(expr), property };
                }
                continue;
            }
            if self.at(&TokenKind::LParen) {
                let args = self.arguments()?;
                expr = Expr::Call { callee: Box::new(expr), args };
                continue;
            }
            break;
        }
        Ok(expr)
    }

    fn primary(&mut self) -> Result<Expr, CompileError> {
        let token = self.bump().clone();
        match token.kind {
            TokenKind::Number(value) => Ok(Expr::Number(value)),
            TokenKind::String(value) => Ok(Expr::String(value)),
            TokenKind::True => Ok(Expr::Bool(true)),
            TokenKind::False => Ok(Expr::Bool(false)),
            TokenKind::Ident(value) => Ok(Expr::Ident(value)),
            TokenKind::New => {
                let class_name = self.ident()?;
                let args = self.arguments()?;
                Ok(Expr::New { class_name, args })
            }
            TokenKind::LBracket => {
                let mut items = Vec::new();
                if self.eat(&TokenKind::RBracket) { return Ok(Expr::Array(items)); }
                loop {
                    items.push(self.expr()?);
                    if self.eat(&TokenKind::RBracket) { break; }
                    self.expect(TokenKind::Comma, "expected `,` or `]` in array literal")?;
                }
                Ok(Expr::Array(items))
            }
            _ => Err(CompileError::new("expected expression", token.span)),
        }
    }

    fn arguments(&mut self) -> Result<Vec<Expr>, CompileError> {
        self.expect(TokenKind::LParen, "expected `(`")?;
        let mut args = Vec::new();
        if self.eat(&TokenKind::RParen) { return Ok(args); }
        loop {
            args.push(self.expr()?);
            if self.eat(&TokenKind::RParen) { break; }
            self.expect(TokenKind::Comma, "expected `,` or `)` in argument list")?;
        }
        Ok(args)
    }

    fn ident(&mut self) -> Result<String, CompileError> {
        let token = self.bump().clone();
        match token.kind {
            TokenKind::Ident(value) => Ok(value),
            _ => Err(CompileError::new("expected identifier", token.span)),
        }
    }

    fn string(&mut self) -> Result<String, CompileError> {
        let token = self.bump().clone();
        match token.kind {
            TokenKind::String(value) => Ok(value),
            _ => Err(CompileError::new("expected string literal", token.span)),
        }
    }

    fn expect(&mut self, expected: TokenKind, message: &'static str) -> Result<(), CompileError> {
        if self.eat(&expected) { Ok(()) } else { Err(self.error_here(message)) }
    }

    fn eat(&mut self, expected: &TokenKind) -> bool {
        if self.at(expected) { self.cursor += 1; true } else { false }
    }

    fn at(&self, expected: &TokenKind) -> bool {
        std::mem::discriminant(self.peek_kind()) == std::mem::discriminant(expected)
    }

    fn peek_kind(&self) -> &TokenKind { &self.tokens[self.cursor].kind }

    fn bump(&mut self) -> &Token {
        let i = self.cursor;
        self.cursor += 1;
        &self.tokens[i]
    }

    fn error_here(&self, message: impl Into<String>) -> CompileError {
        CompileError::new(message, self.tokens[self.cursor].span)
    }
}
