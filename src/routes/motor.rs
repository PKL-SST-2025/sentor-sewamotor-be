use axum::{
    Router,
    routing::{get, post, put, delete},
    extract::{Extension, Json, Path, Query},
    http::StatusCode,
    response::Json as RespJson,
};
use sqlx::{PgPool, Row};
use serde_json;
use crate::model::motor::{
    Motor,
    CreateMotorRequest,
    UpdateMotorRequest,
    MotorQuery,
    MotorListResponse,
};

pub fn motor_router() -> Router {
    println!("🔧 Registering motor routes...");
    Router::new()
        .route("/api/motors", get(list_motors))
        .route("/api/motors", post(create_motor))
        .route("/api/motors/:id", get(get_motor))
        .route("/api/motors/:id", put(update_motor))
        .route("/api/motors/:id", delete(delete_motor))
        .route("/api/motors/test", get(test_endpoint))
}

// Test endpoint
async fn test_endpoint() -> RespJson<serde_json::Value> {
    RespJson(serde_json::json!({
        "status": "ok",
        "message": "Motors API is working",
        "timestamp": chrono::Utc::now()
    }))
}

// List all motors with pagination and filtering
async fn list_motors(
    Extension(pool): Extension<PgPool>,
    Query(params): Query<MotorQuery>,
) -> Result<RespJson<MotorListResponse>, (StatusCode, RespJson<serde_json::Value>)> {
    println!("📋 Listing motors with params: {:?}", params);
    
    let page = params.page.unwrap_or(1).max(1);
    let limit = params.limit.unwrap_or(10).min(100).max(1);
    let offset = (page - 1) * limit;
    
    // Build base query
    let mut where_clauses = Vec::new();
    let mut param_count = 1;
    
    if params.motor_type.is_some() {
        where_clauses.push(format!("motor_type = ${}", param_count));
        param_count += 1;
    }
    
    if params.available_only.unwrap_or(false) {
        where_clauses.push(format!("available = ${}", param_count));
        param_count += 1;
    }
    
    let where_clause = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };
    
    // Count total records
    let count_query = format!("SELECT COUNT(*) as total FROM motors {}", where_clause);
    let mut count_query_builder = sqlx::query(&count_query);
    
    if let Some(motor_type) = &params.motor_type {
        count_query_builder = count_query_builder.bind(motor_type);
    }
    if params.available_only.unwrap_or(false) {
        count_query_builder = count_query_builder.bind(true);
    }
    
    let total_row = count_query_builder
        .fetch_one(&pool)
        .await
        .map_err(|e| {
            println!("🚨 Database error counting records: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, RespJson(serde_json::json!({
                "error": "Database error"
            })))
        })?;
    
    let total: i64 = total_row.try_get("total").unwrap_or(0);
    
    // Fetch records
    let fetch_query = format!(
        "SELECT motor_id, motor_slug, motor_name, motor_type, price_per_day, description, image_url, available, branch
         FROM motors {} ORDER BY motor_id ASC LIMIT ${} OFFSET ${}",
        where_clause, param_count, param_count + 1
    );
    
    let mut fetch_query_builder = sqlx::query(&fetch_query);
    
    if let Some(motor_type) = &params.motor_type {
        fetch_query_builder = fetch_query_builder.bind(motor_type);
    }
    if params.available_only.unwrap_or(false) {
        fetch_query_builder = fetch_query_builder.bind(true);
    }
    
    fetch_query_builder = fetch_query_builder.bind(limit).bind(offset);
    
    let rows = fetch_query_builder
        .fetch_all(&pool)
        .await
        .map_err(|e| {
            println!("🚨 Database error fetching records: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, RespJson(serde_json::json!({
                "error": "Database error"
            })))
        })?;
    
    let motors: Vec<Motor> = rows
        .iter()
        .map(|row| {
            Motor {
                motor_id: row.try_get("motor_id").unwrap(),
                motor_slug: row.try_get("motor_slug").unwrap_or_else(|_| "unknown".to_string()),
                motor_name: row.try_get("motor_name").unwrap(),
                motor_type: row.try_get("motor_type").unwrap(),
                price_per_day: row.try_get("price_per_day").unwrap(),
                description: row.try_get("description").ok(),
                image_url: row.try_get("image_url").ok(),
                available: row.try_get("available").ok(),
                branch: row.try_get("branch").ok(),
            }
        })
        .collect();
    
    let response = MotorListResponse {
        motors,
        total,
        page,
        limit,
    };
    
    Ok(RespJson(response))
}

