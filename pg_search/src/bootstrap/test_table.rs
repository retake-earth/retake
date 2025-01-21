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

use std::str::FromStr;

use anyhow::Result;
use pgrx::pg_sys::BuiltinOid;
use pgrx::spi::Spi;
use pgrx::{datum::RangeBound, prelude::*, JsonB};
use serde::Serialize;
use serde_json::json;
use std::fmt::{Display, Formatter};

#[derive(PostgresEnum, Serialize)]
pub enum TestTable {
    Items,
    Orders,
    Parts,
    Deliveries,
    Customers,
}

impl Display for TestTable {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        match self {
            TestTable::Items => write!(f, "Items"),
            TestTable::Orders => write!(f, "Orders"),
            TestTable::Parts => write!(f, "Parts"),
            TestTable::Deliveries => write!(f, "Deliveries"),
            TestTable::Customers => write!(f, "Customers"),
        }
    }
}

type MockDeliveryRow = (
    i32,
    Range<i32>,
    Range<i64>,
    Range<AnyNumeric>,
    Range<Date>,
    Range<Timestamp>,
    Range<TimestampWithTimeZone>,
);

#[pg_extern(sql = "
CREATE OR REPLACE PROCEDURE paradedb.create_bm25_test_table(table_name VARCHAR DEFAULT 'bm25_test_table', schema_name VARCHAR DEFAULT 'paradedb', table_type paradedb.TestTable DEFAULT 'Items')
LANGUAGE c AS 'MODULE_PATHNAME', '@FUNCTION_NAME@';
", requires = [ TestTable ])]
fn create_bm25_test_table(
    table_name: Option<&str>,
    schema_name: Option<&str>,
    table_type: Option<TestTable>,
) -> Result<()> {
    let table_name = table_name.unwrap_or("bm25_test_table");
    let schema_name = schema_name.unwrap_or("paradedb");
    let table_type = table_type.unwrap_or(TestTable::Items);
    let full_table_name = format!("{}.{}", schema_name, table_name);

    Spi::connect(|mut client| {
        let original_client_min_messages: String = client
            .select("SELECT current_setting('client_min_messages')", None, None)?
            .first()
            .get(1)? // 1-based indexing
            .unwrap_or("NOTICE".into()); // Postgres default

        client.update("SET client_min_messages TO WARNING", None, None)?;

        let table_not_found = client
            .select(
                &format!(
                    "SELECT FROM pg_catalog.pg_tables WHERE schemaname = '{}' AND tablename = '{}'",
                    schema_name, table_name
                ),
                None,
                None,
            )?
            .is_empty();

        if table_not_found {
            match table_type {
                TestTable::Items => {
                    client.update(
                        &format!(
                            "CREATE TABLE {} (
                                id SERIAL PRIMARY KEY,
                                description TEXT,
                                rating INTEGER CHECK (rating BETWEEN 1 AND 5),
                                category VARCHAR(255),
                                in_stock BOOLEAN,
                                metadata JSONB,
                                created_at TIMESTAMP,
                                last_updated_date DATE,
                                latest_available_time TIME,
                                weight_range INT4RANGE
                            )",
                            full_table_name
                        ),
                        None,
                        None,
                    )?;

                    for record in mock_items_data() {
                        client.update(
                            &format!(
                                "INSERT INTO {} (description, rating, category, in_stock, metadata, created_at, last_updated_date, latest_available_time, weight_range) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
                                full_table_name
                            ),
                            Some(1),
                            Some(vec![
                                (PgOid::BuiltIn(BuiltinOid::TEXTOID), record.0.into_datum()),
                                (PgOid::BuiltIn(BuiltinOid::INT4OID), record.1.into_datum()),
                                (PgOid::BuiltIn(BuiltinOid::VARCHAROID), record.2.into_datum()),
                                (PgOid::BuiltIn(BuiltinOid::BOOLOID), record.3.into_datum()),
                                (PgOid::BuiltIn(BuiltinOid::JSONBOID), JsonB(record.4).into_datum()),
                                (PgOid::BuiltIn(BuiltinOid::TIMESTAMPOID), record.5.into_datum()),
                                (PgOid::BuiltIn(BuiltinOid::DATEOID), record.6.into_datum()),
                                (PgOid::BuiltIn(BuiltinOid::TIMEOID), record.7.into_datum()),
                                (PgOid::BuiltIn(BuiltinOid::INT4RANGEOID), record.8.into_datum()),
                            ]),
                        )?;
                    }
                }
                TestTable::Orders => {
                    client.update(
                        &format!(
                            "CREATE TABLE {} (
                                order_id SERIAL PRIMARY KEY,
                                product_id INTEGER NOT NULL,
                                order_quantity INTEGER NOT NULL,
                                order_total DECIMAL(10, 2) NOT NULL,
                                customer_name VARCHAR(255) NOT NULL
                            )",
                            full_table_name
                        ),
                        None,
                        None,
                    )?;

                    for record in mock_orders_data() {
                        client.update(
                            &format!(
                                "INSERT INTO {} (product_id, order_quantity, order_total, customer_name) VALUES ($1, $2, $3, $4)",
                                full_table_name
                            ),
                            Some(1),
                            Some(vec![
                                (PgOid::BuiltIn(BuiltinOid::INT4OID), record.0.into_datum()),
                                (PgOid::BuiltIn(BuiltinOid::INT4OID), record.1.into_datum()),
                                (PgOid::BuiltIn(BuiltinOid::FLOAT4OID), record.2.into_datum()),
                                (PgOid::BuiltIn(BuiltinOid::VARCHAROID), record.3.into_datum()),
                            ]),
                        )?;
                    }
                }
                TestTable::Parts => {
                    client.update(
                        &format!(
                            "CREATE TABLE {} (
                                part_id SERIAL PRIMARY KEY,
                                parent_part_id INTEGER NOT NULL,
                                description VARCHAR(255) NOT NULL
                            )",
                            full_table_name
                        ),
                        None,
                        None,
                    )?;

                    for record in mock_parts_data() {
                        client.update(
                            &format!(
                                "INSERT INTO {} (part_id, parent_part_id, description) VALUES ($1, $2, $3)",
                                full_table_name
                            ),
                            Some(1),
                            Some(vec![
                                (PgOid::BuiltIn(BuiltinOid::INT4OID), record.0.into_datum()),
                                (PgOid::BuiltIn(BuiltinOid::INT4OID), record.1.into_datum()),
                                (PgOid::BuiltIn(BuiltinOid::VARCHAROID), record.2.into_datum()),
                            ]),
                        )?;
                    }
                }
                TestTable::Deliveries => {
                    client.update(
                        &format!(
                            "CREATE TABLE {} (
                                delivery_id SERIAL PRIMARY KEY,
                                weights INT4RANGE,
                                quantities INT8RANGE,
                                prices NUMRANGE,
                                ship_dates DATERANGE,
                                facility_arrival_times TSRANGE,
                                delivery_times TSTZRANGE
                            )",
                            full_table_name
                        ),
                        None,
                        None,
                    )?;

                    for record in mock_deliveries_data() {
                        client.update(
                            &format!(
                                "INSERT INTO {} (weights, quantities, prices, ship_dates, facility_arrival_times, delivery_times) 
                                VALUES ($1, $2, $3, $4, $5, $6)",
                                full_table_name
                            ),
                            Some(1),
                            Some(vec![
                                (
                                    PgOid::BuiltIn(BuiltinOid::INT4RANGEOID),
                                    record.1.into_datum(),
                                ),
                                (
                                    PgOid::BuiltIn(BuiltinOid::INT8RANGEOID),
                                    record.2.into_datum(),
                                ),
                                (
                                    PgOid::BuiltIn(BuiltinOid::NUMRANGEOID),
                                    record.3.into_datum(),
                                ),
                                (
                                    PgOid::BuiltIn(BuiltinOid::DATERANGEOID),
                                    record.4.into_datum(),
                                ),
                                (
                                    PgOid::BuiltIn(BuiltinOid::TSRANGEOID),
                                    record.5.into_datum(),
                                ),
                                (
                                    PgOid::BuiltIn(BuiltinOid::TSTZRANGEOID),
                                    record.6.into_datum(),
                                ),
                            ]),
                        )?;
                    }
                }
                // Then in the match statement, add the Customers variant:
                TestTable::Customers => {
                    client.update(
                        &format!(
                            "CREATE TABLE {} (
                id SERIAL PRIMARY KEY,
                name TEXT,
                crm_data JSONB
            )",
                            full_table_name
                        ),
                        None,
                        None,
                    )?;

                    for record in mock_customers_data() {
                        client.update(
                            &format!(
                                "INSERT INTO {} (name, crm_data) VALUES ($1, $2)",
                                full_table_name
                            ),
                            Some(1),
                            Some(vec![
                                (PgOid::BuiltIn(BuiltinOid::TEXTOID), record.0.into_datum()),
                                (
                                    PgOid::BuiltIn(BuiltinOid::JSONBOID),
                                    JsonB(record.1).into_datum(),
                                ),
                            ]),
                        )?;
                    }
                }
            }
        } else {
            pgrx::warning!("The table {} already exists, skipping.", full_table_name);
        }

        client.update(
            &format!(
                "SET client_min_messages TO '{}'",
                original_client_min_messages
            ),
            None,
            None,
        )?;

        Ok(())
    })
}

