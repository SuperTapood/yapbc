use crate::ast::comments::Comments;
use crate::ast::field::Field;
use crate::ast::message::{Message, Messages};
use crate::ast::penum::PEnum;
use crate::ast::penum_field::PEnumField;
use crate::ast::ptype::PType;
use crate::ast::ptype::PType::{RepeatedCustom, RepeatedInt32, RepeatedPString};
use crate::util::capitalize_first;

impl PType {
    pub fn compile_python(&self) -> (String, String) {
        match &self {
            PType::Int32 => ("int".to_string(), "TYPE_INT32".to_string()),
            RepeatedInt32 => ("List[int]".to_string(), "TYPE_INT32".to_string()),
            PType::PString => ("str".to_string(), "TYPE_STRING".to_string()),
            RepeatedPString => ("List[str]".to_string(), "TYPE_STRING".to_string()),
            PType::Custom(n) => (format!("\"{n}\""), "TYPE_MESSAGE".to_string()),
            RepeatedCustom(n) => (format!("List[\"{n}\"]"), "TYPE_MESSAGE".to_string()),
            PType::Oneof => ("oneof".to_string(), "TYPE_ONEOF".to_string()),
        }
    }

    pub fn default_python(&self) -> String {
        match &self {
            PType::Int32 => "0".to_string(),
            RepeatedInt32 => "None".to_string(),
            PType::PString => "None".to_string(),
            RepeatedPString => "None".to_string(),
            PType::Custom(_) => "None".to_string(),
            RepeatedCustom(_) => "None".to_string(),
            PType::Oneof => "None".to_string(),
        }
    }
}

impl Comments {
    pub fn compile_python(&self) -> String {
        let mut out = String::new();
        if self.comments.is_empty() {
            return out;
        }
        out.push_str("    \"\"\"\n");

        for comment in &self.comments {
            out.push_str(format!("    {}\n", comment).as_str());
        }

        out.push_str("    \"\"\"\n\n");

        out
    }

    pub fn oneliner_python(&self) -> String {
        let mut out = String::new();
        if self.comments.is_empty() {
            return String::from("-");
        }
        for comment in &self.comments {
            out.push_str(format!("{} ", comment).as_str());
        }

        out
    }
}

impl PEnumField {
    pub fn compile_python(&self) -> String {
        let name = &self.name;
        let index = self.index;
        let comments = self.comments.compile_python();

        format!("{name} = {index}\n{comments}")
    }
}

impl Field {
    fn compile_oneof(&self) -> (String, String, String) {
        let mut field_code = String::new();
        let mut parameters = String::from("");
        let types = self.maybe_types.clone().unwrap();

        for field in types {
            let (mut py_type, act_type) = field.ptype.compile_python();
            if field.default.is_some() {
                py_type = format!("Optional[{}]", py_type);
            }
            field_code.push_str(&format!("{}: \"{} | None\"= betterproto2.field({}, betterproto2.{act_type}, optional=True, group=\"{}\")",
                                         field.name,
                                         py_type.replace("\"", ""),
                                         field.index,
                                         self.name,
            ));
            field_code
                .push_str(format!("\n    {}", self.comments.compile_python().as_str()).as_str());

            parameters.push_str(&format!(
                "\n        {}: \"{} | None\" = None,",
                field.name,
                py_type.replace("\"", "")
            ));
        }

        parameters.pop();

        (field_code, parameters, self.comments.oneliner_python())
    }
    pub fn compile_python(&self) -> (String, String, String) {
        if self.ptype == PType::Oneof {
            return self.compile_oneof();
        }
        let mut field_code = String::new();
        let (mut py_type, msg_type) = self.ptype.compile_python();
        let mut is_default = false;
        if self.default.is_some() {
            is_default = true;
            py_type = format!("Optional[{}]", py_type);
        }
        field_code.push_str(&format!(
            "{}: {} = betterproto2.field({}, betterproto2.{}, repeated={}, optional={})",
            self.name,
            py_type,
            self.index,
            msg_type,
            capitalize_first(self.repeated.to_string().as_str()),
            capitalize_first(is_default.to_string().as_str())
        ));

        field_code.push_str(format!("\n{}", self.comments.compile_python().as_str()).as_str());

        (
            field_code,
            format!("{}: {}", self.name, py_type),
            self.comments.oneliner_python(),
        )
    }
}

impl PEnum {
    pub fn compile_python(&self) -> String {
        let name = &self.name;
        let mut fields = String::new();

        for field in &self.fields {
            fields.push_str(format!("    {}", field.compile_python()).as_str());
        }

        let comments = self.comments.compile_python();

        format!(
            "\
class {name}(betterproto2.Enum):
{comments}{fields}
"
        )
    }
}

impl Message {
    pub fn compile_python(&self) -> String {
        let mut code = String::new();
        let mut variables_init = String::new();
        let mut variables_comment = String::new();
        let mut variables_with_default_init = String::new();
        let mut variables_with_default_comment = String::new();
        let mut assignment = String::new();
        for f in &self.fields {
            let (preinit, init, comment) = f.compile_python();
            if f.maybe_types.is_none() {
                if f.default.is_some() {
                    let mut def = f.default.clone().unwrap();
                    if def == "PLACEHOLDER" {
                        def = f.ptype.default_python();
                    }
                    variables_with_default_init
                        .push_str(format!("        {} = {}, \n", init, def).as_str());
                    variables_with_default_comment.push_str(
                        format!("        :param {}: {} \n", f.name.clone(), comment).as_str(),
                    );
                } else {
                    variables_init.push_str(format!("        {}, \n", init).as_str());
                    variables_comment.push_str(
                        format!("        :param {}: {} \n", f.name.clone(), comment).as_str(),
                    );
                }
                code.push_str(format!("    {}", preinit).as_str());
                assignment.push_str(format!("        self.{} = {}\n", f.name, f.name).as_str());
            } else {
                variables_with_default_init.push_str(format!("        {}, \n", init,).as_str());
                code.push_str(format!("    {}", preinit).as_str());
                for maybe_field in f.maybe_types.clone().unwrap() {
                    assignment.push_str(
                        format!("        self.{} = {}\n", maybe_field.name, maybe_field.name)
                            .as_str(),
                    );
                }
            }
        }

        format!(
            "@dataclass(
    kw_only=True,
    init=False,
    eq=False,
    repr=False,
)
class {}(betterproto2.Message):
{}{}
    def __init__(
        self,
        *,
{}{}    ):
        \"\"\"\n{variables_comment}{variables_with_default_comment}        \"\"\"
{assignment}
        self._unknown_fields = b\"\"
        ",
            self.name,
            self.comments.compile_python(),
            code,
            variables_init,
            variables_with_default_init
        )
    }
}
impl Messages {
    pub fn compile_python(&self) -> String {
        let mut code = String::new();
        // for message in &self.messages {
        //     code.push_str(format!("    \"{}\",\n", message.name).as_str());
        // }
        //
        // code.push_str(")\n\n");

        for penum in self.penums.iter() {
            code.push_str(format!("{}\n", penum.compile_python()).as_str());
        }

        for message in self.messages.iter() {
            code.push_str(format!("{}\n", message.compile_python()).as_str());
            code.push_str(
                format!(
                    "default_message_pool.register_message(\"\", \"{}\", {})\n\n",
                    message.name, message.name
                )
                .as_str(),
            );
        }

        code
    }
}
