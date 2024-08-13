use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Bluetooth error: {0}")]
    BluetoothError(#[from] bluer::Error),
    #[error("HTTP request error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("UTF-8 conversion error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
}
