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
            PType::Int32 => { RepeatedInt32 }
            PType::PString => { RepeatedPString }
            PType::Custom(other) => { RepeatedCustom(other.to_string()) }
            _ => { panic!("Unknown repeated type {:?}", self) }
        }
    }


}