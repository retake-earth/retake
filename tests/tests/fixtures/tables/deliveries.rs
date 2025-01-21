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

use bigdecimal::BigDecimal;
use chrono::{NaiveDate, NaiveDateTime};
use soa_derive::StructOfArray;
use sqlx::postgres::types::PgRange;
use sqlx::FromRow;
use std::ops::Range;

#[derive(Debug, PartialEq, FromRow, StructOfArray)]
pub struct DeliveriesTable {
    pub delivery_id: i32,
    pub weights: Range<i32>,
    pub quantities: PgRange<i64>,
    pub prices: BigDecimal,
    pub ship_dates: PgRange<NaiveDate>,
    pub facility_arrival_times: PgRange<NaiveDateTime>,
    pub delivery_times: PgRange<NaiveDateTime>,
}

impl DeliveriesTable {
    pub fn setup() -> String {
        DELIVERIES_TABLE_SETUP.into()
    }
}

static DELIVERIES_TABLE_SETUP: &str = r#"
BEGIN;
    CALL paradedb.create_bm25_test_table(
        schema_name => 'public',
        table_name => 'deliveries',
        table_type => 'Deliveries'
    );
   
    CREATE INDEX deliveries_idx ON deliveries
    USING bm25 (delivery_id, weights, quantities, prices, ship_dates, facility_arrival_times, delivery_times)
    WITH (key_field='delivery_id');
COMMIT;
"#;
