// src/models.rs

#[derive(Debug)]
pub struct LogEntry {
    pub id: i32,
    pub timestamp: String, // 在数据库中存储为 RFC3339 字符串
    pub content: String,
    pub tags: Option<String>,
    pub directory: String,
}
