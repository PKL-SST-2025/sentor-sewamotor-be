use axum::{
    Router,
    routing::{get, post, put, delete},
    extract::{Extension, Json, Path},
    http::{StatusCode, HeaderMap},
    response::Json as RespJson,
};
use sqlx::PgPool;
use uuid::Uuid;
use serde_json;

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

pub fn order_router() -> Router {
    println!("🔧 Registering order routes...");
    Router::new()
        .route("/api/orders", post(create_booking))
        .route("/api/orders/:id", get(get_booking))
        .route("/api/orders/:id", put(update_booking))
        .route("/api/orders/:id", delete(delete_booking))
        .route("/api/orders", get(list_bookings))           // User orders only (with auth)
        .route("/api/orders/all", get(list_all_bookings))   // Admin: all orders
        .route("/api/orders/test", get(test_endpoint))
}

// Test endpoint
async fn test_endpoint() -> RespJson<serde_json::Value> {
    RespJson(serde_json::json!({
        "status": "ok",
        "message": "Orders API is working",
        "timestamp": chrono::Utc::now()
    }))
}

// Create new booking dari form sewa motor
async fn create_booking(
    headers: HeaderMap,
    Extension(pool): Extension<PgPool>,
    Json(payload): Json<serde_json::Value>,
) -> Result<RespJson<serde_json::Value>, (StatusCode, RespJson<serde_json::Value>)> {
    println!("Creating booking with payload: {:?}", payload);
    
    // Authenticate user
    let user_id = match get_user_from_token(&headers, &pool).await {
        Ok(id) => id,
        Err(status) => return Err((status, RespJson(serde_json::json!({"error": "Authentication required"}))))
    };
    
    // Extract booking data dari payload sesuai dengan form sewa motor
    let tanggal_peminjaman = payload.get("tanggalPeminjaman")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, RespJson(serde_json::json!({"error": "Missing tanggalPeminjaman"}))))?;
    
    let jam_peminjaman = payload.get("jamPeminjaman")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, RespJson(serde_json::json!({"error": "Missing jamPeminjaman"}))))?;
    
    let alamat_pengantaran = payload.get("alamatPengantaran")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, RespJson(serde_json::json!({"error": "Missing alamatPengantaran"}))))?;
    
    let tanggal_pengembalian = payload.get("tanggalPengembalian")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, RespJson(serde_json::json!({"error": "Missing tanggalPengembalian"}))))?;
    
    let jam_pengembalian = payload.get("jamPengembalian")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, RespJson(serde_json::json!({"error": "Missing jamPengembalian"}))))?;
    
    let alamat_pengembalian = payload.get("alamatPengembalian")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, RespJson(serde_json::json!({"error": "Missing alamatPengembalian"}))))?;
    
    let pilih_cabang = payload.get("pilihCabang")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, RespJson(serde_json::json!({"error": "Missing pilihCabang"}))))?;
    
    let pilih_motor = payload.get("pilihMotor")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, RespJson(serde_json::json!({"error": "Missing pilihMotor"}))))?;

    // Optional fields  
    let booking_id_value = format!("BWK{}", chrono::Utc::now().timestamp_millis() % 1000000);
    let booking_id = payload.get("bookingId")
        .and_then(|v| v.as_str())
        .unwrap_or(&booking_id_value);
        
    let motor_price = payload.get("motorPrice")
        .and_then(|v| v.as_str())
        .unwrap_or("Rp 50.000/hari");

    // Parse tanggal
    let tanggal_peminjaman_date = chrono::NaiveDate::parse_from_str(tanggal_peminjaman, "%Y-%m-%d")
        .map_err(|_| (StatusCode::BAD_REQUEST, RespJson(serde_json::json!({"error": "Invalid tanggalPeminjaman format"}))))?;
    
    let tanggal_pengembalian_date = chrono::NaiveDate::parse_from_str(tanggal_pengembalian, "%Y-%m-%d")
        .map_err(|_| (StatusCode::BAD_REQUEST, RespJson(serde_json::json!({"error": "Invalid tanggalPengembalian format"}))))?;
    
    let jam_peminjaman_time = chrono::NaiveTime::parse_from_str(jam_peminjaman, "%H:%M")
        .map_err(|_| (StatusCode::BAD_REQUEST, RespJson(serde_json::json!({"error": "Invalid jamPeminjaman format"}))))?;
    
    let jam_pengembalian_time = chrono::NaiveTime::parse_from_str(jam_pengembalian, "%H:%M")
        .map_err(|_| (StatusCode::BAD_REQUEST, RespJson(serde_json::json!({"error": "Invalid jamPengembalian format"}))))?;

    // Insert ke database orders
    let order_id = Uuid::new_v4();
    
    println!("=== SEWA MOTOR INSERT DEBUG ===");
    println!("Order ID: {}", order_id);
    println!("User ID: {}", user_id);
    println!("Booking ID: {}", booking_id);
    println!("Motor: {} - {}", pilih_motor, motor_price);
    println!("Tanggal: {} s/d {}", tanggal_peminjaman, tanggal_pengembalian);
    println!("Cabang: {}", pilih_cabang);
    
    let result = sqlx::query!(
        r#"
        INSERT INTO orders (
            id, user_id, 
            tanggal_peminjaman, jam_peminjaman, alamat_pengantaran,
            tanggal_pengembalian, jam_pengembalian, alamat_pengembalian,
            pilih_cabang, pilih_motor, motor_price,
            status, tanggal_booking, waktu_booking
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'pending', CURRENT_DATE, CURRENT_TIME
        )
        "#,
        order_id,
        user_id,
        tanggal_peminjaman_date,
        jam_peminjaman_time,
        alamat_pengantaran,
        tanggal_pengembalian_date,
        jam_pengembalian_time,
        alamat_pengembalian,
        pilih_cabang,
        pilih_motor,
        motor_price
    )
    .execute(&pool)
    .await;

    match result {
        Ok(_) => {
            println!("✅ Sewa motor booking berhasil disimpan ke database");
            Ok(RespJson(serde_json::json!({
                "success": true,
                "message": "Booking sewa motor berhasil dibuat",
                "booking_id": booking_id,
                "order_id": order_id,
                "data": {
                    "id": order_id,
                    "bookingId": booking_id,
                    "tanggalPeminjaman": tanggal_peminjaman,
                    "jamPeminjaman": jam_peminjaman,
                    "alamatPengantaran": alamat_pengantaran,
                    "tanggalPengembalian": tanggal_pengembalian,
                    "jamPengembalian": jam_pengembalian,
                    "alamatPengembalian": alamat_pengembalian,
                    "pilihCabang": pilih_cabang,
                    "pilihMotor": pilih_motor,
                    "motorPrice": motor_price,
                    "status": "pending"
                }
            })))
        }
        Err(e) => {
            println!("❌ Sewa motor booking database insert failed: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, RespJson(serde_json::json!({"error": format!("Database error: {}", e)}))))
        }
    }
}

