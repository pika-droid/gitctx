//! Multi-language synthetic code templates for test fixture generation.
//!
//! Provides realistic sample source code across Python, Rust, and JavaScript
//! with functions, classes, methods, docstrings, and call sites per D-07.

/// Python code templates for test fixtures.
pub mod python {
    /// Basic math module with functions and docstrings.
    pub const CALCULATOR_V1: &str = r#""""Math utility functions for arithmetic calculations."""

def add(a: int, b: int) -> int:
    """Return the sum of two integers."""
    return a + b

def subtract(a: int, b: int) -> int:
    """Return the difference of two integers."""
    return a - b
"#;

    /// Calculator module updated with multiplication.
    pub const CALCULATOR_V2: &str = r#""""Math utility functions for arithmetic calculations."""

def add(a: int, b: int) -> int:
    """Return the sum of two integers."""
    return a + b

def subtract(a: int, b: int) -> int:
    """Return the difference of two integers."""
    return a - b

def multiply(a: int, b: int) -> int:
    """Return the product of two integers."""
    return a * b
"#;

    /// Calculator module with updated `add` implementation and docstrings.
    pub const CALCULATOR_V3: &str = r#""""Math utility functions for arithmetic calculations with type checking."""

def add(a: int, b: int) -> int:
    """Return the sum of two integers with validation."""
    if not isinstance(a, int) or not isinstance(b, int):
        raise TypeError("Inputs must be integers")
    return a + b

def subtract(a: int, b: int) -> int:
    """Return the difference of two integers."""
    return a - b

def multiply(a: int, b: int) -> int:
    """Return the product of two integers."""
    return a * b
"#;

    /// Calculator module with division and error handling.
    pub const CALCULATOR_V4: &str = r#""""Math utility functions for arithmetic calculations with type checking."""

def add(a: int, b: int) -> int:
    """Return the sum of two integers with validation."""
    if not isinstance(a, int) or not isinstance(b, int):
        raise TypeError("Inputs must be integers")
    return a + b

def subtract(a: int, b: int) -> int:
    """Return the difference of two integers."""
    return a - b

def multiply(a: int, b: int) -> int:
    """Return the product of two integers."""
    return a * b

def divide(a: int, b: int) -> float:
    """Return the division of two numbers, raising ZeroDivisionError if b == 0."""
    if b == 0:
        raise ZeroDivisionError("Cannot divide by zero")
    return float(a) / float(b)
"#;

    /// Calculator module with class-based structure and formatting.
    pub const CALCULATOR_V5: &str = r#""""Math utility calculator class and functions."""

class Calculator:
    """A stateful or static calculator instance."""

    @staticmethod
    def add(a: int, b: int) -> int:
        """Return the sum of two integers."""
        return a + b

    @staticmethod
    def subtract(a: int, b: int) -> int:
        """Return the difference of two integers."""
        return a - b

    @staticmethod
    def multiply(a: int, b: int) -> int:
        """Return the product of two integers."""
        return a * b

    @staticmethod
    def divide(a: int, b: int) -> float:
        """Return division of two numbers."""
        if b == 0:
            raise ZeroDivisionError("division by zero")
        return a / b
"#;

    /// Formatter utility initial version.
    pub const UTILS_FORMATTER: &str = r#""""Formatting and string helper utilities."""

def format_output(value: str, prefix: str = "[INFO]") -> str:
    """Format a message string with a standard prefix tag."""
    return f"{prefix} {value.strip()}"

def format_error(error_msg: str) -> str:
    """Format an error message with error prefix."""
    return f"[ERROR] {error_msg}"
"#;

    /// Formatter utility updated version.
    pub const FORMATTER_UPDATED: &str = r#""""Formatting and string helper utilities."""

def format_output(value: str, prefix: str = "[INFO]", uppercase: bool = false) -> str:
    """Format a message string with prefix and optional capitalization."""
    formatted = f"{prefix} {value.strip()}"
    return formatted.upper() if uppercase else formatted

def format_error(error_msg: str) -> str:
    """Format an error message with error prefix."""
    return f"[ERROR] {error_msg}"
"#;

