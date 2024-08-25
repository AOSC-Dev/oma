pub mod parser;
pub mod searcher;

#[derive(Debug, thiserror::Error)]
pub enum OmaContentsError {
    #[error("Contents does not exist")]
    ContentsNotExist,
    #[error("Execute ripgrep failed: {0:?}")]
    ExecuteRgFailed(std::io::Error),
    #[error("Failed to read dir or file: {0}, kind: {1}")]
    FailedToOperateDirOrFile(String, std::io::Error),
    #[error("Failed to get file {0} metadata: {1}")]
    FailedToGetFileMetadata(String, std::io::Error),
    #[error("Failed to wait ripgrep to exit: {0}")]
    FailedToWaitExit(std::io::Error),
    #[error("Contents entry missing path list: {0}")]
    ContentsEntryMissingPathList(String),
    #[error("Command not found wrong argument")]
    CnfWrongArgument,
    #[error("Ripgrep exited with error")]
    RgWithError,
    #[error(transparent)]
    LzzzErr(#[from] lzzzz::lz4f::Error),
    #[error("")]
    NoResult,
    #[error("Illegal file: {0}")]
    IllegalFile(String),
    #[error(transparent)]
    AhoCorasick(#[from] aho_corasick::BuildError),
}
