// Function for creating a new table
// Function for creating new columns for the new table

use axum::{extract::State, http::StatusCode, response::Result};
use serde::Deserialize;
use serde_json::Value;
use sqlx::{Execute, PgPool, Postgres, QueryBuilder};
use tracing::debug;

use crate::error::AppError;

#[derive(Debug, Deserialize)]
pub struct Table {
    name: String,
    columns: Vec<InitColumn>,
}

#[derive(Debug, Deserialize)]
pub struct InitColumn {
    name: String,
    data_type: String,
    default: Option<Value>, // Optional default value
    is_nullable: bool,      // Sets column to NOT NULL if true
}

#[derive(Debug, Deserialize)]
pub struct Column {
    pub name: String,
    pub value: Value,
}

pub async fn create_table(
    State(pool): State<PgPool>,
    axum::Json(table): axum::Json<Table>,
) -> Result<StatusCode, AppError> {
    let mut q_builder: QueryBuilder<'_, Postgres> =
        QueryBuilder::new("CREATE TABLE IF NOT EXISTS ");

    q_builder.push(&table.name);

    InitColumn::build_columns(&mut q_builder, &table.columns);

    let sql = q_builder.build().sql();

    debug!("{}", sql);

    sqlx::query(sql).execute(&pool).await?;

    Ok(StatusCode::CREATED)
}

impl InitColumn {
    fn build_columns(q_builder: &mut QueryBuilder<'_, Postgres>, columns: &Vec<InitColumn>) {
        q_builder.push(" (");

        for (i, column) in columns.iter().enumerate() {
            q_builder.push(format_args!(
                "{} {}",
                column.name.as_str(),
                column.data_type.as_str()
            ));

            if !column.is_nullable {
                q_builder.push(" NOT NULL");
            }

            if let Some(ref default) = column.default {
                Self::build_default(q_builder, default, column);
            }

            // NOTE: Set first element as primary key for now
            if i == 0 {
                q_builder.push(" PRIMARY KEY");
            }

            if i < columns.len() - 1 {
                q_builder.push(", ");
            }
        }

        q_builder.push(") ");
    }

    // TODO: Handle default values better if the default is a function such as gen_random_uuid()
    fn build_default(
        q_builder: &mut QueryBuilder<'_, Postgres>,
        default: &Value,
        column: &InitColumn,
    ) {
        match (default, column.data_type.as_str()) {
            (Value::String(s), "uuid") => {
                q_builder.push(format_args!(" DEFAULT {}", s));
            }
            (Value::String(s), _) => {
                q_builder.push(format_args!(" DEFAULT '{}'", s));
            }
            _ => {
                q_builder.push(format_args!(" DEFAULT {}", default));
            }
        }
    }
}
