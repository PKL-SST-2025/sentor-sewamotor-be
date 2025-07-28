use axum::{
    Router,
    routing::post,
    extract::{Extension, Json},
    http::StatusCode,
    response::Json as RespJson,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

// Payload untuk register
#[derive(Deserialize)]
pub struct RegisterRequest {
    pub full_name: String,
    pub username: String,
    pub email: String,
    pub phone: String,
    pub password: String,
}

// Payload untuk login
#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

// Response JWT
#[derive(Serialize)]
pub struct TokenResponse {
    pub token: String,
}

// Buat router khusus auth
pub fn auth_router() -> Router {
    Router::new()
        .route("/api/register", post(register))
        .route("/api/login", post(login))
}

// Handler register sederhana (tanpa hash untuk testing)
pub async fn register(
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<RegisterRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    println!("Register attempt - Email: {}, Username: {}, Phone: {}", 
             payload.email, payload.username, payload.phone);
    
    sqlx::query(
        "INSERT INTO users (id, full_name, username, email, phone, password_hash) VALUES ($1,$2,$3,$4,$5,$6)"
    )
    .bind(Uuid::new_v4())
    .bind(payload.full_name)
    .bind(payload.username)
    .bind(payload.email)
    .bind(payload.phone)
    .bind(payload.password) // simpan plain text dulu untuk testing
    .execute(&pool)
    .await
    .map_err(|e| {
        println!("Database insert error: {}", e);
        (StatusCode::CONFLICT, e.to_string())
    })?;

    println!("User registered successfully!");
    Ok(StatusCode::CREATED)
}

// Handler login sederhana (tanpa JWT untuk testing)
pub async fn login(
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<LoginRequest>,
) -> Result<RespJson<TokenResponse>, (StatusCode, String)> {
    println!("Login attempt - Username: {}, Password: {}", payload.username, payload.password);
    
    let row: (Uuid,) = sqlx::query_as(
        "SELECT id FROM users WHERE username = $1 AND password_hash = $2"
    )
    .bind(&payload.username)
    .bind(&payload.password) // cek plain text dulu
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        println!("Database error: {}", e);
        (StatusCode::UNAUTHORIZED, "Username atau password salah".into())
    })?;

    println!("Login successful for user: {}", row.0);
    
    // Return dummy token untuk testing
    Ok(RespJson(TokenResponse { 
        token: format!("dummy_token_for_{}", row.0)
    }))
}