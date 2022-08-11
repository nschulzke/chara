use crate::scanner::Token;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Error {
    ParseError(String, Token),
    TypeError(String, Token),
    UnexpectedEndOfFile(String),
    UnexpectedToken(String, Token),
    EndOfTerm,
}
