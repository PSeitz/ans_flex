use thiserror::Error;

#[derive(Error, Debug)]
pub enum HistError {
    #[error("output is too small")]
    OutputTooSmall,
    #[error("unexpected remaining")]
    UnexpectedRemaining,
    #[error("tablelog too large")]
    TableLogTooLarge,
    #[error("tablelog too small")]
    TableLogTooSmall,
    #[error("max symbol value")]
    MaxSymbolValueTooSmall,
    #[error("corruption detected: `{0}`")]
    CorruptionDetected(String),
    #[error("Incorrect normalized distribution")]
    IncorrectNormalizedDistribution,
}