#[inline]
#[allow(clippy::type_complexity)]
fn mock_items_data() -> Vec<(
    &'static str,
    i32,
    &'static str,
    bool,
    serde_json::Value,
    Timestamp,
    Date,
    Time,
    Range<i32>,
)> {
    vec![
        (
            "Ergonomic metal keyboard",
            4,
            "Electronics",
            true,
            json!({"color": "Silver", "location": "United States"}),
            Timestamp::from_str("2023-05-01 09:12:34").unwrap(),
            Date::from_str("2023-05-03").unwrap(),
            Time::from_str("09:12:34").unwrap(),
            Range::new(RangeBound::Inclusive(1), RangeBound::Exclusive(10)),
        ),
        (
            "Plastic Keyboard",
            4,
            "Electronics",
            false,
            json!({"color": "Black", "location": "Canada"}),
            Timestamp::from_str("2023-04-15 13:27:09").unwrap(),
            Date::from_str("2023-04-16").unwrap(),
            Time::from_str("13:27:09").unwrap(),
            Range::new(RangeBound::Infinite, RangeBound::Inclusive(9)),
        ),
        (
            "Sleek running shoes",
            5,
            "Footwear",
            true,
            json!({"color": "Blue", "location": "China"}),
            Timestamp::from_str("2023-04-28 10:55:43").unwrap(),
            Date::from_str("2023-04-29").unwrap(),
            Time::from_str("10:55:43").unwrap(),
            Range::new(RangeBound::Inclusive(2), RangeBound::Exclusive(10)),
        ),
        (
            "White jogging shoes",
            3,
            "Footwear",
            false,
            json!({"color": "White", "location": "United States"}),
            Timestamp::from_str("2023-04-20 16:38:02").unwrap(),
            Date::from_str("2023-04-22").unwrap(),
            Time::from_str("16:38:02").unwrap(),
            Range::new(RangeBound::Infinite, RangeBound::Exclusive(11)),
        ),
        (
            "Generic shoes",
            4,
            "Footwear",
            true,
            json!({"color": "Brown", "location": "Canada"}),
            Timestamp::from_str("2023-05-02 08:45:11").unwrap(),
            Date::from_str("2023-05-03").unwrap(),
            Time::from_str("08:45:11").unwrap(),
            Range::new(RangeBound::Inclusive(3), RangeBound::Infinite),
        ),
        (
            "Compact digital camera",
            5,
            "Photography",
            false,
            json!({"color": "Black", "location": "China"}),
            Timestamp::from_str("2023-04-25 11:20:35").unwrap(),
            Date::from_str("2023-04-26").unwrap(),
            Time::from_str("11:20:35").unwrap(),
            Range::new(RangeBound::Exclusive(1), RangeBound::Inclusive(9)),
        ),
        (
            "Hardcover book on history",
            2,
            "Books",
            true,
            json!({"color": "Brown", "location": "United States"}),
            Timestamp::from_str("2023-04-18 14:59:27").unwrap(),
            Date::from_str("2023-04-19").unwrap(),
            Time::from_str("14:59:27").unwrap(),
            Range::new(RangeBound::Inclusive(1), RangeBound::Inclusive(11)),
        ),
        (
            "Organic green tea",
            3,
            "Groceries",
            true,
            json!({"color": "Green", "location": "Canada"}),
            Timestamp::from_str("2023-04-30 09:18:45").unwrap(),
            Date::from_str("2023-05-01").unwrap(),
            Time::from_str("09:18:45").unwrap(),
            Range::new(RangeBound::Exclusive(2), RangeBound::Exclusive(9)),
        ),
        (
            "Modern wall clock",
            4,
            "Home Decor",
            false,
            json!({"color": "Silver", "location": "China"}),
            Timestamp::from_str("2023-04-24 12:37:52").unwrap(),
            Date::from_str("2023-04-25").unwrap(),
            Time::from_str("12:37:52").unwrap(),
            Range::new(RangeBound::Infinite, RangeBound::Infinite),
        ),
        (
            "Colorful kids toy",
            1,
            "Toys",
            true,
            json!({"color": "Multicolor", "location": "United States"}),
            Timestamp::from_str("2023-05-04 15:29:12").unwrap(),
            Date::from_str("2023-05-06").unwrap(),
            Time::from_str("15:29:12").unwrap(),
            Range::new(RangeBound::Inclusive(3), RangeBound::Exclusive(11)),
        ),
        (
            "Soft cotton shirt",
            5,
            "Apparel",
            true,
            json!({"color": "Blue", "location": "Canada"}),
            Timestamp::from_str("2023-04-29 08:10:17").unwrap(),
            Date::from_str("2023-04-30").unwrap(),
            Time::from_str("08:10:17").unwrap(),
            Range::new(RangeBound::Inclusive(4), RangeBound::Exclusive(10)),
        ),
        (
            "Innovative wireless earbuds",
            5,
            "Electronics",
            true,
            json!({"color": "Black", "location": "China"}),
            Timestamp::from_str("2023-04-22 10:05:39").unwrap(),
            Date::from_str("2023-04-23").unwrap(),
            Time::from_str("10:05:39").unwrap(),
            Range::new(RangeBound::Exclusive(2), RangeBound::Inclusive(8)),
        ),
        (
            "Sturdy hiking boots",
            4,
            "Footwear",
            true,
            json!({"color": "Brown", "location": "United States"}),
            Timestamp::from_str("2023-05-05 13:45:22").unwrap(),
            Date::from_str("2023-05-07").unwrap(),
            Time::from_str("13:45:22").unwrap(),
            Range::new(RangeBound::Inclusive(3), RangeBound::Exclusive(9)),
        ),
        (
            "Elegant glass table",
            3,
            "Furniture",
            true,
            json!({"color": "Clear", "location": "Canada"}),
            Timestamp::from_str("2023-04-26 17:22:58").unwrap(),
            Date::from_str("2023-04-28").unwrap(),
            Time::from_str("17:22:58").unwrap(),
            Range::new(RangeBound::Infinite, RangeBound::Exclusive(10)),
        ),
        (
            "Refreshing face wash",
            2,
            "Beauty",
            false,
            json!({"color": "White", "location": "China"}),
            Timestamp::from_str("2023-04-27 09:52:04").unwrap(),
            Date::from_str("2023-04-29").unwrap(),
            Time::from_str("09:52:04").unwrap(),
            Range::new(RangeBound::Inclusive(1), RangeBound::Infinite),
        ),
        (
            "High-resolution DSLR",
            4,
            "Photography",
            true,
            json!({"color": "Black", "location": "United States"}),
            Timestamp::from_str("2023-04-21 14:30:19").unwrap(),
            Date::from_str("2023-04-23").unwrap(),
            Time::from_str("14:30:19").unwrap(),
            Range::new(RangeBound::Exclusive(2), RangeBound::Exclusive(11)),
        ),
        (
            "Paperback romantic novel",
            3,
            "Books",
            true,
            json!({"color": "Multicolor", "location": "Canada"}),
            Timestamp::from_str("2023-05-03 10:08:57").unwrap(),
            Date::from_str("2023-05-04").unwrap(),
            Time::from_str("10:08:57").unwrap(),
            Range::new(RangeBound::Inclusive(3), RangeBound::Inclusive(9)),
        ),
        (
            "Freshly ground coffee beans",
            5,
            "Groceries",
            true,
            json!({"color": "Brown", "location": "China"}),
            Timestamp::from_str("2023-04-23 08:40:15").unwrap(),
            Date::from_str("2023-04-25").unwrap(),
            Time::from_str("08:40:15").unwrap(),
            Range::new(RangeBound::Inclusive(1), RangeBound::Exclusive(10)),
        ),
        (
            "Artistic ceramic vase",
            4,
            "Home Decor",
            false,
            json!({"color": "Multicolor", "location": "United States"}),
            Timestamp::from_str("2023-04-19 15:17:29").unwrap(),
            Date::from_str("2023-04-21").unwrap(),
            Time::from_str("15:17:29").unwrap(),
            Range::new(RangeBound::Infinite, RangeBound::Exclusive(11)),
        ),
        (
            "Interactive board game",
            3,
            "Toys",
            true,
            json!({"color": "Multicolor", "location": "Canada"}),
            Timestamp::from_str("2023-05-01 12:25:06").unwrap(),
            Date::from_str("2023-05-02").unwrap(),
            Time::from_str("12:25:06").unwrap(),
            Range::new(RangeBound::Exclusive(3), RangeBound::Exclusive(10)),
        ),
        (
            "Slim-fit denim jeans",
            5,
            "Apparel",
            false,
            json!({"color": "Blue", "location": "China"}),
            Timestamp::from_str("2023-04-28 16:54:33").unwrap(),
            Date::from_str("2023-04-30").unwrap(),
            Time::from_str("16:54:33").unwrap(),
            Range::new(RangeBound::Inclusive(2), RangeBound::Inclusive(8)),
        ),
        (
            "Fast charging power bank",
            4,
            "Electronics",
            true,
            json!({"color": "Black", "location": "United States"}),
            Timestamp::from_str("2023-04-17 11:35:52").unwrap(),
            Date::from_str("2023-04-19").unwrap(),
            Time::from_str("11:35:52").unwrap(),
            Range::new(RangeBound::Exclusive(4), RangeBound::Inclusive(9)),
        ),
        (
            "Comfortable slippers",
            3,
            "Footwear",
            true,
            json!({"color": "Brown", "location": "Canada"}),
            Timestamp::from_str("2023-04-16 09:20:37").unwrap(),
            Date::from_str("2023-04-17").unwrap(),
            Time::from_str("09:20:37").unwrap(),
            Range::new(RangeBound::Infinite, RangeBound::Inclusive(9)),
        ),
        (
            "Classic leather sofa",
            5,
            "Furniture",
            false,
            json!({"color": "Brown", "location": "China"}),
            Timestamp::from_str("2023-05-06 14:45:27").unwrap(),
            Date::from_str("2023-05-08").unwrap(),
            Time::from_str("14:45:27").unwrap(),
            Range::new(RangeBound::Exclusive(2), RangeBound::Exclusive(10)),
        ),
        (
            "Anti-aging serum",
            4,
            "Beauty",
            true,
            json!({"color": "White", "location": "United States"}),
            Timestamp::from_str("2023-05-09 10:30:15").unwrap(),
            Date::from_str("2023-05-10").unwrap(),
            Time::from_str("10:30:15").unwrap(),
            Range::new(RangeBound::Inclusive(1), RangeBound::Infinite),
        ),
        (
            "Portable tripod stand",
            4,
            "Photography",
            true,
            json!({"color": "Black", "location": "Canada"}),
            Timestamp::from_str("2023-05-07 15:20:48").unwrap(),
            Date::from_str("2023-05-09").unwrap(),
            Time::from_str("15:20:48").unwrap(),
            Range::new(RangeBound::Inclusive(3), RangeBound::Inclusive(11)),
        ),
        (
            "Mystery detective novel",
            2,
            "Books",
            false,
            json!({"color": "Multicolor", "location": "China"}),
            Timestamp::from_str("2023-05-04 11:55:23").unwrap(),
            Date::from_str("2023-05-05").unwrap(),
            Time::from_str("11:55:23").unwrap(),
            Range::new(RangeBound::Exclusive(4), RangeBound::Exclusive(9)),
        ),
        (
            "Organic breakfast cereal",
            5,
            "Groceries",
            true,
            json!({"color": "Brown", "location": "United States"}),
            Timestamp::from_str("2023-05-02 07:40:59").unwrap(),
            Date::from_str("2023-05-03").unwrap(),
            Time::from_str("07:40:59").unwrap(),
            Range::new(RangeBound::Inclusive(2), RangeBound::Inclusive(10)),
        ),
        (
            "Designer wall paintings",
            5,
            "Home Decor",
            true,
            json!({"color": "Multicolor", "location": "Canada"}),
            Timestamp::from_str("2023-04-30 14:18:37").unwrap(),
            Date::from_str("2023-05-01").unwrap(),
            Time::from_str("14:18:37").unwrap(),
            Range::new(RangeBound::Exclusive(2), RangeBound::Inclusive(9)),
        ),
        (
            "Robot building kit",
            4,
            "Toys",
            true,
            json!({"color": "Multicolor", "location": "China"}),
            Timestamp::from_str("2023-04-29 16:25:42").unwrap(),
            Date::from_str("2023-05-01").unwrap(),
            Time::from_str("16:25:42").unwrap(),
            Range::new(RangeBound::Inclusive(1), RangeBound::Exclusive(10)),
        ),
        (
            "Sporty tank top",
            4,
            "Apparel",
            true,
            json!({"color": "Blue", "location": "United States"}),
            Timestamp::from_str("2023-04-27 12:09:53").unwrap(),
            Date::from_str("2023-04-28").unwrap(),
            Time::from_str("12:09:53").unwrap(),
            Range::new(RangeBound::Inclusive(2), RangeBound::Inclusive(9)),
        ),
        (
            "Bluetooth-enabled speaker",
            3,
            "Electronics",
            true,
            json!({"color": "Black", "location": "Canada"}),
            Timestamp::from_str("2023-04-26 09:34:11").unwrap(),
            Date::from_str("2023-04-28").unwrap(),
            Time::from_str("09:34:11").unwrap(),
            Range::new(RangeBound::Inclusive(4), RangeBound::Exclusive(8)),
        ),
        (
            "Winter woolen socks",
            5,
            "Footwear",
            false,
            json!({"color": "Gray", "location": "China"}),
            Timestamp::from_str("2023-04-25 14:55:08").unwrap(),
            Date::from_str("2023-04-27").unwrap(),
            Time::from_str("14:55:08").unwrap(),
            Range::new(RangeBound::Exclusive(3), RangeBound::Inclusive(9)),
        ),
        (
            "Rustic bookshelf",
            4,
            "Furniture",
            true,
            json!({"color": "Brown", "location": "United States"}),
            Timestamp::from_str("2023-04-24 08:20:47").unwrap(),
            Date::from_str("2023-04-25").unwrap(),
            Time::from_str("08:20:47").unwrap(),
            Range::new(RangeBound::Inclusive(1), RangeBound::Exclusive(10)),
        ),
        (
            "Moisturizing lip balm",
            4,
            "Beauty",
            true,
            json!({"color": "Pink", "location": "Canada"}),
            Timestamp::from_str("2023-04-23 13:48:29").unwrap(),
            Date::from_str("2023-04-24").unwrap(),
            Time::from_str("13:48:29").unwrap(),
            Range::new(RangeBound::Infinite, RangeBound::Exclusive(11)),
        ),
        (
            "Lightweight camera bag",
            5,
            "Photography",
            false,
            json!({"color": "Black", "location": "China"}),
            Timestamp::from_str("2023-04-22 17:10:55").unwrap(),
            Date::from_str("2023-04-24").unwrap(),
            Time::from_str("17:10:55").unwrap(),
            Range::new(RangeBound::Inclusive(3), RangeBound::Exclusive(9)),
        ),
        (
            "Historical fiction book",
            3,
            "Books",
            true,
            json!({"color": "Multicolor", "location": "United States"}),
            Timestamp::from_str("2023-04-21 10:35:40").unwrap(),
            Date::from_str("2023-04-22").unwrap(),
            Time::from_str("10:35:40").unwrap(),
            Range::new(RangeBound::Exclusive(2), RangeBound::Inclusive(8)),
        ),
        (
            "Pure honey jar",
            4,
            "Groceries",
            true,
            json!({"color": "Yellow", "location": "Canada"}),
            Timestamp::from_str("2023-04-20 15:22:14").unwrap(),
            Date::from_str("2023-04-22").unwrap(),
            Time::from_str("15:22:14").unwrap(),
            Range::new(RangeBound::Inclusive(4), RangeBound::Exclusive(9)),
        ),
        (
            "Handcrafted wooden frame",
            5,
            "Home Decor",
            false,
            json!({"color": "Brown", "location": "China"}),
            Timestamp::from_str("2023-04-19 08:55:06").unwrap(),
            Date::from_str("2023-04-21").unwrap(),
            Time::from_str("08:55:06").unwrap(),
            Range::new(RangeBound::Infinite, RangeBound::Exclusive(10)),
        ),
        (
            "Plush teddy bear",
            4,
            "Toys",
            true,
            json!({"color": "Brown", "location": "United States"}),
            Timestamp::from_str("2023-04-18 11:40:59").unwrap(),
            Date::from_str("2023-04-19").unwrap(),
            Time::from_str("11:40:59").unwrap(),
            Range::new(RangeBound::Inclusive(1), RangeBound::Exclusive(9)),
        ),
        (
            "Warm woolen sweater",
            3,
            "Apparel",
            false,
            json!({"color": "Red", "location": "Canada"}),
            Timestamp::from_str("2023-04-17 14:28:37").unwrap(),
            Date::from_str("2023-04-18").unwrap(),
            Time::from_str("14:28:37").unwrap(),
            Range::new(RangeBound::Inclusive(2), RangeBound::Inclusive(10)),
        ),
    ]
}

