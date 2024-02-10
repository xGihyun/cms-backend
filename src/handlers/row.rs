use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Result,
};
use serde::Deserialize;
use serde_json::Value;
use sqlx::{Execute, PgPool, Postgres, QueryBuilder, Row as SqlxRow};
use tracing::debug;

use crate::{error::AppError, utils};

use super::column;

#[derive(Debug, Deserialize)]
pub enum Order {
    Ascending(String),
    Descending(String),
}

// Json body content
#[derive(Debug, Deserialize)]
pub struct Row {
    table: String,
    // Defaults to '*'
    columns: Option<Vec<column::InsertOnColumn>>,
    // For WHERE clause
    // NOTE: Can only filter one column for now
    filters: Option<column::InsertOnColumn>,
}

// `Cs` stands for "Comma Separated"
type CsString = String;

// /contents/:id?table={table}&columns={columns}&limit={limit}&order_by={order_by}&order={order}
#[derive(Debug, Deserialize)]
pub struct SelectQuery {
    table: String,
    // Comma separated column/s
    columns: Option<CsString>,
    limit: Option<i64>,
    // Comma separated column/s to order by
    order_by: Option<CsString>,
    // ASC or DESC
    // ASC by default
    order: Option<String>,
}

// SELECT * FROM {table} WHERE {conditions} ORDER BY {order} LIMIT {limit}
pub async fn select_many(
    State(pool): State<PgPool>,
    Query(query): Query<SelectQuery>,
) -> Result<(StatusCode, axum::Json<Value>), AppError> {
    let sql = query.push_select().order().limit().sql();

    debug!("{}", sql);

    let pg_rows = sqlx::query(sql.as_str()).fetch_all(&pool).await?;
    let mut json_vec: Vec<Value> = Vec::new();
    let mut json_map = serde_json::Map::new();

    for row in pg_rows.iter() {
        let cols = row.columns();

        utils::insert_col_to_map(row, cols, &mut json_map);

        let json = serde_json::to_value(&json_map)?;

        json_vec.push(json);
        json_map.clear();
    }

    let json: Value = serde_json::to_value(&json_vec)?;

    Ok((StatusCode::OK, axum::Json(json)))
}

// SELECT * FROM {table} WHERE {conditions}
pub async fn select_one(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
    Query(query): Query<SelectQuery>,
) -> Result<(StatusCode, axum::Json<Value>), AppError> {
    let sql = query.push_select().filter(id).sql();

    debug!("{}", sql);

    let pg_row = sqlx::query(sql.as_str()).fetch_one(&pool).await?;
    let mut json_map = serde_json::Map::new();

    utils::insert_col_to_map(&pg_row, pg_row.columns(), &mut json_map);

    let json: Value = serde_json::to_value(&json_map)?;

    Ok((StatusCode::OK, axum::Json(json)))
}

// INSERT INTO {table} {rows} VALUES {values}
pub async fn insert(
    State(pool): State<PgPool>,
    axum::Json(row): axum::Json<Row>,
) -> Result<StatusCode, AppError> {
    let sql = row.push_insert();

    debug!("{}", sql);

    sqlx::query(sql.as_str()).execute(&pool).await?;

    Ok(StatusCode::CREATED)
}

// UPDATE {table} SET {row} = {value}, {row} = {value} WHERE {row} = {value}
pub async fn update(
    State(pool): State<PgPool>,
    axum::Json(row): axum::Json<Row>,
) -> Result<StatusCode, AppError> {
    let sql = row.push_update();

    debug!("{}", sql);

    sqlx::query(sql.as_str()).execute(&pool).await?;

    Ok(StatusCode::OK)
}

// DELETE FROM {table} WHERE id IN ({id})
pub async fn delete(
    State(pool): State<PgPool>,
    axum::Json(row): axum::Json<Row>,
) -> Result<StatusCode, AppError> {
    let sql = row.push_delete();

    debug!("{}", sql);

    sqlx::query(sql.as_str()).execute(&pool).await?;

    Ok(StatusCode::NO_CONTENT)
}

