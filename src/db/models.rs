use serde::{Deserialize, Serialize};

#[derive(Debug, sqlx::FromRow, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub claim: String,
    pub verdict: Option<String>,
    pub reasoning: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, sqlx::FromRow, Serialize, Deserialize)]
pub struct Turn {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: i64,
}