#[inline]
fn mock_orders_data() -> Vec<(i32, i32, f32, &'static str)> {
    vec![
        (1, 3, 99.99, "John Doe"),
        (2, 1, 49.99, "Jane Smith"),
        (3, 5, 249.95, "Alice Johnson"),
        (2, 6, 501.87, "John Doe"),
        (7, 10, 361.38, "Jane Smith"),
        (4, 6, 308.18, "Alice Johnson"),
        (5, 6, 439.05, "Michael Brown"),
        (8, 3, 104.88, "Emily Davis"),
        (3, 5, 132.75, "Chris Wilson"),
        (6, 8, 638.73, "Laura Martinez"),
        (1, 7, 633.94, "David White"),
        (9, 8, 195.11, "Sarah Lewis"),
        (10, 10, 234.32, "Mark Thomas"),
        (2, 4, 55.41, "Rachel Green"),
        (1, 5, 239.31, "Monica Geller"),
        (10, 2, 110.06, "Ross Geller"),
        (1, 1, 74.75, "Chandler Bing"),
        (10, 6, 484.98, "Phoebe Buffay"),
        (8, 9, 319.31, "Joey Tribbiani"),
        (9, 3, 150.90, "Will Smith"),
        (7, 8, 632.08, "Jada Smith"),
        (10, 9, 605.18, "Bruce Wayne"),
        (4, 4, 61.25, "Clark Kent"),
        (2, 7, 258.88, "Diana Prince"),
        (3, 10, 450.57, "Peter Parker"),
        (9, 7, 102.28, "Tony Stark"),
        (2, 7, 676.15, "Natasha Romanoff"),
        (9, 5, 237.22, "Steve Rogers"),
        (9, 4, 381.90, "Thor Odinson"),
        (8, 4, 278.91, "Bruce Banner"),
        (4, 5, 402.69, "Wanda Maximoff"),
        (8, 2, 91.16, "Vision"),
        (3, 9, 194.87, "Scott Lang"),
        (5, 9, 431.54, "Hope Van Dyne"),
        (9, 7, 361.38, "Jane Smith"),
        (4, 6, 308.18, "Alice Johnson"),
        (5, 6, 439.05, "Michael Brown"),
        (8, 3, 104.88, "Emily Davis"),
        (3, 5, 132.75, "Chris Wilson"),
        (6, 8, 638.73, "Laura Martinez"),
        (1, 7, 633.94, "David White"),
        (9, 8, 195.11, "Sarah Lewis"),
        (10, 10, 234.32, "Mark Thomas"),
        (2, 4, 55.41, "Rachel Green"),
        (1, 5, 239.31, "Monica Geller"),
        (10, 2, 110.06, "Ross Geller"),
        (1, 1, 74.75, "Chandler Bing"),
        (10, 6, 484.98, "Phoebe Buffay"),
        (8, 9, 319.31, "Joey Tribbiani"),
        (9, 3, 150.90, "Will Smith"),
        (7, 8, 632.08, "Jada Smith"),
        (10, 9, 605.18, "Bruce Wayne"),
        (4, 4, 61.25, "Clark Kent"),
        (2, 7, 258.88, "Diana Prince"),
        (3, 10, 450.57, "Peter Parker"),
        (9, 7, 102.28, "Tony Stark"),
        (2, 7, 676.15, "Natasha Romanoff"),
        (9, 5, 237.22, "Steve Rogers"),
        (9, 4, 381.90, "Thor Odinson"),
        (8, 4, 278.91, "Bruce Banner"),
        (4, 5, 402.69, "Wanda Maximoff"),
        (8, 2, 91.16, "Vision"),
        (3, 9, 194.87, "Scott Lang"),
        (5, 9, 431.54, "Hope Van Dyne"),
    ]
}

