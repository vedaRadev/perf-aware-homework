use std::{ fmt, slice, rc::Rc, };

#[derive(Debug)]
pub struct InvalidJsonError { at: usize, message: String }

#[derive(PartialEq)]
#[derive(Debug)]
enum JsonTokenType {
    OpenBrace,
    CloseBrace,
    OpenBracket,
    CloseBracket,
    StringLiteral,
    Number,
    True,
    False,
    Null,
    Colon,
    Comma,
}

impl fmt::Display for JsonTokenType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::OpenBrace => write!(f, "open brace ({{)"),
            Self::CloseBrace => write!(f, "close brace (}})"),
            Self::OpenBracket => write!(f, "open bracket ([)"),
            Self::CloseBracket => write!(f, "close bracket (])"),
            Self::StringLiteral => write!(f, "string literal"),
            Self::Number => write!(f, "number"),
            Self::True => write!(f, "true"),
            Self::False => write!(f, "false"),
            Self::Null => write!(f, "null"),
            Self::Colon => write!(f, "colon (:)"),
            Self::Comma => write!(f, "comma (,)"),
        }
    }
}

struct JsonToken<'a> {
    token_type: JsonTokenType,
    value: &'a [u8],
}

#[derive(Debug)]
#[derive(PartialEq)]
#[derive(Default)]
pub struct JsonElement {
    pub label: Option<String>,
    pub value: Option<Vec<u8>>,

    // Should we somehow be storing these in vecs for contiguous memory?
    pub first_child: Option<Rc<JsonElement>>,
    pub next_sibling: Option<Rc<JsonElement>>
}

impl JsonElement {
    pub fn get_element(&self, label: &str) -> Option<Rc<JsonElement>> {
        let mut maybe_sub_element = &self.first_child;
        while let Some(sub_element) = maybe_sub_element {
            let sub_element_label = sub_element.label.as_ref().expect("expected sub-element to have a label but it did not");
            if sub_element_label.as_str() == label {
                return Some(Rc::clone(sub_element));
            }

            maybe_sub_element = &sub_element.next_sibling;
        }

        None
    }
}


