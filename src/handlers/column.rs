use serde::Deserialize;
use serde_json::Value;
use sqlx::{Postgres, QueryBuilder};

#[derive(Debug, Deserialize)]
pub struct BuildColumn {
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

impl BuildColumn {
    pub fn build_columns(q_builder: &mut QueryBuilder<'_, Postgres>, columns: &Vec<BuildColumn>) {
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
