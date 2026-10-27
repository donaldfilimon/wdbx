use super::{LanguageError, Role, Span, Token, TokenKind};
pub(super) fn tokenize(s: &str, base: usize) -> Result<Vec<Token>, LanguageError> {
    let mut out = Vec::new();
    let mut it = s.char_indices().peekable();
    while let Some((start, c)) = it.next() {
        if c.is_whitespace() {
            continue;
        }
        let mut end = start + c.len_utf8();
        let kind;
        let mut lemma = None;
        if c == '"' || c == '\'' {
            let quote = c;
            let content_start = end;
            let mut close = None;
            let mut escaped = false;
            for (i, ch) in it.by_ref() {
                end = i + ch.len_utf8();
                if escaped {
                    escaped = false;
                    continue;
                }
                if ch == '\\' {
                    escaped = true;
                    continue;
                }
                if ch == quote {
                    close = Some(i);
                    break;
                }
            }
            let stop = close.ok_or(LanguageError::UnclosedQuote)?;
            lemma = Some(s[content_start..stop].to_string());
            kind = TokenKind::Quoted;
        } else if c.is_alphabetic() || c == '_' {
            while let Some(&(i, ch)) = it.peek() {
                if ch.is_alphanumeric() || ch == '_' || ch == '\'' || ch == '’' {
                    it.next();
                    end = i + ch.len_utf8()
                } else {
                    break;
                }
            }
            kind = TokenKind::Word;
        } else if c.is_ascii_digit()
            || c == '.' && it.peek().is_some_and(|(_, ch)| ch.is_ascii_digit())
        {
            while let Some(&(i, ch)) = it.peek() {
                if ch.is_ascii_digit() || ch == '.' {
                    it.next();
                    end = i + 1
                } else {
                    break;
                }
            }
            kind = TokenKind::Number;
        } else if "+-*/%^()".contains(c) {
            kind = TokenKind::Operator
        } else {
            kind = TokenKind::Punctuation
        }
        let source = s[start..end].to_string();
        out.push(Token {
            lemma: lemma.unwrap_or_else(|| source.to_lowercase()),
            source,
            span: Span {
                start: base + start,
                end: base + end,
            },
            kind,
            role: Role::Other,
            decimal: None,
            rule_refs: vec![],
        });
    }
    Ok(out)
}
