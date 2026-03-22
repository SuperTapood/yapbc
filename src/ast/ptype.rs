use crate::ast::message::Rule;
use crate::ast::ptype::PType::{RepeatedCustom, RepeatedInt32, RepeatedPString};
use pest::iterators::Pair;

#[derive(Debug, Clone, PartialEq)]
pub enum PType {
    Int32,
    RepeatedInt32,
    PString,
    RepeatedPString,
    Custom(String),
    RepeatedCustom(String),
}

impl PType {
    pub fn parse(record: Pair<Rule>) -> PType {
        let type_str = record.as_str();
        match type_str {
            "int32" => PType::Int32,
            "string" => PType::PString,
            other => PType::Custom(other.to_string()),
        }
    }

    pub fn repeat(&self) -> PType {
        match self {
            PType::Int32 => RepeatedInt32,
            PType::PString => RepeatedPString,
            PType::Custom(other) => RepeatedCustom(other.to_string()),
            _ => {
                panic!("Unknown repeated type {:?}", self)
            }
        }
    }

    pub fn to_string(&self) -> String {
        String::from(match &self {
            PType::Int32 => "int32",
            RepeatedInt32 => "int32",
            PType::PString => "string",
            RepeatedPString => "string",
            PType::Custom(n) => n,
            RepeatedCustom(n) => n,
        })
    }

    pub fn is_nested(&self) -> bool {
        match self {
            PType::RepeatedCustom(_) => true,
            PType::Custom(_) => true,
            _ => false,
        }
    }
}