struct JsonParser<'a> { buffer: &'a [u8], position: usize }
impl<'a> JsonParser<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self { buffer, position: 0 }
    }

    pub fn parse(mut self) -> Result<JsonElement, InvalidJsonError> {
        Self::parse_value(self.buffer, &mut self.position)
    }

    fn parse_value(buffer: &'a [u8], position: &mut usize) -> Result<JsonElement, InvalidJsonError> {
        match Self::lex_next_token(buffer, position)? {
            Some(JsonToken { token_type: JsonTokenType::StringLiteral, value })
            | Some(JsonToken { token_type: JsonTokenType::Number, value })
            | Some(JsonToken { token_type: JsonTokenType::True, value })
            | Some(JsonToken { token_type: JsonTokenType::False, value })
            | Some(JsonToken { token_type: JsonTokenType::Null, value })
            => Ok(JsonElement {
                label: None,
                value: Some(value.to_vec()),
                first_child: None,
                next_sibling: None,
            }),

            Some(JsonToken { token_type: JsonTokenType::OpenBrace, .. }) => Self::parse_object(buffer, position),
            Some(JsonToken { token_type: JsonTokenType::OpenBracket, .. }) => Self::parse_array(buffer, position),

            Some(JsonToken { token_type, .. }) => Err(InvalidJsonError {
                at: *position,
                message: format!(
                    "expected {}, {}, {}, {}, {}, {}, or {} but got {}",
                    JsonTokenType::StringLiteral,
                    JsonTokenType::Number,
                    JsonTokenType::True,
                    JsonTokenType::False,
                    JsonTokenType::Null,
                    JsonTokenType::OpenBrace,
                    JsonTokenType::OpenBracket,
                    token_type
                ),
            }),

            None => Err(InvalidJsonError {
                at: *position,
                message: String::from("unexpected end of JSON input"),
            }),
        }
    }

    /// Recursively parses object elements.
    /// Every element in an object has an explicitly-defined label and value.
    fn parse_object(buffer: &'a [u8], position: &mut usize) -> Result<JsonElement, InvalidJsonError> {
        let mut object = JsonElement::default();
        let mut last_child: Option<Rc<JsonElement>> = None;
        loop {
            // Parse label
            let child_label = match Self::lex_next_token(buffer, position)? {
                Some(JsonToken { token_type: JsonTokenType::StringLiteral, value }) => value,
                Some(JsonToken { token_type, .. }) => return Err(InvalidJsonError {
                    at: *position,
                    message: format!("expected {} but got {}", JsonTokenType::StringLiteral, token_type),
                }),
                None => return Err(InvalidJsonError {
                    at: *position,
                    message: String::from("unexpected end of JSON input"),
                })
            };

            // Parse colon
            match Self::lex_next_token(buffer, position)? {
                Some(JsonToken { token_type: JsonTokenType::Colon, .. }) => {},
                Some(JsonToken { token_type, .. }) => return Err(InvalidJsonError {
                    at: *position,
                    message: format!("expected {} but got {}", JsonTokenType::Colon, token_type),
                }),
                None => return Err(InvalidJsonError {
                    at: *position,
                    message: String::from("unexpected end of JSON input"),
                }),
            };

            let mut child_element = Self::parse_value(buffer, position)?;

            child_element.label = Some(String::from_utf8_lossy(child_label).to_string());
            let child_element = Rc::new(child_element);
            if let Some(last_child) = last_child.as_mut() {
                let last_child = Rc::as_ptr(last_child) as *mut JsonElement;
                unsafe { (*last_child).next_sibling = Some(Rc::clone(&child_element)); }
            } else {
                object.first_child = Some(Rc::clone(&child_element));
            }

            last_child = Some(child_element);

            if Self::container_has_more_values(buffer, position, JsonTokenType::CloseBrace)? {
                continue;
            } else {
                break Ok(object);
            }
        }
    }

    /// Recursively parses array elements.
    /// Array elements have implicitly-defined labels, starting at 0 and monotonically increasing
    /// per element, and explicitly-defined values.
    /// Array elements do NOT have to be of the same type (see JSON spec).
    fn parse_array(buffer: &'a [u8], position: &mut usize) -> Result<JsonElement, InvalidJsonError> {
        let mut array = JsonElement::default();
        let mut last_child: Option<Rc<JsonElement>> = None;

        let mut element_index: usize = 0;
        loop {
            let mut child_element = Self::parse_value(buffer, position)?;

            child_element.label = Some(format!(r#""{}""#, element_index));
            let child_element = Rc::new(child_element);
            if let Some(last_child) = last_child.as_mut() {
                let last_child = Rc::as_ptr(last_child) as *mut JsonElement;
                unsafe { (*last_child).next_sibling = Some(Rc::clone(&child_element)); }
            } else {
                array.first_child = Some(Rc::clone(&child_element));
            }

            last_child = Some(child_element);

            if Self::container_has_more_values(buffer, position, JsonTokenType::CloseBracket)? {
                element_index += 1;
                continue;
            } else {
                break Ok(array);
            }
        }
    }

    fn container_has_more_values(buffer: &'a [u8], position: &mut usize, closing_delimiter: JsonTokenType) -> Result<bool, InvalidJsonError> {
        match Self::lex_next_token(buffer, position)? {
            Some(JsonToken { token_type: JsonTokenType::Comma, .. }) => { Ok(true) },
            Some(JsonToken { token_type, .. }) if token_type == closing_delimiter => { Ok(false) },
            Some(JsonToken { token_type, .. }) => Err(InvalidJsonError {
                at: *position,
                message: format!(
                    "expected {} or {} but got {}",
                    JsonTokenType::Comma,
                    closing_delimiter,
                    token_type
                ),
            }),

            None => Err(InvalidJsonError {
                at: *position,
                message: String::from("unexpected end of JSON input"),
            }),
        }
    }

    fn lex_next_token(buffer: &'a [u8], position: &mut usize) -> Result<Option<JsonToken<'a>>, InvalidJsonError> {
        fn lex_punctuation<'a>(buffer: &'a [u8], position: &mut usize, token_type: JsonTokenType) -> JsonToken<'a> {
            let token = JsonToken { token_type, value: slice::from_ref(&buffer[*position]) };
            *position += 1;
            token
        }

        fn lex_keyword<'a>(buffer: &'a [u8], position: &mut usize, expression: &[u8], token_type: JsonTokenType) -> Result<Option<JsonToken<'a>>, InvalidJsonError> {
            let token_start = *position;
            *position += expression.len(); // advance to just after the last character in the expression
            if *position > buffer.len() {
                return Err(InvalidJsonError {
                    at: token_start,
                    message: format!("expected '{}' but encountered EOF", String::from_utf8_lossy(expression))
                });
            }

            let slice = &buffer[token_start .. *position];
            if slice != expression {
                return Err(InvalidJsonError {
                    at: token_start,
                    message: format!(
                        "expected '{}', received {}",
                        String::from_utf8_lossy(expression),
                        String::from_utf8_lossy(slice)
                    )
                });
            }

            Ok(Some(JsonToken { token_type, value: slice }))
        }

        // skip whitespace
        while Self::is_in_bounds(buffer, position) && buffer[*position].is_ascii_whitespace() {
            *position += 1;
        }

        if !Self::is_in_bounds(buffer, position) { return Ok(None); }

        match buffer[*position] {
            b'[' => Ok(Some(lex_punctuation(buffer, position, JsonTokenType::OpenBracket))),
            b']' => Ok(Some(lex_punctuation(buffer, position, JsonTokenType::CloseBracket))),
            b'{' => Ok(Some(lex_punctuation(buffer, position, JsonTokenType::OpenBrace))),
            b'}' => Ok(Some(lex_punctuation(buffer, position, JsonTokenType::CloseBrace))),
            b':' => Ok(Some(lex_punctuation(buffer, position, JsonTokenType::Colon))),
            b',' => Ok(Some(lex_punctuation(buffer, position, JsonTokenType::Comma))),
            b't' => lex_keyword(buffer, &mut *position, b"true", JsonTokenType::True),
            b'f' => lex_keyword(buffer, &mut *position, b"false", JsonTokenType::False),
            b'n' => lex_keyword(buffer, &mut *position, b"null", JsonTokenType::Null),

            b'-' | b'0' ..= b'9' => {
                let token_start = *position;

                if Self::is_in_bounds(buffer, position) && buffer[*position] == b'-' {
                    *position += 1;
                }

                if Self::is_in_bounds(buffer, position) {
                    if buffer[*position] != b'0' {
                        // advance to decimal point or end of number
                        while Self::is_in_bounds(buffer, position) && buffer[*position].is_ascii_digit() {
                            *position += 1;
                        }
                    } else {
                        *position += 1;
                    }
                }

                if Self::is_in_bounds(buffer, position) && buffer[*position] == b'.' {
                    *position += 1;
                }

                while Self::is_in_bounds(buffer, position) && buffer[*position].is_ascii_digit() {
                    *position += 1;
                }

                if Self::is_in_bounds(buffer, position) && matches!(buffer[*position], b'E' | b'e') {
                    *position += 1;
                }

                if Self::is_in_bounds(buffer, position) && matches!(buffer[*position], b'+' | b'-') {
                    *position += 1;
                }

                while Self::is_in_bounds(buffer, position) && buffer[*position].is_ascii_digit() {
                    *position += 1;
                }

                Ok(Some(JsonToken {
                    token_type: JsonTokenType::Number,
                    value: &buffer[token_start .. *position]
                }))
            },

            // TODO simplify parsing of string literal values.
            // I think we should just assume happy path and then return an error on the next lexing
            // call if the string was malformed e.g. unescaped quote.
            b'"' => {
                let token_start = *position;
                *position += 1;
                loop {
                    if !Self::is_in_bounds(buffer, position) {
                        return Err(InvalidJsonError {
                            at: *position,
                            message: String::from("encountered EOF when parsing string token")
                        });
                    }

                    match buffer[*position] {
                        b'"' => { // FINISH PARSING STRING
                            *position += 1;
                            break;
                        },

                        b'\\' => {
                            *position += 1;
                            if !Self::is_in_bounds(buffer, position) {
                                return Err(InvalidJsonError {
                                    at: *position,
                                    message: String::from("encountered EOF when parsing string token")
                                });
                            }

                            match buffer[*position] {
                                b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't' => *position += 1,
                                b'u' => {
                                    for _ in 0..4 {
                                        *position += 1;
                                        if !Self::is_in_bounds(buffer, position) {
                                            return Err(InvalidJsonError {
                                                at: *position,
                                                message: String::from("encountered EOF when parsing string token")
                                            });
                                        }
                                        if !buffer[*position].is_ascii_hexdigit() {
                                            return Err(InvalidJsonError {
                                                at: *position,
                                                message: format!(
                                                    "invalid escape sequence: expected 4 hex digts, encountered '{}'",
                                                    buffer[*position]
                                                )
                                            });
                                        }
                                    }
                                }
                                _ => return Err(InvalidJsonError {
                                    at: *position,
                                    message: format!("invalid escape sequence: {}", buffer[*position])
                                }),
                            }
                        },

                        _ => *position += 1,
                    }
                }

                Ok(Some(JsonToken {
                    token_type: JsonTokenType::StringLiteral,
                    value: &buffer[token_start .. *position]
                }))
            },

            character => Err(InvalidJsonError {
                at: *position,
                message: format!("unexpected character '{}' encountered", character as char)
            })
        }
    }

    #[inline(always)]
    fn is_in_bounds(buffer: &[u8], position: &usize) -> bool { *position < buffer.len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn element_lookup() {
        let my_str_label = String::from("my_str_label");
        let my_str_val = String::from("Hello World!");
        let object = JsonElement {
            label: None, // top-level objects have no label
            value: None,
            first_child: Some(Rc::new(JsonElement {
                label: Some(String::from("my_num")),
                value: Some(vec![0, 0, 0, 0, 1, 1, 1, 1]),
                first_child: None,
                next_sibling: Some(Rc::new(JsonElement {
                    label: Some(my_str_label.clone()),
                    value: Some(my_str_val.as_bytes().to_vec()),
                    first_child: None,
                    next_sibling: None,
                }))
            })),
            next_sibling: None,
        };

        let maybe_element = object.get_element(&my_str_label);
        assert!(maybe_element.is_some());

        let element = maybe_element.unwrap();
        let obj_element = object.first_child.as_ref().unwrap().next_sibling.as_ref().unwrap();
        assert!(Rc::ptr_eq(&element, obj_element));

        let value = element.value.as_ref().unwrap();
        let string = String::from_utf8_lossy(value);
        assert_eq!(string, my_str_val);
    }

    fn next_token_matches(parser: &mut JsonParser, token_type: JsonTokenType, value: &[u8]) -> bool {
        let result = JsonParser::lex_next_token(parser.buffer, &mut parser.position);
        if let Err(json_error) = result.as_ref() {
            println!(
                "expected token {} with value '{}' but lexer errored: {} at position {}",
                token_type,
                String::from_utf8_lossy(value),
                json_error.message,
                json_error.at,
            );

            return false;
        }

        let token = result.ok().unwrap();
        if token.is_none() {
            println!(
                "expected token {} with value '{}' but lexer returned None",
                token_type,
                String::from_utf8_lossy(value),
            );

            return false;
        }

        let token = token.unwrap();
        let token_types_match = token.token_type == token_type;
        let token_values_match = token.value == value;

        if !token_types_match {
            println!("expected token {} but got token {}", token_type, token.token_type);
        }
        if !token_values_match {
            println!(
                "expected token value '{}' but got '{}'",
                String::from_utf8_lossy(value),
                String::from_utf8_lossy(token.value)
            );
        }

        token_types_match && token_values_match
    }

    fn next_token_is_invalid(parser: &mut JsonParser) -> bool {
        JsonParser::lex_next_token(parser.buffer, &mut parser.position).is_err()
    }

    #[test]
    fn parse_objects() {
        // TEST FLAT OBJECT
        let mut parser = JsonParser::new(br#"{ "s": "world", "number": 12 }"#);
        parser.position += 1; // skip over the opening brace: we already know it's an object
        let object = JsonParser::parse_object(parser.buffer, &mut parser.position)
            .unwrap_or_else(|err| panic!("invalid json at position {}: {}", err.at, err.message));
        let child = object.get_element(r#""s""#).expect("object did not have element with label \"s\"");
        let value = child.value.as_ref().expect("child element with label \"s\" has no value");
        assert_eq!(value, br#""world""#);
        let child = object.get_element(r#""number""#).expect("object did not have element with label \"number\"");
        let value = child.value.as_ref().expect("child element with label \"number\" has no value");
        assert_eq!(value, b"12");

        // TEST NESTED OBJECT
        let mut parser = JsonParser::new(br#"{ "hello": "world", "nested": { "number": 10 } }"#);
        parser.position += 1; // skip over the opening brace: we already know it's an object
        let object = JsonParser::parse_object(parser.buffer, &mut parser.position)
            .unwrap_or_else(|err| panic!("invalid json at position {}: {}", err.at, err.message));
        let object = object.get_element(r#""nested""#).expect("object did not have element with label \"nested\"");
        assert!(object.value.is_none(), "nested object element had a value for some reason");
        let child = object.get_element(r#""number""#).expect("nested object did not have element with label \"number\"");
        let value = child.value.as_ref().expect("child element with label \"number\" has no value");
        assert_eq!(value, b"10");
    }

    #[test]
    fn parse_arrays() {
        let mut parser = JsonParser::new(br#"[ 1, -22.45e10, "hello world", { "bool": true }, null, [ "nested array" ] ]"#);
        parser.position += 1; // skip over opening bracket; we already know it's an array
        let array = JsonParser::parse_array(parser.buffer, &mut parser.position)
            .unwrap_or_else(|err| panic!("invalid json at position {}: {}", err.at, err.message));
        let item = array.get_element(r#""0""#).expect("array did not have element at 0");
        let value = item.value.as_ref().expect("0th element did not have a value");
        assert_eq!(value, b"1");
        let item = array.get_element(r#""1""#).expect("array did not have element at 1");
        let value = item.value.as_ref().expect("element at 1 did not have a value");
        assert_eq!(value, b"-22.45e10");
        let item = array.get_element(r#""2""#).expect("array did not have element at 2");
        let value = item.value.as_ref().expect("element at 2 did not have a value");
        assert_eq!(value, br#""hello world""#);
        let item = array
            .get_element(r#""3""#).expect("array did not have element at 3")
            .get_element(r#""bool""#).expect("nested object in array did not have element named \"bool\"");
        let value = item.value.as_ref().expect("nested object element \"bool\" did not have a value");
        assert_eq!(value, b"true");
        let item = array.get_element(r#""4""#).expect("array did not have element at 4");
        let value = item.value.as_ref().expect("element at 4 did not have a value");
        assert_eq!(value, b"null");
        let item = array
            .get_element(r#""5""#).expect("array did not have element at 5")
            .get_element(r#""0""#).expect("nested array did not have element at 0");
        let value = item.value.as_ref().expect("element at 0 in nested array had no value somehow");
        assert_eq!(value, br#""nested array""#);
    }

    #[test]
    fn parse_simple_values() {
        let mut parser = JsonParser::new(b"false");
        let value = JsonParser::parse_value(parser.buffer, &mut parser.position)
            .unwrap_or_else(|err| panic!("invalid json at position {}: {}", err.at, err.message))
            .value.expect("json element had no value");
        assert_eq!(value, b"false");

        let mut parser = JsonParser::new(b"true");
        let value = JsonParser::parse_value(parser.buffer, &mut parser.position)
            .unwrap_or_else(|err| panic!("invalid json at position {}: {}", err.at, err.message))
            .value.expect("json element had no value");
        assert_eq!(value, b"true");

        let mut parser = JsonParser::new(b"null");
        let value = JsonParser::parse_value(parser.buffer, &mut parser.position)
            .unwrap_or_else(|err| panic!("invalid json at position {}: {}", err.at, err.message))
            .value.expect("json element had no value");
        assert_eq!(value, b"null");

        let mut parser = JsonParser::new(br#""Hello, World!""#);
        let value = JsonParser::parse_value(parser.buffer, &mut parser.position)
            .unwrap_or_else(|err| panic!("invalid json at position {}: {}", err.at, err.message))
            .value.expect("json element had no value");
        assert_eq!(value, br#""Hello, World!""#);

        let mut parser = JsonParser::new(b"-1059.4729887E+744");
        let value = JsonParser::parse_value(parser.buffer, &mut parser.position)
            .unwrap_or_else(|err| panic!("invalid json at position {}: {}", err.at, err.message))
            .value.expect("json element had no value");
        assert_eq!(value, b"-1059.4729887E+744");
    }

    #[test]
    fn lex_string_literals() {
        let string = br#""a""#;
        assert!(next_token_matches(&mut JsonParser::new(string), JsonTokenType::StringLiteral, string));

        let string = br#""Hello World!""#;
        assert!(next_token_matches(&mut JsonParser::new(string), JsonTokenType::StringLiteral, string));

        let string = br#"" \"\\\/\b\f\n\r\t\ufa05 World!""#;
        assert!(next_token_matches(&mut JsonParser::new(string), JsonTokenType::StringLiteral, string));

        let string = br#""bad hex \ufa test""#;
        assert!(next_token_is_invalid(&mut JsonParser::new(string)));

        let string = br#""intentionally missing a quote at the end"#;
        assert!(next_token_is_invalid(&mut JsonParser::new(string)));
    }

    #[test]
    fn lex_punctuation() {
        let mut parser = JsonParser::new(b"[]{}:,");
        assert!(next_token_matches(&mut parser, JsonTokenType::OpenBracket, b"["));
        assert!(next_token_matches(&mut parser, JsonTokenType::CloseBracket, b"]"));
        assert!(next_token_matches(&mut parser, JsonTokenType::OpenBrace, b"{"));
        assert!(next_token_matches(&mut parser, JsonTokenType::CloseBrace, b"}"));
        assert!(next_token_matches(&mut parser, JsonTokenType::Colon, b":"));
        assert!(next_token_matches(&mut parser, JsonTokenType::Comma, b","));
    }

    #[test]
    fn lex_keywords() {
        let mut parser = JsonParser::new(b"truetttt");
        assert!(next_token_matches(&mut parser, JsonTokenType::True, b"true"));
        assert!(next_token_is_invalid(&mut parser));

        let mut parser = JsonParser::new(b"falseffff");
        assert!(next_token_matches(&mut parser, JsonTokenType::False, b"false"));
        assert!(next_token_is_invalid(&mut parser));

        let mut parser = JsonParser::new(b"nullnnnn");
        assert!(next_token_matches(&mut parser, JsonTokenType::Null, b"null"));
        assert!(next_token_is_invalid(&mut parser));
    }

    #[test]
    fn lex_numbers() {
        let mut parser = JsonParser::new(b"100 0.123E+45 -4278.45e12 1.2e+4");
        assert!(next_token_matches(&mut parser, JsonTokenType::Number, b"100"));
        assert!(next_token_matches(&mut parser, JsonTokenType::Number, b"0.123E+45"));
        assert!(next_token_matches(&mut parser, JsonTokenType::Number, b"-4278.45e12"));
        assert!(next_token_matches(&mut parser, JsonTokenType::Number, b"1.2e+4"));
    }
}
