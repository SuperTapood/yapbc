use crate::ast::comments::Comments;
use crate::ast::field::Field;
use crate::ast::message::{Message, Messages};
use crate::ast::penum::PEnum;
use crate::ast::penum_field::PEnumField;
use crate::ast::ptype::PType;
use crate::ast::ptype::PType::{RepeatedCustom, RepeatedInt32, RepeatedPString};
use crate::util::capitalize_first;
use std::fs;
use std::path::PathBuf;

impl PType {
    pub fn compile_python(&self) -> (String, String) {
        match &self {
            PType::Int32 => ("int".to_string(), "TYPE_INT32".to_string()),
            RepeatedInt32 => ("List[int]".to_string(), "TYPE_INT32".to_string()),
            PType::PString => ("str".to_string(), "TYPE_STRING".to_string()),
            RepeatedPString => ("List[str]".to_string(), "TYPE_STRING".to_string()),
            PType::Custom(n) => (n.to_string(), "TYPE_MESSAGE".to_string()),
            RepeatedCustom(n) => (format!("List[{}]", n), "TYPE_MESSAGE".to_string()),
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
    pub fn compile_python(&self) -> (String, String, String) {
        let mut field_code = String::new();
        let (mut py_type, msg_type) = self.ptype.compile_python();
        if self.default.is_some() {
            py_type = format!("Optional[{}]", py_type);
        }
        field_code.push_str(&format!(
            "{}: {} = betterproto2.field({}, betterproto2.{}, repeated={}, optional={})",
            self.name,
            py_type,
            self.index,
            msg_type,
            capitalize_first(self.repeated.to_string().as_str()),
            capitalize_first(self.default.is_some().to_string().as_str())
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

        format!("\
class {name}(betterproto2.Enum):
{comments}{fields}
")
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
            if f.default.is_some() {
                variables_with_default_init.push_str(
                    format!(
                        "        {} = {}, \n",
                        init,
                        f.default.clone().unwrap().as_str()
                    )
                        .as_str(),
                );
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
{}
{}
    ):
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
    pub fn compile_python(&self, files: Vec<PathBuf>, mut output: PathBuf) {
        let files_str = files
            .iter()
            .map(|f| f.file_name().unwrap().to_str().unwrap())
            .collect::<Vec<&str>>()
            .join(", ");
        let mut code = String::new();
        code.push_str(
            format!(
                "\
# Generated by Yet Another Protocol Buffer Compiler. DO NOT EDIT!
# sources: {files_str}
# This file has been @generated

from typing import List, Optional
from dataclasses import dataclass
import betterproto2

default_message_pool = betterproto2.MessagePool()

_COMPILER_VERSION = \"0.9.0\"
betterproto2.check_compiler_version(_COMPILER_VERSION)

__all__ = (
"
            )
                .as_str(),
        );

        for message in &self.messages {
            code.push_str(format!("    \"{}\",\n", message.name).as_str());
        }

        code.push_str(")\n\n");

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

        output.push("__init__.py");

        fs::write(&output, &code).unwrap();
    }
}
