//! Hand-written recursive descent parser with statement-level error recovery:
//! a parse error inside one statement emits a diagnostic and skips to the next
//! statement boundary, so a half-typed line never blanks the rest of the file.

use crate::ast::*;
use crate::lexer::{lex, Lexeme, Token};
use smol_str::SmolStr;

pub fn parse(src: &str) -> Ast {
    Parser::new(src).parse_file()
}

struct Parser<'a> {
    src: &'a str,
    toks: Vec<Lexeme>,
    pos: usize,
    diags: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    fn new(src: &'a str) -> Self {
        Parser { src, toks: lex(src), pos: 0, diags: Vec::new() }
    }

    // ---- token helpers -------------------------------------------------

    /// Next lexeme, transparently skipping comments.
    fn peek(&self) -> Option<Lexeme> {
        self.toks[self.pos..].iter().copied().find(|l| l.token != Token::Comment)
    }

    /// Second-next lexeme (skipping comments), for one-token lookahead.
    fn peek2(&self) -> Option<Lexeme> {
        self.toks[self.pos..]
            .iter()
            .copied()
            .filter(|l| l.token != Token::Comment)
            .nth(1)
    }

    fn bump(&mut self) -> Option<Lexeme> {
        while let Some(l) = self.toks.get(self.pos).copied() {
            self.pos += 1;
            if l.token != Token::Comment {
                return Some(l);
            }
        }
        None
    }

    fn text(&self, l: Lexeme) -> &'a str {
        &self.src[l.start..l.end]
    }

    fn span_of(&self, l: Lexeme) -> Span {
        Span::new(l.start, l.end)
    }

    fn here(&self) -> Span {
        match self.peek() {
            Some(l) => self.span_of(l),
            None => Span::new(self.src.len(), self.src.len()),
        }
    }

    fn expect(&mut self, tok: Token, what: &str) -> Result<Lexeme, ()> {
        match self.peek() {
            Some(l) if l.token == tok => {
                self.bump();
                Ok(l)
            }
            found => {
                let (span, got) = match found {
                    Some(l) => (self.span_of(l), self.text(l).to_string()),
                    None => (Span::new(self.src.len(), self.src.len()), "end of file".into()),
                };
                self.diags.push(Diagnostic::error(
                    span,
                    format!("expected {what}, found `{got}`"),
                ));
                Err(())
            }
        }
    }

    fn at_terminator(&self) -> bool {
        matches!(
            self.peek().map(|l| l.token),
            None | Some(Token::Newline) | Some(Token::Semi) | Some(Token::RBrace)
        )
    }

    fn skip_separators(&mut self) {
        while let Some(l) = self.toks.get(self.pos).copied() {
            match l.token {
                Token::Newline | Token::Semi | Token::Comment => self.pos += 1,
                _ => break,
            }
        }
    }

    /// Error recovery: skip to the next statement boundary at brace depth 0
    /// relative to where we are (an unmatched `}` also stops us, unconsumed).
    fn sync(&mut self) {
        let mut depth = 0i32;
        while let Some(l) = self.toks.get(self.pos).copied() {
            match l.token {
                Token::LBrace => depth += 1,
                Token::RBrace => {
                    if depth == 0 {
                        return;
                    }
                    depth -= 1;
                }
                Token::Newline | Token::Semi if depth == 0 => {
                    self.pos += 1;
                    return;
                }
                _ => {}
            }
            self.pos += 1;
        }
    }

    /// End offset of the last consumed non-comment token.
    fn last_end(&self) -> usize {
        self.toks[..self.pos]
            .iter()
            .rev()
            .find(|l| l.token != Token::Comment)
            .map(|l| l.end)
            .unwrap_or(0)
    }

    // ---- file / statement ----------------------------------------------

    fn parse_file(mut self) -> Ast {
        let mut stmts = Vec::new();
        loop {
            self.skip_separators();
            if self.peek().is_none() {
                break;
            }
            if let Some(l) = self.peek() {
                if l.token == Token::RBrace {
                    self.diags
                        .push(Diagnostic::error(self.span_of(l), "unmatched `}`"));
                    self.bump();
                    continue;
                }
            }
            match self.parse_stmt(false) {
                Ok(stmt) => {
                    stmts.push(stmt);
                    self.expect_stmt_end();
                }
                Err(()) => self.sync(),
            }
        }
        Ast { stmts, diagnostics: self.diags }
    }

    fn expect_stmt_end(&mut self) {
        if !self.at_terminator() {
            let l = self.peek().unwrap();
            self.diags.push(Diagnostic::error(
                self.span_of(l),
                format!("unexpected `{}` after statement", self.text(l)),
            ));
            self.sync();
        }
    }

    fn parse_stmt(&mut self, in_group: bool) -> Result<Stmt, ()> {
        let l = self.peek().ok_or(())?;
        if l.token != Token::Ident {
            self.diags.push(Diagnostic::error(
                self.span_of(l),
                format!("expected a statement, found `{}`", self.text(l)),
            ));
            return Err(());
        }
        let kw = self.text(l);
        match kw {
            "use" if !in_group => self.parse_use(),
            "dir" if self.peek2().map(|n| n.token) == Some(Token::Colon) => self.parse_dir(),
            "default" if matches!(self.peek2(), Some(n) if n.token == Token::Ident) => {
                self.parse_default()
            }
            "class" if !in_group => self.parse_class(),
            "group" => {
                if in_group {
                    self.diags.push(Diagnostic::error(
                        self.span_of(l),
                        "nested groups are not supported",
                    ));
                    return Err(());
                }
                self.parse_group()
            }
            "node" => {
                self.bump();
                self.parse_node(None)
            }
            "use" | "class" => {
                self.diags.push(Diagnostic::error(
                    self.span_of(l),
                    format!("`{kw}` is not allowed inside a group"),
                ));
                Err(())
            }
            _ => {
                // Ident-led: assignment, edge, or bare node declaration.
                match self.peek2().map(|n| n.token) {
                    Some(Token::Eq) => self.parse_assign(),
                    Some(Token::Dot)
                    | Some(Token::ArrowDirected)
                    | Some(Token::ArrowReversed)
                    | Some(Token::ArrowBi)
                    | Some(Token::ArrowUndirected)
                    | Some(Token::ArrowDotted)
                    | Some(Token::ArrowDottedReversed) => self.parse_edge(),
                    _ => {
                        let id = self.bump().unwrap();
                        self.parse_node(Some(id))
                    }
                }
            }
        }
    }

    fn parse_use(&mut self) -> Result<Stmt, ()> {
        let kw = self.bump().unwrap();
        let mut names = Vec::new();
        loop {
            match self.peek().map(|l| l.token) {
                // `use "./local-styles.gfd"` — document-relative import.
                Some(Token::Str) => {
                    let l = self.bump().unwrap();
                    let path = &self.src[l.start + 1..l.end - 1];
                    names.push((SmolStr::new(path), self.span_of(l)));
                }
                _ => {
                    let l = self.expect(Token::Ident, "a library name or \"./path.gfd\"")?;
                    names.push((SmolStr::new(self.text(l)), self.span_of(l)));
                }
            }
            match self.peek().map(|l| l.token) {
                Some(Token::Comma) => {
                    self.bump();
                }
                _ => break,
            }
        }
        Ok(Stmt::Use(UseStmt {
            names,
            stmt_span: Span::new(kw.start, self.last_end()),
        }))
    }

    fn parse_default(&mut self) -> Result<Stmt, ()> {
        let kw = self.bump().unwrap();
        let target_lex = self.expect(Token::Ident, "`node` or `edge` after `default`")?;
        let target = match self.text(target_lex) {
            "node" => DefaultTarget::Node,
            "edge" => DefaultTarget::Edge,
            other => {
                self.diags.push(Diagnostic::error(
                    self.span_of(target_lex),
                    format!("expected `node` or `edge` after `default`, found `{other}`"),
                ));
                return Err(());
            }
        };
        let (attrs, _) = self.parse_bracket_items()?;
        Ok(Stmt::Default(DefaultStmt {
            target,
            attrs,
            stmt_span: Span::new(kw.start, self.last_end()),
        }))
    }

    fn parse_dir(&mut self) -> Result<Stmt, ()> {
        let kw = self.bump().unwrap();
        self.expect(Token::Colon, "`:` after `dir`")?;
        let l = self.expect(Token::Ident, "`TB`, `LR`, `BT` or `RL`")?;
        let dir = match LayoutDir::from_keyword(self.text(l)) {
            Some(dir) => dir,
            None => {
                let other = self.text(l);
                self.diags.push(Diagnostic::error(
                    self.span_of(l),
                    format!("unknown direction `{other}` (expected TB, LR, BT or RL)"),
                ));
                return Err(());
            }
        };
        Ok(Stmt::Dir(DirStmt { dir, stmt_span: Span::new(kw.start, self.last_end()) }))
    }

    // ---- values ---------------------------------------------------------

    fn parse_value(&mut self) -> Result<ValueExpr, ()> {
        let l = self.peek().ok_or_else(|| {
            self.diags.push(Diagnostic::error(
                Span::new(self.src.len(), self.src.len()),
                "expected a value, found end of file",
            ));
        })?;
        match l.token {
            Token::Color => {
                self.bump();
                let span = self.span_of(l);
                match parse_hex_color(self.text(l)) {
                    Some(rgba) => Ok(ValueExpr::Color { rgba, span }),
                    None => {
                        self.diags.push(Diagnostic::error(
                            span,
                            format!("invalid color `{}` (use #rgb, #rgba, #rrggbb or #rrggbbaa)", self.text(l)),
                        ));
                        Ok(ValueExpr::Color { rgba: [255, 0, 255, 255], span })
                    }
                }
            }
            Token::Number => {
                self.bump();
                let value = self.text(l).parse().unwrap_or(0.0);
                Ok(ValueExpr::Num { value, span: self.span_of(l) })
            }
            Token::Str => {
                self.bump();
                Ok(ValueExpr::Str(self.parse_str_lit(l)))
            }
            Token::Dollar => {
                self.bump();
                let name = self.expect(Token::Ident, "a variable name after `$`")?;
                Ok(ValueExpr::VarRef {
                    name: SmolStr::new(self.text(name)),
                    span: Span::new(l.start, name.end),
                })
            }
            Token::LBracket => {
                let (items, span) = self.parse_bracket_items()?;
                Ok(ValueExpr::Bundle { items, span })
            }
            _ => {
                self.diags.push(Diagnostic::error(
                    self.span_of(l),
                    format!("expected a value, found `{}`", self.text(l)),
                ));
                Err(())
            }
        }
    }

    /// Parse a `"..."` lexeme into literal/`$var` segments.
    fn parse_str_lit(&self, l: Lexeme) -> StrLit {
        let raw = &self.src[l.start + 1..l.end - 1];
        let mut segments = Vec::new();
        let mut lit = String::new();
        let mut chars = raw.char_indices().peekable();
        while let Some((_, c)) = chars.next() {
            match c {
                '\\' => {
                    if let Some((_, esc)) = chars.next() {
                        match esc {
                            'n' => lit.push('\n'),
                            't' => lit.push('\t'),
                            other => lit.push(other),
                        }
                    }
                }
                '$' => {
                    let mut name = String::new();
                    while let Some(&(_, nc)) = chars.peek() {
                        if nc.is_ascii_alphanumeric() || nc == '_' {
                            name.push(nc);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    if name.is_empty() {
                        lit.push('$');
                    } else {
                        if !lit.is_empty() {
                            segments.push(StrSeg::Lit(std::mem::take(&mut lit)));
                        }
                        segments.push(StrSeg::Var(SmolStr::new(name)));
                    }
                }
                other => lit.push(other),
            }
        }
        if !lit.is_empty() || segments.is_empty() {
            segments.push(StrSeg::Lit(lit));
        }
        StrLit { segments, span: self.span_of(l) }
    }

    /// `[ item, item, ... ]` — shared by node attrs and bundle values.
    fn parse_bracket_items(&mut self) -> Result<(Vec<AttrItem>, Span), ()> {
        let open = self.expect(Token::LBracket, "`[`")?;
        let mut items = Vec::new();
        loop {
            match self.peek().map(|l| l.token) {
                Some(Token::RBracket) => {
                    let close = self.bump().unwrap();
                    return Ok((items, Span::new(open.start, close.end)));
                }
                Some(Token::Comma) => {
                    self.bump();
                }
                None | Some(Token::Newline) => {
                    self.diags.push(Diagnostic::error(
                        Span::new(open.start, self.last_end()),
                        "unclosed `[`",
                    ));
                    return Err(());
                }
                _ => {
                    if let Some(item) = self.parse_attr_item() {
                        items.push(item);
                    }
                }
            }
        }
    }

    fn parse_attr_item(&mut self) -> Option<AttrItem> {
        let l = self.peek()?;
        match l.token {
            Token::Dollar => {
                self.bump();
                let name = self.expect(Token::Ident, "a variable name after `$`").ok()?;
                Some(AttrItem::Splice {
                    name: SmolStr::new(self.text(name)),
                    span: Span::new(l.start, name.end),
                })
            }
            Token::Ident => {
                let word = self.text(l);
                if let Some(shape) = Shape::from_keyword(word) {
                    self.bump();
                    return Some(AttrItem::Shape(shape));
                }
                match word {
                    "dashed" => { self.bump(); Some(AttrItem::Dashed) }
                    "bold" => { self.bump(); Some(AttrItem::Bold) }
                    "icon" => {
                        self.bump();
                        self.expect(Token::Eq, "`=`").ok()?;
                        // `icon=gear` — a bare identifier is sugar for a string.
                        let value = match self.peek() {
                            Some(n) if n.token == Token::Ident => {
                                self.bump();
                                ValueExpr::Str(StrLit {
                                    segments: vec![StrSeg::Lit(self.text(n).to_string())],
                                    span: self.span_of(n),
                                })
                            }
                            _ => self.parse_value().ok()?,
                        };
                        Some(AttrItem::Icon(value))
                    }
                    "fill" | "stroke" | "text" | "w" | "h" | "width" | "height" => {
                        self.bump();
                        self.expect(Token::Eq, "`=`").ok()?;
                        let value = self.parse_value().ok()?;
                        Some(match word {
                            "fill" => AttrItem::Fill(value),
                            "stroke" => AttrItem::Stroke(value),
                            "text" => AttrItem::TextColor(value),
                            "w" | "width" => AttrItem::Width(value),
                            _ => AttrItem::Height(value),
                        })
                    }
                    other => {
                        self.diags.push(Diagnostic::error(
                            self.span_of(l),
                            format!("unknown attribute `{other}`"),
                        ));
                        self.bump();
                        None
                    }
                }
            }
            _ => {
                self.diags.push(Diagnostic::error(
                    self.span_of(l),
                    format!("unexpected `{}` in attribute list", self.text(l)),
                ));
                self.bump();
                None
            }
        }
    }

    // ---- placement -------------------------------------------------------

    fn peek_placement_start(&self) -> bool {
        match self.peek() {
            Some(l) => match l.token {
                Token::At | Token::KwRightOf | Token::KwLeftOf => true,
                Token::Ident => matches!(self.text(l), "above" | "below"),
                _ => false,
            },
            None => false,
        }
    }

    fn parse_placement(&mut self) -> Result<PlacementAst, ()> {
        let l = self.peek().ok_or(())?;
        match l.token {
            Token::At => {
                self.bump();
                self.expect(Token::LParen, "`(` after `@`")?;
                let xl = self.expect(Token::Number, "an x coordinate")?;
                self.expect(Token::Comma, "`,` between coordinates")?;
                let yl = self.expect(Token::Number, "a y coordinate")?;
                let close = self.expect(Token::RParen, "`)` to close the coordinate")?;
                Ok(PlacementAst::Absolute {
                    x: self.text(xl).parse().unwrap_or(0.0),
                    y: self.text(yl).parse().unwrap_or(0.0),
                    span: Span::new(l.start, close.end),
                })
            }
            _ => {
                let dir = match l.token {
                    Token::KwRightOf => Dir::RightOf,
                    Token::KwLeftOf => Dir::LeftOf,
                    Token::Ident if self.text(l) == "above" => Dir::Above,
                    Token::Ident if self.text(l) == "below" => Dir::Below,
                    _ => {
                        self.diags.push(Diagnostic::error(
                            self.span_of(l),
                            "expected a placement (`@ (x, y)`, `right-of`, `left-of`, `above`, `below`)",
                        ));
                        return Err(());
                    }
                };
                self.bump();
                let anchor = self.expect(Token::Ident, "an anchor node id")?;
                let mut gap = None;
                if let Some(g) = self.peek() {
                    if g.token == Token::Ident && self.text(g) == "gap" {
                        self.bump();
                        let n = self.expect(Token::Number, "a gap distance")?;
                        gap = Some(self.text(n).parse().unwrap_or(0.0));
                    }
                }
                Ok(PlacementAst::Relative {
                    anchor: SmolStr::new(self.text(anchor)),
                    anchor_span: self.span_of(anchor),
                    dir,
                    gap,
                    span: Span::new(l.start, self.last_end()),
                })
            }
        }
    }

    // ---- node / assign / edge -------------------------------------------

    /// `keyword_id` is Some when the leading ident was already consumed
    /// (bare-node form); None right after the `node` keyword.
    fn parse_node(&mut self, keyword_id: Option<Lexeme>) -> Result<Stmt, ()> {
        let (id_lex, stmt_start) = match keyword_id {
            Some(l) => (l, l.start),
            None => {
                let start = self.last_end() - "node".len();
                let l = self.expect(Token::Ident, "a node id")?;
                (l, start)
            }
        };
        let id = SmolStr::new(self.text(id_lex));
        let id_span = self.span_of(id_lex);

        let mut applied_vars = Vec::new();
        while self.peek().map(|l| l.token) == Some(Token::Dollar) {
            let d = self.bump().unwrap();
            let name = self.expect(Token::Ident, "a variable name after `$`")?;
            applied_vars.push((SmolStr::new(self.text(name)), Span::new(d.start, name.end)));
        }

        let mut label = None;
        if let Some(l) = self.peek() {
            if l.token == Token::Str {
                self.bump();
                label = Some(self.parse_str_lit(l));
            }
        }

        let mut attrs = Vec::new();
        if self.peek().map(|l| l.token) == Some(Token::LBracket) {
            attrs = self.parse_bracket_items()?.0;
        }

        let mut placement = None;
        if self.peek_placement_start() {
            placement = Some(self.parse_placement()?);
        }

        Ok(Stmt::Node(NodeStmt {
            id,
            id_span,
            applied_vars,
            label,
            attrs,
            placement,
            stmt_span: Span::new(stmt_start, self.last_end()),
            insert_placement_at: self.last_end(),
        }))
    }

    fn parse_assign(&mut self) -> Result<Stmt, ()> {
        let name_lex = self.bump().unwrap();
        let name = SmolStr::new(self.text(name_lex));
        self.expect(Token::Eq, "`=`")?;

        let rhs = match (self.peek(), self.peek2()) {
            (Some(c), Some(p)) if c.token == Token::Ident && p.token == Token::LParen => {
                let class_lex = self.bump().unwrap();
                self.bump(); // (
                let mut args = Vec::new();
                loop {
                    match self.peek().map(|l| l.token) {
                        Some(Token::RParen) => break,
                        Some(Token::Comma) => {
                            self.bump();
                        }
                        None | Some(Token::Newline) => {
                            self.diags.push(Diagnostic::error(
                                Span::new(class_lex.start, self.last_end()),
                                "unclosed argument list",
                            ));
                            return Err(());
                        }
                        _ => args.push(self.parse_value()?),
                    }
                }
                let close = self.bump().unwrap(); // )
                AssignRhs::Call {
                    class: SmolStr::new(self.text(class_lex)),
                    class_span: self.span_of(class_lex),
                    args,
                    span: Span::new(class_lex.start, close.end),
                }
            }
            _ => AssignRhs::Value(self.parse_value()?),
        };

        let mut placement = None;
        if self.peek_placement_start() {
            placement = Some(self.parse_placement()?);
        }

        Ok(Stmt::Assign(AssignStmt {
            name,
            name_span: self.span_of(name_lex),
            rhs,
            placement,
            stmt_span: Span::new(name_lex.start, self.last_end()),
            insert_placement_at: self.last_end(),
        }))
    }

    fn parse_endpoint(&mut self) -> Result<EndpointRef, ()> {
        let node = self.expect(Token::Ident, "a node id")?;
        let mut port = None;
        let mut end = node.end;
        if self.peek().map(|l| l.token) == Some(Token::Dot) {
            self.bump();
            let p = self.expect(Token::Ident, "a port name after `.`")?;
            port = Some(SmolStr::new(self.text(p)));
            end = p.end;
        }
        Ok(EndpointRef {
            node: SmolStr::new(self.text(node)),
            port,
            span: Span::new(node.start, end),
        })
    }

    fn peek_arrow(&self) -> Option<ArrowTok> {
        match self.peek()?.token {
            Token::ArrowDirected => Some(ArrowTok { kind: ArrowKind::Directed, reversed: false }),
            Token::ArrowReversed => Some(ArrowTok { kind: ArrowKind::Directed, reversed: true }),
            Token::ArrowBi => Some(ArrowTok { kind: ArrowKind::Bidirectional, reversed: false }),
            Token::ArrowUndirected => Some(ArrowTok { kind: ArrowKind::Undirected, reversed: false }),
            Token::ArrowDotted => Some(ArrowTok { kind: ArrowKind::Dotted, reversed: false }),
            Token::ArrowDottedReversed => Some(ArrowTok { kind: ArrowKind::Dotted, reversed: true }),
            _ => None,
        }
    }

    fn parse_edge(&mut self) -> Result<Stmt, ()> {
        let first = self.parse_endpoint()?;
        let mut endpoints = vec![first];
        let mut arrows = Vec::new();

        // Chains: `a -> b -> c` produces one edge per arrow.
        loop {
            let Some(arrow) = self.peek_arrow() else {
                if arrows.is_empty() {
                    let l = self.peek().ok_or(())?;
                    self.diags.push(Diagnostic::error(
                        self.span_of(l),
                        format!(
                            "expected an arrow (`->`, `<-`, `<->`, `--`, `..>`, `<..`), found `{}`",
                            self.text(l)
                        ),
                    ));
                    return Err(());
                }
                break;
            };
            self.bump();
            arrows.push(arrow);
            endpoints.push(self.parse_endpoint()?);
        }

        let mut label = None;
        if self.peek().map(|l| l.token) == Some(Token::Colon) {
            self.bump();
            let l = self.expect(Token::Str, "an edge label string after `:`")?;
            label = Some(self.parse_str_lit(l));
        }

        let mut attrs = Vec::new();
        if self.peek().map(|l| l.token) == Some(Token::LBracket) {
            attrs = self.parse_bracket_items()?.0;
        }

        let stmt_span = Span::new(endpoints[0].span.start, self.last_end());
        Ok(Stmt::Edge(EdgeStmt { endpoints, arrows, label, attrs, stmt_span }))
    }

    // ---- class / group ---------------------------------------------------

    fn parse_class(&mut self) -> Result<Stmt, ()> {
        let kw = self.bump().unwrap();
        let name_lex = self.expect(Token::Ident, "a class name")?;

        let mut params = Vec::new();
        if self.peek().map(|l| l.token) == Some(Token::LParen) {
            self.bump();
            loop {
                match self.peek().map(|l| l.token) {
                    Some(Token::RParen) => {
                        self.bump();
                        break;
                    }
                    Some(Token::Comma) => {
                        self.bump();
                    }
                    Some(Token::Ident) => {
                        let p = self.bump().unwrap();
                        let mut default = None;
                        if self.peek().map(|l| l.token) == Some(Token::Eq) {
                            self.bump();
                            default = Some(self.parse_value()?);
                        }
                        params.push(Param {
                            name: SmolStr::new(self.text(p)),
                            default,
                            span: self.span_of(p),
                        });
                    }
                    _ => {
                        self.diags.push(Diagnostic::error(
                            self.here(),
                            "unclosed parameter list",
                        ));
                        return Err(());
                    }
                }
            }
        }

        self.expect(Token::LBrace, "`{` to open the class body")?;
        let mut vars = Vec::new();
        let mut ports = Vec::new();
        loop {
            self.skip_separators();
            let l = match self.peek() {
                Some(l) => l,
                None => {
                    self.diags.push(Diagnostic::error(
                        Span::new(kw.start, self.last_end()),
                        "unclosed class body",
                    ));
                    return Err(());
                }
            };
            match l.token {
                Token::RBrace => {
                    self.bump();
                    break;
                }
                Token::Ident if self.text(l) == "port" => {
                    self.bump();
                    let name = self.expect(Token::Ident, "a port name")?;
                    let side_lex = self.expect(Token::Ident, "a side (top/bottom/left/right)")?;
                    let side = match self.text(side_lex) {
                        "top" => Side::Top,
                        "bottom" => Side::Bottom,
                        "left" => Side::Left,
                        "right" => Side::Right,
                        other => {
                            self.diags.push(Diagnostic::error(
                                self.span_of(side_lex),
                                format!("unknown side `{other}` (expected top/bottom/left/right)"),
                            ));
                            Side::Right
                        }
                    };
                    ports.push(PortDecl {
                        name: SmolStr::new(self.text(name)),
                        side,
                        span: Span::new(l.start, side_lex.end),
                    });
                }
                Token::Ident => {
                    let v = self.bump().unwrap();
                    self.expect(Token::Eq, "`=` after class variable name")?;
                    let value = self.parse_value()?;
                    vars.push((SmolStr::new(self.text(v)), value));
                }
                _ => {
                    self.diags.push(Diagnostic::error(
                        self.span_of(l),
                        format!("unexpected `{}` in class body", self.text(l)),
                    ));
                    self.bump();
                }
            }
        }

        Ok(Stmt::Class(ClassStmt {
            name: SmolStr::new(self.text(name_lex)),
            name_span: self.span_of(name_lex),
            params,
            vars,
            ports,
            stmt_span: Span::new(kw.start, self.last_end()),
        }))
    }

    fn parse_group(&mut self) -> Result<Stmt, ()> {
        let kw = self.bump().unwrap();
        let id_lex = self.expect(Token::Ident, "a group id")?;

        let mut label = None;
        if let Some(l) = self.peek() {
            if l.token == Token::Str {
                self.bump();
                label = Some(self.parse_str_lit(l));
            }
        }

        let mut placement = None;
        if self.peek_placement_start() {
            placement = Some(self.parse_placement()?);
        }
        let insert_placement_at = self.last_end();

        self.expect(Token::LBrace, "`{` to open the group body")?;
        let mut body = Vec::new();
        let mut dir = None;
        loop {
            self.skip_separators();
            let l = match self.peek() {
                Some(l) => l,
                None => {
                    self.diags.push(Diagnostic::error(
                        Span::new(kw.start, self.last_end()),
                        "unclosed group body",
                    ));
                    return Err(());
                }
            };
            if l.token == Token::RBrace {
                self.bump();
                break;
            }
            match self.parse_stmt(true) {
                Ok(Stmt::Dir(d)) => dir = Some(d.dir),
                Ok(stmt) => {
                    body.push(stmt);
                    if !self.at_terminator() {
                        let l = self.peek().unwrap();
                        self.diags.push(Diagnostic::error(
                            self.span_of(l),
                            format!("unexpected `{}` after statement", self.text(l)),
                        ));
                        self.sync();
                    }
                }
                Err(()) => self.sync(),
            }
        }

        Ok(Stmt::Group(GroupStmt {
            id: SmolStr::new(self.text(id_lex)),
            id_span: self.span_of(id_lex),
            label,
            placement,
            dir,
            body,
            stmt_span: Span::new(kw.start, self.last_end()),
            insert_placement_at,
        }))
    }
}
