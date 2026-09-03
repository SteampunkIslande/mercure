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
pub enum ApiResponse<T> {
    Success { data: T },
    Failure { message: String },
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self::Success { data }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::Failure {
            message: message.into(),
        }
    }
}
