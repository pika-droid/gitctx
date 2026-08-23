use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ResponseEnvelope<T> {
    pub version: &'static str,
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorDetail>,
}

#[derive(Debug, Serialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
}

impl<T> ResponseEnvelope<T> {
    pub fn success(data: T) -> Self {
        Self {
            version: "v1",
            status: "success",
            data: Some(data),
            error: None,
        }
    }

    #[allow(dead_code)]
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            version: "v1",
            status: "error",
            data: None,
            error: Some(ErrorDetail {
                code: code.into(),
                message: message.into(),
            }),
        }
    }
}
