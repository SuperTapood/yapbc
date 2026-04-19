use crate::ast::comments::Comments;
use crate::ast::field::Field;
use crate::ast::penum::PEnum;
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;
use std::process::exit;

#[derive(Parser)]
#[grammar = "proto.pest"]
pub struct ProtoParser;

#[derive(Debug, Clone)]
pub struct Message {
    pub name: String,
    pub fields: Vec<Field>,
    pub comments: Comments,
    pub index: usize,
}

impl Message {
    pub fn parse(record: Pair<Rule>, index: usize) -> Message {
        let mut name = String::new();
        let mut fields = Vec::new();
        let mut comments = Vec::new();
        for record in record.into_inner() {
            match record.as_rule() {
                Rule::ident => name = record.as_str().to_string(),
                Rule::line_comment => {
                    if let Some((_prefix, remainder)) = record.as_str().split_once(' ') {
                        comments.push(remainder.parse().unwrap());
                    } else {
                        comments.push(record.as_str().parse().unwrap());
                    }
                }
                Rule::field => {
                    let actual = record.clone().into_inner().next().unwrap();
                    fields.push(Field::parse(actual));
                }
                other => {
                    panic!("uknown rule {:#?}", other);
                }
            }
        }

        Message {
            name,
            fields,
            comments: Comments { comments },
            index,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Messages {
    pub package: String,
    pub messages: Vec<Message>,
    pub penums: Vec<PEnum>,
    pub imports: Vec<String>,
}

impl Messages {
    pub fn parse(data: String, file: String) -> Messages {
        let successful_parse = ProtoParser::parse(Rule::messages, &data)
            .expect("unsuccessful parse")
            .next()
            .unwrap();

        let inner = successful_parse.into_inner();

        let mut messages = Vec::new();
        let mut penums = Vec::new();
        let mut imports = Vec::new();
        let mut package = String::new();
        let mut object_counter = 0;

        for record in inner {
            match record.as_rule() {
                Rule::objects => {
                    let actual = record.clone().into_inner().next().unwrap();
                    match actual.as_rule() {
                        Rule::message => {
                            messages.push(Message::parse(actual, object_counter));
                            object_counter = object_counter + 1;
                        }
                        Rule::enumeration => {
                            penums.push(PEnum::parse(actual, object_counter));
                            object_counter = object_counter + 1;
                        }
                        _ => panic!("we should not hit this")
                    }
                }
                Rule::package => {
                    package = record.into_inner().as_str().to_string();
                }
                Rule::import => {
                    imports.push(record.into_inner().as_str().to_string());
                }
                _ => {
                    panic!("{}", format!("unrecognised rule: {:?}", record.as_rule()))
                }
            }
        }

        if object_counter == 0 {
            println!("no messages/enums found in file {file}");
            exit(1);
        }

        Messages { package, messages, penums, imports }
    }
}
