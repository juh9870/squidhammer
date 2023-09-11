use crate::value::etype::registry::ETypetId;
use crate::value::etype::{EDataType, ETypeConst};
use crate::value::ENumber;
use anyhow::anyhow;
use itertools::Itertools;
use logos::{Lexer, Logos};
use thiserror::Error;

#[derive(Debug, Clone, Default, PartialEq, Error)]
#[error("Something gone extremely wrong during type parsing")]
struct ParsingError;

fn string_literal<'a>(lex: &mut Lexer<'a, TypeToken<'a>>) -> Option<&'a str> {
    let sliced = lex.slice();

    Some(&sliced[1..(sliced.len() - 1)])
}

#[derive(Logos, Debug)]
#[logos(skip r"[ \t\n\f]+")]
#[logos(error = ParsingError)]
enum TypeToken<'a> {
    #[token("boolean")]
    Boolean,
    #[token("number")]
    Number,
    #[token("string")]
    String,
    #[token("vec2")]
    Vec2,
    #[regex("[\\S:]+:[\\S:]+")]
    TypeIdentifier(&'a str),
    #[regex(r"-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?", |lex| lex.slice().parse::<ENumber>().unwrap())]
    NumericConstant(ENumber),
    #[regex("true|false", |lex| lex.slice() == "true")]
    BooleanConstant(bool),
    #[regex("'[^']+'", string_literal)]
    StringConstant(&'a str),
}

pub fn parse_type_string(input: &str) -> anyhow::Result<EDataType> {
    let data = TypeToken::lexer(input)
        .exactly_one()
        .map_err(|err| anyhow!("{err}"))?
        .map_err(|err| anyhow!("{err}"))?;

    Ok(match data {
        TypeToken::Boolean => EDataType::Boolean,
        TypeToken::Number => EDataType::Scalar,
        TypeToken::String => EDataType::String,
        TypeToken::Vec2 => EDataType::Vec2,
        TypeToken::TypeIdentifier(ty) => {
            let ty = ETypetId::parse(ty)?;
            EDataType::Object { ident: ty }
        }
        TypeToken::NumericConstant(num) => EDataType::Const {
            value: ETypeConst::Scalar(num),
        },
        TypeToken::BooleanConstant(value) => EDataType::Const {
            value: ETypeConst::Boolean(value),
        },
        TypeToken::StringConstant(str) => EDataType::Const {
            value: ETypeConst::String(str.into()),
        },
    })
}
