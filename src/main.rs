mod ast;

use crate::ast::message::Messages;
use std::fs;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "yapbc")]
#[command(about = "Yet Another Protocol Buffer Compiler", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Adds two numbers
    Add {
        a: i32,
        b: i32,
    },
    /// Greets a person
    Greet {
        #[arg(short, long)]
        name: String
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Add { a, b } => {
            println!("Result: {}", a + b);
        }
        Commands::Greet { name } => {
            println!("Hello, {}!", name);
        }
    }
    let input = fs::read_to_string("./src/test.proto").unwrap();
    let messages = Messages::parse(input);
    // python::python_compile(messages);
    let code = messages.python_compile();
    fs::write("./src/test.py", code.to_string()).unwrap();
}