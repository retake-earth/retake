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

// In addition to checking whether all the expected types work for keys, make sure to include tests for anything that
//    is reliant on keys (e.g. stable_sort, alias)

#[rstest]
fn boolean_key(mut conn: PgConnection) {
    // Boolean keys are pretty useless, but they're supported!

    r#"
    CREATE TABLE test_table (
        id BOOLEAN,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES (true, 'bluetooth'), (false, 'blue');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table USING bm25 (id, value)
    WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');
    "#
    .execute(&mut conn);

    // stable_sort
    let rows: Vec<(bool, f32)> = r#"
    SELECT id, paradedb.score(id) FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value', value => 'blue')
    ORDER BY score DESC
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows, vec![(false, 0.25759196), (true, 0.14109309)]);

    // no stable_sort
    let rows: Vec<(f32,)> = r#"
    SELECT paradedb.score(id) FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value', value => 'blue')
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn uuid_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id UUID,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES ('f159c89e-2162-48cd-85e3-e42b71d2ecd0', 'bluetooth');
    INSERT INTO test_table (id, value) VALUES ('38bf27a0-1aa8-42cd-9cb0-993025e0b8d0', 'bluebell');
    INSERT INTO test_table (id, value) VALUES ('b5faacc0-9eba-441a-81f8-820b46a3b57e', 'jetblue');
    INSERT INTO test_table (id, value) VALUES ('eb833eb6-c598-4042-b84a-0045828fceea', 'blue''s clues');
    INSERT INTO test_table (id, value) VALUES ('ea1181a0-5d3e-4f5f-a6ab-b1354ffc91ad', 'blue bloods');
    INSERT INTO test_table (id, value) VALUES ('28b6374a-67d3-41c8-93af-490712f9923e', 'redness');
    INSERT INTO test_table (id, value) VALUES ('f6e85626-298e-4112-9abb-3856f8aa046a', 'yellowtooth');
    INSERT INTO test_table (id, value) VALUES ('88345d21-7b89-4fd6-87e4-83a4f68dbc3c', 'great white');
    INSERT INTO test_table (id, value) VALUES ('40bc9216-66d0-4ae8-87ee-ddb02e3e1b33', 'blue skies');
    INSERT INTO test_table (id, value) VALUES ('02f9789d-4963-47d5-a189-d9c114f5cba4', 'rainbow');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table USING bm25 (id, value)
    WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');
    "#
    .execute(&mut conn);

    // stable_sort
    let rows: Vec<(String, f32)> = r#"
    SELECT CAST(id AS TEXT), paradedb.score(id) FROM test_table WHERE test_table @@@ 
        paradedb.term(field => 'value', value => 'blue') ORDER BY score desc
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![
            (
                "b5faacc0-9eba-441a-81f8-820b46a3b57e".to_string(),
                0.61846066
            ),
            (
                "38bf27a0-1aa8-42cd-9cb0-993025e0b8d0".to_string(),
                0.57459813
            ),
            (
                "f159c89e-2162-48cd-85e3-e42b71d2ecd0".to_string(),
                0.53654534
            ),
            (
                "40bc9216-66d0-4ae8-87ee-ddb02e3e1b33".to_string(),
                0.50321954
            ),
            (
                "ea1181a0-5d3e-4f5f-a6ab-b1354ffc91ad".to_string(),
                0.47379148
            ),
            (
                "eb833eb6-c598-4042-b84a-0045828fceea".to_string(),
                0.44761515
            ),
        ]
    );

    // no stable_sort
    let rows: Vec<(f32,)> = r#"
    SELECT paradedb.score(id) FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value', value => 'blue') ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 6);

    let rows: Vec<(String, String)> = r#"
    SELECT CAST(id AS TEXT), paradedb.snippet(value) FROM test_table WHERE value @@@ 'blue'
    UNION
    SELECT CAST(id AS TEXT), paradedb.snippet(value) FROM test_table WHERE value @@@ 'tooth'
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 8);
}

