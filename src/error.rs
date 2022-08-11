use crate::scanner::Token;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Error {
    ParseError(String, Token),
    UnexpectedEndOfFile,
    UnexpectedToken(Token),
}
