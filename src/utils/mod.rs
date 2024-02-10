use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::{json, Value};
use sqlx::{
    postgres::{PgColumn, PgRow},
    Column, Row,
};

// NOTE: Postgres has a LOT of data types, only support the commonly used ones for now
// A better way to handle this is to pattern match SQLx's PgColumn type_info
pub fn get_value_from_row(row: &PgRow, field: &str) -> Value {
    let value: Value = if let Ok(val) = row.try_get::<Option<uuid::Uuid>, _>(field) {
        json!(val)
    } else if let Ok(val) = row.try_get::<Option<String>, _>(field) {
        // Text and varchar
        json!(val)
    } else if let Ok(val) = row.try_get::<Option<i16>, _>(field) {
        json!(val)
    } else if let Ok(val) = row.try_get::<Option<i32>, _>(field) {
        json!(val)
    } else if let Ok(val) = row.try_get::<Option<f32>, _>(field) {
        json!(val)
    } else if let Ok(val) = row.try_get::<Option<f64>, _>(field) {
        json!(val)
    } else if let Ok(val) = row.try_get::<Option<NaiveDateTime>, _>(field) {
        // Timestamp without timezone
        json!(val)
    } else if let Ok(val) = row.try_get::<Option<DateTime<Utc>>, _>(field) {
        // Timestamp with timezone
        json!(val)
    } else if let Ok(val) = row.try_get::<Option<bool>, _>(field) {
        json!(val)
    } else {
        json!(null)
    };

    value
}

pub fn insert_col_to_map(
    row: &PgRow,
    columns: &[PgColumn],
    map: &mut serde_json::Map<String, Value>,
) {
    for col in columns {
        let name = col.name();
        let type_info = col.type_info();

        let value = get_value_from_row(row, name);

        map.insert(name.to_string(), value);
    }
}

// pub fn to_sql_string(value: &Value) {
//     match value {
//         Value::String(s) => {
//             format_args!("'{}'", s)
//         }
//         _ => value,
//     }
// }
