use crate::on_disk::OnDiskError;
use banyanfs::api::ApiError;
use std::{fmt::Display, string::FromUtf8Error};

#[derive(Debug)]
pub enum NativeError {
    Api(ApiError),
    Config(OnDiskError),
    Custom(String),
}

impl std::error::Error for NativeError {}

impl Display for NativeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NativeError::Api(err) => f.write_str(&err.to_string()),
            NativeError::Config(err) => f.write_str(&err.to_string()),
            NativeError::Custom(err) => f.write_str(err),
        }
    }
}

impl From<ApiError> for NativeError {
    fn from(value: ApiError) -> Self {
        Self::Api(value)
    }
}

impl From<OnDiskError> for NativeError {
    fn from(value: OnDiskError) -> Self {
        Self::Config(value)
    }
}

impl From<std::io::Error> for NativeError {
    fn from(value: std::io::Error) -> Self {
        Self::Config(OnDiskError::Disk(value))
    }
}

impl From<FromUtf8Error> for NativeError {
    fn from(value: FromUtf8Error) -> Self {
        Self::Custom(format!("From UTF8: {value}"))
    }
}
