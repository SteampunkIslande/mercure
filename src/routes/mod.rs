mod admin;
mod auth;
mod generics;
mod listgroups;
mod newform;
mod newgroup;

pub use admin::*;
pub use auth::*;
pub use generics::*;
pub use listgroups::*;
pub use newform::*;
pub use newgroup::*;

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
