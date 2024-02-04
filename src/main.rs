// Ignore unused imports for now to remove some noise
// #![allow(unused_imports)]
// #![allow(warnings)]

use axum::{
    http,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use dotenv::dotenv;
use std::env;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::EnvFilter;

mod error;
mod handlers;
mod utils;

use handlers::{row, table, user};

#[tokio::main]
async fn main() -> anyhow::Result<(), anyhow::Error> {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let db_url = env::var("DATABASE_URL")
        .unwrap_or("postgres://dbuser:password@localhost:5432/cms".to_string());
    let ip_addr = env::var("IP_ADDRESS").unwrap_or("127.0.0.1".to_string());
    let max_connections = env::var("MAX_CONNECTIONS")
        .unwrap_or("10".to_string())
        .parse::<u32>()?;

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(&db_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let app = Router::new()
        .route("/", get(health))
        .route("/users", post(user::create_user))
        .route(
            "/tables",
            get(table::get_tables)
                .post(table::create_table)
                .delete(table::delete_tables),
        )
        .route(
            "/tables/:name",
            get(table::get_table).delete(table::delete_table),
        )
        .route(
            "/rows",
            get(row::select_many)
                .post(row::insert)
                .patch(row::update)
                .delete(row::delete),
        )
        .route("/rows/:id", get(row::select_one))
        .layer(CorsLayer::permissive())
        .with_state(pool);

    let listener = TcpListener::bind(format!("{}:8000", ip_addr)).await?;

    info!("{:<12} - {}", "LISTENING", listener.local_addr()?);

    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

async fn health() -> impl IntoResponse {
    (http::StatusCode::OK, "Hello, World!")
}
