use serde::Deserialize;
use serde::Serialize;

/// Parsed metadata from a `.desktop` file for app search and display.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AppInfo {
    /// Canonical path to the `.desktop` file.
    pub desktop_file: String,
    /// Application name from the `Name` field (falls back to filename stem).
    pub name: String,
    /// Generic name from the `GenericName` field, if present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generic_name: Option<String>,
    /// Comment from the `Comment` field, if present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    /// Keywords from the `Keywords` field, if present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<String>,
    /// Categories from the `Categories` field, if present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub categories: Option<String>,
}

/// Paginated response of available applications.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AvailableAppsResponse {
    /// List of matching applications in the current page.
    pub available_apps: Vec<AppInfo>,
    /// Pagination metadata.
    pub pagination: Pagination,
}

/// Pagination metadata for paginated responses.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Pagination {
    /// Offset of the first item in this page.
    pub offset: usize,
    /// Maximum number of items per page (total count if unlimited).
    pub limit: usize,
    /// Total number of items across all pages.
    pub total: usize,
    /// Number of items returned in this page.
    pub returned: usize,
    /// Whether more pages are available after this one.
    pub has_more: bool,
}
