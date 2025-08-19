use axum::{
    routing::get,
    extract::Extension,
    Router,
};
use tower_http::{
    services::{ServeDir, ServeFile},
    cors::{CorsLayer, Any},
};
use dotenv::dotenv;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;

mod routes;
mod model;
use routes::auth::auth_router;
use routes::orders::order_router;
use routes::motor::motor_router;
use routes::profils::profils_router;
use routes::users::users_router;

#[tokio::main]
async fn main() {
    dotenv().ok();
    
    // Connect to PostgreSQL with retry & better diagnostics
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    if database_url == "postgresql://username:password@host:port/dbname" {
        eprintln!("❌ DATABASE_URL masih default template. Ubah di file .env ke nilai sebenarnya. Contoh: postgresql://postgres:123@localhost:5432/sentor_db");
    }

    println!("🔌 Mencoba konek ke Postgres: {}", database_url);

    // Simpan pesan error terakhir (string) untuk debugging jika semua attempt gagal
    let mut last_err: Option<String> = None;
    let mut pool_opt: Option<PgPool> = None;
    for attempt in 1..=5 {
        println!("➡️  Attempt {}/5 ...", attempt);
        match PgPoolOptions::new()
            .max_connections(10)
            .min_connections(1)
            .acquire_timeout(Duration::from_secs(8))
            .idle_timeout(Duration::from_secs(300))
            .connect(&database_url).await {
            Ok(p) => {
                println!("✅ Berhasil konek ke Postgres pada attempt {}", attempt);
                pool_opt = Some(p);
                break;
            }
            Err(e) => {
                eprintln!("⚠️  Gagal attempt {}: {}", attempt, e);
                last_err = Some(e.to_string());
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }
    }

    if pool_opt.is_none() {
        if let Some(err_msg) = last_err {
            panic!("Failed to connect to Postgres after retries. Last error: {}", err_msg);
        } else {
            panic!("Failed to connect to Postgres after retries (no detailed error captured)");
        }
    }
    let pool = pool_opt.unwrap();

    // Quick sanity check query
    if let Err(e) = sqlx::query("SELECT 1").execute(&pool).await {
        eprintln!("⚠️  Query test SELECT 1 gagal: {}", e);
    }

    let serve_dir = ServeDir::new("../fe/dist")
        .not_found_service(ServeFile::new("../fe/dist/index.html"));

    let app = Router::new()
        // Merge auth routes (register & login)
        .merge(auth_router())
        // Merge order routes (orders & bookings)
        .merge(order_router())
        // Merge motor routes (motors CRUD)
        .merge(motor_router())
        // Merge profils routes (profils CRUD)
        .nest("/api/profils", profils_router())
        // Merge users routes (users CRUD)
        .nest("/api/users", users_router())
        // Your API routes should come first
        .route("/api/hello", get(|| async { "Hello from your Axum backend!" }))
        
        // This makes the static file service handle all other requests
        .fallback_service(serve_dir)
        // Add database pool
        .layer(Extension(pool))
        // Add CORS for frontend
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any));

    let host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("SERVER_PORT").unwrap_or_else(|_| "8000".to_string());
    let addr = format!("{}:{}", host, port);
    println!("🚀 Listening on http://{}", addr);
    println!("📦 Pool status: max=10");

    // Create the TCP listener
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap();
    
    // This is the correct way to run the server in Axum 0.7
    axum::serve(listener, app).await.unwrap();
}