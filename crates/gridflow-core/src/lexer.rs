//! logos-based lexer. Byte spans on every token; the same token stream feeds
//! the parser and the editor's syntax highlighter.
//!
//! Comments are `//` to end of line (`#` is reserved for hex colors).
//! Identifiers do not allow `-` so `a--b` can never shadow the `--` arrow;
//! the multi-word placement keywords `right-of`/`left-of` are their own tokens.

use logos::Logos;

#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq)]
#[logos(skip r"[ \t\r]+")]
pub enum Token {
    #[token("right-of")]
    KwRightOf,
    #[token("left-of")]
    KwLeftOf,
    // `-->`/`<--` are accepted as aliases for `->`/`<-` (mermaid muscle memory).
    #[token("->")]
    #[token("-->")]
    ArrowDirected,
    #[token("<-")]
    #[token("<--")]
    ArrowReversed,
    #[token("<->")]
    #[token("<-->")]
    ArrowBi,
    #[token("--")]
    ArrowUndirected,
    #[token("..>")]
    ArrowDotted,
    #[token("<..")]
    ArrowDottedReversed,

    #[regex(r"[A-Za-z_][A-Za-z0-9_]*")]
    Ident,
    #[regex(r"-?[0-9]+(\.[0-9]+)?")]
    Number,
    #[regex(r#""([^"\\\n]|\\.)*""#)]
    Str,
    #[regex(r"#[0-9a-fA-F]+")]
    Color,
    #[regex(r"//[^\n]*", allow_greedy = true)]
    Comment,

    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token(";")]
    Semi,
    #[token("@")]
    At,
    #[token("$")]
    Dollar,
    #[token(".")]
    Dot,
    #[token("=")]
    Eq,
    #[token("\n")]
    Newline,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lexeme {
    pub token: Token,
    pub start: usize,
    pub end: usize,
}

/// Lex the whole input. Unrecognized bytes are skipped (each produces one
/// `Lexeme` with `token == None` is NOT emitted; instead the caller sees a gap —
/// the parser reports errors at the statement level, so lost bytes surface as
/// "unexpected token" on whatever follows). Returns lexemes including comments
/// and newlines; the parser filters as needed, the highlighter uses everything.
pub fn lex(src: &str) -> Vec<Lexeme> {
    let mut out = Vec::new();
    let mut lexer = Token::lexer(src);
    while let Some(result) = lexer.next() {
        let span = lexer.span();
        if let Ok(token) = result {
            out.push(Lexeme { token, start: span.start, end: span.end });
        }
        // Err(()) => unrecognized byte: skip; parser recovery handles the fallout.
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<Token> {
        lex(src).into_iter().map(|l| l.token).collect()
    }

    #[test]
    fn arrows_and_idents() {
        assert_eq!(
            kinds("a -> b <-> c -- d ..> e"),
            vec![
                Token::Ident,
                Token::ArrowDirected,
                Token::Ident,
                Token::ArrowBi,
                Token::Ident,
                Token::ArrowUndirected,
                Token::Ident,
                Token::ArrowDotted,
                Token::Ident
            ]
        );
    }

    #[test]
    fn colors_vs_comments() {
        assert_eq!(kinds("#ff6b35"), vec![Token::Color]);
        assert_eq!(kinds("// #ff6b35 in a comment"), vec![Token::Comment]);
    }

    #[test]
    fn right_of_is_one_token() {
        assert_eq!(kinds("b right-of a"), vec![Token::Ident, Token::KwRightOf, Token::Ident]);
    }

    #[test]
    fn negative_numbers() {
        assert_eq!(kinds("@ (-10, 42.5)"), vec![
            Token::At,
            Token::LParen,
            Token::Number,
            Token::Comma,
            Token::Number,
            Token::RParen
        ]);
    }

    #[test]
    fn spans_are_byte_accurate() {
        let ls = lex("ab = #fff");
        assert_eq!((ls[0].start, ls[0].end), (0, 2));
        assert_eq!((ls[1].start, ls[1].end), (3, 4));
        assert_eq!((ls[2].start, ls[2].end), (5, 9));
    }
}
