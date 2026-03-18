mod ast;

use crate::ast::message::Messages;
use std::fs;

fn main() {
    let input = fs::read_to_string("./src/test.proto").unwrap();
    let messages = Messages::parse(input);
    // python::python_compile(messages);
    let code = messages.python_compile();
    fs::write("./src/test.py", code.to_string()).unwrap();
}