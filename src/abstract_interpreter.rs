use std::collections::HashMap;
use crate::error::Error;
use crate::parser::{Factor, parse, Parser};
use crate::scanner::Token;
use crate::typechecker::Type;

enum Rule {
    IsFunction
}

struct AbstractInterpreter {
    in_stack: Vec<Type>,
    out_stack: Vec<Type>,
    param_count: usize,
}

impl AbstractInterpreter {
    pub fn new() -> AbstractInterpreter {
        AbstractInterpreter {
            in_stack: Vec::new(),
            out_stack: Vec::new(),
            param_count: 0,
        }
    }

    fn new_param(&mut self) -> Type {
        let parameter_count = self.param_count;
        self.param_count += 1;
        Type::Param(parameter_count)
    }

    fn pop(&mut self) -> Type {
        let popped = if self.out_stack.is_empty() {
            let p = self.new_param();
            self.in_stack.push(p.clone());
            p
        } else {
            self.out_stack.pop().unwrap()
        };
        popped
    }

    fn push(&mut self, t: Type) {
        self.out_stack.push(t);
    }

    pub fn interpret(mut self, factors: &Vec<Factor>) -> Result<Type, Error> {
        for factor in factors {
            self.interpret_factor(factor)?;
        }
        Ok(Type::Function(self.in_stack, self.out_stack))
    }

    fn interpret_factor(&mut self, factor: &Factor) -> Result<(), Error> {
        match factor {
            Factor::Dup(_) => {
                let a = self.pop();
                let b = a.clone();
                self.push(a);
                self.push(b);
                Ok(())
            }
            Factor::Drop(_) => {
                self.pop();
                Ok(())
            }
            Factor::Quote(_) => {
                let a = self.pop();
                let wrapped = Type::Function(vec![], vec![a]);
                self.push(wrapped);
                Ok(())
            }
            Factor::Call(_) => {
                let a= self.pop();
                self.call_as_function(a)
            }
            Factor::Cat(_) => { unimplemented!() }
            Factor::Swap(_) => { unimplemented!() }
            Factor::Ifte(_) => { unimplemented!() }
            Factor::Int(_, _) => { self.push(Type::Int); Ok(()) }
            Factor::Bool(_, _) => { self.push(Type::Bool); Ok(()) }
            Factor::String(_, _) => { self.push(Type::String); Ok(()) }
            Factor::Identifier(_, _) => { unimplemented!() }
            Factor::Quotation(factors) => {
                let interpreter = AbstractInterpreter::new();
                let t = interpreter.interpret(factors)?;
                self.push(t);
                Ok(())
            }
        }
    }

    fn call_as_function(&mut self, a: Type) -> Result<(), Error> {
        match a {
            Type::Function(t_in, t_out) => {
                let mut learned: HashMap<usize, Type> = HashMap::new();
                for t_expected in t_in.iter().rev() {
                    let t_actual = self.pop();
                    if let Type::Param(in_p) = t_expected {
                        learned.insert(*in_p, t_actual);
                    } else {
                        if t_expected != &t_actual {
                            return Err(Error::TypeError(
                                format!("Expected {:?} but got {:?}", t_expected, t_actual),
                                Token::unknown(),
                            ));
                        }
                    }
                }
                for out in t_out.into_iter() {
                    match out {
                        Type::Param(param) => {
                            if let Some(t) = learned.get(&param) {
                                self.push(t.clone());
                            }
                        }
                        Type::Function(t_in, t_out) => {
                            let new_in = Self::substitute_learned(&mut learned, t_in);
                            let new_out = Self::substitute_learned(&mut learned, t_out);
                            self.push(Type::Function(new_in, new_out));
                        }
                        _ => {
                            self.push(out);
                        }
                    }
                }
                Ok(())
            }
            Type::Param(param) => {
                // TODO: Somehow learn that this should be a function
                Ok(())
            }
            _ => panic!("Expected function"),
        }
    }

    fn substitute_learned(learned: &mut HashMap<usize, Type>, t: Vec<Type>) -> Vec<Type> {
        let mut new = Vec::new();
        for t in t.into_iter() {
            match t {
                Type::Param(param) => {
                    if let Some(t) = learned.get(&param) {
                        new.push(t.clone());
                    }
                }
                t => {
                    new.push(t);
                }
            }
        }
        new
    }
}

#[cfg(test)]
mod tests {
    use crate::abstract_interpreter::AbstractInterpreter;
    use crate::error::Error;
    use crate::parser::{Cycle, parse};
    use crate::typechecker::Type;