#[inline]
fn mock_parts_data() -> Vec<(i32, i32, &'static str)> {
    vec![
        (1, 0, "Chassis Assembly"),
        (2, 1, "Engine Block"),
        (3, 1, "Transmission System"),
        (4, 1, "Suspension System"),
        (5, 2, "Cylinder Head"),
        (6, 2, "Piston"),
        (7, 2, "Crankshaft"),
        (8, 3, "Gearbox"),
        (9, 3, "Clutch"),
        (10, 4, "Shock Absorber"),
        (11, 4, "Control Arm"),
        (12, 5, "Valve"),
        (13, 5, "Camshaft"),
        (14, 6, "Connecting Rod"),
        (15, 6, "Piston Rings"),
        (16, 8, "Gear Set"),
        (17, 8, "Synchromesh"),
        (18, 9, "Pressure Plate"),
        (19, 9, "Clutch Disc"),
        (20, 12, "Valve Spring"),
        (21, 13, "Timing Gear"),
        (22, 14, "Crank Pin"),
        (23, 15, "Oil Scraper Rings"),
        (24, 16, "Gear Teeth"),
        (25, 16, "Shaft Bearing"),
        (26, 17, "Synchronizer Hub"),
        (27, 18, "Clutch Bearing"),
        (28, 19, "Friction Disc"),
        (29, 24, "Bearing Cage"),
        (30, 24, "Thrust Washer"),
        (31, 25, "Sealing Ring"),
        (32, 25, "Roller Bearing"),
        (33, 26, "Hub Spring"),
        (34, 26, "Shift Fork"),
        (35, 27, "Release Bearing"),
        (36, 28, "Wear Plate"),
    ]
}

