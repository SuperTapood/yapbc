use crate::ast::field::Field;
use crate::ast::message::{Message, Messages};
use crate::ast::ptype::PType;
use crate::util::{capitalize_first, pascal_to_snake, snake_to_pascal};
use prost::Message as ProstMessage;
use prost_types::field_descriptor_proto::{Label, Type};
use prost_types::{DescriptorProto, EnumDescriptorProto, EnumValueDescriptorProto, FieldDescriptorProto, FileDescriptorProto, FileOptions};
use std::fs;
use std::path::PathBuf;
use std::process::exit;

impl PType {
    pub fn compile_go(&self) -> String {
        match &self {
            PType::Int32 => { "int32".parse().unwrap() }
            PType::RepeatedInt32 => { "[]int32".parse().unwrap() }
            PType::PString => { "string".parse().unwrap() }
            PType::RepeatedPString => { "[]string".parse().unwrap() }
            PType::Custom(n) => { n.as_str().parse().unwrap() }
            PType::RepeatedCustom(n) => format!("[]*{n}").as_str().parse().unwrap()
        }
    }

    pub fn default_go(&self) -> String {
        String::from(
            match &self {
                PType::Int32 => "0",
                PType::RepeatedInt32 => "nil",
                PType::PString => "\"\"",
                PType::RepeatedPString => "nil",
                PType::Custom(_) => "nil",
                PType::RepeatedCustom(_) => "nil",
            }
        )
    }
}

impl Field {
    fn get_proto_type(&self, type_name: &str) -> i32 {
        match type_name {
            "double" => Type::Double as i32,
            "float" => Type::Float as i32,
            "int64" => Type::Int64 as i32,
            "uint64" => Type::Uint64 as i32,
            "int32" => Type::Int32 as i32,
            "fixed64" => Type::Fixed64 as i32,
            "fixed32" => Type::Fixed32 as i32,
            "bool" => Type::Bool as i32,
            "string" => Type::String as i32,
            "bytes" => Type::Bytes as i32,
            "message" => Type::Message as i32,
            _ => Type::String as i32,
        }
    }
    fn add_field_to_desc(
        &self,
        msg: &mut DescriptorProto,
        is_enum: bool,
    ) {
        let mut field = FieldDescriptorProto::default();
        field.name = Some(self.name.clone());
        field.number = Some(self.index as i32);

        // Set Label (Repeated vs Optional)
        field.label = Some(if self.repeated {
            Label::Repeated as i32
        } else {
            Label::Optional as i32
        });

        field.r#type = Some(match &self.ptype {
            PType::Custom(_) | PType::RepeatedCustom(_) => {
                if is_enum { 14 } else { 11 } // 14 = Enum, 11 = Message
            }
            _ => self.get_proto_type(&*self.ptype.to_string()),
        });
        field.json_name = Some(self.name.clone());

        if self.ptype.is_nested() {
            field.type_name = Some(format!(".{}", self.ptype.to_string()));
        }

        msg.field.push(field);
    }
    pub fn compile_go(&self, message: String, desc: &mut DescriptorProto, enum_types: &Vec<String>) -> (String, String, Option<String>) {
        let name = snake_to_pascal(&*self.name);
        let str_type = self.ptype.compile_go();
        let default = if self.default.is_some() {
            self.default.clone().unwrap()
        } else {
            self.ptype.default_go()
        };
        let struct_var = format!("    {name}  {str_type}\n");

        let getter = format!("
func (x *{message}) Get{name}() {str_type} {{
    if x != nil {{
        return x.{name}
    }}
    return {default}
}}
        ");

        let dependency = match &self.ptype {
            PType::Custom(n) | PType::RepeatedCustom(n) => Some(n.clone()),
            _ => None,
        };

        let mut is_enum = false;
        let (typ, _) = self.ptype.compile_python();

        for enum_type in enum_types {
            if enum_type.eq(&typ) || format!("List[{enum_type}]").eq(&typ) {
                is_enum = true;
            }
        }

        self.add_field_to_desc(desc, is_enum);

        (struct_var, getter, dependency)
    }
}