#[rstest]
fn i64_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id BIGINT,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES (1, 'bluetooth');
    INSERT INTO test_table (id, value) VALUES (2, 'bluebell');
    INSERT INTO test_table (id, value) VALUES (3, 'jetblue');
    INSERT INTO test_table (id, value) VALUES (4, 'blue''s clues');
    INSERT INTO test_table (id, value) VALUES (5, 'blue bloods');
    INSERT INTO test_table (id, value) VALUES (6, 'redness');
    INSERT INTO test_table (id, value) VALUES (7, 'yellowtooth');
    INSERT INTO test_table (id, value) VALUES (8, 'great white');
    INSERT INTO test_table (id, value) VALUES (9, 'blue skies');
    INSERT INTO test_table (id, value) VALUES (10, 'rainbow');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table USING bm25 (id, value)
    WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');
    "#
    .execute(&mut conn);

    // stable_sort
    let rows: Vec<(i64, f32)> = r#"
    SELECT id, paradedb.score(id) FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value', value => 'blue') ORDER BY score DESC
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![
            (3, 0.61846066),
            (2, 0.57459813),
            (1, 0.53654534),
            (9, 0.50321954),
            (5, 0.47379148),
            (4, 0.44761515),
        ]
    );

    // no stable_sort
    let rows: Vec<(f32,)> = r#"
    SELECT paradedb.score(id) FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value', value => 'blue') ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 6);

    let rows: Vec<(i64, String)> = r#"
    SELECT id, paradedb.snippet(value) FROM test_table WHERE value @@@ 'blue'
    UNION
    SELECT id, paradedb.snippet(value) FROM test_table WHERE value @@@ 'tooth'
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 8);
}

#[rstest]
fn i32_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id INT,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES (1, 'bluetooth');
    INSERT INTO test_table (id, value) VALUES (2, 'bluebell');
    INSERT INTO test_table (id, value) VALUES (3, 'jetblue');
    INSERT INTO test_table (id, value) VALUES (4, 'blue''s clues');
    INSERT INTO test_table (id, value) VALUES (5, 'blue bloods');
    INSERT INTO test_table (id, value) VALUES (6, 'redness');
    INSERT INTO test_table (id, value) VALUES (7, 'yellowtooth');
    INSERT INTO test_table (id, value) VALUES (8, 'great white');
    INSERT INTO test_table (id, value) VALUES (9, 'blue skies');
    INSERT INTO test_table (id, value) VALUES (10, 'rainbow');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table USING bm25 (id, value)
    WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');
    "#
    .execute(&mut conn);

    // stable_sort
    let rows: Vec<(i32, f32)> = r#"
    SELECT id, paradedb.score(id) FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value', value => 'blue') ORDER BY score DESC
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![
            (3, 0.61846066),
            (2, 0.57459813),
            (1, 0.53654534),
            (9, 0.50321954),
            (5, 0.47379148),
            (4, 0.44761515),
        ]
    );

    // no stable_sort
    let rows: Vec<(f32,)> = r#"
    SELECT paradedb.score(id) FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value', value => 'blue') ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 6);
}

#[rstest]
fn i16_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id SMALLINT,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES (1, 'bluetooth');
    INSERT INTO test_table (id, value) VALUES (2, 'bluebell');
    INSERT INTO test_table (id, value) VALUES (3, 'jetblue');
    INSERT INTO test_table (id, value) VALUES (4, 'blue''s clues');
    INSERT INTO test_table (id, value) VALUES (5, 'blue bloods');
    INSERT INTO test_table (id, value) VALUES (6, 'redness');
    INSERT INTO test_table (id, value) VALUES (7, 'yellowtooth');
    INSERT INTO test_table (id, value) VALUES (8, 'great white');
    INSERT INTO test_table (id, value) VALUES (9, 'blue skies');
    INSERT INTO test_table (id, value) VALUES (10, 'rainbow');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table USING bm25 (id, value)
    WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');
    "#
    .execute(&mut conn);

    // stable_sort
    let rows: Vec<(i16, f32)> = r#"
    SELECT id, paradedb.score(id) FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value', value => 'blue') ORDER BY score DESC
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![
            (3, 0.61846066),
            (2, 0.57459813),
            (1, 0.53654534),
            (9, 0.50321954),
            (5, 0.47379148),
            (4, 0.44761515),
        ]
    );

    // no stable_sort
    let rows: Vec<(f32,)> = r#"
    SELECT paradedb.score(id) FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value', value => 'blue')
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 6);
}

