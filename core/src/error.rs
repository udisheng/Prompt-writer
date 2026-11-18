use thiserror::Error;

#[derive(Debug, Error)]
pub enum RewriteError {
    #[error("Add an OpenRouter API key in Settings.")]
    MissingApiKey,
    #[error("Choose a model in Settings.")]
    MissingModel,
    #[error("{0}")]
    InvalidConfig(String),
    #[error("{0}")]
    Unreachable(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    ModelMissing(String),
    #[error("{0}")]
    Provider(String),
    #[error("The model returned an empty reply.")]
    EmptyCompletion,
    #[error("{0}")]
    Template(String),
}
