// Copyright (c) 2023-2025 Retake, Inc.
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
#![allow(dead_code)]
use super::db::*;

use sqlx::PgConnection;
use std::path::PathBuf;

pub fn database_oid(conn: &mut PgConnection) -> String {
    let db_name = "SELECT current_database()".fetch_one::<(String,)>(conn).0;

    format!("SELECT oid FROM pg_database WHERE datname='{db_name}'")
        .fetch_one::<(sqlx::postgres::types::Oid,)>(conn)
        .0
         .0
        .to_string()
}

pub fn schema_oid(conn: &mut PgConnection, schema_name: &str) -> String {
    format!("SELECT oid FROM pg_namespace WHERE nspname='{schema_name}'")
        .to_string()
        .fetch_one::<(sqlx::postgres::types::Oid,)>(conn)
        .0
         .0
        .to_string()
}

pub fn table_oid(conn: &mut PgConnection, schema_name: &str, table_name: &str) -> String {
    format!("SELECT oid FROM pg_class WHERE relname='{table_name}' AND relnamespace=(SELECT oid FROM pg_namespace WHERE nspname='{schema_name}')")
        .to_string()
        .fetch_one::<(sqlx::postgres::types::Oid,)>(conn)
        .0
        .0
        .to_string()
}

pub fn default_database_path(conn: &mut PgConnection) -> PathBuf {
    let data_dir = "SHOW data_directory".fetch_one::<(String,)>(conn).0;
    let deltalake_dir = "deltalake";
    let database_oid = database_oid(conn);

    PathBuf::from(&data_dir)
        .join(deltalake_dir)
        .join(database_oid)
}

pub fn default_schema_path(conn: &mut PgConnection, schema_name: &str) -> PathBuf {
    let schema_oid = schema_oid(conn, schema_name);
    default_database_path(conn).join(schema_oid)
}

pub fn default_table_path(conn: &mut PgConnection, schema_name: &str, table_name: &str) -> PathBuf {
    let table_oid = table_oid(conn, schema_name, table_name);
    default_schema_path(conn, schema_name).join(table_oid)
}
