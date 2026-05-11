use thiserror::Error;

#[derive(Debug, Error)]
pub enum KernelError {
    #[error("circular dependency detected involving activity '{0}'")]
    CircularDependency(String),

    #[error("unknown activity id '{0}'")]
    UnknownActivity(String),

    #[error("unknown resource id '{0}'")]
    UnknownResource(String),

    #[error("unknown calendar id '{0}'")]
    UnknownCalendar(String),

    #[error("date parse error: {0}")]
    DateParse(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),
}