// Get booking by ID
async fn get_booking(
    Extension(pool): Extension<PgPool>,
    Path(booking_id): Path<String>,
) -> Result<RespJson<serde_json::Value>, (StatusCode, RespJson<serde_json::Value>)> {
    let order_uuid = Uuid::parse_str(&booking_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, RespJson(serde_json::json!({"error": "Invalid booking ID"}))))?;
    
    let row = sqlx::query!(
        "SELECT id, user_id, tanggal_peminjaman, jam_peminjaman, alamat_pengantaran, tanggal_pengembalian, jam_pengembalian, alamat_pengembalian, pilih_cabang, pilih_motor, motor_price, status, tanggal_booking, waktu_booking FROM orders WHERE id = $1",
        order_uuid
    )
    .fetch_optional(&pool)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, RespJson(serde_json::json!({"error": "Database error"}))))?;
    
    match row {
        Some(order) => {
            Ok(RespJson(serde_json::json!({
                "id": order.id,
                "user_id": order.user_id,
                "bookingId": booking_id,
                "tanggalPeminjaman": order.tanggal_peminjaman,
                "jamPeminjaman": order.jam_peminjaman,
                "alamatPengantaran": order.alamat_pengantaran,
                "tanggalPengembalian": order.tanggal_pengembalian,
                "jamPengembalian": order.jam_pengembalian,
                "alamatPengembalian": order.alamat_pengembalian,
                "pilihCabang": order.pilih_cabang,
                "pilihMotor": order.pilih_motor,
                "motorPrice": order.motor_price,
                "status": order.status,
                "tanggalBooking": order.tanggal_booking,
                "waktuBooking": order.waktu_booking
            })))
        }
        None => Err((StatusCode::NOT_FOUND, RespJson(serde_json::json!({"error": "Booking not found"}))))
    }
}