#[rstest]
fn f32_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id FLOAT4,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES (1.1, 'bluetooth');
    INSERT INTO test_table (id, value) VALUES (2.2, 'bluebell');
    INSERT INTO test_table (id, value) VALUES (3.3, 'jetblue');
    INSERT INTO test_table (id, value) VALUES (4.4, 'blue''s clues');
    INSERT INTO test_table (id, value) VALUES (5.5, 'blue bloods');
    INSERT INTO test_table (id, value) VALUES (6.6, 'redness');
    INSERT INTO test_table (id, value) VALUES (7.7, 'yellowtooth');
    INSERT INTO test_table (id, value) VALUES (8.8, 'great white');
    INSERT INTO test_table (id, value) VALUES (9.9, 'blue skies');
    INSERT INTO test_table (id, value) VALUES (10.1, 'rainbow');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table USING bm25 (id, value)
    WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');
    "#
    .execute(&mut conn);

    // stable_sort
    let rows: Vec<(f32, f32)> = r#"
    SELECT id, paradedb.score(id) FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value', value => 'blue') ORDER BY score DESC
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![
            (3.3, 0.61846066),
            (2.2, 0.57459813),
            (1.1, 0.53654534),
            (9.9, 0.50321954),
            (5.5, 0.47379148),
            (4.4, 0.44761515),
        ]
    );

    // no stable_sort
    let rows: Vec<(f32,)> = r#"
    SELECT paradedb.score(id) FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value', value => 'blue')
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 6);
}

#[rstest]
fn f64_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id FLOAT8,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES (1.1, 'bluetooth');
    INSERT INTO test_table (id, value) VALUES (2.2, 'bluebell');
    INSERT INTO test_table (id, value) VALUES (3.3, 'jetblue');
    INSERT INTO test_table (id, value) VALUES (4.4, 'blue''s clues');
    INSERT INTO test_table (id, value) VALUES (5.5, 'blue bloods');
    INSERT INTO test_table (id, value) VALUES (6.6, 'redness');
    INSERT INTO test_table (id, value) VALUES (7.7, 'yellowtooth');
    INSERT INTO test_table (id, value) VALUES (8.8, 'great white');
    INSERT INTO test_table (id, value) VALUES (9.9, 'blue skies');
    INSERT INTO test_table (id, value) VALUES (10.1, 'rainbow');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table USING bm25 (id, value)
    WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');
    "#
    .execute(&mut conn);

    // stable_sort
    let rows: Vec<(f64, f32)> = r#"
    SELECT id, paradedb.score(id) FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value', value => 'blue') ORDER BY score DESC
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![
            (3.3, 0.61846066),
            (2.2, 0.57459813),
            (1.1, 0.53654534),
            (9.9, 0.50321954),
            (5.5, 0.47379148),
            (4.4, 0.44761515),
        ]
    );

    // no stable_sort
    let rows: Vec<(f32,)> = r#"
    SELECT paradedb.score(id) FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value', value => 'blue')
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 6);
}

#[rstest]
fn numeric_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id NUMERIC,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES (1.1, 'bluetooth');
    INSERT INTO test_table (id, value) VALUES (2.2, 'bluebell');
    INSERT INTO test_table (id, value) VALUES (3.3, 'jetblue');
    INSERT INTO test_table (id, value) VALUES (4.4, 'blue''s clues');
    INSERT INTO test_table (id, value) VALUES (5.5, 'blue bloods');
    INSERT INTO test_table (id, value) VALUES (6.6, 'redness');
    INSERT INTO test_table (id, value) VALUES (7.7, 'yellowtooth');
    INSERT INTO test_table (id, value) VALUES (8.8, 'great white');
    INSERT INTO test_table (id, value) VALUES (9.9, 'blue skies');
    INSERT INTO test_table (id, value) VALUES (10.1, 'rainbow');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table USING bm25 (id, value)
    WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');
    "#
    .execute(&mut conn);

    // stable_sort
    let rows: Vec<(f64, f32)> = r#"
    SELECT CAST(id AS FLOAT8), paradedb.score(id) FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value', value => 'blue') ORDER BY score DESC
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![
            (3.3, 0.61846066),
            (2.2, 0.57459813),
            (1.1, 0.53654534),
            (9.9, 0.50321954),
            (5.5, 0.47379148),
            (4.4, 0.44761515),
        ]
    );

    // no stable_sort
    let rows: Vec<(f32,)> = r#"
    SELECT paradedb.score(id) FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value', value => 'blue')
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 6);
}

