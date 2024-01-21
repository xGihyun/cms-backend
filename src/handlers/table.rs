// Function for creating a new table
// Function for creating new columns for the new table

use std::fmt::Display;

use axum::{extract::State, http::StatusCode, response::Result};
use serde::{Deserialize, Serialize};
use serde_json::{
    map::{Keys, Values},
    Value,
};
use sqlx::{
    query_builder::{self, Separated},
    Execute, PgPool, Postgres, QueryBuilder,
};
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
    default: Option<Value>,
    is_optional: bool,
}

pub async fn create_table(
    State(pool): State<PgPool>,
    axum::Json(table): axum::Json<Table>,
) -> Result<StatusCode, AppError> {
    let mut q_builder: QueryBuilder<'_, Postgres> =
        QueryBuilder::new("CREATE TABLE IF NOT EXISTS ");

    q_builder.push(&table.name);

    Column::build_columns(&mut q_builder, &table.columns);

    let sql = q_builder.sql();

    debug!("{}", sql);

    sqlx::query(sql).execute(&pool).await?;

    Ok(StatusCode::CREATED)
}

#[derive(Debug, Deserialize)]
pub struct Row {
    table: String,
    value: Value,
}

pub async fn create_row(
    State(pool): State<PgPool>,
    axum::Json(row): axum::Json<Row>,
) -> Result<StatusCode, AppError> {
    let obj = row.value.as_object().ok_or(sqlx::Error::Protocol(
        "Payload must be a JSON object".into(),
    ))?;

    let mut q_builder: QueryBuilder<'_, Postgres> = QueryBuilder::new("INSERT INTO ");

    q_builder.push(row.table.as_str());

    row.push_columns(&mut q_builder, obj.keys());
    row.push_values(&mut q_builder, obj.values());

    let sql = q_builder.build().sql();

    debug!("{}", sql);

    sqlx::query(sql).execute(&pool).await?;

    Ok(StatusCode::CREATED)
}

impl Column {
    fn build_columns(q_builder: &mut QueryBuilder<'_, Postgres>, columns: &Vec<Column>) {
        q_builder.push(" (");

        for (i, column) in columns.iter().enumerate() {
            if !column.is_optional {
                q_builder.push(" NOT NULL ");
            }

            if let Some(ref default) = column.default {
                Self::build_default(q_builder, default, column);
            }

            // NOTE: Set first element as primary key for now
            if i == 0 {
                q_builder.push(" PRIMARY KEY ");
            }

            if i < columns.len() - 1 {
                q_builder.push(", ");
            }
        }

        q_builder.push(") ");
    }

    // TODO: Handle default values better if the default is a function such as gen_random_uuid()
    fn build_default(q_builder: &mut QueryBuilder<'_, Postgres>, default: &Value, column: &Column) {
        match (default, column.data_type.as_str()) {
            (Value::String(s), "uuid") => {
                q_builder.push(format_args!(" DEFAULT {} ", s));
            }
            (Value::String(s), _) => {
                q_builder.push(format_args!(" DEFAULT '{}' ", s));
            }
            _ => {
                q_builder.push(format_args!(" DEFAULT {} ", default));
            }
        }
    }
}

impl Row {
    fn push_columns(
        &self,
        q_builder: &mut QueryBuilder<'_, Postgres>,
        keys: Keys,
    ) -> Result<(), AppError> {
        let mut separated = q_builder.separated(", ");
        separated.push_unseparated(" (");

        for key in keys {
            separated.push(key);
        }

        separated.push_unseparated(") ");

        Ok(())
    }

    fn push_values(&self, q_builder: &mut QueryBuilder<'_, Postgres>, values: Values) {
        let mut separated = q_builder.separated(", ");
        separated.push_unseparated("VALUES (");

        for val in values {
            match val {
                Value::String(s) => {
                    separated.push(format_args!("'{}'", s));
                }
                _ => {
                    separated.push(val);
                }
            }
        }

        separated.push_unseparated(") ");
    }
}
