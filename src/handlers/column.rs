use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{prelude::FromRow, Postgres, QueryBuilder};

#[derive(Debug, Deserialize)]
pub struct BuildColumn {
    pub name: String,
    pub data_type: String,
    pub default: Option<Value>, // Optional default value
    pub is_nullable: bool,      // Sets column to NOT NULL if true
    pub is_primary_key: bool,
    pub is_unique: bool,
}

#[derive(Debug, Deserialize)]
pub struct EditColumn {
    pub name: String,
    pub data_type: String,
    pub default: Option<Value>, // Optional default value
    pub is_nullable: bool,      // Sets column to NOT NULL if true
    pub is_primary_key: bool,
    pub is_unique: bool,
    // NOTE: Can this be an enum even if data is from JSON?
    pub state: String, // "added" | "removed" | "modified" | "unchanged"
}

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct InsertOnColumn {
    pub name: String,
    pub value: Value,
    pub is_db_expression: bool,
}

impl BuildColumn {
    pub fn build_columns(q_builder: &mut QueryBuilder<'_, Postgres>, columns: &[BuildColumn]) {
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

            // NOTE: Assume there's one primary key for now
            if column.is_primary_key {
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
        column: &BuildColumn,
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
