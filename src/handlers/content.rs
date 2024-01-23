use axum::{extract::State, http::StatusCode, response::Result};
use serde::Deserialize;
use serde_json::Value;
use sqlx::{Execute, PgPool, Postgres, QueryBuilder, Row};
use tracing::debug;

use crate::{error::AppError, utils};

use super::table;

#[derive(Debug, Deserialize)]
pub struct Content {
    table: String,
    // Defaults to '*'
    columns: Option<Vec<table::Column>>,
    // For WHERE clause
    // NOTE: Can only filter one column for now
    filters: Option<table::Column>,
    limit: Option<i64>,
}

// If `limit` is None, fetch all
// If `limit` == 1, fetch one as a single object
// If `limit` is any other number, fetch all based on the limit
pub async fn select(
    State(pool): State<PgPool>,
    axum::Json(row): axum::Json<Content>,
) -> Result<(StatusCode, axum::Json<Value>), AppError> {
    let mut q_builder: QueryBuilder<'_, Postgres> = QueryBuilder::new("SELECT ");

    row.push_select(&mut q_builder);

    let sql = q_builder.build().sql();

    debug!("{}", sql);

    let pg_rows = sqlx::query(sql).fetch_all(&pool).await?;
    let mut json_vec: Vec<Value> = Vec::new();
    let mut json_map = serde_json::Map::new();

    for row in pg_rows.iter() {
        let cols = row.columns();

        utils::insert_col_to_map(row, cols, &mut json_map);

        let json = serde_json::to_value(&json_map)?;

        json_vec.push(json);

        json_map.clear();
    }

    let json: Value = match row.limit {
        Some(1) => serde_json::to_value(json_vec.first().unwrap_or(&Value::Null))?,
        _ => serde_json::to_value(&json_vec)?,
    };

    Ok((StatusCode::OK, axum::Json(json)))
}

// INSERT INTO {table} (rows) VALUES (values)
pub async fn insert(
    State(pool): State<PgPool>,
    axum::Json(row): axum::Json<Content>,
) -> Result<StatusCode, AppError> {
    let mut q_builder: QueryBuilder<'_, Postgres> = QueryBuilder::new("INSERT INTO ");

    q_builder.push(row.table.as_str());

    row.push_insert(&mut q_builder);

    let sql = q_builder.build().sql();

    debug!("{}", sql);

    sqlx::query(sql).execute(&pool).await?;

    Ok(StatusCode::CREATED)
}

// UPDATE {table} SET {row} = {value}, {row} = {value} WHERE {row} = {value}
pub async fn update(
    State(pool): State<PgPool>,
    axum::Json(row): axum::Json<Content>,
) -> Result<StatusCode, AppError> {
    let mut q_builder: QueryBuilder<'_, Postgres> = QueryBuilder::new("UPDATE ");

    q_builder.push(row.table.as_str());

    row.push_update(&mut q_builder);

    let sql = q_builder.build().sql();

    debug!("{}", sql);

    sqlx::query(sql).execute(&pool).await?;

    Ok(StatusCode::OK)
}

impl Content {
    fn push_update(&self, q_builder: &mut QueryBuilder<'_, Postgres>) {
        if let (Some(columns), Some(filters)) = (self.columns.as_ref(), self.filters.as_ref()) {
            let mut comma_sep = q_builder.separated(", ");

            comma_sep.push_unseparated(" SET ");

            columns.iter().for_each(|col| {
                comma_sep.push_unseparated(format_args!("{} = ", col.name));

                match &col.value {
                    Value::String(s) => {
                        comma_sep.push(format_args!("'{}'", s));
                    }
                    _ => {
                        comma_sep.push(&filters.value);
                    }
                }
            });

            self.push_filter(q_builder);
        }
    }

    fn push_insert(&self, q_builder: &mut QueryBuilder<'_, Postgres>) {
        if let Some(ref columns) = self.columns {
            let mut comma_sep = q_builder.separated(", ");

            comma_sep.push_unseparated(" (");

            columns.iter().for_each(|col| {
                comma_sep.push(&col.name);
            });

            comma_sep.push_unseparated(") VALUES (");

            let mut comma_sep = q_builder.separated(", ");

            columns.iter().for_each(|col| match &col.value {
                Value::String(s) => {
                    comma_sep.push(format_args!("'{}'", s));
                }
                _ => {
                    comma_sep.push(&col.value);
                }
            });

            comma_sep.push_unseparated(") ");
        }
    }

    fn push_select(&self, q_builder: &mut QueryBuilder<'_, Postgres>) {
        if let Some(ref columns) = self.columns {
            let mut separated = q_builder.separated(", ");

            columns.iter().for_each(|col| {
                separated.push(&col.name);
            })
        } else {
            q_builder.push("*");
        };

        q_builder.push(format_args!(" FROM {}", self.table));

        self.push_filter(q_builder);

        if let Some(ref limit) = self.limit {
            q_builder.push(format_args!(" LIMIT {}", limit));
        }
    }

    fn push_filter(&self, q_builder: &mut QueryBuilder<'_, Postgres>) {
        if let Some(ref filters) = self.filters {
            q_builder.push(format_args!(" WHERE {} = ", filters.name));

            match &filters.value {
                Value::String(s) => {
                    q_builder.push(format_args!("'{}'", s));
                }
                _ => {
                    q_builder.push(&filters.value);
                }
            }
        }
    }
}
