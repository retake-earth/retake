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

use chrono::{NaiveDate, NaiveDateTime};
use soa_derive::StructOfArray;
use sqlx::FromRow;

#[derive(Debug, PartialEq, FromRow, StructOfArray, Default)]
pub struct SimpleProductsTable {
    pub id: i32,
    pub description: String,
    pub category: String,
    pub rating: i32,
    pub in_stock: bool,
    pub metadata: serde_json::Value,
    pub created_at: NaiveDateTime,
    pub last_updated_date: NaiveDate,
}

impl SimpleProductsTable {
    pub fn setup() -> String {
        SIMPLE_PRODUCTS_TABLE_SETUP.into()
    }
}

static SIMPLE_PRODUCTS_TABLE_SETUP: &str = r#"
BEGIN;
    CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');

    CREATE INDEX bm25_search_bm25_index
    ON paradedb.bm25_search
    USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time)
    WITH (
        key_field='id',
        text_fields='{
            "description": { "stored": true },
            "category": { "stored": true }
        }',
        numeric_fields='{"rating": { "stored": true } }',
        boolean_fields='{"in_stock": { "stored": true } }',
        json_fields='{"metadata": { "stored": true } }',
        datetime_fields='{
            "created_at": { "stored": true },
            "last_updated_date": { "stored": true },
            "latest_available_time": { "stored": true }
        }'
    );
COMMIT;
"#;
