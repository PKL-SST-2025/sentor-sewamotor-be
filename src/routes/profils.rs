use axum::{
    Router,
    routing::{get, post, put, delete},
    extract::{Extension, Json, Path},
    http::{StatusCode, HeaderMap},
    response::Json as RespJson,
};
use serde_json;
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::model::profils::{CreateProfilRequest, UpdateProfilRequest, ProfilResponse};

// Helper struct for query results - simplified to match profil needs
#[derive(Debug)]
struct UserRow {
    pub id: Uuid,
    pub full_name: String,
    pub email: String,
    pub phone: String,
    pub created_at: Option<DateTime<Utc>>,
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

// Create profils router
pub fn profils_router() -> Router {
    Router::new()
        .route("/", post(create_profil))          // POST /api/profils
        .route("/", get(list_profils))            // GET /api/profils  
        .route("/me", get(get_my_profil))         // GET /api/profils/me - ambil profil user yang login
        .route("/:id", get(get_profil))           // GET /api/profils/{id}
        .route("/:id", put(update_profil))        // PUT /api/profils/{id}
        .route("/:id", delete(delete_profil))     // DELETE /api/profils/{id}
        .route("/user/:user_id", get(get_profil_by_user_id)) // GET /api/profils/user/{user_id}
        .route("/test", get(test_endpoint))       // GET /api/profils/test
}

// Test endpoint
async fn test_endpoint() -> RespJson<serde_json::Value> {
    RespJson(serde_json::json!({
        "message": "Profils endpoint is working!",
        "timestamp": chrono::Utc::now(),
        "available_routes": [
            "GET /api/profils/test",
            "GET /api/profils/me - ambil profil user yang login dari tabel users",
            "GET /api/profils",
            "POST /api/profils",
            "GET /api/profils/{id}",
            "PUT /api/profils/{id}",
            "DELETE /api/profils/{id}",
            "GET /api/profils/user/{user_id}"
        ],
        "note": "Endpoint /me mengambil data profil dari tabel users berdasarkan token JWT"
    }))
}

// Create new profil
async fn create_profil(
    Extension(pool): Extension<PgPool>,
    headers: HeaderMap,
    Json(request): Json<CreateProfilRequest>,
) -> Result<RespJson<ProfilResponse>, (StatusCode, RespJson<serde_json::Value>)> {
    println!("🔧 Creating new profil: {:?}", request);

    // Prioritas user_id: 1. Dari request body, 2. Dari token, 3. Generate baru
    let user_id = if let Some(req_user_id) = request.user_id {
        // Konversi i32 ke Uuid (anggap user_id adalah integer di frontend)
        // Untuk sementara kita buat UUID baru jika user_id dari frontend
        println!("📝 Using user_id from request: {}", req_user_id);
        get_user_from_token(&headers, &pool).await.unwrap_or_else(|_| Uuid::new_v4())
    } else {
        // Fallback ke token authentication
        get_user_from_token(&headers, &pool).await.unwrap_or_else(|_| Uuid::new_v4())
    };

    println!("🔑 Using user_id: {}", user_id);

    // Check if user already exists, if not create new one
    let existing_user = sqlx::query!("SELECT id FROM users WHERE id = $1", user_id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| {
            println!("❌ Database error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, RespJson(serde_json::json!({
                "error": "Database error"
            })))
        })?;

    let result = if existing_user.is_some() {
        // Update existing user - hanya data profil
        sqlx::query_as!(
            UserRow,
            "UPDATE users SET full_name = $2, email = $3, phone = $4 
             WHERE id = $1 
             RETURNING id, full_name, email, phone, created_at",
            user_id,
            request.nama,
            request.email,
            request.no_hp
        )
        .fetch_one(&pool)
        .await
    } else {
        // Insert new user - generate username otomatis untuk keperluan sistem
        let username = request.nama.to_lowercase().replace(" ", "");
        let default_password = "password123";
        
        sqlx::query_as!(
            UserRow,
            "INSERT INTO users (id, full_name, username, email, phone, password_hash, created_at) 
             VALUES ($1, $2, $3, $4, $5, $6, NOW()) 
             RETURNING id, full_name, email, phone, created_at",
            user_id,
            request.nama,
            username,
            request.email,
            request.no_hp,
            default_password
        )
        .fetch_one(&pool)
        .await
    };

    let user = result.map_err(|e| {
        println!("❌ Database operation failed: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, RespJson(serde_json::json!({
            "error": "Failed to save profil"
        })))
    })?;

    let response = ProfilResponse {
        id: user.id.to_string(),
        nama: user.full_name,
        email: user.email,
        no_hp: user.phone,
        username: None, // Tidak perlu username untuk profil
        created_at: user.created_at.unwrap_or_else(|| Utc::now()).format("%Y-%m-%d %H:%M:%S").to_string(),
        updated_at: user.created_at.unwrap_or_else(|| Utc::now()).format("%Y-%m-%d %H:%M:%S").to_string(),
    };

    println!("✅ Profil created/updated successfully");
    Ok(RespJson(response))
}

