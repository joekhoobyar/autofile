#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ResourceList<T> {
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub items: Vec<T>,
}