#[inline]
fn mock_deliveries_data() -> Vec<MockDeliveryRow> {
    vec![
        (
            1,
            Range::new(Some(1), Some(10)),
            Range::new(None, Some(200000)),
            Range::new(
                AnyNumeric::try_from(1.5).unwrap(),
                RangeBound::Exclusive(AnyNumeric::try_from(10.5).unwrap()),
            ),
            Range::new(
                Some(Date::from_str("2024-01-01").unwrap()),
                Some(Date::from_str("2024-01-10").unwrap()),
            ),
            Range::new(
                Some(Timestamp::from_str("2024-01-01T12:00:00Z").unwrap()),
                Some(Timestamp::from_str("2024-01-01T18:00:00Z").unwrap()),
            ),
            Range::new(
                Some(TimestampWithTimeZone::from_str("2024-01-01T10:00:00+00:00").unwrap()),
                Some(TimestampWithTimeZone::from_str("2024-01-01T17:00:00+00:00").unwrap()),
            ),
        ),
        (
            2,
            Range::new(None, Some(13)),
            Range::new(Some(150000), Some(250000)),
            Range::new(
                Some(AnyNumeric::try_from(2.5).unwrap()),
                Some(AnyNumeric::try_from(12.5).unwrap()),
            ),
            Range::new(
                Some(Date::from_str("2024-02-01").unwrap()),
                RangeBound::Exclusive(Date::from_str("2024-02-05").unwrap()),
            ),
            Range::new(
                Some(Timestamp::from_str("2024-02-01T08:00:00Z").unwrap()),
                Some(Timestamp::from_str("2024-02-01T14:00:00Z").unwrap()),
            ),
            Range::new(
                Some(TimestampWithTimeZone::from_str("2024-02-01T06:00:00+00:00").unwrap()),
                RangeBound::Exclusive(
                    TimestampWithTimeZone::from_str("2024-02-01T13:00:00+00:00").unwrap(),
                ),
            ),
        ),
        (
            3,
            Range::new(Some(8), RangeBound::Exclusive(18)),
            Range::new(Some(120000), Some(220000)),
            Range::new(
                RangeBound::Exclusive(AnyNumeric::try_from(4.0).unwrap()),
                Some(AnyNumeric::try_from(9.2).unwrap()),
            ),
            Range::new(
                Some(Date::from_str("2024-03-01").unwrap()),
                Some(Date::from_str("2024-03-06").unwrap()),
            ),
            Range::new(
                Some(Timestamp::from_str("2024-03-01T09:30:00Z").unwrap()),
                Some(Timestamp::from_str("2024-03-01T15:00:00Z").unwrap()),
            ),
            Range::new(
                Some(TimestampWithTimeZone::from_str("2024-03-01T07:00:00+00:00").unwrap()),
                Some(TimestampWithTimeZone::from_str("2024-03-01T13:00:00+00:00").unwrap()),
            ),
        ),
        (
            4,
            Range::new(None, RangeBound::Exclusive(20)),
            Range::new(Some(180000), Some(280000)),
            Range::new(
                Some(AnyNumeric::try_from(3.0).unwrap()),
                RangeBound::Exclusive(AnyNumeric::try_from(4.1).unwrap()),
            ),
            Range::new(
                Some(Date::from_str("2024-04-01").unwrap()),
                RangeBound::Exclusive(Date::from_str("2024-04-10").unwrap()),
            ),
            Range::new(
                Some(Timestamp::from_str("2024-04-01T11:45:00Z").unwrap()),
                Some(Timestamp::from_str("2024-04-01T16:30:00Z").unwrap()),
            ),
            Range::new(
                None,
                Some(TimestampWithTimeZone::from_str("2024-04-01T12:00:00+00:00").unwrap()),
            ),
        ),
        (
            5,
            Range::new(Some(2), Some(12)),
            Range::new(RangeBound::Exclusive(170000), RangeBound::Exclusive(270000)),
            Range::new(
                RangeBound::Exclusive(AnyNumeric::try_from(3.5).unwrap()),
                Some(AnyNumeric::try_from(11.2).unwrap()),
            ),
            Range::new(
                Some(Date::from_str("2024-05-01").unwrap()),
                Some(Date::from_str("2024-05-07").unwrap()),
            ),
            Range::new(
                RangeBound::Exclusive(Timestamp::from_str("2024-05-01T14:00:00Z").unwrap()),
                Some(Timestamp::from_str("2024-05-01T19:00:00Z").unwrap()),
            ),
            Range::new(
                RangeBound::Exclusive(
                    TimestampWithTimeZone::from_str("2024-05-01T10:30:00+00:00").unwrap(),
                ),
                Some(TimestampWithTimeZone::from_str("2024-05-01T16:00:00+00:00").unwrap()),
            ),
        ),
    ]
}