    /// Monolith file before cross-file move.
    pub const MONOLITH_INITIAL: &str = r#""""Monolithic application entry point containing helper and main logic."""

def helper_fn(data: list[str]) -> list[str]:
    """Clean and filter a list of raw string records."""
    return [item.strip().lower() for item in data if item and item.strip()]

def main_fn() -> None:
    """Execute the main processing pipeline."""
    raw_data = [" Hello ", "", "WORLD", "  gitctx "]
    cleaned = helper_fn(raw_data)
    print("Processed items:", cleaned)

if __name__ == "__main__":
    main_fn()
"#;

    /// Monolith file after helper extraction.
    pub const MONOLITH_REFACTORED: &str = r#""""Monolithic application entry point delegating to helpers."""
from helpers import helper_fn

def main_fn() -> None:
    """Execute the main processing pipeline."""
    raw_data = [" Hello ", "", "WORLD", "  gitctx "]
    cleaned = helper_fn(raw_data)
    print("Processed items:", cleaned)

if __name__ == "__main__":
    main_fn()
"#;

    /// Extracted helper file.
    pub const HELPERS_EXTRACTED: &str = r#""""Extracted helper functions for data processing."""

def helper_fn(data: list[str]) -> list[str]:
    """Clean and filter a list of raw string records."""
    return [item.strip().lower() for item in data if item and item.strip()]
"#;

    /// Extracted helper file modified.
    pub const HELPERS_MODIFIED: &str = r#""""Extracted helper functions for data processing."""

def helper_fn(data: list[str], drop_duplicates: bool = True) -> list[str]:
    """Clean, filter, and optionally deduplicate raw string records."""
    cleaned = [item.strip().lower() for item in data if item and item.strip()]
    if drop_duplicates:
        seen = set()
        result = []
        for x in cleaned:
            if x not in seen:
                seen.add(x)
                result.push(x) if hasattr(result, "push") else result.append(x)
        return result
    return cleaned
"#;

    /// Base app for squash merge.
    pub const APP_BASE: &str = r#""""Application base module."""

def run() -> str:
    """Start application services."""
    return "Application running"
"#;

    /// Auth module added on feature branch.
    pub const AUTH_INITIAL: &str = r#""""Authentication module."""

def authenticate(token: str) -> bool:
    """Verify bearer token validity."""
    return token.startswith("Bearer ") and len(token) > 10
"#;

    /// Auth module refined on feature branch.
    pub const AUTH_TWEAKED: &str = r#""""Authentication module with role checking."""

def authenticate(token: str) -> bool:
    """Verify bearer token validity and expiration."""
    return token.startswith("Bearer ") and len(token) > 15

def get_role(token: str) -> str:
    """Extract role claim from valid token."""
    return "admin" if "admin" in token else "user"
"#;

    /// Auth tests added on feature branch.
    pub const AUTH_TESTS: &str = r#""""Unit tests for authentication module."""
from auth import authenticate, get_role

def test_authenticate():
    assert authenticate("Bearer secret_valid_token")
    assert not authenticate("invalid")

def test_roles():
    assert get_role("Bearer admin_token") == "admin"
    assert get_role("Bearer user_token") == "user"
"#;

    /// Service file for evil merge base.
    pub const SERVICE_BASE: &str = r#""""Data ingestion service."""

def process_data(data: str) -> dict:
    """Process input raw string data into a structured dictionary."""
    return {"status": "ok", "payload": data.strip()}
"#;

    /// Service file branch A (adds timeout).
    pub const SERVICE_BRANCH_A: &str = r#""""Data ingestion service with timeout support."""

def process_data(data: str, timeout: int = 30) -> dict:
    """Process input raw string data with network timeout constraint."""
    return {"status": "ok", "payload": data.strip(), "timeout": timeout}
"#;

