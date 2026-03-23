use crate::ast::comments::Comments;
use crate::ast::message::Rule;
use pest::iterators::Pair;

#[derive(Debug, Clone)]
pub struct PEnumField {
    pub name: String,
    pub index: usize,
    pub comments: Comments,
}

impl PEnumField {
    pub fn parse(record: Pair<Rule>) -> PEnumField {
        let inner = record.into_inner();
        let (comments, mut record) = Comments::parse(inner);

        let f_name = record.next().unwrap().as_str();

        let next = record.next().unwrap();

        let index = next.as_str().parse::<usize>().unwrap();

        PEnumField {
            index,
            name: f_name.to_string(),
            comments,
        }
    }
}