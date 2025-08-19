    use axum::{
    Router,
    routing::get,
    extract::{Extension, Path},
    http::{StatusCode, HeaderMap},
    response::Json as RespJson,
};
use serde_json;
use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;

#[derive(Debug, serde::Serialize)]
struct UserResponse {
    pub id: String,
    pub username: String,
    pub full_name: String,
    pub email: String,
    pub phone: String,
    pub created_at: String,
}

// Helper function untuk ambil user dari token
async fn get_user_from_token(headers: &HeaderMap, pool: &PgPool) -> Result<Uuid, StatusCode> {
    // Ambil token dari header Authorization
    let auth_header = headers
        .get("authorization")
        .and_then(|header| header.to_str().ok())
        .and_then(|header| {
            if header.starts_with("Bearer ") {
                Some(&header[7..])
            } else {
                None
            }
        })
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Parse dummy token format: "dummy_token_for_{user_id}"
    let user_id_str = auth_header.strip_prefix("dummy_token_for_")
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    let user_id = Uuid::parse_str(user_id_str)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Verify user exists in database
    let exists = sqlx::query!("SELECT id FROM users WHERE id = $1", user_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .is_some();

    if !exists {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(user_id)
}

// Create users router
pub fn users_router() -> Router {
    Router::new()
        .route("/:id", get(get_user))  // GET /api/users/{id}
}

// Get user by ID
async fn get_user(
    Extension(pool): Extension<PgPool>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<RespJson<UserResponse>, (StatusCode, RespJson<serde_json::Value>)> {
    println!("🔧 Getting user with ID: {}", id);

    // Verify user authentication
    let _current_user_id = get_user_from_token(&headers, &pool).await
        .map_err(|status| {
            println!("❌ Authentication failed");
            (status, RespJson(serde_json::json!({"error": "Authentication required"})))
        })?;

    // Handle special case for default-id or invalid UUIDs
    if id == "default-id" || id.is_empty() {
        println!("❌ Invalid user ID: {}", id);
        return Err((StatusCode::BAD_REQUEST, RespJson(serde_json::json!({
            "error": "Invalid user ID format. Please provide a valid UUID."
        }))));
    }

    let user_id = Uuid::parse_str(&id).map_err(|e| {
        println!("❌ Invalid UUID format: {} - Error: {}", id, e);
        (StatusCode::BAD_REQUEST, RespJson(serde_json::json!({
            "error": format!("Invalid user ID format: {}", e)
        })))
    })?;

    let result = sqlx::query!(
        "SELECT id, username, full_name, email, phone, created_at FROM users WHERE id = $1",
        user_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        println!("❌ Database error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, RespJson(serde_json::json!({
            "error": "Database error"
        })))
    })?;

    match result {
        Some(user) => {
            let response = UserResponse {
                id: user.id.to_string(),
                username: user.username,
                full_name: user.full_name,
                email: user.email,
                phone: user.phone,
                created_at: user.created_at.unwrap_or_else(|| Utc::now()).format("%Y-%m-%d %H:%M:%S").to_string(),
            };

            println!("✅ User found");
            Ok(RespJson(response))
        }
        None => {
            println!("❌ User not found");
            Err((StatusCode::NOT_FOUND, RespJson(serde_json::json!({
                "error": "User not found"
            }))))
        }
    }
}