#[inline]
fn mock_customers_data() -> Vec<(&'static str, serde_json::Value)> {
    vec![
        (
            "Customer A",
            json!([{
                "interaction": "call",
                "details": {
                    "subject": "Welcome Call",
                    "date": "2023-09-01"
                }
            }, {
                "interaction": "email",
                "details": {
                    "subject": "Goodbye Email",
                    "date": "2023-09-02"
                }
            }]),
        ),
        (
            "Customer Deep",
            json!({
                "level1": {
                    "level2": [{
                        "level3": "deep_value",
                        "extra": "metadata"
                    }]
                },
                "other_data": "some value"
            }),
        ),
        (
            "Customer B",
            json!([{
                "interaction": "sms",
                "details": {
                    "subject": "Reminder",
                    "date": "2023-09-01"
                }
            }, {
                "interaction": "email",
                "details": {
                    "subject": "Update",
                    "date": "2023-09-03"
                }
            }]),
        ),
        (
            "Customer C",
            json!([{
                "interaction": "call",
                "details": {
                    "subject": "Service Call",
                    "date": "2023-09-04"
                }
            }, {
                "interaction": "sms",
                "details": {
                    "subject": "Follow-up",
                    "date": "2023-09-05"
                }
            }]),
        ),
        (
            "Customer D",
            json!([{
                "interaction": "email",
                "details": {
                    "subject": "Promotion",
                    "date": "2023-09-06"
                }
            }, {
                "interaction": "sms",
                "details": {
                    "subject": "Discount",
                    "date": "2023-09-07"
                }
            }]),
        ),
        (
            "Customer E",
            json!([{
                "interaction": "call",
                "details": {
                    "subject": "Inquiry",
                    "date": "2023-09-08"
                }
            }, {
                "interaction": "email",
                "details": {
                    "subject": "Notification",
                    "date": "2023-09-09"
                }
            }]),
        ),
    ]
}