// Get profil user yang sedang login dari tabel users
async fn get_my_profil(
    Extension(pool): Extension<PgPool>,
    headers: HeaderMap,
) -> Result<RespJson<ProfilResponse>, (StatusCode, RespJson<serde_json::Value>)> {
    println!("🔧 Getting my profil from users table");

    // Ambil user ID dari token
    let current_user_id = get_user_from_token(&headers, &pool).await
        .map_err(|status| {
            println!("❌ Authentication failed");
            (status, RespJson(serde_json::json!({"error": "Authentication required"})))
        })?;

    println!("🔑 Current user ID: {}", current_user_id);

    // Ambil data user dari tabel users
    let result = sqlx::query!(
        "SELECT id, full_name, username, email, phone, created_at FROM users WHERE id = $1",
        current_user_id
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
            let response = ProfilResponse {
                id: user.id.to_string(),
                nama: user.full_name,
                email: user.email,
                no_hp: user.phone,
                username: Some(user.username), // Include username untuk info
                created_at: user.created_at.unwrap_or_else(|| Utc::now()).format("%Y-%m-%d %H:%M:%S").to_string(),
                updated_at: user.created_at.unwrap_or_else(|| Utc::now()).format("%Y-%m-%d %H:%M:%S").to_string(),
            };

            println!("✅ My profil found from users table");
            Ok(RespJson(response))
        }
        None => {
            println!("❌ User not found in users table");
            Err((StatusCode::NOT_FOUND, RespJson(serde_json::json!({
                "error": "User not found"
            }))))
        }
    }
}

// Get profil by user ID
async fn get_profil_by_user_id(
    Extension(pool): Extension<PgPool>,
    headers: HeaderMap,
    Path(user_id): Path<String>,
) -> Result<RespJson<ProfilResponse>, (StatusCode, RespJson<serde_json::Value>)> {
    println!("🔧 Getting profil for user ID: {}", user_id);

    // Verify user authentication
    let _current_user_id = get_user_from_token(&headers, &pool).await
        .map_err(|status| {
            println!("❌ Authentication failed");
            (status, RespJson(serde_json::json!({"error": "Authentication required"})))
        })?;

    // Handle special case for default-id or invalid UUIDs
    if user_id == "default-id" || user_id.is_empty() {
        println!("❌ Invalid user ID: {}", user_id);
        return Err((StatusCode::BAD_REQUEST, RespJson(serde_json::json!({
            "error": "Invalid user ID format. Please provide a valid UUID."
        }))));
    }

    let user_uuid = Uuid::parse_str(&user_id).map_err(|e| {
        println!("❌ Invalid UUID format: {} - Error: {}", user_id, e);
        (StatusCode::BAD_REQUEST, RespJson(serde_json::json!({
            "error": format!("Invalid user ID format: {}", e)
        })))
    })?;

    let result = sqlx::query!(
        "SELECT id, full_name, email, phone, created_at FROM users WHERE id = $1",
        user_uuid
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
            let response = ProfilResponse {
                id: user.id.to_string(),
                nama: user.full_name,
                email: user.email,
                no_hp: user.phone,
                username: None,
                created_at: user.created_at.unwrap_or_else(|| Utc::now()).format("%Y-%m-%d %H:%M:%S").to_string(),
                updated_at: user.created_at.unwrap_or_else(|| Utc::now()).format("%Y-%m-%d %H:%M:%S").to_string(),
            };

            println!("✅ User profil found");
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

// Get profil by ID
async fn get_profil(
    Extension(pool): Extension<PgPool>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<RespJson<ProfilResponse>, (StatusCode, RespJson<serde_json::Value>)> {
    println!("🔧 Getting profil with ID: {}", id);

    // Verify user authentication
    let _current_user_id = get_user_from_token(&headers, &pool).await
        .map_err(|status| {
            println!("❌ Authentication failed");
            (status, RespJson(serde_json::json!({"error": "Authentication required"})))
        })?;

    // Handle special case for default-id or invalid UUIDs
    if id == "default-id" || id.is_empty() {
        println!("❌ Invalid profil ID: {}", id);
        return Err((StatusCode::BAD_REQUEST, RespJson(serde_json::json!({
            "error": "Invalid profil ID format. Please provide a valid UUID."
        }))));
    }

    let user_id = Uuid::parse_str(&id).map_err(|e| {
        println!("❌ Invalid UUID format: {} - Error: {}", id, e);
        (StatusCode::BAD_REQUEST, RespJson(serde_json::json!({
            "error": format!("Invalid profil ID format: {}", e)
        })))
    })?;

    let result = sqlx::query!(
        "SELECT id, full_name, email, phone, created_at FROM users WHERE id = $1",
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
            let response = ProfilResponse {
                id: user.id.to_string(),
                nama: user.full_name,
                email: user.email,
                no_hp: user.phone,
                username: None, // Tidak perlu username untuk profil
                created_at: user.created_at.unwrap_or_else(|| Utc::now()).format("%Y-%m-%d %H:%M:%S").to_string(),
                updated_at: user.created_at.unwrap_or_else(|| Utc::now()).format("%Y-%m-%d %H:%M:%S").to_string(),
            };

            println!("✅ Profil found");
            Ok(RespJson(response))
        }
        None => {
            println!("❌ Profil not found");
            Err((StatusCode::NOT_FOUND, RespJson(serde_json::json!({
                "error": "Profil not found"
            }))))
        }
    }
}

// Update profil
async fn update_profil(
    Extension(pool): Extension<PgPool>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<UpdateProfilRequest>,
) -> Result<RespJson<ProfilResponse>, (StatusCode, RespJson<serde_json::Value>)> {
    println!("🔧 Updating profil with ID: {}", id);

    // Verify user authentication
    let _current_user_id = get_user_from_token(&headers, &pool).await
        .map_err(|status| {
            println!("❌ Authentication failed");
            (status, RespJson(serde_json::json!({"error": "Authentication required"})))
        })?;

    let user_id = Uuid::parse_str(&id).map_err(|_| {
        (StatusCode::BAD_REQUEST, RespJson(serde_json::json!({
            "error": "Invalid ID format"
        })))
    })?;

    // Get current user data
    let current_user = sqlx::query!(
        "SELECT id, full_name, email, phone, created_at FROM users WHERE id = $1",
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

    let current = current_user.ok_or_else(|| {
        (StatusCode::NOT_FOUND, RespJson(serde_json::json!({
            "error": "User not found"
        })))
    })?;

    // Use provided values or keep current ones - hanya untuk profil data
    let new_name = request.nama.unwrap_or(current.full_name.clone());
    let new_email = request.email.unwrap_or(current.email.clone());
    let new_phone = request.no_hp.unwrap_or(current.phone.clone());

    // Update user - hanya update data profil yang diperlukan
    let updated_user = sqlx::query!(
        "UPDATE users SET full_name = $2, email = $3, phone = $4 
         WHERE id = $1 
         RETURNING id, full_name, email, phone, created_at",
        user_id,
        new_name,
        new_email,
        new_phone
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        println!("❌ Database update failed: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, RespJson(serde_json::json!({
            "error": "Failed to update profil"
        })))
    })?;

    let response = ProfilResponse {
        id: updated_user.id.to_string(),
        nama: updated_user.full_name,
        email: updated_user.email,
        no_hp: updated_user.phone,
        username: None, // Tidak perlu username untuk profil
        created_at: updated_user.created_at.unwrap_or_else(|| Utc::now()).format("%Y-%m-%d %H:%M:%S").to_string(),
        updated_at: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    };

    println!("✅ Profil updated successfully");
    Ok(RespJson(response))
}

