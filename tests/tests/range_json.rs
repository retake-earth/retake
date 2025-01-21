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

mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn integer_range(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value_int4 INTEGER,
        value_int8 BIGINT
    );

    INSERT INTO test_table (value_int4, value_int8) VALUES 
        (-1111, -11111111),
        (2222, 22222222), 
        (3333, 33333333), 
        (4444, 44444444);
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON public.test_table
    USING bm25 (id, value_int4, value_int8)
    WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    // INT4
    let rows: Vec<(i32, i32)> = r#"
    SELECT id, value_int4 FROM test_table
    WHERE test_table @@@ '{
        "range": {
            "field": "value_int4",
            "lower_bound": {"included": 2222},
            "upper_bound": {"included": 4444}
        }
    }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 3);

    // INT8
    let rows: Vec<(i32, i64)> = r#"
    SELECT id, value_int8 FROM test_table
    WHERE test_table @@@ '{
        "range": {
            "field": "value_int8",
            "lower_bound": {"included": 0},
            "upper_bound": {"excluded": 50000000}
        }
    }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 3);
}

#[rstest]
fn unbounded_integer_range(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value_int4 INTEGER,
        value_int8 BIGINT
    );
    INSERT INTO test_table (value_int4, value_int8) VALUES 
        (-1111, -11111111),
        (2222, 22222222), 
        (3333, 33333333), 
        (4444, 44444444);
    "#
    .execute(&mut conn);
    r#"
    CREATE INDEX test_index ON public.test_table
    USING bm25 (id, value_int4, value_int8)
    WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    // Test unbounded upper range for INT4
    let rows: Vec<(i32, i32)> = r#"
    SELECT id, value_int4 FROM test_table
    WHERE test_table @@@ '{
        "range": {
            "field": "value_int4",
            "lower_bound": {"included": 2222},
            "upper_bound": null
        }
    }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].1, 2222);
    assert_eq!(rows[2].1, 4444);

    // Test unbounded lower range for INT4
    let rows: Vec<(i32, i32)> = r#"
    SELECT id, value_int4 FROM test_table
    WHERE test_table @@@ '{
        "range": {
            "field": "value_int4",
            "lower_bound": null,
            "upper_bound": {"included": 2222}
        }
    }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].1, -1111);
    assert_eq!(rows[1].1, 2222);

    // Test unbounded upper range for INT8
    let rows: Vec<(i32, i64)> = r#"
    SELECT id, value_int8 FROM test_table
    WHERE test_table @@@ '{
        "range": {
            "field": "value_int8",
            "lower_bound": {"included": 0},
            "upper_bound": null
        }
    }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].1, 22222222);
    assert_eq!(rows[2].1, 44444444);

    // Test unbounded lower range for INT8
    let rows: Vec<(i32, i64)> = r#"
    SELECT id, value_int8 FROM test_table
    WHERE test_table @@@ '{
        "range": {
            "field": "value_int8",
            "lower_bound": null,
            "upper_bound": {"included": -5000000}
        }
    }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1, -11111111);
}

#[rstest]
fn float_range(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value_float4 FLOAT4,
        value_float8 FLOAT8,
        value_numeric NUMERIC
    );

    INSERT INTO test_table (value_float4, value_float8, value_numeric) VALUES
        (-1.1, -1111.1111, -111.11111),
        (2.2, 2222.2222, 222.22222),
        (3.3, 3333.3333, 333.33333),
        (4.4, 4444.4444, 444.44444);
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON public.test_table
    USING bm25 (id, value_float4, value_float8, value_numeric)
    WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    // FLOAT4
    let rows: Vec<(i32, f32)> = r#"
    SELECT id, value_float4 FROM test_table
    WHERE test_table @@@ '{
        "range": {
            "field": "value_float4",
            "lower_bound": {"included": -2.0},
            "upper_bound": {"included": 3.0}
        }
    }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);

    // FLOAT8
    let rows: Vec<(i32, f64)> = r#"
    SELECT id, value_float8 FROM test_table
    WHERE test_table @@@ '{
        "range": {
            "field": "value_float8",
            "lower_bound": {"excluded": 2222.2222},
            "upper_bound": {"included": 3333.3333}
        }
    }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 1);

    // NUMERIC
    let rows: Vec<(i32,)> = r#"
    SELECT id FROM test_table
    WHERE test_table @@@ '{
        "range": {
            "field": "value_numeric",
            "lower_bound": {"included": 0.0},
            "upper_bound": {"excluded": 400.0}
        }
    }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn datetime_range(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value_date DATE,
        value_timestamp TIMESTAMP,
        value_timestamptz TIMESTAMP WITH TIME ZONE
    );

    INSERT INTO test_table (value_date, value_timestamp, value_timestamptz) VALUES 
        (DATE '2023-05-03', TIMESTAMP '2023-04-15 13:27:09', TIMESTAMP WITH TIME ZONE '2023-04-15 13:27:09 PST'),
        (DATE '2022-07-14', TIMESTAMP '2022-05-16 07:38:43', TIMESTAMP WITH TIME ZONE '2022-05-16 07:38:43 EST'),
        (DATE '2021-04-30', TIMESTAMP '2021-06-08 08:49:21', TIMESTAMP WITH TIME ZONE '2021-06-08 08:49:21 CST'),
        (DATE '2020-06-28', TIMESTAMP '2020-07-09 15:52:13', TIMESTAMP WITH TIME ZONE '2020-07-09 15:52:13 MST');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON public.test_table
    USING bm25 (id, value_date, value_timestamp, value_timestamptz)
    WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    // DATE
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@ '{
        "range": {
            "field": "value_date",
            "lower_bound": {"included": "2020-05-20T00:00:00.000000Z"},
            "upper_bound": {"included": "2022-06-13T00:00:00.000000Z"},
            "is_datetime": true
        }
    }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);

    // TIMESTAMP
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@ '{
        "range": {
            "field": "value_timestamp",
            "lower_bound": {"included": "2019-08-02T07:52:43.000000Z"},
            "upper_bound": {"included": "2021-06-10T10:32:41.000000Z"},
            "is_datetime": true
        }
    }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);

    // TIMESTAMP WITH TIME ZONE
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@ '{
        "range": {
            "field": "value_timestamptz",
            "lower_bound": {"included": "2020-07-09T21:52:13.000000Z"},
            "upper_bound": {"included": "2022-05-16T12:38:43.000000Z"},
            "is_datetime": true
        }
    }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 3);
}