#[rstest]
fn string_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id TEXT,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES ('f159c89e-2162-48cd-85e3-e42b71d2ecd0', 'bluetooth');
    INSERT INTO test_table (id, value) VALUES ('38bf27a0-1aa8-42cd-9cb0-993025e0b8d0', 'bluebell');
    INSERT INTO test_table (id, value) VALUES ('b5faacc0-9eba-441a-81f8-820b46a3b57e', 'jetblue');
    INSERT INTO test_table (id, value) VALUES ('eb833eb6-c598-4042-b84a-0045828fceea', 'blue''s clues');
    INSERT INTO test_table (id, value) VALUES ('ea1181a0-5d3e-4f5f-a6ab-b1354ffc91ad', 'blue bloods');
    INSERT INTO test_table (id, value) VALUES ('28b6374a-67d3-41c8-93af-490712f9923e', 'redness');
    INSERT INTO test_table (id, value) VALUES ('f6e85626-298e-4112-9abb-3856f8aa046a', 'yellowtooth');
    INSERT INTO test_table (id, value) VALUES ('88345d21-7b89-4fd6-87e4-83a4f68dbc3c', 'great white');
    INSERT INTO test_table (id, value) VALUES ('40bc9216-66d0-4ae8-87ee-ddb02e3e1b33', 'blue skies');
    INSERT INTO test_table (id, value) VALUES ('02f9789d-4963-47d5-a189-d9c114f5cba4', 'rainbow');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table USING bm25 (id, value)
    WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');
    "#
    .execute(&mut conn);

    // stable_sort
    let rows: Vec<(String, f32)> = r#"
    SELECT id, paradedb.score(id) FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value', value => 'blue') ORDER BY score DESC
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![
            (
                "b5faacc0-9eba-441a-81f8-820b46a3b57e".to_string(),
                0.61846066
            ),
            (
                "38bf27a0-1aa8-42cd-9cb0-993025e0b8d0".to_string(),
                0.57459813
            ),
            (
                "f159c89e-2162-48cd-85e3-e42b71d2ecd0".to_string(),
                0.53654534
            ),
            (
                "40bc9216-66d0-4ae8-87ee-ddb02e3e1b33".to_string(),
                0.50321954
            ),
            (
                "ea1181a0-5d3e-4f5f-a6ab-b1354ffc91ad".to_string(),
                0.47379148
            ),
            (
                "eb833eb6-c598-4042-b84a-0045828fceea".to_string(),
                0.44761515
            ),
        ]
    );

    // no stable_sort
    let rows: Vec<(f32,)> = r#"
    SELECT paradedb.score(id) FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value', value => 'blue')
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 6);
}

#[rstest]
fn date_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id DATE,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES ('2023-05-03', 'bluetooth');
    INSERT INTO test_table (id, value) VALUES ('2023-05-04', 'bluebell');
    INSERT INTO test_table (id, value) VALUES ('2023-05-05', 'jetblue');
    INSERT INTO test_table (id, value) VALUES ('2023-05-06', 'blue''s clues');
    INSERT INTO test_table (id, value) VALUES ('2023-05-07', 'blue bloods');
    INSERT INTO test_table (id, value) VALUES ('2023-05-08', 'redness');
    INSERT INTO test_table (id, value) VALUES ('2023-05-09', 'yellowtooth');
    INSERT INTO test_table (id, value) VALUES ('2023-05-10', 'great white');
    INSERT INTO test_table (id, value) VALUES ('2023-05-11', 'blue skies');
    INSERT INTO test_table (id, value) VALUES ('2023-05-12', 'rainbow');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table USING bm25 (id, value)
    WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');
    "#
    .execute(&mut conn);

    // stable_sort
    let rows: Vec<(String, f32)> = r#"
    SELECT CAST(id AS TEXT), paradedb.score(id) FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value', value => 'blue') ORDER BY score DESC
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![
            ("2023-05-05".to_string(), 0.61846066),
            ("2023-05-04".to_string(), 0.57459813),
            ("2023-05-03".to_string(), 0.53654534),
            ("2023-05-11".to_string(), 0.50321954),
            ("2023-05-07".to_string(), 0.47379148),
            ("2023-05-06".to_string(), 0.44761515),
        ]
    );

    // no stable_sort
    let rows: Vec<(f32,)> = r#"
    SELECT paradedb.score(id) FROM test_table WHERE test_table @@@ 
        paradedb.term(field => 'value', value => 'blue')
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 6);
}