impl Message {
    pub fn compile_go(&self, enum_types: &Vec<String>) -> (String, String, DescriptorProto, Vec<String>) {
        let name = self.name.clone();
        let mut desc = DescriptorProto::default();
        let mut deps = Vec::new();
        desc.name = Some(name.clone());
        let mut struct_code = format!(
            "

type {name} struct {{
    state         protoimpl.MessageState
    unknownFields protoimpl.UnknownFields
	sizeCache     protoimpl.SizeCache
"
        );
        let mut getters = String::new();

        for field in &self.fields {
            let (struct_var, getter, dep) = field.compile_go(name.clone(), &mut desc, enum_types);
            struct_code.push_str(struct_var.as_str());
            getters.push_str(getter.as_str());
            if let Some(d) = dep {
                deps.push(d);
            }
        }

        struct_code.push_str("
}
        ");

        (struct_code, getters, desc, deps)
    }
}

impl Messages {
    fn generate_go_string(&self, raw_bytes: &[u8], var_name: &str) -> String {
        let mut go_str_lines = vec![format!("const {} = \"\" +", var_name)];

        let mut current_line = String::from("\t\"");

        for &b in raw_bytes {
            match b {
                10 => {
                    current_line.push_str("\\n");
                    current_line.push_str("\" +");
                    go_str_lines.push(current_line);
                    current_line = String::from("\t\"");
                }
                13 => current_line.push_str("\\r"),
                9 => current_line.push_str("\\t"),
                11 => current_line.push_str("\\v"),
                34 => current_line.push_str("\\\""),
                92 => current_line.push_str("\\\\"),
                32..=126 => current_line.push(b as char),
                _ => current_line.push_str(&format!("\\x{:02x}", b)),
            }
        }

        if current_line != "\t\"" {
            current_line.push('"');
            go_str_lines.push(current_line);
        } else {
            if let Some(last_line) = go_str_lines.last_mut() {
                if last_line.ends_with(" +") {
                    last_line.truncate(last_line.len() - 2);
                }
            }
        }

        go_str_lines.join("\n")
    }
    pub fn compile_go(&mut self, file: PathBuf, output: PathBuf, module: Option<String>) {
        let parent = file.parent().unwrap().to_str().unwrap().strip_prefix(output.to_str().unwrap()).unwrap();
        if self.package.is_empty() {
            let split_parent = parent.split("\\").collect::<Vec<_>>();
            self.package = split_parent[split_parent.len() - 1].parse().unwrap();
        }
        let source_file = file.to_str().unwrap().to_string();
        let package = self.package.clone();
        let mut imports = String::new();

        for import in &self.imports {
            let split = import.split("/").collect::<Vec<_>>();
            let object = split[split.len() - 1];
            if !module.is_some() {
                println!("cannot specify import without giving module name");
                exit(1);
            }
            let mut tmp = import.clone();
            tmp = tmp.strip_suffix(format!("/{object}").as_str()).unwrap().to_string();
            imports.push_str(format!("    {tmp} \"{}/{tmp}\"\n", module.clone().unwrap()).as_str());

            for message in &mut self.messages {
                for field in &mut message.fields {
                    if field.ptype.is_nested() {
                        let actual = field.ptype.to_string();
                        println!("{:?}", field.ptype);
                        println!("{:?}", actual);
                        println!("{}", field.ptype.is_repeated());
                        if field.ptype.is_repeated() {
                            field.ptype = PType::RepeatedCustom(format!("{tmp}.{actual}"));
                        } else {
                            field.ptype = PType::Custom(format!("{tmp}.{actual}"));
                        }
                    }
                }
            }
        }

        let mut total_code = String::from(format!(
            "\
// Code generated by yapbc. DO NOT EDIT.
// source: {source_file}

package {package}

import (
{imports}
	protoreflect \"google.golang.org/protobuf/reflect/protoreflect\"
	protoimpl \"google.golang.org/protobuf/runtime/protoimpl\"
	reflect \"reflect\"
	sync \"sync\"
	unsafe \"unsafe\"
)

const (
	// Verify that this generated code is sufficiently up-to-date.
	_ = protoimpl.EnforceVersion(20 - protoimpl.MinVersion)
	// Verify that runtime/protoimpl is sufficiently up-to-date.
	_ = protoimpl.EnforceVersion(protoimpl.MaxVersion - 20)
)"
        ));
        println!("import deez nuts {:?}", self.imports);
        let stem = pascal_to_snake(file.file_stem().and_then(|s| s.to_str()).unwrap());
        let msg_types = format!("file_{stem}");
        let mut go_types = String::new();
        // 1. Initialize the File Descriptor
        let mut file_desc = FileDescriptorProto::default();
        file_desc.name = Some(file.to_str().unwrap().to_string());
        file_desc.syntax = Some("proto3".to_string());

        // Add Go package options
        let mut options = FileOptions::default();
        options.go_package = Some(self.package.to_string());
        file_desc.options = Some(options);
        let mut message_map = std::collections::HashMap::new();
        let mut all_deps_as_indices = Vec::new();

        for penum in self.penums.iter() {
            message_map.insert(penum.name.clone(), penum.index);
        };

        // Pre-map message names to their index in goTypes
        for msg in self.messages.iter() {
            message_map.insert(msg.name.clone(), msg.index);
        };

        let mut total_structs_and_getters = String::new();
        //let mut go_types_list = String::new();

        let penums = self.penums.len();
        let messages = self.messages.len();
        let mut enum_types = String::from("nil");
        let mut counter = 0;
        let mut enums = Vec::new();

        for (i, penum) in self.penums.iter().enumerate() {
            let name = &penum.name;
            enums.push(name.clone());
            let mut const_values = String::new();
            let mut name_map = String::new();
            let mut value_map = String::new();
            enum_types = format!("{msg_types}_enumTypes").as_str().parse().unwrap();

            let mut enum_desc = EnumDescriptorProto::default();
            enum_desc.name = Some(name.clone());


            for field in &penum.fields {
                let field_name = &field.name;
                let field_index = &field.index;

                let mut enum_val = EnumValueDescriptorProto::default();
                enum_val.name = Some(field_name.clone());
                enum_val.number = Some(*field_index as i32);
                enum_desc.value.push(enum_val);

                const_values.push_str(format!("    {name}_{field_name}    {name} = {field_index};\n").as_str());
                name_map.push_str(format!("        {field_index}: \"{field_name}\",\n").as_str());
                value_map.push_str(format!("        \"{field_name}\": {field_index},\n").as_str());
            }
            file_desc.enum_type.push(enum_desc);
            go_types.push_str(format!("    ({name})(0), // {counter}: {name}\n").as_str());
            counter += 1;

            total_code.push_str(format!("

type {name} int32

const (
{const_values})

// Enum value maps for {name}
var (
    {name}_name = map[int32]string{{
{name_map}    }}
    {name}_value = map[string]int32{{
{value_map}    }}
)

func (x {name}) Enum() *{name} {{
    p := new({name})
	*p = x
	return p
}}

func (x {name}) String() string {{
	return protoimpl.X.EnumStringOf(x.Descriptor(), protoreflect.EnumNumber(x))
}}

func ({name}) Descriptor() protoreflect.EnumDescriptor {{
	return {msg_types}_enumTypes[{i}].Descriptor()
}}

func ({name}) Type() protoreflect.EnumType {{
	return &{msg_types}_enumTypes[{i}]
}}

func (x {name}) Number() protoreflect.EnumNumber {{
	return protoreflect.EnumNumber(x)
}}

// Deprecated: Use {name}.Descriptor instead.
func ({name}) EnumDescriptor() ([]byte, []int) {{
	return {msg_types}_rawDescGZIP(), []int{{{i}}}
}}

var {msg_types}_enumTypes = make([]protoimpl.EnumInfo, {penums})

").as_str())
        }

        for (i, message) in self.messages.iter().enumerate() {
            let (struct_code, getters, desc, deps) = message.compile_go(&enums);
            file_desc.message_type.push(desc);
            total_structs_and_getters.push_str(&struct_code);
            total_structs_and_getters.push_str(&getters);
            //go_types_list.push_str(&format!("    (*{})(nil), // {}: {}\n", message.name, i, message.name));
            for dep_name in deps {
                if let Some(&idx) = message_map.get(&dep_name) {
                    all_deps_as_indices.push((idx, format!("{}.{}:type_name -> {}", message.name, dep_name.to_lowercase(), dep_name)));
                }
            }

            let message_name = message.name.clone();
            total_code.push_str(struct_code.as_str());
            go_types.push_str(format!("    (*{message_name})(nil), // {counter}: {message_name}\n").as_str());
            counter += 1;
            total_code.push_str(format!("
func (x *{message_name}) Reset() {{
    *x = {message_name}{{}}
	mi := &{msg_types}_msgTypes[{i}]
	ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
	ms.StoreMessageInfo(mi)
}}

func (x *{message_name}) String() string {{
	return protoimpl.X.MessageStringOf(x)
}}

func (*{message_name}) ProtoMessage() {{}}

func (x *{message_name}) ProtoReflect() protoreflect.Message {{
	mi := &{msg_types}_msgTypes[{i}]
	if x != nil {{
		ms := protoimpl.X.MessageStateOf(protoimpl.Pointer(x))
		if ms.LoadMessageInfo() == nil {{
			ms.StoreMessageInfo(mi)
		}}
		return ms
	}}
	return mi.MessageOf(x)
}}

// Deprecated: Use {message_name}.ProtoReflect.Descriptor instead.
func (*{message_name}) Descriptor() ([]byte, []int) {{
	return {msg_types}_rawDescGZIP(), []int{{{i}}}
}}

{getters}

").as_str())
        }
        let total_deps_count = all_deps_as_indices.len() as i32;
        let mut dep_idxs_code = String::from("\n");

        for (i, (idx, comment)) in all_deps_as_indices.iter().enumerate() {
            dep_idxs_code.push_str(&format!("    {}, // {}: {}\n", idx, i, comment));
        }

        dep_idxs_code.push_str(&format!("    {total_deps_count}, // [1:1] is the sub-list for method output_type\n"));
        dep_idxs_code.push_str(&format!("    {total_deps_count}, // [1:1] is the sub-list for method input_type\n"));
        dep_idxs_code.push_str(&format!("    {total_deps_count}, // [1:1] is the sub-list for extension type_name\n"));
        dep_idxs_code.push_str(&format!("    {total_deps_count}, // [1:1] is the sub-list for extension extendee\n"));
        dep_idxs_code.push_str(&format!("    0, // [0:{total_deps_count}] is the sub-list for field type_name\n", ));
        let cap_msg_type = capitalize_first(&*msg_types.clone());
        let raw_bytes = file_desc.encode_to_vec();
        let go_code = self.generate_go_string(&raw_bytes, format!("{msg_types}_rawDesc").as_str());
        total_code.push_str(format!("\
var {cap_msg_type} protoreflect.FileDescriptor

{go_code}

var (
    {msg_types}_rawDescOnce sync.Once
    {msg_types}_rawDescData []byte
)

func {msg_types}_rawDescGZIP() []byte {{
	{msg_types}_rawDescOnce.Do(func() {{
		{msg_types}_rawDescData = protoimpl.X.CompressGZIP(unsafe.Slice(unsafe.StringData({msg_types}_rawDesc), len({msg_types}_rawDesc)))
	}})
	return {msg_types}_rawDescData
}}

var {msg_types}_msgTypes = make([]protoimpl.MessageInfo, {messages})
var {msg_types}_goTypes = []any{{
{go_types}}}
var {msg_types}_depIdxs = []int32{{{dep_idxs_code}}}

func init() {{ {msg_types}_init() }}
func {msg_types}_init() {{
	if {cap_msg_type} != nil {{
		return
	}}
	type x struct{{}}
	out := protoimpl.TypeBuilder{{
		File: protoimpl.DescBuilder{{
			GoPackagePath: reflect.TypeOf(x{{}}).PkgPath(),
			RawDescriptor: unsafe.Slice(unsafe.StringData({msg_types}_rawDesc), len({msg_types}_rawDesc)),
			NumEnums:      {penums},
			NumMessages:   {messages},
			NumExtensions: 0,
			NumServices:   0,
		}},
		GoTypes:           {msg_types}_goTypes,
		DependencyIndexes: {msg_types}_depIdxs,
		MessageInfos:      {msg_types}_msgTypes,
        EnumInfos:         {enum_types},
	}}.Build()
	{cap_msg_type} = out.File
	{msg_types}_goTypes = nil
	{msg_types}_depIdxs = nil
}}
        ").as_str());

        let mut filename = output.clone();
        filename.push(format!(
            "{parent}/{stem}.pb.go",
        ));
        fs::write(&filename, &total_code).unwrap();
    }
}