use std::fmt::Display;

use logos::{Logos, Span};
use snafu::Snafu;

#[derive(Logos, Copy, Clone, Debug, PartialEq)]
#[logos(skip r"[ \t\n\f]+")] // ignore whitespace and newlines
pub enum VersionToken<'source> {
    #[token("=")]
    Eq,
    #[token("==")]
    EqEq,
    #[token("!=")]
    NotEq,
    #[token(">=")]
    GtEq,
    #[token("<=")]
    LtEq,
    #[token(">")]
    Gt,
    #[token("<")]
    Lt,
    #[token("||")]
    Or,
    #[token("&&")]
    And,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[regex(r"[a-fA-F0-9]+", priority = 3)]
    Hexadecimal(&'source str),
    #[regex(r"(\d+:)?[0-9][0-9A-Za-z.+\-~]*")]
    VersionNumber(&'source str),
}

impl<'source> Display for VersionToken<'source> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionToken::Eq => write!(f, "="),
            VersionToken::EqEq => write!(f, "=="),
            VersionToken::NotEq => write!(f, "!="),
            VersionToken::GtEq => write!(f, ">="),
            VersionToken::LtEq => write!(f, "<="),
            VersionToken::Gt => write!(f, ">"),
            VersionToken::Lt => write!(f, "<"),
            VersionToken::Or => write!(f, "||"),
            VersionToken::And => write!(f, "&&"),
            VersionToken::LParen => write!(f, "("),
            VersionToken::RParen => write!(f, ")"),
            VersionToken::Hexadecimal(hex) => write!(f, "{hex}"),
            VersionToken::VersionNumber(version) => write!(f, "{version}"),
        }
    }
}

impl<'source> VersionToken<'source> {
    pub fn is_op(&self) -> bool {
        matches!(
            self,
            VersionToken::Eq
                | VersionToken::EqEq
                | VersionToken::NotEq
                | VersionToken::GtEq
                | VersionToken::LtEq
                | VersionToken::Gt
                | VersionToken::Lt
                | VersionToken::Or
                | VersionToken::And
        )
    }

    pub fn is_cmp_op(&self) -> bool {
        matches!(
            self,
            VersionToken::Eq
                | VersionToken::EqEq
                | VersionToken::NotEq
                | VersionToken::GtEq
                | VersionToken::LtEq
                | VersionToken::Gt
                | VersionToken::Lt
        )
    }

    pub fn precedence(&self) -> u8 {
        match self {
            VersionToken::Eq
            | VersionToken::GtEq
            | VersionToken::LtEq
            | VersionToken::Gt
            | VersionToken::Lt
            | VersionToken::NotEq => 10,
            VersionToken::Or | VersionToken::And => 1,
            _ => 0, // invalid operator
        }
    }
}

// const ZERO_STRING: &'static str = "0";
const VERSION_PLACEHOLDER: &str = "$VER";
const VERSION_PLACEHOLDER_TOKEN: VersionToken = VersionToken::VersionNumber(VERSION_PLACEHOLDER);

#[derive(Debug, Snafu)]
pub enum VersionParseError {
    #[snafu(display("Invalid version expression at position {span:?}"))]
    VersionExpr { span: Span },
    #[snafu(display("Unexpected string '{s}' at position {span:?}"))]
    UnexpectedString { s: String, span: Span },
    #[snafu(display("Unmatched '(' at position {span:?}"))]
    UnmatchedLeft { span: Span },
}

pub fn parse_version_expr(input: &str) -> Result<Vec<VersionToken<'_>>, VersionParseError> {
    let mut lexer = VersionToken::lexer(input);
    let mut stack: Vec<VersionToken> = Vec::with_capacity(8);
    let mut operators: Vec<VersionToken> = Vec::with_capacity(8);
    let mut prev_is_op = false;

    // convert infix notation to RPN
    while let Some(maybe_token) = lexer.next() {
        let token =
            maybe_token.map_err(|_| VersionParseError::VersionExpr { span: lexer.span() })?;

        if token.is_cmp_op() {
            // since we use a very simplified expression format, we don't have a LHS in our "binary expression"
            // we will push a dummy VERSION_PLACEHOLDER_TOKEN to the stack, and later replace it with the actual version
            stack.push(VERSION_PLACEHOLDER_TOKEN);
        }

        match token {
            VersionToken::Eq
            | VersionToken::EqEq
            | VersionToken::NotEq
            | VersionToken::GtEq
            | VersionToken::LtEq
            | VersionToken::Gt
            | VersionToken::Lt
            | VersionToken::Or
            | VersionToken::And => {
                if let Some(last_op) = operators.last()
                    && last_op.precedence() >= token.precedence()
                {
                    let last = operators.pop().unwrap();
                    stack.push(last);
                    operators.push(token);
                    prev_is_op = token.is_op();
                    continue;
                }
                operators.push(token);
            }
            VersionToken::LParen => operators.push(token),
            VersionToken::RParen => {
                // drain all operators and push them back to the output stack
                while let Some(op) = operators.pop() {
                    if op == VersionToken::LParen {
                        break;
                    }
                    stack.push(op);
                }
            }
            VersionToken::Hexadecimal(_) => {
                return Err(VersionParseError::VersionExpr { span: lexer.span() });
            }
            VersionToken::VersionNumber(_) => {
                if !prev_is_op {
                    return Err(VersionParseError::UnexpectedString {
                        s: token.to_string(),
                        span: lexer.span(),
                    });
                }
                stack.push(token);
            }
        }

        prev_is_op = token.is_op();
    }

    // drain all remaining operators and add them to the output stack
    while let Some(op) = operators.pop() {
        if op == VersionToken::LParen {
            return Err(VersionParseError::UnmatchedLeft { span: lexer.span() });
        }
        stack.push(op);
    }

    Ok(stack)
}

#[test]
fn test_lexer() {
    let input = "1.2.3+4-5";
    let mut lexer = VersionToken::lexer(input);
    let token = lexer.next().unwrap();
    assert_eq!(token, Ok(VersionToken::VersionNumber(input)));
    assert_eq!(lexer.slice(), "1.2.3+4-5");
}

#[test]
fn test_parser_simple() {
    let input_expr = "(=1.2.3 || =4.5.6) && <7.8.9";
    let tokens = parse_version_expr(input_expr).unwrap();
    assert_eq!(
        tokens,
        vec![
            VERSION_PLACEHOLDER_TOKEN,
            VersionToken::VersionNumber("1.2.3"),
            VersionToken::Eq,
            VERSION_PLACEHOLDER_TOKEN,
            VersionToken::VersionNumber("4.5.6"),
            VersionToken::Eq,
            VersionToken::Or,
            VERSION_PLACEHOLDER_TOKEN,
            VersionToken::VersionNumber("7.8.9"),
            VersionToken::Lt,
            VersionToken::And,
        ]
    );
}
