use std::fmt::Display;

#[derive(Debug)]
pub enum OnDiskError {
    // Common error types we might find
    Disk(std::io::Error),
    SerdeJson(serde_json::Error),
    //
    Implementation(String),
}

impl From<std::io::Error> for OnDiskError {
    fn from(value: std::io::Error) -> Self {
        Self::Disk(value)
    }
}
impl From<serde_json::Error> for OnDiskError {
    fn from(value: serde_json::Error) -> Self {
        Self::SerdeJson(value)
    }
}
impl Display for OnDiskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{:?}", self))
    }
}
impl std::error::Error for OnDiskError {}
