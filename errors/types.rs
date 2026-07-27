use serde::{Deserialize, Serialize};

/// Các loại lỗi được phân loại tự động
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ErrorType {
    /// Lỗi biên dịch (compile errors)
    Compile,
    /// Lỗi runtime (panics, exceptions)
    Runtime,
    /// Lỗi mạng/API (timeout, connection refused, 4xx/5xx)
    Network,
    /// Lỗi logic nghiệp vụ
    Logic,
    /// Lỗi phân quyền (permission denied, auth failed)
    Permission,
    /// Lỗi cú pháp/syntax
    Syntax,
    /// Lỗi dependency/missing package
    Dependency,
    /// Lỗi tool execution
    ToolExecution,
    /// Lỗi LLM (invalid response, context overflow)
    Llm,
    /// Lỗi không xác định
    Unknown,
}

impl std::fmt::Display for ErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorType::Compile => write!(f, "compile"),
            ErrorType::Runtime => write!(f, "runtime"),
            ErrorType::Network => write!(f, "network"),
            ErrorType::Logic => write!(f, "logic"),
            ErrorType::Permission => write!(f, "permission"),
            ErrorType::Syntax => write!(f, "syntax"),
            ErrorType::Dependency => write!(f, "dependency"),
            ErrorType::ToolExecution => write!(f, "tool_execution"),
            ErrorType::Llm => write!(f, "llm"),
            ErrorType::Unknown => write!(f, "unknown"),
        }
    }
}

/// Parse an ErrorType from its string representation
pub fn parse_error_type_str(s: &str) -> ErrorType {
    match s.to_lowercase().as_str() {
        "compile" => ErrorType::Compile,
        "runtime" => ErrorType::Runtime,
        "network" => ErrorType::Network,
        "logic" => ErrorType::Logic,
        "permission" => ErrorType::Permission,
        "syntax" => ErrorType::Syntax,
        "dependency" => ErrorType::Dependency,
        "tool_execution" => ErrorType::ToolExecution,
        "llm" => ErrorType::Llm,
        _ => ErrorType::Unknown,
    }
}

/// Categorize an error message string into an ErrorType
pub fn classify_error(error_msg: &str) -> ErrorType {
    let lower = error_msg.to_lowercase();

    if lower.contains("compile") || lower.contains("cannot find") || lower.contains("undefined reference")
        || lower.contains("error[") || lower.contains("error:") || lower.contains("expected")
        || lower.contains("no such file") || lower.contains("not found")
    {
        ErrorType::Compile
    } else if lower.contains("panic") || lower.contains("unreachable") || lower.contains("segmentation fault")
        || lower.contains("segfault") || lower.contains("null pointer") || lower.contains("stack overflow")
        || lower.contains("index out of") || lower.contains("unexpected error")
    {
        ErrorType::Runtime
    } else if lower.contains("timeout") || lower.contains("connection refused") || lower.contains("connection reset")
        || lower.contains("network") || lower.contains("dns") || lower.contains("resolve")
        || lower.contains("4") || lower.contains("5") || lower.contains("http ")
        || lower.contains("api error") || lower.contains("rate limit") || lower.contains("too many requests")
    {
        ErrorType::Network
    } else if lower.contains("permission") || lower.contains("unauthorized") || lower.contains("forbidden")
        || lower.contains("auth") || lower.contains("access denied") || lower.contains("credentials")
        || lower.contains("api key") || lower.contains("token")
    {
        ErrorType::Permission
    } else if lower.contains("syntax") || lower.contains("parse error") || lower.contains("unexpected token")
        || lower.contains("invalid syntax") || lower.contains("unterminated")
    {
        ErrorType::Syntax
    } else if lower.contains("dependency") || lower.contains("missing package") || lower.contains("crate")
        || lower.contains("module") || lower.contains("package") || lower.contains("install")
    {
        ErrorType::Dependency
    } else if lower.contains("tool") || lower.contains("execution error") || lower.contains("command failed")
        || lower.contains("exit code") || lower.contains("non-zero")
    {
        ErrorType::ToolExecution
    } else if lower.contains("context") && lower.contains("token") || lower.contains("content_filter")
        || lower.contains("llm") || lower.contains("model") || lower.contains("completion")
        || lower.contains("invalid response") || lower.contains("prompt")
    {
        ErrorType::Llm
    } else if lower.contains("logic") || lower.contains("business") || lower.contains("validation")
        || lower.contains("invalid input") || lower.contains("incorrect") || lower.contains("wrong")
    {
        ErrorType::Logic
    } else {
        ErrorType::Unknown
    }
}

/// Một entry lỗi trong Error Memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEntry {
    pub id: String,
    pub error_type: ErrorType,
    pub title: String,
    pub description: String,
    pub context: String,
    pub solution: String,
    pub code_snippet: Option<String>,
    pub stack_trace: Option<String>,
    pub timestamp: String,
    pub last_seen: String,
    pub frequency: u32,
    pub resolved: bool,
    pub related_errors: Vec<String>,
    pub tags: Vec<String>,
}

impl ErrorEntry {
    pub fn new(
        error_type: ErrorType,
        title: &str,
        description: &str,
        context: &str,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            error_type,
            title: title.to_string(),
            description: description.to_string(),
            context: context.to_string(),
            solution: String::new(),
            code_snippet: None,
            stack_trace: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
            last_seen: chrono::Utc::now().to_rfc3339(),
            frequency: 1,
            resolved: false,
            related_errors: Vec::new(),
            tags: Vec::new(),
        }
    }
}
