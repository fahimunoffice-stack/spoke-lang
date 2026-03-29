// src/rules/field_types.rs
// Semantic type inference from field names.
// This is the core "magic" — you write `email` and get a validated email field.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldType {
    // Text
    String { max_len: Option<usize> },
    Text,           // long text, no max
    Email,          // validated email
    Password,       // hashed, never exposed
    Url,            // validated URL
    Slug,           // url-safe string

    // Numbers
    Integer { min: Option<i64>, max: Option<i64> },
    Float,
    Decimal { precision: u8, scale: u8 },  // for prices/money

    // Boolean
    Boolean,

    // Time
    DateTime,
    Date,
    Time,

    // Special
    Status,         // Enum inferred from context
    File,           // file upload reference
    Image,          // image upload reference
    Json,           // arbitrary JSON

    // Relations
    ForeignKey { to: String },
}

impl FieldType {
    /// Prisma type string for web target
    pub fn to_prisma(&self) -> &'static str {
        match self {
            FieldType::String { .. } | FieldType::Email | FieldType::Url |
            FieldType::Slug | FieldType::Password  => "String",
            FieldType::Text                        => "String",
            FieldType::Integer { .. }              => "Int",
            FieldType::Float                       => "Float",
            FieldType::Decimal { .. }              => "Decimal",
            FieldType::Boolean                     => "Boolean",
            FieldType::DateTime | FieldType::Date |
            FieldType::Time                        => "DateTime",
            FieldType::Status                      => "String",
            FieldType::File | FieldType::Image     => "String",  // stored as URL
            FieldType::Json                        => "Json",
            FieldType::ForeignKey { .. }           => "String",  // cuid ref
        }
    }

    /// Dart/Flutter type for mobile target
    pub fn to_dart(&self) -> &'static str {
        match self {
            FieldType::String { .. } | FieldType::Email | FieldType::Url |
            FieldType::Slug | FieldType::Password | FieldType::Text |
            FieldType::Status | FieldType::File | FieldType::Image => "String",
            FieldType::Integer { .. }  => "int",
            FieldType::Float           => "double",
            FieldType::Decimal { .. }  => "double",
            FieldType::Boolean         => "bool",
            FieldType::DateTime        => "DateTime",
            FieldType::Date            => "DateTime",
            FieldType::Time            => "String",
            FieldType::Json            => "Map<String, dynamic>",
            FieldType::ForeignKey {..} => "String",
        }
    }

    /// TypeScript type for web target
    pub fn to_typescript(&self) -> &'static str {
        match self {
            FieldType::String { .. } | FieldType::Email | FieldType::Url |
            FieldType::Slug | FieldType::Password | FieldType::Text |
            FieldType::Status | FieldType::File | FieldType::Image => "string",
            FieldType::Integer { .. }  => "number",
            FieldType::Float           => "number",
            FieldType::Decimal { .. }  => "number",
            FieldType::Boolean         => "boolean",
            FieldType::DateTime        => "Date",
            FieldType::Date            => "Date",
            FieldType::Time            => "string",
            FieldType::Json            => "Record<string, unknown>",
            FieldType::ForeignKey {..} => "string",
        }
    }

    /// Go type for server target
    pub fn to_go(&self) -> &'static str {
        match self {
            FieldType::String { .. } | FieldType::Email | FieldType::Url |
            FieldType::Slug | FieldType::Password | FieldType::Text |
            FieldType::Status | FieldType::File | FieldType::Image => "string",
            FieldType::Integer { .. }  => "int64",
            FieldType::Float           => "float64",
            FieldType::Decimal { .. }  => "float64",
            FieldType::Boolean         => "bool",
            FieldType::DateTime | FieldType::Date => "time.Time",
            FieldType::Time            => "string",
            FieldType::Json            => "map[string]interface{}",
            FieldType::ForeignKey {..} => "string",
        }
    }
}

