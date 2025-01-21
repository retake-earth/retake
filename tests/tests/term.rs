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
fn boolean_term(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value BOOLEAN
    );

    INSERT INTO test_table (value) VALUES (true), (false), (false), (true);
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table
    USING bm25 (id, value) WITH (key_field='id', boolean_fields='{"value": {}}');
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, bool)> = r#"
    SELECT * FROM test_table
    WHERE test_table @@@ paradedb.term(field => 'value', value => true)
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(1, true), (4, true)]);
}

#[rstest]
fn integer_term(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value_int2 SMALLINT,
        value_int4 INTEGER,
        value_int8 BIGINT
    );

    INSERT INTO test_table (value_int2, value_int4, value_int8) VALUES 
        (-11, -1111, -11111111),
        (22, 2222, 22222222), 
        (33, 3333, 33333333), 
        (44, 4444, 44444444);
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table
    USING bm25 (id, value_int2, value_int4, value_int8) WITH (key_field='id', numeric_fields='{"value_int2": {}, "value_int4": {}, "value_int8": {}}');
    "#
    .execute(&mut conn);

    // INT2
    let rows: Vec<(i32, i16)> = r#"
    SELECT id, value_int2 FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_int2', value => -11)
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(1, -11)]);

    // INT4
    let rows: Vec<(i32, i32)> = r#"
    SELECT id, value_int4 FROM test_table WHERE test_table @@@
    paradedb.term(field => 'value_int4', value => 2222)
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(2, 2222)]);

    // INT8
    let rows: Vec<(i32, i64)> = r#"
    SELECT id, value_int8 FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_int8', value => 33333333)
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(3, 33333333)]);
}

#[rstest]
fn float_term(mut conn: PgConnection) {
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
    CREATE INDEX test_index ON test_table
    USING bm25 (id, value_float4, value_float8, value_numeric) WITH (key_field='id', numeric_fields='{"value_float4": {}, "value_float8": {}, "value_numeric": {}}');
    "#
    .execute(&mut conn);

    // FLOAT4
    let rows: Vec<(i32, f32)> = r#"
    SELECT id, value_float4 FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_float4', value => -1.1::float4)
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(1, -1.1)]);

    // FLOAT8
    let rows: Vec<(i32, f64)> = r#"
    SELECT id, value_float8 FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_float8', value => 4444.4444::float8)
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(4, 4444.4444)]);

    // NUMERIC - no sqlx::Type for numerics, so just check id
    let rows: Vec<(i32,)> = r#"
    SELECT id FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_numeric', value => 333.33333::numeric)
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(3,)]);
}

#[rstest]
fn text_term(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value_text TEXT,
        value_varchar VARCHAR(64),
        value_uuid UUID
    );

    INSERT INTO test_table (value_text, value_varchar, value_uuid) VALUES
        ('abc', 'var abc', 'a99e7330-37e6-4f14-8c95-985052ee74f3'::uuid),
        ('def', 'var def', '2fe779f1-2a74-4035-9f1a-9477bae0364c'::uuid),
        ('ghi', 'var ghi', 'b9592b87-82ea-4d7b-8865-f6be819d4f0f'::uuid),
        ('jkl', 'var jkl', 'ae9d4a8c-8382-452d-96fb-a9a1c4192a03'::uuid);
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table
    USING bm25 (id, value_text, value_varchar, value_uuid) WITH (key_field='id', text_fields='{
        "value_text": {}, 
        "value_varchar": {}, 
        "value_uuid": {"tokenizer": {"type": "raw"}, "normalizer": "raw", "record": "basic", "fieldnorms": false}
    }');
    "#
    .execute(&mut conn);

    // TEXT
    let rows: Vec<(i32, String)> = r#"
    SELECT id, value_text FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_text', value => 'abc')
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(1, "abc".into())]);

    // VARCHAR
    let rows: Vec<(i32, String)> = r#"
    SELECT id, value_varchar FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_varchar', value => 'ghi')
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(3, "var ghi".into())]);

    // UUID - sqlx doesn't have a uuid type, so we just look for id
    let rows: Vec<(i32,)> = r#"
    SELECT id FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_uuid', value => 'ae9d4a8c-8382-452d-96fb-a9a1c4192a03')
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(4,)]);
}

#[rstest]
fn datetime_term(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SERIAL PRIMARY KEY,
        value_date DATE,
        value_timestamp TIMESTAMP,
        value_timestamptz TIMESTAMP WITH TIME ZONE,
        value_time TIME,
        value_timetz TIME WITH TIME ZONE
    );

    INSERT INTO test_table (value_date, value_timestamp, value_timestamptz, value_time, value_timetz) VALUES 
        (DATE '2023-05-03', TIMESTAMP '2023-04-15 13:27:09', TIMESTAMP WITH TIME ZONE '2023-04-15 13:27:09 PST', TIME '08:09:10', TIME WITH TIME ZONE '08:09:10 PST'),
        (DATE '2021-06-28', TIMESTAMP '2019-08-02 07:52:43.123', TIMESTAMP WITH TIME ZONE '2019-08-02 07:52:43.123 EST', TIME '11:43:21.456', TIME WITH TIME ZONE '11:43:21.456 EST');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table
    USING bm25 (id, value_date, value_timestamp, value_timestamptz, value_time, value_timetz) WITH (key_field='id', datetime_fields='{
        "value_date": {}, 
        "value_timestamp": {}, 
        "value_timestamptz": {}, 
        "value_time": {}, 
        "value_timetz": {}
    }');
    "#
    .execute(&mut conn);

    // DATE
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_date', value => DATE '2023-05-03')
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(1,)]);

    // TIMESTAMP
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_timestamp', value => TIMESTAMP '2019-08-02 07:52:43.123')
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(2,)]);

    // TIMESTAMP WITH TIME ZONE
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_timestamptz', value => TIMESTAMP WITH TIME ZONE '2023-04-15 13:27:09 PST')
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(1,)]);

    // TIMESTAMP WITH TIME ZONE: Change time zone in query
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_timestamptz', value => TIMESTAMP WITH TIME ZONE '2023-04-15 16:27:09 EST')
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(1,)]);

    // TIME
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_time', value => TIME '11:43:21.456')
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(2,)]);

    // TIME WITH TIME ZONE
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_timetz', value => TIME WITH TIME ZONE '11:43:21.456 EST')
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(2,)]);

    // TIME WITH TIME ZONE: Change time zone in query
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_timetz', value => TIME WITH TIME ZONE '08:43:21.456 PST')
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(2,)]);

    // TIMESTAMP WITH TIME ZONE: Query no time zone with time zone
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_timestamp', value => TIMESTAMP WITH TIME ZONE '2023-04-15 13:27:09 GMT')
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(1,)]);

    // TIMESTAMP: Query time zone with no time zone (GMT = EST + 5)
    let rows: Vec<(i32,)> = r#"
    SELECT * FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value_timestamptz', value => TIMESTAMP '2019-08-02 12:52:43.123')
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(2,)]);
}