impl Row {
    fn push_update(&self) -> String {
        let mut q_builder: QueryBuilder<'_, Postgres> = QueryBuilder::new("UPDATE ");

        q_builder.push(self.table.as_str());

        if let (Some(columns), Some(filters)) = (self.columns.as_ref(), self.filters.as_ref()) {
            let mut comma_sep = q_builder.separated(", ");

            comma_sep.push_unseparated(" SET ");

            columns.iter().for_each(|col| {
                comma_sep.push_unseparated(format_args!("{} = ", col.name));

                match &col.value {
                    Value::String(s) => {
                        if s == "gen_random_uuid()" || s == "now()" {
                            comma_sep.push(s);
                        } else {
                            comma_sep.push(format_args!("'{}'", s));
                        }
                    }
                    _ => {
                        comma_sep.push(&filters.value);
                    }
                }
            });

            self.push_filter(&mut q_builder);
        }

        q_builder.build().sql().to_string()
    }

    fn push_insert(&self) -> String {
        let mut q_builder: QueryBuilder<'_, Postgres> = QueryBuilder::new("INSERT INTO ");

        q_builder.push(self.table.as_str());

        if let Some(ref columns) = self.columns {
            let mut comma_sep = q_builder.separated(", ");

            comma_sep.push_unseparated(" (");

            columns.iter().for_each(|col| {
                comma_sep.push(&col.name);
            });

            comma_sep.push_unseparated(") VALUES (");

            let mut comma_sep = q_builder.separated(", ");

            columns
                .iter()
                .for_each(|col| match (&col.value, &col.is_db_expression) {
                    (Value::String(s), true) => {
                        comma_sep.push(s);
                    }
                    (Value::String(s), false) => {
                        comma_sep.push(format_args!("'{}'", s));
                    }
                    _ => {
                        comma_sep.push(&col.value);
                    }
                });

            comma_sep.push_unseparated(")");
        }

        q_builder.push(" RETURNING *");

        q_builder.build().sql().to_string()
    }

    fn push_delete(&self) -> String {
        let mut q_builder: QueryBuilder<'_, Postgres> = QueryBuilder::new("DELETE FROM ");

        q_builder.push(self.table.as_str());

        self.push_filter(&mut q_builder);

        q_builder.build().sql().to_string()
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

struct SelectBuilder<'a> {
    builder: QueryBuilder<'a, Postgres>,
    table: String,
    // Comma separated column/s
    columns: Option<CsString>,
    limit: Option<i64>,
    // Comma separated column/s to order by
    order_by: Option<CsString>,
    // ASC or DESC
    // ASC by default
    order: Option<String>,
}

impl SelectQuery {
    fn push_select(self) -> SelectBuilder<'static> {
        let mut q_builder: QueryBuilder<'_, Postgres> = QueryBuilder::new("SELECT ");

        if let Some(ref columns) = self.columns {
            q_builder.push(columns);
        } else {
            q_builder.push("*");
        };

        q_builder.push(format_args!(" FROM {}", self.table));

        SelectBuilder {
            builder: q_builder,
            table: self.table,
            order: self.order,
            columns: self.columns,
            order_by: self.order_by,
            limit: self.limit,
        }
    }
}

impl SelectBuilder<'_> {
    fn sql(&self) -> String {
        self.builder.sql().to_string()
    }

    fn order(&mut self) -> &mut Self {
        if let (Some(ref column), Some(ref order)) = (&self.order_by, &self.order) {
            self.builder
                .push(format_args!(" ORDER BY {} {}", column, order));
        }

        self
    }

    fn limit(&mut self) -> &mut Self {
        if let Some(ref limit) = self.limit {
            self.builder.push(format_args!(" LIMIT {}", limit));
        }

        self
    }

    // NOTE: Assumes the filter is always an `id`
    fn filter(&mut self, filter: String) -> &mut Self {
        self.builder.push(format_args!(" WHERE id = '{}'", filter));

        self
    }
}
