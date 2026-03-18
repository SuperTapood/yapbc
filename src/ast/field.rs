use crate::ast::comments::Comments;
use crate::ast::message::Rule;
use crate::ast::ptype::PType;
use pest::iterators::Pair;

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ptype: PType,
    pub comments: Comments,
    pub optional: bool,
    pub repeated: bool,
    pub index: usize,
}

impl Field {
    pub fn parse(record: Pair<Rule>) -> Field {
        let inner = record.into_inner();
        let (comments, mut record) = Comments::parse(inner);

        let mut next = record.next().unwrap();
        let mut optional = false;
        let mut repeated = false;

        if next.as_rule() == Rule::optional {
            next = record.next().unwrap();
            optional = true;
        }

        let ptype = if next.as_rule() == Rule::repeated {
            next = record.next().unwrap();
            repeated = true;
            PType::parse(next).repeat()
        } else {
            PType::parse(next)
        };


        let f_name = record.next().unwrap().as_str();

        let index = record.next().unwrap().as_str().parse::<usize>().unwrap();

        Field { index, repeated, optional, comments, name: f_name.to_string(), ptype }
    }

    fn capitalize_first(s: &str) -> String {
        s.chars()
            .take(1)
            .flat_map(|f| f.to_uppercase())
            .chain(s.chars().skip(1))
            .collect()
    }

    pub fn python_compile(&self) -> String {
        let mut field_code = String::new();
        let (py_type, msg_type) = self.ptype.python_compile();
        field_code.push_str(
            &format!("{}: {} = betterproto2.field({}, betterproto2.{}, repeated={}, optional={})",
                     self.name,
                     py_type,
                self.index,
                     msg_type,
                Self::capitalize_first(self.repeated.to_string().as_str()),
                     Self::capitalize_first(self.optional.to_string().as_str())));

        field_code.push_str(format!("\n{}", self.comments.python_compile().as_str()).as_str());

        field_code
    }
}