#[rstest]
fn time_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id TIME,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES ('08:09:10', 'bluetooth');
    INSERT INTO test_table (id, value) VALUES ('09:10:11', 'bluebell');
    INSERT INTO test_table (id, value) VALUES ('10:11:12', 'jetblue');
    INSERT INTO test_table (id, value) VALUES ('11:12:13', 'blue''s clues');
    INSERT INTO test_table (id, value) VALUES ('12:13:14', 'blue bloods');
    INSERT INTO test_table (id, value) VALUES ('13:14:15', 'redness');
    INSERT INTO test_table (id, value) VALUES ('14:15:16', 'yellowtooth');
    INSERT INTO test_table (id, value) VALUES ('15:16:17', 'great white');
    INSERT INTO test_table (id, value) VALUES ('16:17:18', 'blue skies');
    INSERT INTO test_table (id, value) VALUES ('17:18:19', 'rainbow');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table USING bm25 (id, value)
    WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');
    "#
    .execute(&mut conn);

    // stable_sort
    let rows: Vec<(String, f32)> = r#"
    SELECT CAST(id AS TEXT), paradedb.score(id) FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value', value => 'blue') ORDER BY score DESC
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![
            ("10:11:12".to_string(), 0.61846066),
            ("09:10:11".to_string(), 0.57459813),
            ("08:09:10".to_string(), 0.53654534),
            ("16:17:18".to_string(), 0.50321954),
            ("12:13:14".to_string(), 0.47379148),
            ("11:12:13".to_string(), 0.44761515),
        ]
    );

    // no stable_sort
    let rows: Vec<(f32,)> = r#"
    SELECT paradedb.score(id) FROM test_table WHERE test_table @@@ 
        paradedb.term(field => 'value', value => 'blue')
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 6);
}

#[rstest]
fn timestamp_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id TIMESTAMP,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES ('2023-05-03 08:09:10', 'bluetooth');
    INSERT INTO test_table (id, value) VALUES ('2023-05-04 09:10:11', 'bluebell');
    INSERT INTO test_table (id, value) VALUES ('2023-05-05 10:11:12', 'jetblue');
    INSERT INTO test_table (id, value) VALUES ('2023-05-06 11:12:13', 'blue''s clues');
    INSERT INTO test_table (id, value) VALUES ('2023-05-07 12:13:14', 'blue bloods');
    INSERT INTO test_table (id, value) VALUES ('2023-05-08 13:14:15', 'redness');
    INSERT INTO test_table (id, value) VALUES ('2023-05-09 14:15:16', 'yellowtooth');
    INSERT INTO test_table (id, value) VALUES ('2023-05-10 15:16:17', 'great white');
    INSERT INTO test_table (id, value) VALUES ('2023-05-11 16:17:18', 'blue skies');
    INSERT INTO test_table (id, value) VALUES ('2023-05-12 17:18:19', 'rainbow');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table USING bm25 (id, value)
    WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');
    "#
    .execute(&mut conn);

    // stable_sort
    let rows: Vec<(String, f32)> = r#"
    SELECT CAST(id AS TEXT), paradedb.score(id) FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value', value => 'blue') ORDER BY score DESC
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![
            ("2023-05-05 10:11:12".to_string(), 0.61846066),
            ("2023-05-04 09:10:11".to_string(), 0.57459813),
            ("2023-05-03 08:09:10".to_string(), 0.53654534),
            ("2023-05-11 16:17:18".to_string(), 0.50321954),
            ("2023-05-07 12:13:14".to_string(), 0.47379148),
            ("2023-05-06 11:12:13".to_string(), 0.44761515),
        ]
    );

    // no stable_sort
    let rows: Vec<(f32,)> = r#"
    SELECT paradedb.score(id) FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value', value => 'blue')
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 6);
}

