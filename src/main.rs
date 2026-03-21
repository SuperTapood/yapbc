mod ast;
mod util;

use crate::ast::message::Messages;
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use std::process::exit;

#[derive(Parser)]
#[command(name = "yapbc")]
#[command(about = "Yet Another Protocol Buffer Compiler", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum Language {
    Python,
    Go,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile protobuf to a specific language
    Compile {
        #[arg(short, long)]
        /// The protobuf files you want to compile
        files: Vec<PathBuf>,
        #[arg(short, long)]
        /// The output directory for the compiled code.
        output: PathBuf,
        #[arg(short, long, value_enum)]
        /// The Language you want to compile to
        language: Language,
    },
}

fn compile(files: Vec<PathBuf>, output: PathBuf, language: Language) {
    if files.is_empty() {
        println!("No files specified! Specify some using -f/--files");
        exit(1);
    }

    match language {
        Language::Python => {
            let mut total_input = String::new();

            for file in &files {
                if !file.try_exists().expect("Failed to check file existence") {
                    println!("File does not exist: {}", file.display());
                    exit(1);
                }

                total_input.push_str(fs::read_to_string(file).unwrap().as_str())
            }

            let messages = Messages::parse(total_input);
            messages.compile_python(files, output)
        }
        Language::Go => {
            for file in &files {
                if !file.try_exists().expect("Failed to check file existence") {
                    println!("File does not exist: {}", file.display());
                    exit(1);
                }
                let messages = Messages::parse(fs::read_to_string(file).unwrap());
                messages.compile_go(file.clone(), output.clone());
            }
        }
    }
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Compile {
            files,
            output,
            language,
        } => compile(files.clone(), output.clone(), language.clone()),
    }
}


use prost::Message;
use prost_types::{
    field_descriptor_proto::{Label, Type},
    DescriptorProto, FieldDescriptorProto, FileDescriptorProto, FileOptions,
};

/// Maps a human-readable type string to prost's internal Type enum.
/// This supports the wide range of Protobuf primitives.
fn get_proto_type(type_name: &str) -> i32 {
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
        _ => Type::String as i32, // Default fallback
    }
}

/// Helper function to build and add a field to a message.
fn add_field(
    msg: &mut DescriptorProto,
    name: &str,
    number: i32,
    proto_type: &str,
    repeated: bool,
    type_name: Option<&str>,
) {
    let mut field = FieldDescriptorProto::default();
    field.name = Some(name.to_string());
    field.number = Some(number);

    // Set Label (Repeated vs Optional)
    field.label = Some(if repeated {
        Label::Repeated as i32
    } else {
        Label::Optional as i32
    });

    field.r#type = Some(get_proto_type(proto_type));
    field.json_name = Some(name.to_string());

    // If it's a nested message, we must provide the specific type name (e.g., ".Container")
    if proto_type == "message" {
        if let Some(tn) = type_name {
            field.type_name = Some(tn.to_string());
        }
    }

    msg.field.push(field);
}

/// Converts raw binary bytes into a Go-formatted raw string literal.
fn generate_go_string(raw_bytes: &[u8], var_name: &str) -> String {
    let mut go_str_lines = vec![format!("const {} = \"\" +", var_name)];
    let chunk_size = 40; // Max characters per line for readability

    for chunk in raw_bytes.chunks(chunk_size) {
        let mut line = String::from("    \"");
        for &b in chunk {
            match b {
                10 => line.push_str("\\n"),
                13 => line.push_str("\\r"),
                9 => line.push_str("\\t"),
                34 => line.push_str("\\\""),
                92 => line.push_str("\\\\"),
                32..=126 => line.push(b as char), // Printable ASCII
                _ => line.push_str(&format!("\\x{:02x}", b)), // Hex escape
            }
        }
        line.push_str("\" +");
        go_str_lines.push(line);
    }

    // Remove the trailing " +" from the very last line
    if let Some(last_line) = go_str_lines.last_mut() {
        if last_line.ends_with(" +") {
            last_line.truncate(last_line.len() - 2);
        }
    }

    go_str_lines.join("\n")
}

fn main2() {
    // 1. Initialize the File Descriptor
    let mut file_desc = FileDescriptorProto::default();
    file_desc.name = Some("k8s/pod.proto".to_string());
    file_desc.syntax = Some("proto3".to_string());

    // Add Go package options
    let mut options = FileOptions::default();
    options.go_package = Some("github.com/SuperTapood/Flint/core/generated/k8s".to_string());
    file_desc.options = Some(options);

    // 2. Build Container Message
    let mut container = DescriptorProto::default();
    container.name = Some("Container".to_string());
    add_field(&mut container, "name", 1, "string", false, None);
    add_field(&mut container, "image", 2, "string", false, None);
    add_field(&mut container, "ports", 3, "int32", true, None);
    file_desc.message_type.push(container);

    // 3. Build VolumeMount Message
    let mut vol = DescriptorProto::default();
    vol.name = Some("VolumeMount".to_string());
    add_field(&mut vol, "name", 1, "string", false, None);
    add_field(&mut vol, "mount_path", 2, "string", false, None);
    file_desc.message_type.push(vol);

    // 4. Build Pod Message
    let mut pod = DescriptorProto::default();
    pod.name = Some("Pod".to_string());
    add_field(&mut pod, "name", 1, "string", false, None);
    add_field(&mut pod, "containers", 2, "message", true, Some(".Container"));
    add_field(&mut pod, "mounts", 3, "message", true, Some(".VolumeMount"));
    add_field(&mut pod, "restart_policy", 4, "string", false, None);
    file_desc.message_type.push(pod);

    // 5. Serialize to bytes and format
    let raw_bytes = file_desc.encode_to_vec();
    let go_code = generate_go_string(&raw_bytes, "file_k8s_pod_proto_rawDesc");

    println!("{}", go_code);

    main2();
}