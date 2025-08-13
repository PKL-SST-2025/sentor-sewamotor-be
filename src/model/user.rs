use sqlx::FromRow;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct User {
    pub id: Uuid,
    pub full_name: String,
    pub username: String,
    pub email: String,
    pub phone: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}