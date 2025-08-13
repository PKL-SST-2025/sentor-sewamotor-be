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
    
    // Connect to PostgreSQL
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url).await.expect("Failed to connect to Postgres");

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

    let addr = "127.0.0.1:8000";
    println!("🚀 Listening on http://{}", addr);

    // Create the TCP listener
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap();
    
    // This is the correct way to run the server in Axum 0.7
    axum::serve(listener, app).await.unwrap();
}