use std::{fmt::Display, io};

use niri_ipc::Reply;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BjuwkError {
    #[error("{1}: {0}")]
    Io(#[source] io::Error, String),
    #[error("JSON: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("YAML: {0}")]
    SerdeYml(#[from] serde_yml::Error),
    #[error("Niri IPC reply: {0:?}")]
    NiriReply(Box<Reply>),
    #[error("Invalid snapshot: {0}")]
    InvalidSnapshot(String),
    #[error("{0}")]
    Other(String),
}

pub type BjuwkResult<T> = Result<T, BjuwkError>;

pub trait IoContextExt<T> {
    fn context<C>(self, context: C) -> BjuwkResult<T>
    where
        C: Display + Send + Sync + 'static;
}

impl<T> IoContextExt<T> for Result<T, io::Error> {
    fn context<C>(self, context: C) -> BjuwkResult<T>
    where
        C: Display + Send + Sync + 'static,
    {
        self.map_err(|e| BjuwkError::Io(e, context.to_string()))
    }
}

impl From<Reply> for BjuwkError {
    fn from(reply: Reply) -> Self {
        Self::NiriReply(Box::new(reply))
    }
}