    /// Service file branch B (adds retries).
    pub const SERVICE_BRANCH_B: &str = r#""""Data ingestion service with retry support."""

def process_data(data: str, retries: int = 3) -> dict:
    """Process input raw string data with retry capability."""
    return {"status": "ok", "payload": data.strip(), "retries": retries}
"#;

    /// Service file evil merge resolution (introduces both params + extra changes not in parents).
    pub const SERVICE_EVIL_MERGE: &str = r#""""Data ingestion service with timeout, retry, and custom telemetry."""

def process_data(data: str, timeout: int = 30, retries: int = 3) -> dict:
    """Process input raw string data with combined timeout, retries, and evil telemetry."""
    telemetry_tag = "evil_merge_v1"
    return {
        "status": "ok",
        "payload": data.strip(),
        "timeout": timeout,
        "retries": retries,
        "telemetry": telemetry_tag,
    }
"#;

    /// Valid config file alongside broken parser.
    pub const CONFIG_VALID: &str = r#""""Configuration constants."""
APP_NAME = "gitctx-demo"
MAX_WORKERS = 8
TIMEOUT_SECONDS = 60
"#;

    /// Valid parser initial.
    pub const PARSER_VALID: &str = r#""""Payload parser."""

def parse_record(line: str) -> list[str]:
    """Split comma-separated values into a list of strings."""
    return [col.strip() for col in line.split(",")]
"#;

    /// Broken syntax in parser (unclosed bracket/string error).
    pub const PARSER_BROKEN: &str = r#""""Payload parser with syntax error."""

def parse_record(line: str) -> list[str]:
    """Broken implementation with unterminated string literal and missing paren."""
    return [col.strip() for col in line.split(","
    # Syntax error: unclosed parenthesis above and trailing incomplete code
"#;

    /// Fixed parser with new feature.
    pub const PARSER_FIXED: &str = r#""""Payload parser fixed with delimiter support."""

def parse_record(line: str, delimiter: str = ",") -> list[str]:
    """Split delimited values into a list of strings."""
    return [col.strip() for col in line.split(delimiter)]

def parse_batch(lines: list[str]) -> list[list[str]]:
    """Parse multiple lines in batch."""
    return [parse_record(line) for line in lines if line.strip()]
"#;
}

/// Rust code templates for test fixtures.
pub mod rust {
    /// Math library initial version.
    pub const MATH_LIB_V1: &str = r"//! Math utility functions.

/// Calculates the sum of two integers.
pub fn calculate_sum(a: i64, b: i64) -> i64 {
    a + b
}

/// Calculates the product of two integers.
pub fn calculate_product(a: i64, b: i64) -> i64 {
    a * b
}
";

    /// Struct definition and impl.
    pub const POINT_STRUCT: &str = r"//! Geometry types.

/// Represents a point in 2D coordinate space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    /// Creates a new point at (x, y).
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Computes Euclidean distance to another point.
    pub fn distance_to(&self, other: &Point) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}
";
}

/// JavaScript code templates for test fixtures.
pub mod javascript {
    /// String helper utilities.
    pub const STRING_UTILS: &str = r"/**
 * String manipulation helpers.
 */

export function formatText(text, prefix = '[LOG]') {
  return `${prefix} ${text.trim()}`;
}

export function truncate(text, maxLength = 80) {
  if (text.length <= maxLength) return text;
  return `${text.slice(0, maxLength)}...`;
}
";

    /// User manager ES6 class.
    pub const USER_MANAGER: &str = r"/**
 * User session management.
 */

export class UserManager {
  constructor() {
    this.users = new Map();
  }

  addUser(id, name) {
    this.users.set(id, { id, name, createdAt: Date.now() });
  }

  getUser(id) {
    return this.users.get(id) || null;
  }
}
";
}
