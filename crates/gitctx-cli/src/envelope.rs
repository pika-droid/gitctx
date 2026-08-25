use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResponseEnvelope<T> {
    pub version: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
}

impl<T> ResponseEnvelope<T> {
    pub fn success(data: T) -> Self {
        Self {
            version: "v1".to_string(),
            status: "success".to_string(),
            data: Some(data),
            error: None,
        }
    }

    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            version: "v1".to_string(),
            status: "error".to_string(),
            data: None,
            error: Some(ErrorDetail {
                code: code.into(),
                message: message.into(),
            }),
        }
    }
}
