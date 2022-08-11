use std::fmt::Debug;
use crate::error::{Error};
use crate::scanner::Token;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Value {
    Integer(i64),
    Boolean(bool),
    String(String),
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Factor {
    Dup(Token),    // [A] -> [A] [A]
    Drop(Token),   // [A] [A] -> [A]
    Quote(Token),  // [A] -> [ [ A ] ]
    Call(Token),   // S [S -> A] -> A
    Cat(Token),    // [A] [B] -> [A B]
    Swap(Token),   // [A] [B] -> [B] [A]
    Ifte(Token),   // S [S -> Bool] [S -> T] [S -> F] -> T|F
    Integer(Value, Token),
    Boolean(Value, Token),
    String(Value, Token),
    Identifier(String, Token),
    Quotation(Vec<Factor>),
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Cycle {
    Definition(String, Factor),
    Term(Vec<Factor>),
}

pub struct Parser {
    pub tokens: Vec<Token>,
    pub current: usize,
    pub cycles: Vec<Cycle>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Parser {
        Parser {
            tokens,
            current: 0,
            cycles: Vec::new(),
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(0)
    }

    fn next(&mut self) -> Option<Token> {
        if self.tokens.len() > 0 {
            self.tokens.drain(0..1).next()
        } else {
            None
        }
    }

    fn is_valid_identifier(token: &Token) -> bool {
        // Identifiers cannot contain braces, parens, brackets, or quotes.
        !token.value.contains('{') && !token.value.contains('(') && !token.value.contains('[')
            && !token.value.contains('}') && !token.value.contains(')') && !token.value.contains(']')
            && !token.value.contains('"')
    }

    fn parse(&mut self) -> Result<Vec<Cycle>, Error> {
        let mut cycles: Vec<Cycle> = Vec::new();
        while let Some(token) = self.peek() {
            let cycle = if token.value == "def" {
                self.parse_definition()
            } else {
                Ok(Cycle::Term(self.parse_term()?))
            }?;
            cycles.push(cycle);
        }
        Ok(cycles)
    }

    // Parse a definition.
    /// definition ::= "def" identifier "=" factor "."
    fn parse_definition(&mut self) -> Result<Cycle, Error> {
        let def = self.next().ok_or(Error::UnexpectedEndOfFile(format!("Unexpected EOF, expected def")))?;
        if def.value != "def" {
            return Err(Error::UnexpectedToken(def));
        }
        let name = self.next().ok_or(Error::UnexpectedEndOfFile(format!("Unexpected EOF, expected name")))?;
        if Self::is_valid_identifier(&name) == false {
            return Err(Error::UnexpectedToken(name));
        }
        let equals = self.next().ok_or(Error::UnexpectedEndOfFile(format!("Unexpected EOF, expected =")))?;
        if equals.value != "=" {
            return Err(Error::UnexpectedToken(equals));
        }
        let factor = self.parse_factor()?;
        let dot = self.next().ok_or(Error::UnexpectedEndOfFile(format!("Unexpected EOF, expected .")))?;
        if dot.value != "." {
            return Err(Error::UnexpectedToken(dot));
        }
        Ok(Cycle::Definition(name.value, factor))
    }

    /// Parse a factor.
    /// term ::= { factor }
    fn parse_term(&mut self) -> Result<Vec<Factor>, Error> {
        let mut factors = Vec::new();
        while let Ok(factor) = self.parse_factor() {
            factors.push(factor);
        }
        Ok(factors)
    }

    /// Parse a factor.
    /// factor ::=
    ///          "[" term "]"
    ///        | integer_literal | boolean_literal | string_literal | identifier | "(" term ")"
    fn parse_factor(&mut self) -> Result<Factor, Error> {
        self.current += 1;
        let token = self.next().ok_or(Error::UnexpectedEndOfFile(format!("Unexpected EOF, expected factor")))?;
        match token.value.as_str() {
            "[" => {
                let term = self.parse_term()?;
                self.current += 1;
                let close = self.next().ok_or(Error::UnexpectedEndOfFile(format!("Unexpected EOF, expected ]")))?;
                if close.value != "]" {
                    return Err(Error::UnexpectedToken(close));
                }
                Ok(Factor::Quotation(term))
            }
            "dup" => Ok(Factor::Dup(token)),
            "drop" => Ok(Factor::Drop(token)),
            "quote" => Ok(Factor::Quote(token)),
            "call" => Ok(Factor::Call(token)),
            "cat" => Ok(Factor::Cat(token)),
            "swap" => Ok(Factor::Swap(token)),
            "ifte" => Ok(Factor::Ifte(token)),
            _ => match token.value.parse::<i64>() {
                Ok(i) => Ok(Factor::Integer(Value::Integer(i), token)),
                Err(_) => match token.value.parse::<bool>() {
                    Ok(b) => Ok(Factor::Boolean(Value::Boolean(b), token)),
                    Err(_) => {
                        if Self::is_valid_identifier(&token) {
                            Ok(Factor::Identifier(token.value.to_string(), token))
                        } else if token.value.starts_with('"') && token.value.ends_with('"') {
                            Ok(Factor::String(Value::String(token.value.trim_matches('"').to_string()), token))
                        } else {
                            Err(Error::UnexpectedToken(token))
                        }
                    }
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::scanner::{scan};

    #[test]
    fn parses_simple_addition() {
        let tokens = scan("1 2 + ").unwrap();
        let mut parser = super::Parser::new(tokens);
        let cycles = parser.parse().unwrap();
        assert_eq!(cycles.len(), 1);
        match cycles[0] {
            super::Cycle::Term(ref terms) => {
                assert_eq!(terms.len(), 3);
                match terms[0] {
                    super::Factor::Integer(super::Value::Integer(1), _) => {}
                    _ => panic!("Expected 1, got {:?}", terms[0]),
                }
                match terms[1] {
                    super::Factor::Integer(super::Value::Integer(2), _) => {}
                    _ => panic!("Expected 2, got {:?}", terms[1]),
                }
                match &terms[2] {
                    super::Factor::Identifier(s, _) if s == "+" => {}
                    _ => panic!("Expected 3, got {:?}", terms[2]),
                }
            }
            _ => panic!("Expected Term, got {:?}", cycles[0]),
        }
    }

    #[test]
    fn parses_strings() {
        let tokens = scan("\"Hello\"").unwrap();
        let mut parser = super::Parser::new(tokens);
        let cycles = parser.parse().unwrap();
        assert_eq!(cycles.len(), 1);
        match cycles[0] {
            super::Cycle::Term(ref terms) => {
                assert_eq!(terms.len(), 1);
                match &terms[0] {
                    super::Factor::String(super::Value::String(s), _) if s == "Hello" => {}
                    _ => panic!("Expected Hello, got {:?}", terms[0]),
                }
            }
            _ => panic!("Expected Term, got {:?}", cycles[0]),
        }
    }

    #[test]
    fn parses_definitions() {
        let tokens = scan("def a = 1.").unwrap();
        let mut parser = super::Parser::new(tokens);
        let cycles = parser.parse().unwrap();
        assert_eq!(cycles.len(), 1);
        match cycles[0] {
            super::Cycle::Definition(ref name, ref factor) => {
                assert_eq!(name, "a");
                match factor {
                    super::Factor::Integer(super::Value::Integer(1), _) => {}
                    _ => panic!("Expected 1, got {:?}", factor),
                }
            }
            _ => panic!("Expected Definition, got {:?}", cycles[0]),
        }
    }
}