// Update booking status
async fn update_booking(
    Extension(pool): Extension<PgPool>,
    Path(booking_id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Result<RespJson<serde_json::Value>, (StatusCode, RespJson<serde_json::Value>)> {
    let order_uuid = Uuid::parse_str(&booking_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, RespJson(serde_json::json!({"error": "Invalid booking ID"}))))?;
    
    let status = payload.get("status").and_then(|v| v.as_str()).unwrap_or("pending");
    
    let result = sqlx::query!(
        "UPDATE orders SET status = $1 WHERE id = $2",
        status,
        order_uuid
    )
    .execute(&pool)
    .await;

    match result {
        Ok(query_result) => {
            if query_result.rows_affected() > 0 {
                Ok(RespJson(serde_json::json!({
                    "success": true,
                    "message": "Booking status updated successfully"
                })))
            } else {
                Err((StatusCode::NOT_FOUND, RespJson(serde_json::json!({"error": "Booking not found"}))))
            }
        }
        Err(e) => {
            println!("Update booking error: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, RespJson(serde_json::json!({"error": "Database error"}))))
        }
    }
}

// Delete booking
async fn delete_booking(
    Extension(pool): Extension<PgPool>,
    Path(booking_id): Path<String>,
) -> Result<RespJson<serde_json::Value>, (StatusCode, RespJson<serde_json::Value>)> {
    let order_uuid = Uuid::parse_str(&booking_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, RespJson(serde_json::json!({"error": "Invalid booking ID"}))))?;
    
    let result = sqlx::query!(
        "DELETE FROM orders WHERE id = $1",
        order_uuid
    )
    .execute(&pool)
    .await;

    match result {
        Ok(query_result) => {
            if query_result.rows_affected() > 0 {
                Ok(RespJson(serde_json::json!({
                    "success": true,
                    "message": "Booking deleted successfully"
                })))
            } else {
                Err((StatusCode::NOT_FOUND, RespJson(serde_json::json!({"error": "Booking not found"}))))
            }
        }
        Err(e) => {
            println!("Delete booking error: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, RespJson(serde_json::json!({"error": "Database error"}))))
        }
    }
}

// List bookings untuk user yang sedang login (dengan authentication)
async fn list_bookings(
    headers: HeaderMap,
    Extension(pool): Extension<PgPool>,
) -> Result<RespJson<serde_json::Value>, (StatusCode, RespJson<serde_json::Value>)> {
    // Authenticate user
    let user_id = match get_user_from_token(&headers, &pool).await {
        Ok(id) => id,
        Err(status) => return Err((status, RespJson(serde_json::json!({"error": "Authentication required"}))))
    };

    println!("🔍 Fetching orders for user: {}", user_id);

    // Query orders hanya untuk user yang sedang login
    let rows = sqlx::query!(
        "SELECT id, user_id, tanggal_peminjaman, jam_peminjaman, alamat_pengantaran, tanggal_pengembalian, jam_pengembalian, alamat_pengembalian, pilih_cabang, pilih_motor, motor_price, status, tanggal_booking, waktu_booking FROM orders WHERE user_id = $1 ORDER BY tanggal_booking DESC, waktu_booking DESC",
        user_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        println!("❌ Database error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, RespJson(serde_json::json!({"error": "Database error"})))
    })?;
    
    println!("✅ Found {} orders for user {}", rows.len(), user_id);
    
    let bookings: Vec<serde_json::Value> = rows.into_iter().map(|row| {
        serde_json::json!({
            "id": row.id,
            "user_id": row.user_id,
            "bookingId": format!("BWK{}", row.id.to_string().chars().take(6).collect::<String>()),
            "tanggalPeminjaman": row.tanggal_peminjaman,
            "jamPeminjaman": row.jam_peminjaman,
            "alamatPengantaran": row.alamat_pengantaran,
            "tanggalPengembalian": row.tanggal_pengembalian,
            "jamPengembalian": row.jam_pengembalian,
            "alamatPengembalian": row.alamat_pengembalian,
            "pilihCabang": row.pilih_cabang,
            "pilihMotor": row.pilih_motor,
            "motorPrice": row.motor_price,
            "status": row.status,
            "tanggalBooking": row.tanggal_booking,
            "waktuBooking": row.waktu_booking
        })
    }).collect();

    Ok(RespJson(serde_json::json!({
        "success": true,
        "data": bookings,
        "total": bookings.len(),
        "user_id": user_id
    })))
}

// Admin endpoint: List ALL bookings (tanpa filter user_id)
async fn list_all_bookings(
    Extension(pool): Extension<PgPool>,
) -> Result<RespJson<serde_json::Value>, (StatusCode, RespJson<serde_json::Value>)> {
    println!("🔍 Admin: Fetching all orders");

    let rows = sqlx::query!(
        "SELECT o.id, o.user_id, u.username, o.tanggal_peminjaman, o.jam_peminjaman, o.alamat_pengantaran, o.tanggal_pengembalian, o.jam_pengembalian, o.alamat_pengembalian, o.pilih_cabang, o.pilih_motor, o.motor_price, o.status, o.tanggal_booking, o.waktu_booking FROM orders o JOIN users u ON o.user_id = u.id ORDER BY o.tanggal_booking DESC, o.waktu_booking DESC"
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        println!("❌ Database error: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, RespJson(serde_json::json!({"error": "Database error"})))
    })?;
    
    println!("✅ Found {} total orders", rows.len());
    
    let bookings: Vec<serde_json::Value> = rows.into_iter().map(|row| {
        serde_json::json!({
            "id": row.id,
            "user_id": row.user_id,
            "username": row.username,  // Include username for admin
            "bookingId": format!("BWK{}", row.id.to_string().chars().take(6).collect::<String>()),
            "tanggalPeminjaman": row.tanggal_peminjaman,
            "jamPeminjaman": row.jam_peminjaman,
            "alamatPengantaran": row.alamat_pengantaran,
            "tanggalPengembalian": row.tanggal_pengembalian,
            "jamPengembalian": row.jam_pengembalian,
            "alamatPengembalian": row.alamat_pengembalian,
            "pilihCabang": row.pilih_cabang,
            "pilihMotor": row.pilih_motor,
            "motorPrice": row.motor_price,
            "status": row.status,
            "tanggalBooking": row.tanggal_booking,
            "waktuBooking": row.waktu_booking
        })
    }).collect();

    Ok(RespJson(serde_json::json!({
        "success": true,
        "data": bookings,
        "total": bookings.len(),
        "type": "admin_view"
    })))
}