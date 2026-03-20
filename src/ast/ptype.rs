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

    pub fn python_compile(&self) -> (String, String) {
        match &self {
            PType::Int32 => ("int".to_string(), "TYPE_INT32".to_string()),
            RepeatedInt32 => ("List[int]".to_string(), "TYPE_INT32".to_string()),
            PType::PString => ("str".to_string(), "TYPE_STRING".to_string()),
            RepeatedPString => ("List[str]".to_string(), "TYPE_STRING".to_string()),
            PType::Custom(n) => (n.to_string(), "TYPE_MESSAGE".to_string()),
            RepeatedCustom(n) => (format!("List[{}]", n), "TYPE_MESSAGE".to_string()),
        }
    }

    pub fn python_default(&self) -> String {
        match &self {
            PType::Int32 => "0".to_string(),
            RepeatedInt32 => "None".to_string(),
            PType::PString => "None".to_string(),
            RepeatedPString => "None".to_string(),
            PType::Custom(_) => "None".to_string(),
            RepeatedCustom(_) => "None".to_string(),
        }
    }
}