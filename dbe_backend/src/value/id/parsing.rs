use crate::etype::econst::ETypeConst;
use crate::etype::EDataType;
use crate::value::id::editor_id::EditorId;
use crate::value::id::ETypeId;
use logos::{Lexer, Logos};
use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug)]
pub enum ParsedId {
    Simple(EDataType),
    GenericObject {
        ident: ETypeId,
        generics: Vec<(String, ParsedId)>,
    },
    List(Box<ParsedId>),
    Map {
        key: Box<ParsedId>,
        value: Box<ParsedId>,
    },
}

pub fn parse_full_id(id: &str) -> Result<ParsedId, IdParseError> {
    let mut lex = PeekableLexer::new(Token::lexer(id));
    let ty = parse_type(&mut lex)?;

    match lex.next() {
        None => Ok(ty),
        Some(Err(())) => Err(IdParseError::BadToken(lex.slice().to_string())),
        Some(Ok(_)) => Err(IdParseError::UnexpectedToken(
            lex.slice().to_string(),
            "end of input",
        )),
    }
}

/// Normalizes an identifier, stripping redundant whitespaces
pub fn normalize_id(id: &str) -> String {
    let mut lex = Token::lexer(id);
    let mut result = String::new();
    while lex.next().is_some() {
        result.push_str(lex.slice());
    }

    result
}

/// Escapes a string for the standard display
pub fn escape_const_string(str: &str) -> String {
    str.replace("'", "''")
}

pub fn unescape_const_string(str: &str) -> String {
    let str = str.strip_prefix("'").unwrap().strip_suffix("'").unwrap();
    str.replace("''", "'")
}

#[derive(Debug, Clone, Error, Diagnostic)]
pub enum IdParseError {
    #[error("unexpected end of input")]
    UnexpectedEnd,
    #[error("unexpected token {0}, expected {1}")]
    UnexpectedToken(String, &'static str),
    #[error("invalid input: `{0}`")]
    BadToken(String),
}

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")]
enum Token {
    #[regex("[a-zA-Z0-9_]+:[a-zA-Z0-9/_]+", |lex| EditorId::parse(lex.slice()).unwrap())]
    Id(EditorId),
    #[token("number")]
    Number,
    #[token("string")]
    String,
    #[token("boolean")]
    Boolean,
    #[token("unknown")]
    Unknown,
    #[token("null")]
    Null,
    #[token("List")]
    List,
    #[token("Map")]
    Map,
    #[token("Item")]
    Item,
    #[token("Key")]
    Key,
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    OtherAlphanumeric,
    #[regex(r"[-+]?(?:0|[1-9]\d*)(?:\.\d*)?(?:[eE][-+]?\d+)?", |lex| lex.slice().parse::<f64>().unwrap()
    )]
    ConstNumber(f64),
    #[regex(r"'(?:[^']*(?:''[^']*)*)'", |lex| unescape_const_string(lex.slice()))]
    ConstString(String),
    #[regex(r"true|false", |lex| lex.slice().parse::<bool>().unwrap())]
    ConstBool(bool),
    #[token("<")]
    OpenAngle,
    #[token(">")]
    CloseAngle,
    #[token("=")]
    Equals,
    #[token(",")]
    Comma,
}

impl Token {
    fn expected_str(&self) -> &'static str {
        match self {
            Token::Id(_) => "identifier",
            Token::Number => "number",
            Token::String => "string",
            Token::Boolean => "boolean",
            Token::Unknown => "unknown",
            Token::Null => "null",
            Token::List => "List",
            Token::Map => "Map",
            Token::ConstNumber(_) => "constant number",
            Token::ConstString(_) => "constant string",
            Token::ConstBool(_) => "constant boolean",
            Token::OpenAngle => "`<` symbol",
            Token::CloseAngle => "`>` symbol",
            Token::Equals => "`=` symbol",
            Token::Comma => "`,` symbol",
            Token::OtherAlphanumeric => "alphanumeric identifier",
            Token::Item => "`Item` identifier",
            Token::Key => "`Key` identifier",
        }
    }

    fn is_alphanumeric(&self) -> bool {
        matches!(
            self,
            Token::Number
                | Token::String
                | Token::Boolean
                | Token::Unknown
                | Token::Null
                | Token::List
                | Token::Map
                | Token::Item
                | Token::Key
                | Token::OtherAlphanumeric
        )
    }

    fn expect_same(&self, self_slice: &str, expected: Token) -> Result<(), IdParseError> {
        if self == &expected {
            Ok(())
        } else {
            Err(IdParseError::UnexpectedToken(
                self_slice.to_string(),
                expected.expected_str(),
            ))
        }
    }
}