#[rstest]
fn timestamptz_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id TIMESTAMP WITH TIME ZONE,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES ('2023-05-03 08:09:10 EST', 'bluetooth');
    INSERT INTO test_table (id, value) VALUES ('2023-05-04 09:10:11 PST', 'bluebell');
    INSERT INTO test_table (id, value) VALUES ('2023-05-05 10:11:12 MST', 'jetblue');
    INSERT INTO test_table (id, value) VALUES ('2023-05-06 11:12:13 CST', 'blue''s clues');
    INSERT INTO test_table (id, value) VALUES ('2023-05-07 12:13:14 EST', 'blue bloods');
    INSERT INTO test_table (id, value) VALUES ('2023-05-08 13:14:15 PST', 'redness');
    INSERT INTO test_table (id, value) VALUES ('2023-05-09 14:15:16 MST', 'yellowtooth');
    INSERT INTO test_table (id, value) VALUES ('2023-05-10 15:16:17 CST', 'great white');
    INSERT INTO test_table (id, value) VALUES ('2023-05-11 16:17:18 EST', 'blue skies');
    INSERT INTO test_table (id, value) VALUES ('2023-05-12 17:18:19 PST', 'rainbow');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table USING bm25 (id, value)
    WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');
    "#
    .execute(&mut conn);

    // stable_sort
    let rows: Vec<(String, f32)> = r#"
    SELECT CAST(id AS TEXT), paradedb.score(id) FROM test_table
    WHERE test_table @@@ paradedb.term(field => 'value', value => 'blue') ORDER BY score DESC
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![
            ("2023-05-05 17:11:12+00".to_string(), 0.61846066),
            ("2023-05-04 17:10:11+00".to_string(), 0.57459813),
            ("2023-05-03 13:09:10+00".to_string(), 0.53654534),
            ("2023-05-11 21:17:18+00".to_string(), 0.50321954),
            ("2023-05-07 17:13:14+00".to_string(), 0.47379148),
            ("2023-05-06 17:12:13+00".to_string(), 0.44761515),
        ]
    );

    // no stable_sort
    let rows: Vec<(f32,)> = r#"
    SELECT  paradedb.score(id) FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value', value => 'blue') 
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 6);

    let rows: Vec<(String, String)> = r#"
    SELECT CAST(id AS TEXT), paradedb.snippet(value) FROM test_table WHERE value @@@ 'blue'
    UNION
    SELECT CAST(id AS TEXT), paradedb.snippet(value) FROM test_table WHERE value @@@ 'tooth'
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 8);
}

#[rstest]
fn timetz_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_table (
        id TIME WITH TIME ZONE,
        value TEXT
    );

    INSERT INTO test_table (id, value) VALUES ('08:09:10 EST', 'bluetooth');
    INSERT INTO test_table (id, value) VALUES ('09:10:11 PST', 'bluebell');
    INSERT INTO test_table (id, value) VALUES ('10:11:12 MST', 'jetblue');
    INSERT INTO test_table (id, value) VALUES ('11:12:13 CST', 'blue''s clues');
    INSERT INTO test_table (id, value) VALUES ('12:13:14 EST', 'blue bloods');
    INSERT INTO test_table (id, value) VALUES ('13:14:15 PST', 'redness');
    INSERT INTO test_table (id, value) VALUES ('14:15:16 MST', 'yellowtooth');
    INSERT INTO test_table (id, value) VALUES ('15:16:17 CST', 'great white');
    INSERT INTO test_table (id, value) VALUES ('16:17:18 EST', 'blue skies');
    INSERT INTO test_table (id, value) VALUES ('17:18:19 PST', 'rainbow');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX test_index ON test_table USING bm25 (id, value)
    WITH (key_field='id', text_fields='{"value": {"tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false}}}');
    "#
    .execute(&mut conn);

    let rows: Vec<(String,)> = r#"
    SELECT CAST(id AS TEXT) FROM test_table"#
        .fetch_collect(&mut conn);

    println!("{:#?}", rows);

    // stable_sort
    let rows: Vec<(String, f32)> = r#"
    SELECT CAST(id AS TEXT), paradedb.score(id) FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value', value => 'blue') ORDER BY score DESC
    "#
    .fetch_collect(&mut conn);
    assert_eq!(
        rows,
        vec![
            ("10:11:12-07".to_string(), 0.61846066),
            ("09:10:11-08".to_string(), 0.57459813),
            ("08:09:10-05".to_string(), 0.53654534),
            ("16:17:18-05".to_string(), 0.50321954),
            ("12:13:14-05".to_string(), 0.47379148),
            ("11:12:13-06".to_string(), 0.44761515),
        ]
    );

    // no stable_sort
    let rows: Vec<(f32,)> = r#"
    SELECT paradedb.score(id) FROM test_table WHERE test_table @@@ 
    paradedb.term(field => 'value', value => 'blue')
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 6);
}
