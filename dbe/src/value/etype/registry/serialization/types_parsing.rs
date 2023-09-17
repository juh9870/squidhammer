use crate::value::etype::registry::ETypetId;
use crate::value::etype::{EDataType, ETypeConst};
use crate::value::ENumber;
use anyhow::anyhow;
use itertools::Itertools;
use logos::{Lexer, Logos};
use thiserror::Error;

#[derive(Debug, Clone, Default, PartialEq, Error)]
#[error("Failed to parse type string")]
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
    #[regex(r"[^\s:]+:[^\s:]+")]
    TypeIdentifier(&'a str),
    #[regex(r"-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?", | lex | lex.slice().parse::< ENumber > ().unwrap())]
    NumericConstant(ENumber),
    #[regex("true|false", | lex | lex.slice() == "true")]
    BooleanConstant(bool),
    #[regex("'[^']+'", string_literal)]
    StringConstant(&'a str),
}

pub fn parse_type_string(input: &str) -> anyhow::Result<EDataType> {
    let data = TypeToken::lexer(input)
        .exactly_one()
        .map_err(|err| anyhow!("{err}"))?
        .map_err(|err| anyhow!("{err}: `{input}`"))?;

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
            value: ETypeConst::Scalar(num.into()),
        },
        TypeToken::BooleanConstant(value) => EDataType::Const {
            value: ETypeConst::Boolean(value),
        },
        TypeToken::StringConstant(str) => EDataType::Const {
            value: ETypeConst::String(str.into()),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::parse_type_string;
    use super::EDataType;
    use crate::value::etype::registry::ETypetId;
    use rstest::rstest;

    #[test]
    fn should_parse_string() {
        assert_eq!(
            parse_type_string("string").expect("Should parse"),
            EDataType::String
        )
    }

    #[test]
    fn should_parse_boolean() {
        assert_eq!(
            parse_type_string("boolean").expect("Should parse"),
            EDataType::Boolean
        )
    }

    #[test]
    fn should_parse_number() {
        assert_eq!(
            parse_type_string("number").expect("Should parse"),
            EDataType::Scalar
        )
    }

    #[test]
    fn should_parse_vec2() {
        assert_eq!(
            parse_type_string("vec2").expect("Should parse"),
            EDataType::Vec2
        )
    }

    #[rstest]
    #[case("eh:objects/faction")]
    #[case("some_long_namespace:this_is_valid_name")]
    fn should_parse_type_id(#[case] id: &str) {
        let type_id = ETypetId::parse(id).expect("Should be a valid ID");
        assert_eq!(
            parse_type_string(id).expect("Should parse"),
            EDataType::Object { ident: type_id }
        )
    }
}