fn parse_type(tokens: &mut PeekableLexer) -> Result<ParsedId, IdParseError> {
    fn next_token_opt(tokens: &mut PeekableLexer) -> Result<Option<Token>, IdParseError> {
        tokens
            .next()
            .transpose()
            .map_err(|()| IdParseError::BadToken(tokens.slice().to_string()))
    }

    fn peek_token_opt<'a>(
        tokens: &'a mut PeekableLexer<'_>,
    ) -> Result<Option<&'a Token>, IdParseError> {
        let slice = tokens.peek_slice();
        match tokens.peek() {
            None => Ok(None),
            Some(Err(())) => Err(IdParseError::BadToken(slice.to_string())),
            Some(Ok(token)) => Ok(Some(token)),
        }
    }

    fn next_token(tokens: &mut PeekableLexer) -> Result<Token, IdParseError> {
        next_token_opt(tokens)?.ok_or(IdParseError::UnexpectedEnd)
    }

    fn expect_next(tokens: &mut PeekableLexer, expected: Token) -> Result<(), IdParseError> {
        next_token(tokens)?.expect_same(tokens.slice(), expected)
    }

    fn parse_generics(tokens: &mut PeekableLexer) -> Result<Vec<(String, ParsedId)>, IdParseError> {
        let mut generics = Vec::new();
        let next = peek_token_opt(tokens)?;
        match next {
            None => {
                return Ok(generics);
            }
            Some(token) => {
                match token {
                    Token::CloseAngle | Token::Comma => {
                        return Ok(generics);
                    }
                    Token::OpenAngle => {
                        // continue
                    }
                    _ => {
                        return Err(IdParseError::UnexpectedToken(
                            tokens.peek_slice().to_string(),
                            "generic arguments",
                        ));
                    }
                }
            }
        }
        loop {
            let ident = next_token(tokens)?;
            if !ident.is_alphanumeric() {
                return Err(IdParseError::UnexpectedToken(
                    tokens.slice().to_string(),
                    "alphanumeric identifier",
                ));
            }
            let identifier = tokens.slice();

            expect_next(tokens, Token::Equals)?;

            let ty = parse_type(tokens)?;
            generics.push((identifier.to_string(), ty));

            let next = next_token(tokens)?;

            match next {
                Token::CloseAngle => {
                    return Ok(generics);
                }
                Token::Comma => {
                    // continue
                }
                _ => {
                    return Err(IdParseError::UnexpectedToken(
                        tokens.slice().to_string(),
                        "`,` or `>`",
                    ))
                }
            }
        }
    }

    match next_token(tokens)? {
        Token::Id(id) => {
            let generics = parse_generics(tokens)?;

            if generics.is_empty() {
                return Ok(ParsedId::Simple(EDataType::Object { ident: ETypeId(id) }));
            }

            Ok(ParsedId::GenericObject {
                ident: ETypeId(id),
                generics,
            })
        }
        Token::Map => {
            expect_next(tokens, Token::OpenAngle)?;
            expect_next(tokens, Token::Key)?;
            expect_next(tokens, Token::Equals)?;
            let key = Box::new(parse_type(tokens)?);
            expect_next(tokens, Token::Comma)?;
            expect_next(tokens, Token::Item)?;
            expect_next(tokens, Token::Equals)?;
            let value = Box::new(parse_type(tokens)?);
            expect_next(tokens, Token::CloseAngle)?;
            Ok(ParsedId::Map { key, value })
        }
        Token::List => {
            expect_next(tokens, Token::OpenAngle)?;
            expect_next(tokens, Token::Item)?;
            expect_next(tokens, Token::Equals)?;
            let ty = Box::new(parse_type(tokens)?);
            expect_next(tokens, Token::CloseAngle)?;
            Ok(ParsedId::List(ty))
        }
        Token::Number => Ok(ParsedId::Simple(EDataType::Number)),
        Token::String => Ok(ParsedId::Simple(EDataType::String)),
        Token::Boolean => Ok(ParsedId::Simple(EDataType::Boolean)),
        Token::Unknown => Ok(ParsedId::Simple(EDataType::Unknown)),
        Token::Null => Ok(ParsedId::Simple(EDataType::null())),
        Token::ConstNumber(num) => Ok(ParsedId::Simple(EDataType::Const { value: num.into() })),
        Token::ConstString(value) => Ok(ParsedId::Simple(EDataType::Const {
            value: ETypeConst::String(value.into()),
        })),
        Token::ConstBool(value) => Ok(ParsedId::Simple(EDataType::Const {
            value: value.into(),
        })),
        Token::OpenAngle
        | Token::CloseAngle
        | Token::Equals
        | Token::Comma
        | Token::Item
        | Token::Key
        | Token::OtherAlphanumeric => Err(IdParseError::UnexpectedToken(
            tokens.slice().to_string(),
            "type or id",
        )),
    }
}

struct PeekableLexer<'a> {
    lexer: Lexer<'a, Token>,
    peek_state: Option<PeekState<'a>>,
}

struct PeekState<'a> {
    next: Option<Result<Token, ()>>,
    cur_slice: &'a str,
}

impl<'a> PeekableLexer<'a> {
    pub fn new(lexer: Lexer<'a, Token>) -> Self {
        Self {
            lexer,
            peek_state: None,
        }
    }

    pub fn slice(&mut self) -> &'a str {
        if let Some(state) = self.peek_state.as_mut() {
            state.cur_slice
        } else {
            self.lexer.slice()
        }
    }

    pub fn peek(&mut self) -> Option<&Result<Token, ()>> {
        self.ensure_peeked();

        self.peek_state.as_ref().unwrap().next.as_ref()
    }

    pub fn peek_slice(&mut self) -> &'a str {
        self.ensure_peeked();

        self.lexer.slice()
    }

    fn ensure_peeked(&mut self) {
        if self.peek_state.is_none() {
            self.peek_state = Some(PeekState {
                cur_slice: self.lexer.slice(),
                next: self.lexer.next(),
            });
        }
    }
}

impl<'a> Iterator for PeekableLexer<'a> {
    type Item = Result<Token, ()>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(state) = self.peek_state.take() {
            return state.next;
        }
        self.lexer.next()
    }
}
