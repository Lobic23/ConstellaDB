mod ast;
mod parser;
mod error;

pub use ast::Command;
pub use error::CmdError;

pub fn parse_command(input: &str) -> Result<Command, CmdError> {
    parser::parse(input)
}
