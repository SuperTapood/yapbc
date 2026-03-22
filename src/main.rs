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