use crate::error::Error;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Token {
    pub value: String,
    pub line: usize,
    pub col: usize,
}

pub fn scan(string: &str) -> Result<Vec<Token>, Error> {
    let mut chars = string.chars().enumerate().peekable();
    let mut tokens = Vec::new();
    let mut line = 1;
    let mut col = 1;
    let mut token_size = 0;
    let mut token_start = 0;
    while let Some((index, c)) = chars.next() {
        match c {
            ' ' | '\t' | '\r' | '\n' => {
                if token_size > 0 {
                    tokens.push(Token {
                        value: string[token_start..index].to_string(),
                        line,
                        col: col - token_size,
                    });
                    token_size = 0;
                }
                if c == '\n' {
                    line += 1;
                    col = 1;
                } else {
                    col += 1;
                }
                token_start = index + 1;
            }
            '{' | '}' | '(' | ')' | '[' | ']' | '.' | ',' | ';' | ':' => {
                // These characters are always tokens by themselves
                if token_size > 0 {
                    tokens.push(Token {
                        value: string[token_start..index].to_string(),
                        line,
                        col: col - token_size,
                    });
                    token_size = 0;
                }
                tokens.push(Token {
                    value: string[index..index + 1].to_string(),
                    line,
                    col,
                });
                token_start = index + 1;
            }
            '"' => {
                col += 1;
                token_size += 1;
                while let Some((index, c)) = chars.next() {
                    col += 1;
                    token_size += 1;
                    match c {
                        '"' => {
                            tokens.push(Token {
                                value: string[token_start..(index+1)].to_string(),
                                line,
                                col: col - token_size,
                            });
                            token_size = 0;
                            break;
                        }
                        '\n' => {
                            return Err(Error::ParseError("Unterminated string".to_string(), Token { line, col, value: string[token_start..index].to_string() }));
                        }
                        '\\' => {
                            // Whatever the escape sequence is, we just skip it at this stage.
                            chars.next();
                            col += 1;
                            token_size += 1;
                        }
                        _ => {
                            // Just move on to the next character
                        }
                    }
                }
                if token_size > 0 {
                    return Err(Error::ParseError("Unterminated string".to_string(), Token { line, col, value: string[token_start..index].to_string() }));
                }
            }
            _ => {
                col += 1;
                token_size += 1;
            }
        }
    };
    if token_size > 0 {
        tokens.push(Token {
            value: string[token_start..].to_string(),
            line,
            col,
        });
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    #[test]
    fn scans_simple_string() {
        let tokens = super::scan("\"Hello, world!\"").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, "\"Hello, world!\"");
    }

    #[test]
    fn scans_simple_string_with_escapes() {
        let tokens = super::scan("\"Hello, \\nworld!\"").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].value, "\"Hello, \\nworld!\"");
    }

    #[test]
    fn brackets_are_their_own_tokens() {
        let tokens = super::scan("[Hello, world!]").unwrap();
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].value, "[");
        assert_eq!(tokens[1].value, "Hello");
        assert_eq!(tokens[2].value, ",");
        assert_eq!(tokens[3].value, "world!");
        assert_eq!(tokens[4].value, "]");
    }

    #[test]
    fn parens_are_their_own_tokens() {
        let tokens = super::scan("(Hello, world!)").unwrap();
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].value, "(");
        assert_eq!(tokens[1].value, "Hello");
        assert_eq!(tokens[2].value, ",");
        assert_eq!(tokens[3].value, "world!");
        assert_eq!(tokens[4].value, ")");
    }

    #[test]
    fn braces_are_their_own_tokens() {
        let tokens = super::scan("{Hello, world!}").unwrap();
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].value, "{");
        assert_eq!(tokens[1].value, "Hello");
        assert_eq!(tokens[2].value, ",");
        assert_eq!(tokens[3].value, "world!");
        assert_eq!(tokens[4].value, "}");
    }
}
