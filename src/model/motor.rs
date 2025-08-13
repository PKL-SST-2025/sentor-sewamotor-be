use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Motor {
    pub motor_id: i32,
    pub motor_slug: String,
    pub motor_name: String,
    pub motor_type: String,
    pub price_per_day: i32,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub available: Option<bool>,
    pub branch: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMotorRequest {
    pub motor_slug: String,
    pub motor_name: String,
    pub motor_type: String,
    pub price_per_day: i32,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub available: Option<bool>,
    pub branch: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMotorRequest {
    pub motor_slug: Option<String>,
    pub motor_name: Option<String>,
    pub motor_type: Option<String>,
    pub price_per_day: Option<i32>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub available: Option<bool>,
    pub branch: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MotorQuery {
    pub page: Option<i32>,
    pub limit: Option<i32>,
    pub motor_type: Option<String>,
    pub available_only: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct MotorListResponse {
    pub motors: Vec<Motor>,
    pub total: i64,
    pub page: i32,
    pub limit: i32,
}

impl Motor {
    #[allow(dead_code)]
    pub fn new(
        motor_slug: String,
        motor_name: String,
        motor_type: String,
        price_per_day: i32,
        description: Option<String>,
        image_url: Option<String>,
        available: Option<bool>,
        branch: Option<String>,
    ) -> Self {
        Self {
            motor_id: 0, // Will be set by database
            motor_slug,
            motor_name,
            motor_type,
            price_per_day,
            description,
            image_url,
            available,
            branch,
        }
    }

    #[allow(dead_code)]
    pub fn is_available(&self) -> bool {
        self.available.unwrap_or(false)
    }

    #[allow(dead_code)]
    pub fn set_availability(&mut self, available: bool) {
        self.available = Some(available);
    }
}