// Get motor by ID
async fn get_motor(
    Extension(pool): Extension<PgPool>,
    Path(motor_id): Path<i32>,
) -> Result<RespJson<Motor>, (StatusCode, RespJson<serde_json::Value>)> {
    println!("🔍 Getting motor with ID: {}", motor_id);
    
    let row = sqlx::query(
        "SELECT motor_id, motor_slug, motor_name, motor_type, price_per_day, description, image_url, available, branch
         FROM motors WHERE motor_id = $1"
    )
    .bind(motor_id)
    .fetch_optional(&pool)
        .await
    .map_err(|e| {
        println!("🚨 Database error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, RespJson(serde_json::json!({
            "error": "Database error"
        })))
    })?;
    
    match row {
        Some(motor_row) => {
            let motor = Motor {
                motor_id: motor_row.try_get("motor_id").unwrap(),
                motor_slug: motor_row.try_get("motor_slug").unwrap_or_else(|_| "unknown".to_string()),
                motor_name: motor_row.try_get("motor_name").unwrap(),
                motor_type: motor_row.try_get("motor_type").unwrap(),
                price_per_day: motor_row.try_get("price_per_day").unwrap(),
                description: motor_row.try_get("description").ok(),
                image_url: motor_row.try_get("image_url").ok(),
                available: motor_row.try_get("available").ok(),
                branch: motor_row.try_get("branch").ok(),
            };
            
            Ok(RespJson(motor))
        }
        None => {
            Err((StatusCode::NOT_FOUND, RespJson(serde_json::json!({
                "error": "Motor not found"
            }))))
        }
    }
}

// Create new motor
async fn create_motor(
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<CreateMotorRequest>,
) -> Result<RespJson<Motor>, (StatusCode, RespJson<serde_json::Value>)> {
    println!("=== CREATE MOTOR DEBUG ===");
    println!("Motor slug: {}", payload.motor_slug);
    println!("Motor name: {}", payload.motor_name);
    println!("Motor type: {}", payload.motor_type);
    println!("Price per day: {}", payload.price_per_day);
    println!("Available: {:?}", payload.available);
    
    // Insert motor into database
    let result = sqlx::query(
        "INSERT INTO motors (motor_slug, motor_name, motor_type, price_per_day, description, image_url, available, branch) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) 
         RETURNING motor_id, motor_slug, motor_name, motor_type, price_per_day, description, image_url, available, branch"
    )
    .bind(&payload.motor_slug)
    .bind(&payload.motor_name)
    .bind(&payload.motor_type)
    .bind(payload.price_per_day)
    .bind(&payload.description)
    .bind(&payload.image_url)
    .bind(payload.available.unwrap_or(true))
    .bind(&payload.branch)
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        println!("🚨 DATABASE INSERT ERROR: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, RespJson(serde_json::json!({
            "error": format!("Database error: {}", e)
        })))
    })?;

    let motor = Motor {
        motor_id: result.try_get("motor_id").unwrap(),
        motor_slug: result.try_get("motor_slug").unwrap(),
        motor_name: result.try_get("motor_name").unwrap(),
        motor_type: result.try_get("motor_type").unwrap(),
        price_per_day: result.try_get("price_per_day").unwrap(),
        description: result.try_get("description").ok(),
        image_url: result.try_get("image_url").ok(),
        available: result.try_get("available").ok(),
        branch: result.try_get("branch").ok(),
    };

    println!("Motor created successfully with ID: {}", motor.motor_id);
    Ok(RespJson(motor))
}