/// Core function: infer FieldType from field name alone.
/// This is entirely deterministic — same name always gives same type.
pub fn infer_type(name: &str) -> FieldType {
    let name_lower = name.to_lowercase();
    let name_norm  = name_lower.replace('_', "").replace('-', "").replace(' ', "");

    // ── Exact matches first ────────────────────────────────────────────────────
    match name_norm.as_str() {
        "email"           => return FieldType::Email,
        "password" | "pwd"| "pass" | "hashedpassword"
                          => return FieldType::Password,
        "url" | "website" | "link" | "homepage" | "siteurl"
                          => return FieldType::Url,
        "slug"            => return FieldType::Slug,
        "status"          => return FieldType::Status,
        "bio" | "body" | "content" | "description" | "about"
        | "notes" | "message" | "summary" | "details" | "text"
        | "longdescription"
                          => return FieldType::Text,
        "price" | "cost" | "amount" | "total" | "subtotal"
        | "fee" | "rate" | "salary" | "budget" | "revenue"
        | "totalprice" | "totalamount" | "deliverycharge"
                          => return FieldType::Decimal { precision: 10, scale: 2 },
        "age" | "count" | "quantity" | "stock" | "stockcount"
        | "views" | "likes" | "score" | "rank" | "priority"
        | "order" | "position" | "attempts" | "retries"
                          => return FieldType::Integer { min: Some(0), max: None },
        "rating"          => return FieldType::Integer { min: Some(1), max: Some(5) },
        "latitude" | "lat"=> return FieldType::Float,
        "longitude" | "lng" | "lon"
                          => return FieldType::Float,
        "active" | "enabled" | "published" | "verified"
        | "deleted" | "archived" | "featured" | "default"
        | "isactive" | "isenabled" | "ispublished" | "isverified"
                          => return FieldType::Boolean,
        _                 => {}
    }

    // ── Suffix / contains patterns ─────────────────────────────────────────────

    // DateTime patterns
    if name_norm.ends_with("at") || name_norm.ends_with("date")
       || name_norm.ends_with("time") || name_norm.ends_with("on")
       || name_norm.contains("createdat") || name_norm.contains("updatedat")
       || name_norm.contains("deletedat") || name_norm.contains("deadline")
       || name_norm.contains("expiry") || name_norm.contains("expiration")
       || name_norm.contains("birthday") || name_norm.contains("dob")
       || name_norm == "timestamp"
    {
        return FieldType::DateTime;
    }

    // Image/File patterns
    if name_norm.contains("image") || name_norm.contains("photo")
       || name_norm.contains("picture") || name_norm.contains("avatar")
       || name_norm.contains("thumbnail") || name_norm.contains("banner")
       || name_norm.contains("cover") || name_norm.contains("logo")
    {
        return FieldType::Image;
    }

    if name_norm.contains("file") || name_norm.contains("document")
       || name_norm.contains("attachment") || name_norm.contains("upload")
       || name_norm.contains("pdf") || name_norm.contains("video")
       || name_norm.contains("audio")
    {
        return FieldType::File;
    }

    // Boolean patterns
    if name_norm.starts_with("is") || name_norm.starts_with("has")
       || name_norm.starts_with("can") || name_norm.starts_with("show")
       || name_norm.starts_with("allow") || name_norm.starts_with("enable")
    {
        return FieldType::Boolean;
    }

    // Foreign key patterns
    if name_norm.ends_with("id") && name_norm.len() > 2 {
        let entity = &name[..name.len()-2].trim_end_matches('_');
        return FieldType::ForeignKey { to: entity.to_string() };
    }

    if name_norm.ends_with("userid") || name_norm == "owner" || name_norm == "author"
       || name_norm == "creator" || name_norm == "assignee"
    {
        return FieldType::ForeignKey { to: "user".to_string() };
    }

    // Count/number patterns
    if name_norm.ends_with("count") || name_norm.ends_with("number")
       || name_norm.ends_with("num") || name_norm.ends_with("qty")
       || name_norm.ends_with("amount") && !name_norm.contains("price")
    {
        return FieldType::Integer { min: Some(0), max: None };
    }

    // Price patterns
    if name_norm.ends_with("price") || name_norm.ends_with("cost")
       || name_norm.ends_with("fee") || name_norm.ends_with("amount")
    {
        return FieldType::Decimal { precision: 10, scale: 2 };
    }

    // Default: String with reasonable max length
    FieldType::String { max_len: Some(255) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email() {
        assert!(matches!(infer_type("email"), FieldType::Email));
    }

    #[test]
    fn test_password() {
        assert!(matches!(infer_type("password"), FieldType::Password));
    }

    #[test]
    fn test_price() {
        assert!(matches!(infer_type("price"), FieldType::Decimal { .. }));
        assert!(matches!(infer_type("total_price"), FieldType::Decimal { .. }));
    }

    #[test]
    fn test_datetime() {
        assert!(matches!(infer_type("created_at"), FieldType::DateTime));
        assert!(matches!(infer_type("deadline"), FieldType::DateTime));
        assert!(matches!(infer_type("recorded_at"), FieldType::DateTime));
    }

    #[test]
    fn test_boolean() {
        assert!(matches!(infer_type("is_active"), FieldType::Boolean));
        assert!(matches!(infer_type("published"), FieldType::Boolean));
    }

    #[test]
    fn test_image() {
        assert!(matches!(infer_type("profile_picture"), FieldType::Image));
        assert!(matches!(infer_type("avatar"), FieldType::Image));
    }

    #[test]
    fn test_integer() {
        assert!(matches!(infer_type("age"), FieldType::Integer { .. }));
        assert!(matches!(infer_type("stock_count"), FieldType::Integer { .. }));
        assert!(matches!(infer_type("rating"), FieldType::Integer { min: Some(1), max: Some(5) }));
    }

    #[test]
    fn test_text() {
        assert!(matches!(infer_type("description"), FieldType::Text));
        assert!(matches!(infer_type("bio"), FieldType::Text));
    }

    #[test]
    fn test_default_string() {
        assert!(matches!(infer_type("title"), FieldType::String { .. }));
        assert!(matches!(infer_type("name"), FieldType::String { .. }));
    }
}
