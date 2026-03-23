use crate::ast::comments::Comments;
use crate::ast::message::Rule;
use crate::ast::penum_field::PEnumField;
use pest::iterators::Pair;

#[derive(Debug, Clone)]
pub struct PEnum {
    pub name: String,
    pub fields: Vec<PEnumField>,
    pub comments: Comments,
    pub index: usize,
}


impl PEnum {
    pub fn parse(record: Pair<Rule>, index: usize) -> PEnum {
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
                Rule::enum_field => {
                    fields.push(PEnumField::parse(record));
                }
                _ => (),
            }
        }

        PEnum {
            name,
            fields,
            comments: Comments { comments },
            index,
        }
    }
}