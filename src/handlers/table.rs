// Function for creating a new table
// Function for creating new columns for the new table

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Result,
};
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, Execute, PgPool, Postgres, QueryBuilder};
use tracing::{debug, warn};

use crate::error::AppError;

use super::column;

#[derive(Debug, Deserialize)]
pub struct Table {
    name: String,
    columns: Vec<column::BuildColumn>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct TableColumnInfo {
    table_name: String,
    column_name: String,
    data_type: String,
    is_nullable: String,
    character_maximum_length: Option<i32>,
}

pub async fn get_tables(
    State(pool): State<PgPool>,
) -> Result<(StatusCode, axum::Json<Vec<TableColumnInfo>>), AppError> {
    let tables = sqlx::query_as::<_, TableColumnInfo>(
        r#"
        SELECT
            table_name,
            column_name,
            data_type,
            is_nullable,
            character_maximum_length
        FROM
            information_schema.columns
        WHERE
            table_schema = 'public' AND table_name <> '_sqlx_migrations';
        "#,
    )
    .fetch_all(&pool)
    .await?;

    Ok((StatusCode::OK, axum::Json(tables)))
}

pub async fn get_table(
    State(pool): State<PgPool>,
    Path(name): Path<String>,
) -> Result<(StatusCode, axum::Json<TableColumnInfo>), AppError> {
    let table = sqlx::query_as::<_, TableColumnInfo>(
        r#"
        SELECT
            table_name,
            column_name,
            data_type,
            is_nullable,
            character_maximum_length
        FROM
            information_schema.columns
        WHERE
            table_name = ($1);
        "#,
    )
    .bind(name)
    .fetch_one(&pool)
    .await?;

    Ok((StatusCode::OK, axum::Json(table)))
}

pub async fn create_table(
    State(pool): State<PgPool>,
    axum::Json(table): axum::Json<Table>,
) -> Result<(StatusCode, axum::Json<Vec<TableColumnInfo>>), AppError> {
    let mut txn = pool.begin().await?;

    let exists = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT FROM information_schema.tables 
            WHERE table_schema = 'public' AND table_name = ($1)
        );
        "#,
    )
    .bind(&table.name)
    .fetch_one(&mut *txn)
    .await?;

    if exists {
        txn.rollback().await?;

        return Err(AppError::new(
            StatusCode::CONFLICT,
            format!("Table \"{}\" already exists.", table.name),
        ));
    }

    let mut q_builder: QueryBuilder<'_, Postgres> =
        QueryBuilder::new("CREATE TABLE IF NOT EXISTS ");

    q_builder.push(&table.name);

    column::BuildColumn::build_columns(&mut q_builder, &table.columns);

    let sql = q_builder.build().sql();

    debug!("{}", sql);

    sqlx::query(sql).execute(&mut *txn).await?;

    let table = sqlx::query_as::<_, TableColumnInfo>(
        r#"
        SELECT
            table_name,
            column_name,
            data_type,
            is_nullable,
            character_maximum_length
        FROM
            information_schema.columns
        WHERE
            table_name = ($1);
        "#,
    )
    .bind(table.name)
    .fetch_all(&mut *txn)
    .await?;

    txn.commit().await?;

    Ok((StatusCode::CREATED, axum::Json(table)))
}

pub async fn delete_table(
    State(pool): State<PgPool>,
    Path(name): Path<String>,
) -> Result<StatusCode, AppError> {
    warn!("Deleting table: {}", name);

    // NOTE: .bind() doesn't work?
    let sql = format!("DROP TABLE IF EXISTS {}", name);

    sqlx::query(sql.as_str()).execute(&pool).await?;

    Ok(StatusCode::NO_CONTENT)
}
