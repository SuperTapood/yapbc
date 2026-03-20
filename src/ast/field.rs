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
}

impl Field {
    pub fn parse(record: Pair<Rule>) -> Field {
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

        Field { index, repeated, default, comments, name: f_name.to_string(), ptype }
    }

    pub(crate) fn capitalize_first(s: &str) -> String {
        s.chars()
            .take(1)
            .flat_map(|f| f.to_uppercase())
            .chain(s.chars().skip(1))
            .collect()
    }
}