use thiserror::Error;

#[derive(Error, Debug)]
pub enum TokensmithError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("runtime error: {0}")]
    Runtime(String),
}
