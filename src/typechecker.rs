use std::collections::HashMap;
use crate::error::Error;
use crate::parser::{Cycle, Factor, TypeAnnotation};

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Type {
    Param(usize),
    Int,
    Bool,
    String,
    Function(Vec<Type>, Vec<Type>),
}

struct TypeChecker {
    environment: HashMap<String, Type>,
    param_count: usize,
}

impl TypeChecker {
    fn new() -> Self {
        let mut environment: HashMap<String, Type> = HashMap::new();
        environment.insert("+".to_string(), Type::Function(vec![Type::Int, Type::Int], vec![Type::Int]));
        environment.insert("-".to_string(), Type::Function(vec![Type::Int, Type::Int], vec![Type::Int]));
        environment.insert("*".to_string(), Type::Function(vec![Type::Int, Type::Int], vec![Type::Int]));
        environment.insert("/".to_string(), Type::Function(vec![Type::Int, Type::Int], vec![Type::Int]));
        environment.insert("<".to_string(), Type::Function(vec![Type::Int, Type::Int], vec![Type::Bool]));
        environment.insert(">".to_string(), Type::Function(vec![Type::Int, Type::Int], vec![Type::Bool]));
        environment.insert("=".to_string(), Type::Function(vec![Type::Int, Type::Int], vec![Type::Bool]));
        environment.insert("not".to_string(), Type::Function(vec![Type::Bool], vec![Type::Bool]));
        environment.insert("and".to_string(), Type::Function(vec![Type::Bool, Type::Bool], vec![Type::Bool]));
        environment.insert("or".to_string(), Type::Function(vec![Type::Bool, Type::Bool], vec![Type::Bool]));
        Self {
            environment,
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
                Ok(Type::Function(in_types, out_types))
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
                Type::Function(t_in, mut t_out) => {
                    for t_expected in t_in.into_iter().rev() {
                        if out_stack.len() == 0 {
                            in_stack.push(t_expected);
                        } else {
                            let t_actual = out_stack.pop().unwrap();
                            if let Type::Param(n_expected) = t_expected {
                                if let Type::Param(n_actual) = t_actual {
                                    t_out.iter_mut().for_each(|el| {
                                        match el {
                                            Type::Param(n) if n == &n_expected => {
                                                *el = Type::Param(n_actual);
                                            },
                                            _ => {},
                                        }
                                    });
                                }
                            }
                        }
                    }
                    out_stack.extend(t_out.into_iter());
                }
            }
        }
        Ok(Type::Function(in_stack, out_stack))
    }

    fn check_factor(&mut self, factor: &Factor) -> Result<Type, Error> {
        match factor {
            Factor::Dup(_) => {
                let t = self.new_param();
                Ok(Type::Function(vec![t.clone()], vec![t.clone(), t]))
            },
            Factor::Drop(_) => {
                let t = self.new_param();
                Ok(Type::Function(vec![t], vec![]))
            },
            Factor::Quote(_) => {
                let t = self.new_param();
                Ok(Type::Function(vec![t.clone()], vec![Type::Function(vec![], vec![t])]))
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
                Ok(Type::Function(vec![a.clone(), b.clone()], vec![b, a]))
            },
            Factor::Ifte(_) => {
                let t_in = self.new_param();
                let t_out = self.new_param();
                let t_condition = Type::Function(vec![t_in], vec![Type::Bool]);
                let t_body = Type::Function(vec![Type::Bool], vec![t_out.clone()]);
                let t_input = vec![t_condition, t_body.clone(), t_body.clone()];
                let t_output = vec![t_out];
                Ok(Type::Function(t_input, t_output))
            },
            Factor::Int(_, _) => Ok(Type::Function(vec![], vec![Type::Int])),
            Factor::Bool(_, _) => Ok(Type::Function(vec![], vec![Type::Bool])),
            Factor::String(_, _) => Ok(Type::Function(vec![], vec![Type::String])),
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
    use super::{Type};

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

    #[test]
    fn gets_correct_simple_type() {
        let input = parse("1 2 +").unwrap();
        let mut typechecker = super::TypeChecker::new();
        let t = typechecker.check_cycle(&input[0]).unwrap();
        match t {
            super::Type::Function(ref t_in, ref t_out) => {
                assert_eq!(t_in.len(), 0);
                assert_eq!(t_out.len(), 1);
                assert_eq!(t_out[0], super::Type::Int);
            }
            _ => panic!("Expected Function"),
        }
    }

    #[test]
    fn gets_correct_param_types() {
        let input = parse("[dup drop dup]").unwrap();
        let mut typechecker = super::TypeChecker::new();
        let t = typechecker.check_cycle(&input[0]).unwrap();
        match t {
            super::Type::Function(ref t_in, ref t_out) => {
                assert_eq!(t_in.len(), 1);
                assert_eq!(t_in[0], super::Type::Param(0));
                assert_eq!(t_out.len(), 2);
                assert_eq!(t_out[0], super::Type::Param(0));
                assert_eq!(t_out[1], super::Type::Param(0));
            }
            _ => panic!("Expected Function"),
        }
    }

    #[test]
    fn gets_correct_param_types_complicated() {
        let input = parse("[ifte dup drop]").unwrap();
        let mut typechecker = super::TypeChecker::new();
        let t = typechecker.check_cycle(&input[0]).unwrap();
        match t {
            Type::Function(ref t_in, ref t_out) => {
                assert_eq!(t_in.len(), 3);
                assert_eq!(t_in[0], Type::Function(vec![Type::Param(0)], vec![Type::Bool]));
                assert_eq!(t_in[1], Type::Function(vec![Type::Bool], vec![Type::Param(1)]));
                assert_eq!(t_in[2], Type::Function(vec![Type::Bool], vec![Type::Param(1)]));
                assert_eq!(t_out.len(), 1);
                assert_eq!(t_out[0], Type::Param(1));
            }
            _ => panic!("Expected Function"),
        }
    }
}