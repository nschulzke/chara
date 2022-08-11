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
pub enum TypeAnnotation {
    Function(Vec<TypeAnnotation>, Vec<TypeAnnotation>, Token, Token),
    Identifier(String, Token),
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Cycle {
    Definition(String, TypeAnnotation, Vec<Factor>),
    Term(Vec<Factor>),
}

pub struct Parser {
    pub tokens: Vec<Token>,
    pub cycles: Vec<Cycle>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Parser {
        Parser {
            tokens,
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
        !token.value.contains(|c| match c {
            '{' | '}' | '(' | ')' | '[' | ']' | '.' | ',' | ';' | ':' | '"' => true,
            _ => false,
        })
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

    /// Parse a definition.
    /// definition ::= "def" identifier ":" type "=" factor ";"
    fn parse_definition(&mut self) -> Result<Cycle, Error> {
        let def = self.next().ok_or(Error::UnexpectedEndOfFile(format!("Unexpected EOF, expected def")))?;
        if def.value != "def" {
            return Err(Error::UnexpectedToken("def".to_string(), def));
        }
        let name = self.next().ok_or(Error::UnexpectedEndOfFile(format!("Unexpected EOF, expected name")))?;
        if Self::is_valid_identifier(&name) == false {
            return Err(Error::UnexpectedToken("identifier".to_string(), name));
        }
        let colon = self.next().ok_or(Error::UnexpectedEndOfFile(format!("Unexpected EOF, expected colon")))?;
        if colon.value != ":" {
            return Err(Error::UnexpectedToken(":".to_string(), colon));
        }
        let type_ = self.parse_type()?;
        let equals = self.next().ok_or(Error::UnexpectedEndOfFile(format!("Unexpected EOF, expected =")))?;
        if equals.value != "=" {
            return Err(Error::UnexpectedToken("=".to_string(), equals));
        }
        let term = self.parse_term()?;
        let semi = self.next().ok_or(Error::UnexpectedEndOfFile(format!("Unexpected EOF, expected ;")))?;
        if semi.value != ";" {
            return Err(Error::UnexpectedToken(";".to_string(), semi));
        }
        Ok(Cycle::Definition(name.value, type_, term))
    }

    /// Parse a type annotation
    /// type ::= "Int" | "Bool" | "String" | identifier | "(" type { "," type } -> type { "," type } ")"
    fn parse_type(&mut self) -> Result<TypeAnnotation, Error> {
        let first_token = self.next().ok_or(Error::UnexpectedEndOfFile(format!("Unexpected EOF, expected type")))?;
        if first_token.value == "Int" {
            Ok(TypeAnnotation::Identifier("Int".to_string(), first_token))
        } else if first_token.value == "Bool" {
            Ok(TypeAnnotation::Identifier("Bool".to_string(), first_token))
        } else if first_token.value == "String" {
            Ok(TypeAnnotation::Identifier("String".to_string(), first_token))
        } else if Self::is_valid_identifier(&first_token) {
            Ok(TypeAnnotation::Identifier(first_token.value.to_string(), first_token))
        } else if first_token.value == "(" {
            let mut in_types: Vec<TypeAnnotation> = Vec::new();
            in_types.push(self.parse_type()?);
            while let Some(token) = self.next() {
                if token.value == "->" {
                    break;
                } else if token.value == "," {
                    in_types.push(self.parse_type()?);
                } else {
                    return Err(Error::UnexpectedToken(",".to_string(), token));
                }
            }
            let mut out_types: Vec<TypeAnnotation> = Vec::new();
            out_types.push(self.parse_type()?);
            let mut last_token = first_token.clone();
            while let Some(token) = self.next() {
                if token.value == ")" {
                    last_token = token;
                    break;
                } else if token.value == "," {
                    out_types.push(self.parse_type()?);
                } else {
                    return Err(Error::UnexpectedToken(",".to_string(), token));
                }
                last_token = token;
            }
            Ok(TypeAnnotation::Function(in_types, out_types, first_token, last_token))
        } else {
            Err(Error::UnexpectedToken("type".to_string(), first_token))
        }
    }

    /// Parse a factor.
    /// term ::= { factor }
    fn parse_term(&mut self) -> Result<Vec<Factor>, Error> {
        let mut factors = Vec::new();
        loop {
            let factor = self.parse_factor();
            match factor {
                Ok(factor) => factors.push(factor),
                Err(Error::EndOfTerm) => break,
                Err(err) => return Err(err),
            }
        }
        Ok(factors)
    }

    /// Parse a factor.
    /// factor ::=
    ///          "[" term "]"
    ///        | integer_literal | boolean_literal | string_literal | identifier | "(" term ")"
    fn parse_factor(&mut self) -> Result<Factor, Error> {
        let token = self.peek().ok_or(Error::EndOfTerm)?;
        match token.value.as_str() {
            "[" => {
                let _brace = self.next().unwrap();
                let term = self.parse_term()?;
                let close = self.next().ok_or(Error::UnexpectedEndOfFile(format!("Unexpected EOF, expected ]")))?;
                if close.value != "]" {
                    return Err(Error::UnexpectedToken("]".to_string(), close));
                }
                Ok(Factor::Quotation(term))
            }
            "dup" => Ok(Factor::Dup(self.next().unwrap())),
            "drop" => Ok(Factor::Drop(self.next().unwrap())),
            "quote" => Ok(Factor::Quote(self.next().unwrap())),
            "call" => Ok(Factor::Call(self.next().unwrap())),
            "cat" => Ok(Factor::Cat(self.next().unwrap())),
            "swap" => Ok(Factor::Swap(self.next().unwrap())),
            "ifte" => Ok(Factor::Ifte(self.next().unwrap())),
            _ => match token.value.parse::<i64>() {
                Ok(i) => Ok(Factor::Integer(Value::Integer(i), self.next().unwrap())),
                Err(_) => match token.value.parse::<bool>() {
                    Ok(b) => Ok(Factor::Boolean(Value::Boolean(b), self.next().unwrap())),
                    Err(_) => {
                        if Self::is_valid_identifier(&token) {
                            Ok(Factor::Identifier(token.value.to_string(), self.next().unwrap()))
                        } else if token.value.starts_with('"') && token.value.ends_with('"') {
                            Ok(Factor::String(Value::String(token.value.trim_matches('"').to_string()), self.next().unwrap()))
                        } else {
                            Err(Error::EndOfTerm)
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
    fn terminates_if_given_a_bad_definition() {
        let tokens = scan("def a: Int = 1 [").unwrap();
        let mut parser = super::Parser::new(tokens);
        let error = parser.parse();
        assert!(error.is_err());
    }

    #[test]
    fn parses_definitions() {
        let tokens = scan("def a: Int = 1;").unwrap();
        let mut parser = super::Parser::new(tokens);
        let cycles = parser.parse().unwrap();
        assert_eq!(cycles.len(), 1);
        match cycles[0] {
            super::Cycle::Definition(ref name, ref annotation, ref factors) => {
                assert_eq!(name, "a");
                match annotation {
                    super::TypeAnnotation::Identifier(s, _) if s == "Int" => {}
                    _ => panic!("Expected Int, got {:?}", annotation),
                }
                assert_eq!(factors.len(), 1);
                match &factors[0] {
                    super::Factor::Integer(super::Value::Integer(1), _) => {}
                    _ => panic!("Expected 1, got {:?}", factors[0]),
                }
            }
            _ => panic!("Expected Definition, got {:?}", cycles[0]),
        }
    }

    #[test]
    fn parses_definitions_with_function_types() {
        let tokens = scan("def a: (Int, String -> Int, String) = 1 drop;").unwrap();
        let mut parser = super::Parser::new(tokens);
        let cycles = parser.parse().unwrap();
        assert_eq!(cycles.len(), 1);
        match cycles[0] {
            super::Cycle::Definition(ref name, ref annotation, ref factors) => {
                assert_eq!(name, "a");
                match annotation {
                    super::TypeAnnotation::Function(ref in_types, out_types, _, _)
                        if in_types.len() == 2 && out_types.len() == 2 => {
                        match &in_types[0] {
                            super::TypeAnnotation::Identifier(s, _) if s == "Int" => {}
                            _ => panic!("Expected Int, got {:?}", in_types[0]),
                        }
                        match &in_types[1] {
                            super::TypeAnnotation::Identifier(s, _) if s == "String" => {}
                            _ => panic!("Expected String, got {:?}", in_types[1]),
                        }
                        match &out_types[0] {
                            super::TypeAnnotation::Identifier(s, _) if s == "Int" => {}
                            _ => panic!("Expected Int, got {:?}", out_types[0]),
                        }
                        match &out_types[1] {
                            super::TypeAnnotation::Identifier(s, _) if s == "String" => {}
                            _ => panic!("Expected String, got {:?}", out_types[0]),
                        }
                    }
                    _ => panic!("Expected Function, got {:?}", annotation),
                }
                assert_eq!(factors.len(), 2);
                match &factors[0] {
                    super::Factor::Integer(super::Value::Integer(1), _) => {}
                    _ => panic!("Expected 1, got {:?}", factors[0]),
                }
                match &factors[1] {
                    super::Factor::Drop(_) => {}
                    _ => panic!("Expected drop, got {:?}", factors[1]),
                }
            }
            _ => panic!("Expected Definition, got {:?}", cycles[0]),
        }
    }
}
