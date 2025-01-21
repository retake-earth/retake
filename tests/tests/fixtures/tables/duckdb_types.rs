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

use sqlx::postgres::types::PgInterval;
use sqlx::types::{BigDecimal, Json, Uuid};
use sqlx::FromRow;
use std::collections::HashMap;
use time::{Date, OffsetDateTime, PrimitiveDateTime, Time};

#[derive(Debug, PartialEq, FromRow)]
pub struct DuckdbTypesTable {
    pub bool_col: bool,
    pub tinyint_col: i16,
    pub smallint_col: i16,
    pub integer_col: i32,
    pub bigint_col: i64,
    pub utinyint_col: i32,
    pub usmallint_col: i32,
    pub uinteger_col: i64,
    pub ubigint_col: BigDecimal,
    pub float_col: f64,
    pub double_col: f64,
    pub timestamp_col: PrimitiveDateTime,
    pub date_col: Date,
    pub time_col: Time,
    pub interval_col: PgInterval,
    pub hugeint_col: f64,
    pub uhugeint_col: f64,
    pub varchar_col: String,
    pub blob_col: String,
    pub decimal_col: BigDecimal,
    pub timestamp_s_col: PrimitiveDateTime,
    pub timestamp_ms_col: PrimitiveDateTime,
    pub timestamp_ns_col: PrimitiveDateTime,
    pub list_col: Vec<i32>,
    pub struct_col: Json<HashMap<String, String>>,
    pub array_col: [i32; 3],
    pub uuid_col: Uuid,
    pub time_tz_col: Time,
    pub timestamp_tz_col: OffsetDateTime,
}

impl DuckdbTypesTable {
    pub fn create_duckdb_table() -> String {
        DUCKDB_TYPES_TABLE_CREATE.to_string()
    }

    pub fn export_duckdb_table(path: &str) -> String {
        format!("COPY duckdb_types_test TO '{path}' (FORMAT PARQUET)")
    }

    pub fn populate_duckdb_table() -> String {
        DUCKDB_TYPES_TABLE_INSERT.to_string()
    }

    pub fn create_foreign_table(path: &str) -> String {
        format!(
            r#"
            CREATE FOREIGN DATA WRAPPER parquet_wrapper HANDLER parquet_fdw_handler VALIDATOR parquet_fdw_validator;
            CREATE SERVER parquet_server FOREIGN DATA WRAPPER parquet_wrapper;
            CREATE FOREIGN TABLE duckdb_types_test () SERVER parquet_server OPTIONS (files '{path}');
        "#
        )
    }
}

static DUCKDB_TYPES_TABLE_CREATE: &str = r#"
CREATE TABLE duckdb_types_test (
    bool_col BOOLEAN,
    tinyint_col TINYINT,
    smallint_col SMALLINT,
    integer_col INTEGER,
    bigint_col BIGINT,
    utinyint_col UTINYINT,
    usmallint_col USMALLINT,
    uinteger_col UINTEGER,
    ubigint_col UBIGINT,
    float_col FLOAT,
    double_col DOUBLE,
    timestamp_col TIMESTAMP,
    date_col DATE,
    time_col TIME,
    interval_col INTERVAL,
    hugeint_col HUGEINT,
    uhugeint_col UHUGEINT,
    varchar_col VARCHAR,
    blob_col BLOB,
    decimal_col DECIMAL,
    timestamp_s_col TIMESTAMP_S,
    timestamp_ms_col TIMESTAMP_MS,
    timestamp_ns_col TIMESTAMP_NS,
    list_col INTEGER[],
    struct_col STRUCT(a VARCHAR, b VARCHAR),
    array_col INTEGER[3],
    uuid_col UUID,
    time_tz_col TIMETZ,
    timestamp_tz_col TIMESTAMPTZ
);
"#;

static DUCKDB_TYPES_TABLE_INSERT: &str = r#"
INSERT INTO duckdb_types_test VALUES (
    TRUE,
    127,
    32767,
    2147483647,
    9223372036854775807,
    255,
    65535,
    4294967295,
    18446744073709551615,
    1.23,
    2.34,
    '2023-06-27 12:34:56',
    '2023-06-27',
    '12:34:56',
    INTERVAL '1 day',
    12345678901234567890,
    12345678901234567890,
    'Example text',
    '\x41',
    12345.67,
    '2023-06-27 12:34:56',
    '2023-06-27 12:34:56.789',
    '2023-06-27 12:34:56.789123',
    [1, 2, 3],
    ROW('abc', 'def'),
    [1, 2, 3],
    '550e8400-e29b-41d4-a716-446655440000',
    '12:34:56+02',
    '2023-06-27 12:34:56+02'
);
"#;
