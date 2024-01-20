// Function for creating a new table
// Function for creating new columns for the new table

use axum::{extract::State, http::StatusCode, response::Result};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};
use tracing::debug;

use crate::error::AppError;

#[derive(Debug, Deserialize)]
pub struct Table {
    name: String,
    columns: Vec<Column>,
}

#[derive(Debug, Deserialize)]
pub struct Column {
    name: String,
    data_type: String,
    is_optional: bool,
}

pub async fn create_table(
    State(pool): State<PgPool>,
    axum::Json(payload): axum::Json<Table>,
) -> Result<StatusCode, AppError> {
    let mut q_builder: QueryBuilder<'_, Postgres> =
        QueryBuilder::new(format!("CREATE TABLE IF NOT EXISTS {} (", payload.name));

    // Table name
    // q_builder.push(format_args!(" {} (", payload.name));

    // Table columns
    for (i, column) in payload.columns.iter().enumerate() {
        q_builder.push(format_args!(" {} {} ", column.name, column.data_type));

        if !column.is_optional {
            q_builder.push(" NOT NULL ");
        }

        if i < payload.columns.len() - 1 {
            q_builder.push(" , ");
        }
    }

    q_builder.push(" ) ");

    // TODO: Set "not null" if not optional

    sqlx::query(q_builder.sql()).execute(&pool).await?;

    debug!("{:?}", payload);

    Ok(StatusCode::CREATED)
}

#[derive(Debug, Deserialize)]
pub struct RowData {
    table: String,
    value: serde_json::Value,
}

pub async fn create_row(
    State(pool): State<PgPool>,
    axum::Json(payload): axum::Json<RowData>,
) -> Result<StatusCode, AppError> {
    let obj = payload.value.as_object().ok_or(sqlx::Error::Protocol(
        "Payload must be a JSON object".into(),
    ))?;

    let mut q_builder: QueryBuilder<'_, Postgres> =
        QueryBuilder::new(format!("INSERT INTO {} (", payload.table));

    for (i, k) in obj.keys().into_iter().enumerate() {
        q_builder.push(k);

        if i < obj.len() - 1 {
            q_builder.push(", ");
        }
    }

    q_builder.push(") VALUES (");

    // TODO: Determine data type
    for (i, v) in obj.values().into_iter().enumerate() {
        q_builder.push(format_args!("'{}'", v));

        if i < obj.len() - 1 {
            q_builder.push(", ");
        }
    }

    q_builder.push(") ");

    println!("{}", q_builder.sql());

    sqlx::query(q_builder.sql()).execute(&pool).await?;

    Ok(StatusCode::CREATED)
}
