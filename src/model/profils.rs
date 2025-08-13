use serde::{Deserialize, Serialize};

// Request untuk membuat profil baru
#[derive(Debug, Deserialize)]
pub struct CreateProfilRequest {
    pub user_id: Option<i32>, // Frontend bisa mengirim user_id
    pub nama: String,
    pub email: String,
    pub no_hp: String,
}

// Request untuk update profil
#[derive(Debug, Deserialize)]
pub struct UpdateProfilRequest {
    pub nama: Option<String>,
    pub email: Option<String>,
    pub no_hp: Option<String>,
}

// Response untuk profil (sesuai dengan frontend)
#[derive(Debug, Serialize)]
pub struct ProfilResponse {
    pub id: String,
    pub nama: String,
    pub email: String,
    pub no_hp: String,
    pub username: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
