use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct User {
    pub id: Uuid,
    pub full_name: String,
    pub username: String,
    pub email: String,
    pub phone: String,
    pub password_hash: String,
}