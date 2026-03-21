use crate::ast::message::Rule;
use pest::iterators::Pairs;
#[derive(Debug, Clone)]
pub struct Comments {
    pub comments: Vec<String>,
}

impl Comments {
    pub fn parse(mut records: Pairs<Rule>) -> (Comments, Pairs<Rule>) {
        let mut comments = Vec::new();
        let mut value = records.peek().unwrap();
        loop {
            if value.as_rule() != Rule::line_comment {
                break;
            }
            value = records.next().unwrap();
            if let Some((_prefix, remainder)) = value.as_str().split_once(' ') {
                comments.push(remainder.parse().unwrap());
            } else {
                comments.push(value.as_str().parse().unwrap());
            }
            value = records.peek().unwrap();
        }

        (Comments { comments }, records)
    }
}
