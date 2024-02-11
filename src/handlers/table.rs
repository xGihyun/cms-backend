// Function for creating a new table
// Function for creating new columns for the new table

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Result,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{prelude::FromRow, query_builder, Execute, PgPool, Postgres, QueryBuilder, Row};
use tracing::{debug, info, warn};

use crate::error::AppError;

use super::column;

#[derive(Debug, Deserialize)]
pub struct Table {
    name: String,
    columns: Vec<column::BuildColumn>,
}

#[derive(Debug, Deserialize)]
pub struct EditTable {
    name: String,
    columns: Vec<column::EditColumn>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct TableColumnInfoPk {
    table_name: String,
    column_name: String,
    column_default: Option<String>,
    data_type: String,
    is_nullable: String,
    character_maximum_length: Option<i32>,
    is_primary_key: bool,
}

#[derive(Debug, Serialize, FromRow)]
pub struct TableColumnInfo {
    table_name: String,
    column_name: String,
    column_default: Option<String>,
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
            column_default,
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
) -> Result<(StatusCode, axum::Json<Vec<TableColumnInfoPk>>), AppError> {
    let table = sqlx::query_as::<_, TableColumnInfoPk>(
        r#"
        WITH PrimaryKey AS (
            SELECT 
                a.attname, 
                format_type(a.atttypid, a.atttypmod) AS data_type
            FROM pg_index i
            JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey)
            WHERE i.indrelid = ($1)::regclass
            AND i.indisprimary
        )
        SELECT
            cols.table_name,
            cols.column_name,
            cols.column_default,
            cols.data_type,
            cols.is_nullable,
            cols.character_maximum_length,
            CASE 
                WHEN pk.attname IS NOT NULL THEN true
                ELSE false
            END AS is_primary_key
        FROM
            information_schema.columns AS cols
        LEFT JOIN PrimaryKey pk ON pk.attname = cols.column_name
        WHERE
            table_name = ($1);
        "#,
    )
    .bind(name)
    .fetch_all(&pool)
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
            column_default,
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

pub async fn update_table(
    State(pool): State<PgPool>,
    Path(name): Path<String>,
    axum::Json(table): axum::Json<EditTable>,
) -> Result<StatusCode, AppError> {
    info!("Updating table: {}", name);

    let mut q_builder: QueryBuilder<'_, Postgres> =
        QueryBuilder::new(format!("ALTER TABLE {} ", name));

    let mut comma_sep = q_builder.separated(", ");

    table
        .columns
        .iter()
        .for_each(|col| match col.state.as_str() {
            "added" => {
                info!("Adding column: {}", col.name);
                comma_sep.push(format_args!(
                    "ADD COLUMN IF NOT EXISTS {} {}",
                    col.name.as_str(),
                    col.data_type.as_str()
                ));
            }
            "removed" => {
                warn!("Removing column: {}", col.name);
                comma_sep.push(format_args!("DROP COLUMN IF EXISTS {}", col.name.as_str()));
            }
            "modified" => {
                // NOTE: I might need a reference to the old table in order to modify
                info!("Updating column: {}", col.name);
            }
            "unchanged" => {}
            _ => {
                warn!("Invalid column state.")
            }
        });

    let sql = q_builder.build().sql();

    debug!("{}", sql);

    sqlx::query(sql).execute(&pool).await?;

    Ok(StatusCode::OK)
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

#[derive(Debug, Deserialize)]
pub struct DeleteTableQuery {
    names: String, // Comma separated table names
}

// NOTE: Cascades when dropping tables
pub async fn delete_tables(
    State(pool): State<PgPool>,
    Query(query): Query<DeleteTableQuery>,
) -> Result<StatusCode, AppError> {
    let mut q_builder: QueryBuilder<'_, Postgres> = QueryBuilder::new("DROP TABLE IF EXISTS ");
    // let mut comma_sep = q_builder.separated(", ");
    //
    // tables.iter().for_each(|table| {
    //     warn!("Deleting table: {}", table.name);
    //     comma_sep.push(table.name.as_str());
    // });

    // comma_sep.push_unseparated(" CASCADE");

    q_builder.push(query.names);
    q_builder.push(" CASCADE");

    let sql = q_builder.build().sql();

    debug!("{sql}");

    sqlx::query(sql).execute(&pool).await?;

    Ok(StatusCode::NO_CONTENT)
}
