use std::collections::HashMap;
use crate::error::Error;
use crate::parser::{Cycle, Factor, TypeAnnotation};

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Type {
    Param(usize),
    Int,
    Bool,
    String,
    Unit,
    Tuple(Vec<Type>),
    Function(Box<Type>, Box<Type>),
}

impl Type {
    fn function(arg: Type, ret: Type) -> Type {
        Type::Function(Box::new(arg), Box::new(ret))
    }
}

struct TypeChecker {
    environment: HashMap<String, Type>,
    param_count: usize,
}

impl TypeChecker {
    fn new() -> Self {
        Self {
            environment: HashMap::new(),
            param_count: 0,
        }
    }

    fn new_param(&mut self) -> Type {
        let parameter_count = self.param_count;
        self.param_count += 1;
        Type::Param(parameter_count)
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
                Ok(Type::function(Type::Tuple(in_types), Type::Tuple(out_types)))
            },
            TypeAnnotation::Identifier(name, _) if name == "Int" => Ok(Type::Int),
            TypeAnnotation::Identifier(name, _) if name == "Bool" => Ok(Type::Bool),
            TypeAnnotation::Identifier(name, _) if name == "String" => Ok(Type::String),
            TypeAnnotation::Identifier(name, token) => Err(Error::TypeError(format!("Unknown type {}", name), token.clone())),
        }
    }

    pub fn check(&mut self, cycles: &Vec<Cycle>) -> Result<(), Error> {
        for cycle in cycles {
            self.check_cycle(cycle)?;
        }
        Ok(())
    }

    pub fn check_cycle(&mut self, cycle: &Cycle) -> Result<Type, Error> {
        let t = match cycle {
            Cycle::Definition(name, annotation, factors) => {
                self.check_definition(name, &self.type_from_annotation(annotation)?, factors)?
            }
            Cycle::Term(factors) => {
                self.check_term(factors)?
            }
        };
        Ok(t)
    }

    fn check_definition(&mut self, name: &str, annotation: &Type, factors: &Vec<Factor>) -> Result<Type, Error> {
        self.environment.insert(name.to_string(), annotation.clone());
        self.check_term(factors)
    }

    fn check_term(&mut self, factors: &Vec<Factor>) -> Result<Type, Error> {
        let mut in_stack: Vec<Type> = Vec::new();
        let mut out_stack: Vec<Type> = Vec::new();
        for factor in factors {
            let t = self.check_factor(factor)?;
            match t {
                Type::Param(_) => out_stack.push(t),
                Type::Int => out_stack.push(t),
                Type::Bool => out_stack.push(t),
                Type::String => out_stack.push(t),
                Type::Unit => { /* no-op */ }
                Type::Tuple(ts) => out_stack.extend(ts),
                Type::Function(t_in, t_out) => {
                    let mut expected_in: Vec<Type> = Vec::new();
                    match *t_in {
                        Type::Param(_) => expected_in.push(*t_in),
                        Type::Int => expected_in.push(*t_in),
                        Type::Bool => expected_in.push(*t_in),
                        Type::String => expected_in.push(*t_in),
                        Type::Unit => { /* no-op */ }
                        Type::Tuple(ts) => expected_in.extend(ts),
                        Type::Function(_, _) => expected_in.push(*t_in),
                    }
                    for el in expected_in.into_iter().rev() {
                        if out_stack.len() == 0 {
                            in_stack.push(el);
                        } else {
                            out_stack.pop(); // TODO: Error if invalid value
                        }
                    }
                    match *t_out {
                        Type::Param(_) => out_stack.push(*t_out),
                        Type::Int => out_stack.push(*t_out),
                        Type::Bool => out_stack.push(*t_out),
                        Type::String => out_stack.push(*t_out),
                        Type::Unit => { /* no-op */ }
                        Type::Tuple(ts) => out_stack.extend(ts),
                        Type::Function(_, _) => out_stack.push(*t_out),
                    }
                }
            }
        }
        Ok(Type::function(Type::Tuple(in_stack), Type::Tuple(out_stack)))
    }

    fn check_factor(&mut self, factor: &Factor) -> Result<Type, Error> {
        match factor {
            Factor::Dup(_) => {
                let t = self.new_param();
                Ok(Type::function(t.clone(), t))
            },
            Factor::Drop(_) => {
                let t = self.new_param();
                Ok(Type::function(t, Type::Unit))
            },
            Factor::Quote(_) => {
                let t = self.new_param();
                Ok(Type::function(t.clone(), Type::function(Type::Unit, t)))
            },
            Factor::Call(_) => {
                unimplemented!()
            },
            Factor::Cat(_) => {
                unimplemented!()
            },
            Factor::Swap(_) => {
                let a = self.new_param();
                let b = self.new_param();
                Ok(Type::function(Type::function(a.clone(), b.clone()), Type::function(b, a)))
            },
            Factor::Ifte(_) => {
                let t_in = self.new_param();
                let t_out = self.new_param();
                let t_condition = Type::function(t_in, Type::Bool);
                let t_body = Type::function(Type::Bool, t_out.clone());
                let t_input = Type::Tuple(vec![t_condition, t_body.clone(), t_body.clone()]);
                let t_output = Type::Tuple(vec![t_out]);
                Ok(Type::function(t_input, t_output))
            },
            Factor::Int(_, _) => Ok(Type::function(Type::Unit, Type::Int)),
            Factor::Bool(_, _) => Ok(Type::function(Type::Unit, Type::Bool)),
            Factor::String(_, _) => Ok(Type::function(Type::Unit, Type::String)),
            Factor::Identifier(name, token) => {
                if !self.environment.contains_key(name) {
                    return Err(Error::TypeError(format!("Unknown identifier {}", name), token.clone()));
                }
                Ok(self.environment[name].clone())
            }
            Factor::Quotation(term) => {
                self.check_term(term)
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
        let input = parse("[dup drop dup]").unwrap();
        let mut typechecker = super::TypeChecker::new();
        typechecker.check(&input).unwrap();
    }
}