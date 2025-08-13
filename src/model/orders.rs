use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};

// Model utama untuk Order (sesuai dengan database)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Order {
    pub id: Uuid,
    pub user_id: Uuid,
    pub motor_id: i32,
    
    // Data peminjaman
    pub tanggal_peminjaman: NaiveDate,     // pickup_date
    pub jam_peminjaman: NaiveTime,         // pickup_time
    pub alamat_pengantaran: String,        // pickup_address
    
    // Data pengembalian
    pub tanggal_pengembalian: NaiveDate,   // return_date
    pub jam_pengembalian: NaiveTime,       // return_time
    pub alamat_pengembalian: String,       // return_address
    
    // Data booking
    pub pilih_cabang: String,              // branch
    pub pilih_motor: String,               // motor_name
    pub motor_price: String,               // harga motor
    pub status: String,
    
    // Metadata
    pub tanggal_booking: NaiveDate,
    pub waktu_booking: NaiveTime,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

// Response untuk frontend (sesuai dengan struktur yang diminta)
#[derive(Debug, Serialize)]
pub struct OrderResponse {
    pub id: String,
    
    // Data peminjaman
    pub tanggal_peminjaman: String,        // pickup_date
    pub jam_peminjaman: String,            // pickup_time
    pub alamat_pengantaran: String,        // pickup_address
    
    // Data pengembalian
    pub tanggal_pengembalian: String,      // return_date
    pub jam_pengembalian: String,          // return_time
    pub alamat_pengembalian: String,       // return_address
    
    // Data booking
    pub pilih_cabang: String,              // branch
    pub pilih_motor: String,               // motor_name
    pub motor_price: String,               // harga motor
    pub status: String,
    
    // Metadata booking
    pub tanggal_booking: String,           // booking date
    pub waktu_booking: String,             // booking time
}

// List response dengan pagination
#[derive(Debug, Serialize)]
pub struct OrderListResponse {
    pub orders: Vec<OrderResponse>,
    pub total: i64,
    pub page: i32,
    pub limit: i32,
}
