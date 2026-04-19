use crate::ast::comments::Comments;
use crate::ast::message::Rule;
use crate::ast::ptype::PType;
use pest::iterators::Pair;

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ptype: PType,
    pub comments: Comments,
    // pub optional: bool,
    pub repeated: bool,
    pub index: usize,
    pub default: Option<String>,
    pub maybe_types: Option<Vec<Field>>,
}

impl Field {
    fn parse_regular(record: Pair<Rule>) -> Field {
        let inner = record.into_inner();
        let (comments, mut record) = Comments::parse(inner);

        let mut next = record.next().unwrap();
        let mut repeated = false;
        let mut default = None;

        if next.as_rule() == Rule::optional {
            next = record.next().unwrap();
            default = Some("PLACEHOLDER".to_string());
        }

        let ptype = if next.as_rule() == Rule::repeated {
            next = record.next().unwrap();
            repeated = true;
            PType::parse(next).repeat()
        } else {
            PType::parse(next)
        };

        let f_name = record.next().unwrap().as_str();

        next = record.next().unwrap();

        if next.as_rule() == Rule::default {
            default = Some(next.into_inner().as_str().parse().unwrap());
            next = record.next().unwrap();
        } else if default.is_some() {
            default = Some(ptype.default_python());
        }

        let index = next.as_str().parse::<usize>().unwrap();

        Field {
            index,
            repeated,
            default,
            comments,
            name: f_name.to_string(),
            ptype,
            maybe_types: None,
        }
    }

    fn parse_oneof(record: Pair<Rule>) -> Field {
        let inner = record.into_inner();
        let (comments, mut record) = Comments::parse(inner);

        let next = record.next().unwrap();

        let name = next.as_str();
        let mut maybe_types = Vec::new();
        loop {
            let maybe_next = record.next();
            if maybe_next.is_none() {
                break;
            }
            let next = maybe_next.unwrap();
            maybe_types.push(Field::parse_regular(next));
        }

        Field {
            index: 0,
            repeated: false,
            default: None,
            comments,
            name: name.parse().unwrap(),
            ptype: PType::Oneof,
            maybe_types: Some(maybe_types),
        }
    }
    pub fn parse(record: Pair<Rule>) -> Field {
        match record.as_rule() {
            Rule::regular_field => Field::parse_regular(record),
            Rule::oneof_field => Field::parse_oneof(record),
            _ => unreachable!()
        }
    }
}
