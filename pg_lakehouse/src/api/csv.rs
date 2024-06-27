// Copyright (c) 2023-2024 Retake, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use anyhow::Result;
use duckdb::types::Value;
use pgrx::*;

use crate::duckdb::connection;
use crate::duckdb::utils;

type SniffCsvRow = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<i32>,
    Option<bool>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

#[allow(clippy::type_complexity)]
#[pg_extern]
pub fn sniff_csv(
    files: &str,
    sample_size: default!(Option<i64>, "NULL"),
) -> iter::TableIterator<(
    name!(delimiter, Option<String>),
    name!(quote, Option<String>),
    name!(escape, Option<String>),
    name!(new_line_delimiter, Option<String>),
    name!(skip_rows, Option<i32>),
    name!(has_header, Option<bool>),
    name!(columns, Option<String>),
    name!(date_format, Option<String>),
    name!(timestamp_format, Option<String>),
    name!(user_arguments, Option<String>),
    name!(prompt, Option<String>),
)> {
    let rows = sniff_csv_impl(files, sample_size).unwrap_or_else(|e| {
        panic!("{}", e);
    });
    iter::TableIterator::new(rows)
}

#[inline]
fn sniff_csv_impl(files: &str, sample_size: Option<i64>) -> Result<Vec<SniffCsvRow>> {
    let schema_str = vec![
        Some(utils::format_csv(files)),
        sample_size.map(|s| format!("sample_size = {}", s)),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<String>>()
    .join(", ");
    let conn = unsafe { &*connection::get_global_connection().get() };
    let query = format!("SELECT * FROM sniff_csv({schema_str})");
    let mut stmt = conn.prepare(&query)?;

    Ok(stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<i32>>(4)?,
                row.get::<_, Option<bool>>(5)?,
                row.get::<_, Option<Value>>(6)?.and_then(|v| match v {
                    Value::List(vec) => Some(format!("{}", ValueVec(vec))),
                    _ => None,
                }),
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, Option<String>>(10)?,
            ))
        })?
        .map(|row| row.unwrap())
        .collect::<Vec<SniffCsvRow>>())
}

struct ValueVec(Vec<Value>);

impl std::fmt::Display for ValueVec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut formatted_columns = String::from("[");
        let mut is_first = true;

        for value in &self.0 {
            if let Value::Struct(ordered_map) = value {
                if !is_first {
                    formatted_columns.push_str(", ");
                }
                formatted_columns.push('{');
                let mut name = String::new();
                let mut type_str = String::new();

                for (key, value) in ordered_map.iter() {
                    match (key.as_str(), value) {
                        ("name", Value::Text(name_value)) => {
                            name.clone_from(name_value);
                        }
                        ("type", Value::Text(type_value)) => {
                            type_str.clone_from(type_value);
                        }
                        _ => {}
                    }
                }

                formatted_columns.push_str(&format!("'name': {}, 'type': {}", name, type_str));
                formatted_columns.push('}');
                is_first = false;
            }
        }

        formatted_columns.push(']');
        write!(f, "{}", formatted_columns)
    }
}
