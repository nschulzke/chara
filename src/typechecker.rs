use std::collections::HashMap;
use crate::error::Error;
use crate::parser::{Cycle, Factor, TypeAnnotation};

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Type {
    Int,
    Bool,
    String,
    Function(Vec<Type>, Vec<Type>),
}

struct TypeChecker {
    environment: HashMap<String, Type>,
}

impl TypeChecker {
    fn new() -> Self {
        Self {
            environment: HashMap::new(),
        }
    }

    fn type_from_annotation(&self, annotation: &TypeAnnotation) -> Result<Type, Error> {
        match annotation {
            TypeAnnotation::Function(in_types, out_types, token, _) => {
                let (in_types, in_type_errors): (Vec<_>, Vec<_>) =
                    in_types.iter()
                        .map(|t| self.type_from_annotation(t))
                        .partition(Result::is_ok);
                let in_types: Vec<_> = in_types.into_iter().map(Result::unwrap).collect();
                let (out_types, out_type_errors): (Vec<_>, Vec<_>) =
                    out_types.iter()
                        .map(|t| self.type_from_annotation(t))
                        .partition(Result::is_ok);
                let out_types: Vec<_> = out_types.into_iter().map(Result::unwrap).collect();
                if in_type_errors.len() > 0 || out_type_errors.len() > 0 {
                    return Err(Error::TypeError("Error in function type".to_string(), token.clone(), ));
                }
                Ok(Type::Function(in_types, out_types))
            },
            TypeAnnotation::Identifier(name, _) if name == "Int" => Ok(Type::Int),
            TypeAnnotation::Identifier(name, _) if name == "Bool" => Ok(Type::Bool),
            TypeAnnotation::Identifier(name, _) if name == "String" => Ok(Type::String),
            TypeAnnotation::Identifier(name, token) => Err(Error::TypeError(format!("Unknown type {}", name), token.clone())),
        }
    }

    fn check(&mut self, cycles: &Vec<Cycle>) -> Result<(), Error> {
        for cycle in cycles {
            self.check_cycle(cycle)?;
        }
        Ok(())
    }

    fn check_cycle(&mut self, cycle: &Cycle) -> Result<(), Error> {
        match cycle {
            Cycle::Definition(name, annotation, factors) => {
                self.check_definition(name, &self.type_from_annotation(annotation)?, factors)?;
            }
            Cycle::Term(factors) => {
                self.check_term(factors)?;
            }
        }
        Ok(())
    }

    fn check_definition(&mut self, name: &str, annotation: &Type, factors: &Vec<Factor>) -> Result<(), Error> {
        self.environment.insert(name.to_string(), annotation.clone());
        for factor in factors {
            self.check_factor(factor)?;
        }
        Ok(())
    }

    fn check_term(&mut self, factors: &Vec<Factor>) -> Result<(), Error> {
        for factor in factors {
            self.check_factor(factor)?;
        }
        Ok(())
    }

    fn check_factor(&mut self, factor: &Factor) -> Result<(), Error> {
        match factor {
            Factor::Dup(_) => Ok(()),
            Factor::Drop(_) => Ok(()),
            Factor::Quote(_) => Ok(()),
            Factor::Call(_) => Ok(()),
            Factor::Cat(_) => Ok(()),
            Factor::Swap(_) => Ok(()),
            Factor::Ifte(_) => Ok(()),
            Factor::Int(value, _) => Ok(()),
            Factor::Bool(value, _) => Ok(()),
            Factor::String(value, _) => Ok(()),
            Factor::Identifier(name, token) => {
                if !self.environment.contains_key(name) {
                    return Err(Error::TypeError(format!("Unknown identifier {}", name), token.clone()));
                }
                Ok(())
            }
            Factor::Quotation(term) => {
                self.check_term(term)?;
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::parse;

    #[test]
    fn recognizes_unknown_identifiers() {
        let input = parse("[a b c]").unwrap();
        let mut typechecker = super::TypeChecker::new();
        let error = typechecker.check(&input).unwrap_err();
        match error {
            super::Error::TypeError(message, token) => {
                assert_eq!(message, "Unknown identifier a");
                assert_eq!(token.value, "a");
            }
            _ => panic!("Expected TypeError"),
        }
    }

    #[test]
    fn allows_known_identifiers() {
        let input = parse("[dup drop cat]").unwrap();
        let mut typechecker = super::TypeChecker::new();
        typechecker.check(&input).unwrap();
    }
}