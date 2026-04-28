use crate::ast::field::Field;
use crate::ast::message::{Message, Messages};
use crate::ast::ptype::PType;
use crate::util::{capitalize_first, pascal_to_snake, snake_to_pascal};
use prost::Message as ProstMessage;
use prost_types::field_descriptor_proto::{Label, Type};
use prost_types::{DescriptorProto, EnumDescriptorProto, EnumValueDescriptorProto, FieldDescriptorProto, FileDescriptorProto, FileOptions};
use std::collections::{HashMap, HashSet};
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
            PType::Custom(n) => { format!("*{}", n.as_str()) }
            PType::RepeatedCustom(n) => format!("[]*{n}").as_str().parse().unwrap(),
            PType::Oneof => { "".parse().unwrap() }
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
                PType::Oneof => "nil",
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
        oneof_index: Option<i32>,
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

        if oneof_index.is_some() {
            field.oneof_index = oneof_index;
        }

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
    pub fn compile_go(&self, message: String, desc: &mut DescriptorProto, enum_types: &Vec<String>, oneof_index: Option<i32>) -> (String, String, Option<String>) {
        let name = snake_to_pascal(&*self.name);
        let str_type = if self.maybe_types.is_none() {
            self.ptype.compile_go()
        } else {
            format!("is{message}_{name}")
        };
        let default = if self.default.is_some() {
            self.default.clone().unwrap()
        } else {
            self.ptype.default_go()
        };

        let parse_type = match self.ptype {
            PType::Int32 | PType::RepeatedInt32 => "varint".to_string(),
            _ => String::from("bytes")
        };

        let index = self.index;
        let snake_name = self.name.clone();

        let struct_var = if self.maybe_types.is_none() {
            format!("    {name}  {str_type} `protobuf:\"{parse_type},{index},opt,name={snake_name},proto3\" json:\"{snake_name},omitempty\"`\n")
        } else {
            format!("    {name}  {str_type} `protobuf_oneof:\"{}\"`\n", self.name)
        };

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

        self.add_field_to_desc(desc, is_enum, oneof_index);

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
    state protoimpl.MessageState `protogen:\"open.v1\"`
    unknownFields protoimpl.UnknownFields
    sizeCache     protoimpl.SizeCache
"
        );
        let mut getters = String::new();

        let mut oneof_idx = None;

        for field in &self.fields {
            if let Some(ref nested_fields) = field.maybe_types {
                oneof_idx = Some(desc.oneof_decl.len() as i32);
                desc.oneof_decl.push(prost_types::OneofDescriptorProto {
                    name: Some(field.name.clone()),
                    options: None,
                });
                let pascal_name = snake_to_pascal(&field.name);
                let interface_name = format!("is{name}_{pascal_name}");
                struct_code.push_str(&format!("    {pascal_name}  {interface_name} `protobuf_oneof:\"{}\"`\n", field.name));
                for nested in nested_fields {
                    nested.add_field_to_desc(&mut desc, false, oneof_idx);
                    let branch_name = snake_to_pascal(&nested.name);
                    let branch_type = nested.ptype.compile_go();
                    let default_val = nested.ptype.default_go();

                    getters.push_str(&format!("
func (x *{name}) Get{branch_name}() {branch_type} {{
    if x != nil {{
        if x, ok := x.{pascal_name}.(*{name}_{branch_name}); ok {{
            return x.{branch_name}
        }}
    }}
    return {default_val}
}}
"));
                }
                getters.push_str(&format!("
func (x *{name}) Get{pascal_name}() {interface_name} {{
    if x != nil {{
        return x.{pascal_name}
    }}
    return nil
}}
"));
            } else {
                let (struct_var, getter, dep) = field.compile_go(name.clone(), &mut desc, enum_types, oneof_idx);
                struct_code.push_str(struct_var.as_str());
                getters.push_str(getter.as_str());
                if let Some(d) = dep {

                    deps.push(d);
                }
            }
        }

        struct_code.push_str("}\n");

        // 4. Generate the Interface and Wrapper Structs
        for field in &self.fields {
            if let Some(ref nested_fields) = field.maybe_types {
                let pascal_name = snake_to_pascal(&field.name);
                let is_type_name = format!("is{}_{}", self.name, pascal_name);

                struct_code.push_str(&format!("\ntype {is_type_name} interface {{\n    {is_type_name}()\n}}\n"));

                for nested in nested_fields {
                    let type_name = nested.ptype.compile_go();
                    let branch_struct_name = snake_to_pascal(&nested.name);
                    let index = nested.index;

                    struct_code.push_str(&format!("
type {name}_{branch_struct_name} struct {{
    {branch_struct_name} {type_name} `protobuf:\"bytes,{index},opt,name={},proto3,oneof\"`
}}

func (*{name}_{branch_struct_name}) {is_type_name}() {{}}\n", nested.name));
                }
            }
        }

        (struct_code, getters, desc, deps)
    }
}

fn import_field(field: &mut Field, object: String, tmp: String, module: &Messages) {
    if let Some(ref mut nested_fields) = field.maybe_types {
        for nested_field in nested_fields {
            import_field(nested_field, object.clone(), tmp.clone(), module);
        }
    }

    for module_message in module.messages.clone() {
        if field.ptype.to_string() == module_message.name.to_string() {
            if field.ptype.is_nested() {
                let actual = field.ptype.to_string();
                if field.ptype.is_repeated() {
                    field.ptype = PType::RepeatedCustom(format!("{tmp}.{actual}"));
                } else {
                    field.ptype = PType::Custom(format!("{tmp}.{actual}"));
                }
            }
        }
    }
}

impl Messages {
    fn generate_go_string(&self, raw_bytes: &[u8], var_name: &str) -> String {
        let mut escaped = String::from(format!("const {} = \"\" +\n    \"", var_name));
        for &b in raw_bytes {
            let mut result = match b {
                b'\n' => "\\n\" + \n    \"",
                b'\\' => "\\\\",
                b'\"' => "\\\"",
                32..=126 => &*(b as char).to_string(),
                _ => &format!("\\x{:02x}", b),
            };

            if result == "\\x09" {
                result = "\\t";
            }

            if result == "\\x08" {
                result = "\\b";
            }

            escaped.push_str(result);
        }

        escaped + "\""
    }

    pub fn compile_go(&mut self, file: PathBuf, input: PathBuf, output: PathBuf, module: Option<String>, all_messages: HashMap<String, (PathBuf, PathBuf, Messages)>) {
        // println!("{:?}", file);
        // println!("{:?}", file.parent().unwrap());
        // println!("{:?}", input);
        // println!("{:?}", file.to_str().unwrap().strip_prefix(output.to_str().unwrap()));
        let parent = file.parent().unwrap().to_str().unwrap().strip_prefix(input.to_str().unwrap()).unwrap();

        // panic!();
        let mut source_file = file.to_str().unwrap().to_string();
        source_file = source_file.trim_start_matches(".\\").parse().unwrap();
        let package = self.package.clone();
        let mut imports = String::new();
        let stem = pascal_to_snake(file.file_stem().and_then(|s| s.to_str()).unwrap());
        let msg_types = format!("file_{package}_{stem}_proto");
        let mut oneof_wrappers = String::new();

        let mut used_imports = HashSet::new();

        for import in self.imports.clone() {
            let file = import.trim_start_matches("\"").trim_end_matches("\"");
            let split = import.split("/").collect::<Vec<_>>();
            let object = split[split.len() - 1].trim_end_matches("\"");
            if !module.is_some() {
                println!("cannot specify import without giving module name");
                exit(1);
            }
            let mut tmp = file.to_string();
            tmp = tmp.strip_suffix(format!("/{object}").as_str()).unwrap().to_string().to_string();
            if !used_imports.contains(&tmp) && !tmp.eq(parent.trim_start_matches("\\")) {
                used_imports.insert(tmp.clone());
                imports.push_str(format!("    {tmp} \"{}/{tmp}\"\n", module.clone().unwrap()).as_str());
            }

            // println!("{:?} => {:?}", file, self.package);
            for message in &mut self.messages {
                for field in &mut message.fields {
                    let (_, _, module) = all_messages.get(file).unwrap();
                    import_field(field, object.parse().unwrap(), tmp.clone(), module);
                }
            }
        }

        for message in &mut self.messages {
            for field in &mut message.fields {
                if field.maybe_types.is_some() {
                    let index = message.index;
                    oneof_wrappers.push_str(format!("    {msg_types}_msgTypes[{index}].OneofWrappers = []any{{\n").as_str());
                    for field in &mut field.maybe_types.clone().unwrap() {
                        oneof_wrappers.push_str(format!("        (*{}_{})(nil),\n", message.name, snake_to_pascal(&field.name)).as_str());
                    }
                    oneof_wrappers.push_str("    }\n");
                }
            }
        }

        let mut total_code = String::from(format!(
            "\
// Code generated by yapbc. DO NOT EDIT.
// source: {source_file}

package {package}

import (
{imports}    protoreflect \"google.golang.org/protobuf/reflect/protoreflect\"
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
        let mut go_types = String::new();
        // 1. Initialize the File Descriptor
        let mut file_desc = FileDescriptorProto::default();
        file_desc.name = Some(source_file.to_string().replace("\\", "/"));
        file_desc.syntax = Some("proto3".to_string());

        // Add Go package options
        let mut options = FileOptions::default();
        options.go_package = Some(module.unwrap() + "/" + &*self.package.to_string());
        options.java_multiple_files = Some(true);
        options.java_outer_classname = Some(snake_to_pascal(&stem) + "Proto");
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

        dep_idxs_code.push_str(&format!("    {total_deps_count}, // [0:0] is the sub-list for method output_type\n"));
        dep_idxs_code.push_str(&format!("    {total_deps_count}, // [0:0] is the sub-list for method input_type\n"));
        dep_idxs_code.push_str(&format!("    {total_deps_count}, // [0:0] is the sub-list for extension type_name\n"));
        dep_idxs_code.push_str(&format!("    {total_deps_count}, // [0:0] is the sub-list for extension extendee\n"));
        dep_idxs_code.push_str(&format!("    0, // [0:{total_deps_count}] is the sub-list for field type_name\n", ));
        let cap_msg_type = capitalize_first(&*msg_types.clone());
        let mut raw_bytes = Vec::new();
        file_desc.encode(&mut raw_bytes).unwrap();
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
{oneof_wrappers}	type x struct{{}}
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

        let mut filename = PathBuf::new();
        filename.push(output.to_str().unwrap());
        filename.push(parent.trim_start_matches("\\"));
        filename.push(stem + ".pb.go");
        fs::create_dir_all(filename.clone().parent().unwrap()).unwrap();

        fs::write(&filename, &total_code).unwrap();
    }
}