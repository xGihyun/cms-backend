use axum::{extract::State, http::StatusCode, response::Result};
use sqlx::PgPool;

pub async fn create_user(State(pool): State<PgPool>) -> Result<StatusCode> {
    sqlx::query(
        r#"
        INSERT INTO users (email, password, first_name, last_name) VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind("email@gmail.com")
    .bind("password")
    .bind("First")
    .bind("Last")
    .execute(&pool)
    .await
    .unwrap();

    Ok(StatusCode::CREATED)
}
