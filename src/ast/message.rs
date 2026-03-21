use crate::ast::comments::Comments;
use crate::ast::field::Field;
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
}

impl Message {
    pub fn parse(record: Pair<Rule>) -> Message {
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
                    fields.push(Field::parse(record));
                }
                _ => (),
            }
        }

        Message {
            name,
            fields,
            comments: Comments { comments },
        }
    }
}

#[derive(Debug, Clone)]
pub struct Messages {
    pub package: String,
    pub messages: Vec<Message>,
}

impl Messages {
    pub fn parse(data: String) -> Messages {
        let successful_parse = ProtoParser::parse(Rule::messages, &data)
            .expect("unsuccessful parse")
            .next()
            .unwrap();

        let inner = successful_parse.into_inner();

        let mut messages = Vec::new();
        let mut package = String::new();
        let mut found_message = false;

        for record in inner {
            match record.as_rule() {
                Rule::message => {
                    messages.push(Message::parse(record));
                    found_message = true;
                }
                Rule::package => {
                    package = record.into_inner().as_str().to_string();
                }
                _ => {
                    panic!("{}", format!("unrecognised rule: {:?}", record.as_rule()))
                }
            }
        }

        if !found_message {
            println!("no messages found");
            exit(1);
        }

        Messages { package, messages }
    }
}
