mod back_end;
mod front_end;

// Back-end re-export
pub mod backend {
    pub use super::back_end::*;
}

// Front-end re-export
pub mod frontend {
    pub use super::front_end::*;
}

#[derive(Debug, serde::Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            message: String::from("Success"),
            data: Some(data),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: None,
        }
    }
}
