mod ast;

use crate::ast::message::Messages;
use std::fs;
use std::path::PathBuf;
use std::process::exit;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "yapbc")]
#[command(about = "Yet Another Protocol Buffer Compiler", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(
    clap::ValueEnum, Clone, Debug,
)]
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

    let mut total_input = String::new();

    for file in &files {
        if !file.try_exists().expect("Failed to check file existence") {
            println!("File does not exist: {}", file.display());
            exit(1);
        }

        total_input.push_str(fs::read_to_string(file).unwrap().as_str())
    }

    let messages = Messages::parse(total_input);
    match language {
        Language::Python => messages.compile_python(files, output),
        Language::Go => messages.compile_go(files, output),
    };
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Compile { files, output, language } =>
            compile(files.clone(), output.clone(), language.clone()),
    }

}