    #[test]
    fn literal_int() {
        let input = "1";
        let actual = interpret(input).unwrap();
        let expected = Type::Function(vec![], vec![Type::Int]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn literal_bool() {
        let input = "true";
        let actual = interpret(input).unwrap();
        let expected = Type::Function(vec![], vec![Type::Bool]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn literal_string() {
        let input = "\"hello\"";
        let actual = interpret(input).unwrap();
        let expected = Type::Function(vec![], vec![Type::String]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn literal_int_and_bool() {
        let input = "1 true";
        let actual = interpret(input).unwrap();
        let expected = Type::Function(vec![], vec![Type::Int, Type::Bool]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn literal_int_and_string() {
        let input = "1 \"hello\"";
        let actual = interpret(input).unwrap();
        let expected = Type::Function(vec![], vec![Type::Int, Type::String]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn literal_bool_and_string() {
        let input = "true \"hello\"";
        let actual = interpret(input).unwrap();
        let expected = Type::Function(vec![], vec![Type::Bool, Type::String]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn literal_int_and_bool_and_string() {
        let input = "1 true \"hello\"";
        let actual = interpret(input).unwrap();
        let expected = Type::Function(vec![], vec![Type::Int, Type::Bool, Type::String]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn dup_with_parameter() {
        let input = "dup";
        let actual = interpret(input).unwrap();
        let expected = Type::Function(vec![Type::Param(0)], vec![Type::Param(0), Type::Param(0)]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn dup_with_concrete_type() {
        let input = "1 dup";
        let actual = interpret(input).unwrap();
        let expected = Type::Function(vec![], vec![Type::Int, Type::Int]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn dup_with_leftovers() {
        let input = "1 2 dup";
        let actual = interpret(input).unwrap();
        let expected = Type::Function(vec![], vec![Type::Int, Type::Int, Type::Int]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn drop_with_parameter() {
        let input = "drop";
        let actual = interpret(input).unwrap();
        let expected = Type::Function(vec![Type::Param(0)], vec![]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn drop_with_concrete_type() {
        let input = "1 drop";
        let actual = interpret(input).unwrap();
        let expected = Type::Function(vec![], vec![]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn drop_with_leftovers() {
        let input = "1 2 drop";
        let actual = interpret(input).unwrap();
        let expected = Type::Function(vec![], vec![Type::Int]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn quote_with_parameter() {
        let input = "quote";
        let actual = interpret(input).unwrap();
        let expected = Type::Function(vec![Type::Param(0)], vec![Type::Function(vec![], vec![Type::Param(0)])]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn quote_with_concrete_type() {
        let input = "1 quote";
        let actual = interpret(input).unwrap();
        let expected = Type::Function(vec![], vec![Type::Function(vec![], vec![Type::Int])]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn quote_with_leftovers() {
        let input = "1 2 quote";
        let actual = interpret(input).unwrap();
        let expected = Type::Function(vec![], vec![Type::Int, Type::Function(vec![], vec![Type::Int])]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn call_with_parameter() {
        let input = "call";
        let actual = interpret(input).unwrap();
        let expected = Type::Function(vec![Type::Param(0)], vec![]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn call_with_concrete_type() {
        let input = "[1] call";
        let actual = interpret(input).unwrap();
        let expected = Type::Function(vec![], vec![Type::Int]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn call_with_leftovers() {
        let input = "1 [2] call";
        let actual = interpret(input).unwrap();
        let expected = Type::Function(vec![], vec![Type::Int, Type::Int]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn call_with_multiple_returns() {
        let input = "[1 2] call";
        let actual = interpret(input).unwrap();
        let expected = Type::Function(vec![], vec![Type::Int, Type::Int]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn call_dup_using_previous_stack() {
        let input = "1 [dup] call";
        let actual = interpret(input).unwrap();
        let expected = Type::Function(vec![], vec![Type::Int, Type::Int]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn call_dup_multiple_times() {
        let input = "[1 dup dup] call";
        let actual = interpret(input).unwrap();
        let expected = Type::Function(vec![], vec![Type::Int, Type::Int, Type::Int]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn call_drop_using_previous_stack() {
        let input = "1 [drop] call";
        let actual = interpret(input).unwrap();
        let expected = Type::Function(vec![], vec![]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn call_drop_multiple_times() {
        let input = "1 1 [drop drop] call";
        let actual = interpret(input).unwrap();
        let expected = Type::Function(vec![], vec![]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn call_quote_using_previous_stack() {
        let input = "1 [quote] call";
        let actual = interpret(input).unwrap();
        let expected = Type::Function(vec![], vec![Type::Function(vec![], vec![Type::Int])]);
        assert_eq!(actual, expected);
    }

    #[test]
    fn call_quote_multiple_times() {
        let input = "1 [quote 1 quote] call";
        let actual = interpret(input).unwrap();
        let expected = Type::Function(
            vec![],
            vec![
                Type::Function(vec![], vec![Type::Int]),
                Type::Function(vec![], vec![Type::Int]),
            ],
        );
        assert_eq!(actual, expected);
    }

    fn interpret(input: &str) -> Result<Type, Error> {
        let cycles = parse(input)?;
        let interpreter = AbstractInterpreter::new();
        // TODO: This isn't right.
        for cycle in cycles {
            match cycle {
                Cycle::Term(factors) => {
                    return Ok(interpreter.interpret(&factors)?);
                }
                _ => {}
            }
        }
        Err(Error::UnexpectedEndOfFile("".to_string()))
    }
}