// Update motor
async fn update_motor(
    Extension(pool): Extension<PgPool>,
    Path(motor_id): Path<i32>,
    Json(payload): Json<UpdateMotorRequest>,
) -> Result<RespJson<Motor>, (StatusCode, RespJson<serde_json::Value>)> {
    println!("🔄 Updating motor with ID: {}", motor_id);
    
    // Build dynamic update query
    let mut query_parts = Vec::new();
    let mut param_count = 1;
    
    if payload.motor_slug.is_some() {
        query_parts.push(format!("motor_slug = ${}", param_count));
        param_count += 1;
    }
    
    if payload.motor_name.is_some() {
        query_parts.push(format!("motor_name = ${}", param_count));
        param_count += 1;
    }
    
    if payload.motor_type.is_some() {
        query_parts.push(format!("motor_type = ${}", param_count));
        param_count += 1;
    }
    
    if payload.price_per_day.is_some() {
        query_parts.push(format!("price_per_day = ${}", param_count));
        param_count += 1;
    }
    
    if payload.description.is_some() {
        query_parts.push(format!("description = ${}", param_count));
        param_count += 1;
    }
    
    if payload.image_url.is_some() {
        query_parts.push(format!("image_url = ${}", param_count));
        param_count += 1;
    }
    
    if payload.available.is_some() {
        query_parts.push(format!("available = ${}", param_count));
        param_count += 1;
    }
    
    if payload.branch.is_some() {
        query_parts.push(format!("branch = ${}", param_count));
        param_count += 1;
    }
    
    if query_parts.is_empty() {
        return Err((StatusCode::BAD_REQUEST, RespJson(serde_json::json!({
            "error": "No valid fields to update"
        }))));
    }
    
    let query_str = format!(
        "UPDATE motors SET {} WHERE motor_id = ${} RETURNING motor_id, motor_slug, motor_name, motor_type, price_per_day, description, image_url, available, branch",
        query_parts.join(", "),
        param_count
    );
    
    let mut query = sqlx::query(&query_str);
    
    // Bind parameters in the same order as query_parts
    if let Some(motor_slug) = &payload.motor_slug {
        query = query.bind(motor_slug);
    }
    if let Some(motor_name) = &payload.motor_name {
        query = query.bind(motor_name);
    }
    if let Some(motor_type) = &payload.motor_type {
        query = query.bind(motor_type);
    }
    if let Some(price_per_day) = payload.price_per_day {
        query = query.bind(price_per_day);
    }
    if let Some(description) = &payload.description {
        query = query.bind(description);
    }
    if let Some(image_url) = &payload.image_url {
        query = query.bind(image_url);
    }
    if let Some(available) = payload.available {
        query = query.bind(available);
    }
    if let Some(branch) = &payload.branch {
        query = query.bind(branch);
    }
    
    query = query.bind(motor_id);
    
    let row = query
        .fetch_optional(&pool)
        .await
        .map_err(|e| {
            println!("🚨 Database error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, RespJson(serde_json::json!({
                "error": "Database error"
            })))
        })?;
    
    match row {
        Some(motor_row) => {
            let motor = Motor {
                motor_id: motor_row.try_get("motor_id").unwrap(),
                motor_slug: motor_row.try_get("motor_slug").unwrap_or_else(|_| "unknown".to_string()),
                motor_name: motor_row.try_get("motor_name").unwrap(),
                motor_type: motor_row.try_get("motor_type").unwrap(),
                price_per_day: motor_row.try_get("price_per_day").unwrap(),
                description: motor_row.try_get("description").ok(),
                image_url: motor_row.try_get("image_url").ok(),
                available: motor_row.try_get("available").ok(),
                branch: motor_row.try_get("branch").ok(),
            };
            
            Ok(RespJson(motor))
        }
        None => {
            Err((StatusCode::NOT_FOUND, RespJson(serde_json::json!({
                "error": "Motor not found"
            }))))
        }
    }
}

// Delete motor
async fn delete_motor(
    Extension(pool): Extension<PgPool>,
    Path(motor_id): Path<i32>,
) -> Result<RespJson<serde_json::Value>, (StatusCode, RespJson<serde_json::Value>)> {
    println!("🗑️ Deleting motor with ID: {}", motor_id);
    
    let result = sqlx::query("DELETE FROM motors WHERE motor_id = $1")
        .bind(motor_id)
        .execute(&pool)
        .await
        .map_err(|e| {
            println!("🚨 Database error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, RespJson(serde_json::json!({
                "error": "Database error"
            })))
        })?;
    
    if result.rows_affected() == 0 {
        Err((StatusCode::NOT_FOUND, RespJson(serde_json::json!({
            "error": "Motor not found"
        }))))
    } else {
        Ok(RespJson(serde_json::json!({
            "message": "Motor deleted successfully"
        })))
    }
}
