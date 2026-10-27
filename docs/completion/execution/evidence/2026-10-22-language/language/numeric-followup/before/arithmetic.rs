use super::{Capture, LanguageError, Token, TokenKind};
/// Canonical exact decimal evidence. Construction checks significant digits and scale.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CanonicalDecimal {
    coefficient: i128,
    scale: u8,
}
impl CanonicalDecimal {
    pub fn parse(s: &str) -> Result<Self, LanguageError> {
        let (negative, s) = if let Some(s) = s.strip_prefix('-') {
            (true, s)
        } else {
            (false, s.strip_prefix('+').unwrap_or(s))
        };
        let mut digits = String::new();
        let mut scale = 0usize;
        let mut dot = false;
        for c in s.chars() {
            if c == '.' && !dot {
                dot = true;
                continue;
            }
            if !c.is_ascii_digit() {
                return Err(LanguageError::Decimal);
            }
            digits.push(c);
            if dot {
                scale += 1
            }
        }
        if digits.is_empty() || scale > 18 || digits.trim_start_matches('0').len() > 18 {
            return Err(LanguageError::Decimal);
        }
        let mut coefficient = digits.parse::<i128>().map_err(|_| LanguageError::Decimal)?;
        if negative {
            coefficient = -coefficient
        }
        while scale > 0 && coefficient % 10 == 0 {
            coefficient /= 10;
            scale -= 1
        }
        Ok(Self {
            coefficient,
            scale: scale as u8,
        })
    }
    pub fn coefficient(&self) -> i128 {
        self.coefficient
    }
    pub fn scale(&self) -> u8 {
        self.scale
    }
    pub fn to_f64(&self) -> f64 {
        self.coefficient as f64 / 10_f64.powi(i32::from(self.scale))
    }
}
fn number_word(s: &str) -> Option<&'static str> {
    match s {
        "zero" => Some("0"),
        "one" => Some("1"),
        "two" => Some("2"),
        "three" => Some("3"),
        "four" => Some("4"),
        "five" => Some("5"),
        "six" => Some("6"),
        "seven" => Some("7"),
        "eight" => Some("8"),
        "nine" => Some("9"),
        "ten" => Some("10"),
        _ => None,
    }
}
fn operator_word(s: &str) -> Option<&'static str> {
    match s {
        "plus" => Some("+"),
        "minus" => Some("-"),
        "times" => Some("*"),
        "divided" => Some("/"),
        "modulo" => Some("%"),
        "power" => Some("^"),
        _ => None,
    }
}
pub(super) fn normalize(tokens: &mut [Token], steps: usize) -> Result<Option<f64>, LanguageError> {
    normalize_counted(tokens, steps).map(|v| v.map(|(value, _)| value))
}
fn normalize_counted(
    tokens: &mut [Token],
    steps: usize,
) -> Result<Option<(f64, usize)>, LanguageError> {
    if tokens.is_empty()
        || tokens.iter().any(|t| {
            t.quoted()
                || !(t.kind == TokenKind::Number
                    || t.kind == TokenKind::Operator
                    || number_word(&t.lemma).is_some()
                    || operator_word(&t.lemma).is_some())
        })
    {
        return Ok(None);
    }
    for t in tokens.iter_mut() {
        if let Some(s) = number_word(&t.lemma) {
            t.lemma = s.into();
            t.kind = TokenKind::Number
        }
        if let Some(s) = operator_word(&t.lemma) {
            t.lemma = s.into();
            t.kind = TokenKind::Operator
        }
        if t.kind == TokenKind::Number {
            t.decimal = Some(CanonicalDecimal::parse(&t.lemma)?);
        }
    }
    let mut parser = Parser {
        tokens,
        pos: 0,
        left: steps,
    };
    let value = parser.expr(0, 1)?;
    if parser.pos != tokens.len() {
        return Err(LanguageError::Syntax);
    }
    Ok(Some((value, steps - parser.left)))
}
pub fn evaluate_math(source: &str, max_steps: usize) -> Result<f64, LanguageError> {
    evaluate_counted(source, max_steps).map(|(value, _)| value)
}
pub(super) fn evaluate_counted(
    source: &str,
    max_steps: usize,
) -> Result<(f64, usize), LanguageError> {
    if source.chars().count() > 2000 {
        return Err(LanguageError::InputLimit);
    }
    if max_steps == 0 || max_steps > 500 {
        return Err(LanguageError::InvalidBudget);
    }
    let mut tokens = super::tokenize::tokenize(source, 0)?;
    normalize_counted(&mut tokens, max_steps)?.ok_or(LanguageError::TypeMismatch)
}
pub(super) fn captures(tokens: &[Token]) -> Vec<Capture> {
    tokens
        .iter()
        .filter_map(|t| {
            t.decimal.as_ref().map(|d| Capture {
                source: t.source.clone(),
                span: t.span,
                decimal: d.clone(),
            })
        })
        .collect()
}
struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    left: usize,
}
impl Parser<'_> {
    fn tick(&mut self) -> Result<(), LanguageError> {
        self.left = self.left.checked_sub(1).ok_or(LanguageError::StepBudget)?;
        Ok(())
    }
    fn expr(&mut self, min: u8, depth: usize) -> Result<f64, LanguageError> {
        if depth > 16 {
            return Err(LanguageError::DepthLimit);
        }
        self.tick()?;
        let t = self.tokens.get(self.pos).ok_or(LanguageError::Syntax)?;
        self.pos += 1;
        let mut lhs = match t.lemma.as_str() {
            "+" => self.expr(25, depth + 1)?,
            "-" => -self.expr(25, depth + 1)?,
            "(" => {
                let v = self.expr(0, depth + 1)?;
                if self.tokens.get(self.pos).map(|t| t.lemma.as_str()) != Some(")") {
                    return Err(LanguageError::Syntax);
                }
                self.pos += 1;
                v
            }
            _ => t.decimal.as_ref().ok_or(LanguageError::Syntax)?.to_f64(),
        };
        while let Some(t) = self.tokens.get(self.pos) {
            let (l, r) = match t.lemma.as_str() {
                "+" | "-" => (10, 11),
                "*" | "/" | "%" => (20, 21),
                "^" => (30, 30),
                _ => break,
            };
            if l < min {
                break;
            }
            let op = t.lemma.clone();
            self.pos += 1;
            self.tick()?;
            let rhs = self.expr(r, depth + 1)?;
            lhs = match op.as_str() {
                "+" => lhs + rhs,
                "-" => lhs - rhs,
                "*" => lhs * rhs,
                "/" | "%" if rhs == 0.0 => return Err(LanguageError::DivisionByZero),
                "/" => lhs / rhs,
                "%" => lhs % rhs,
                "^" => libm::pow(lhs, rhs),
                _ => unreachable!(),
            };
            if !lhs.is_finite() {
                return Err(LanguageError::NonFinite);
            }
        }
        Ok(lhs)
    }
}
