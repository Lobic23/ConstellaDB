use thiserror::Error;

#[derive(Debug, Error)]
pub enum CmdError {
  #[error("syntax error: {0}")]
  Syntax(String),
  #[error("unsupported statement: {0}")]
  Unsupported(String),
  #[error("unsupported expression: {0}")]
  UnsupportedExpr(String),
  #[error("empty input")]
  Empty,
}