// Delete profil
async fn delete_profil(
    Extension(pool): Extension<PgPool>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<RespJson<serde_json::Value>, (StatusCode, RespJson<serde_json::Value>)> {
    println!("🔧 Deleting profil with ID: {}", id);

    // Verify user authentication
    let _current_user_id = get_user_from_token(&headers, &pool).await
        .map_err(|status| {
            println!("❌ Authentication failed");
            (status, RespJson(serde_json::json!({"error": "Authentication required"})))
        })?;

    let user_id = Uuid::parse_str(&id).map_err(|_| {
        (StatusCode::BAD_REQUEST, RespJson(serde_json::json!({
            "error": "Invalid ID format"
        })))
    })?;

    let result = sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(&pool)
        .await
        .map_err(|e| {
            println!("❌ Database delete failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, RespJson(serde_json::json!({
                "error": "Failed to delete profil"
            })))
        })?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, RespJson(serde_json::json!({
            "error": "Profil not found"
        }))));
    }

    println!("✅ Profil deleted successfully");
    Ok(RespJson(serde_json::json!({
        "message": "Profil deleted successfully"
    })))
}

// List all profils (admin function)
async fn list_profils(
    Extension(pool): Extension<PgPool>,
    headers: HeaderMap,
) -> Result<RespJson<serde_json::Value>, (StatusCode, RespJson<serde_json::Value>)> {
    println!("🔧 Getting list of profils");

    // Verify user authentication
    let _current_user_id = get_user_from_token(&headers, &pool).await
        .map_err(|status| {
            println!("❌ Authentication failed");
            (status, RespJson(serde_json::json!({"error": "Authentication required"})))
        })?;

    let results = sqlx::query!(
        "SELECT id, full_name, email, phone, created_at FROM users ORDER BY created_at DESC LIMIT 50"
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        println!("❌ Database error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, RespJson(serde_json::json!({
            "error": "Database error"
        })))
    })?;

    let profils: Vec<ProfilResponse> = results.into_iter().map(|user| {
        ProfilResponse {
            id: user.id.to_string(),
            nama: user.full_name,
            email: user.email,
            no_hp: user.phone,
            username: None, // Tidak perlu username untuk profil
            created_at: user.created_at.unwrap_or_else(|| Utc::now()).format("%Y-%m-%d %H:%M:%S").to_string(),
            updated_at: user.created_at.unwrap_or_else(|| Utc::now()).format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }).collect();

    println!("✅ Found {} profils", profils.len());
    Ok(RespJson(serde_json::json!({
        "profils": profils,
        "total": profils.len()
    })))
}
