#![allow(dead_code)]
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

use core::panic;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::{PgConnection, Row};

#[rstest]
fn boolean_tree(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
    paradedb.boolean(
        should => ARRAY[
            paradedb.parse('description:shoes'),
            paradedb.phrase_prefix(field => 'description', phrases => ARRAY['book']),
            paradedb.term(field => 'description', value => 'speaker'),
		    paradedb.fuzzy_term(field => 'description', value => 'wolo', transposition_cost_one => false, distance => 1, prefix => true)
        ]
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3, 4, 5, 7, 10, 32, 33, 34, 37, 39, 41]);
}

#[rstest]
fn fuzzy_term(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ paradedb.fuzzy_term(field => 'category', value => 'elector', prefix => true)
    ORDER BY id"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2, 12, 22, 32], "wrong results");

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
    paradedb.term(field => 'category', value => 'electornics')
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert!(columns.is_empty(), "without fuzzy field should be empty");

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
        paradedb.fuzzy_term(
            field => 'description',
            value => 'keybaord',
            transposition_cost_one => false,
            distance => 1,
            prefix => true
        ) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert!(
        columns.is_empty(),
        "transposition_cost_one false should be empty"
    );

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
        paradedb.fuzzy_term(
            field => 'description',
            value => 'keybaord',
            transposition_cost_one => true,
            distance => 1,
            prefix => true
        ) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(
        columns.id,
        vec![1, 2],
        "incorrect transposition_cost_one true"
    );

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
        paradedb.fuzzy_term(
            field => 'description',
            value => 'keybaord',
            prefix => true
        ) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2], "incorrect defaults");
}

#[rstest]
fn single_queries(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // All
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
    paradedb.all() ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // Boost
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
    paradedb.boost(query => paradedb.all(), factor => 1.5)
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // ConstScore
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ paradedb.const_score(query => paradedb.all(), score => 3.9)
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // DisjunctionMax
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
    paradedb.disjunction_max(disjuncts => ARRAY[paradedb.parse('description:shoes')])
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 3);

    // Empty
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ paradedb.empty() ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 0);

    // FuzzyTerm
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ paradedb.fuzzy_term(
        field => 'description',
        value => 'wolo',
        transposition_cost_one => false,
        distance => 1,
        prefix => true
    ) ORDER BY ID"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 4);

    // Parse
    let columns: SimpleProductsTableVec = r#"
        SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
        paradedb.parse('description:teddy') ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // PhrasePrefix
    let columns: SimpleProductsTableVec = r#"
        SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
        paradedb.phrase_prefix(field => 'description', phrases => ARRAY['har'])
        ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // Phrase with invalid term list
    match r#"
        SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
        paradedb.phrase(field => 'description', phrases => ARRAY['robot'])
        ORDER BY id"#
        .fetch_result::<SimpleProductsTable>(&mut conn)
    {
        Err(err) => assert!(err
            .to_string()
            .contains("required to have strictly more than one term")),
        _ => panic!("phrase prefix query should require multiple terms"),
    }

    // Phrase
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ paradedb.phrase(
        field => 'description',
        phrases => ARRAY['robot', 'building', 'kit']
    ) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // Range
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
        paradedb.range(field => 'last_updated_date', range => '[2023-05-01,2023-05-03]'::daterange)
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 7);

    // Regex
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ paradedb.regex(
        field => 'description',
        pattern => '(hardcover|plush|leather|running|wireless)'
    ) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 5);

    // Test regex anchors
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ paradedb.regex(
        field => 'description',
        pattern => '^running'
    ) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(
        columns.len(),
        1,
        "start anchor ^ should match exactly one item"
    );

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ paradedb.regex(
        field => 'description',
        pattern => 'keyboard$'
    ) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 2, "end anchor $ should match two items");

    // Regex Phrase
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ paradedb.regex_phrase(
        field => 'description',
        regexes => ARRAY['.*bot', '.*ing', 'kit']
    ) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
    '{
        "regex_phrase": {
            "field": "description",
            "regexes": [".*eek", "shoes"],
            "slop": 1,
            "max_expansion": 10
        }
    }'::jsonb;"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // Regex Phrase
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ paradedb.regex_phrase(
        field => 'description',
        regexes => ARRAY['.*bot', '.*ing', 'kit']
    ) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@
    '{
        "regex_phrase": {
            "field": "description",
            "regexes": [".*eek", "shoes"],
            "slop": 1,
            "max_expansion": 10
        }
    }'::jsonb;"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // Term
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ paradedb.term(field => 'description', value => 'shoes')
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 3);

    // Term with no field (should search all columns)
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ paradedb.term(value => 'shoes') ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 3);

    // TermSet with invalid term list
    match r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ paradedb.term_set(
        terms => ARRAY[
            paradedb.regex(field => 'description', pattern => '.+')
        ]
    ) ORDER BY id"#
        .fetch_result::<SimpleProductsTable>(&mut conn)
    {
        Err(err) => assert!(err
            .to_string()
            .contains("only term queries can be passed to term_set")),
        _ => panic!("term set query should only accept terms"),
    }

    // TermSet
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ paradedb.term_set(
        terms => ARRAY[
            paradedb.term(field => 'description', value => 'shoes'),
            paradedb.term(field => 'description', value => 'novel')
        ]
    ) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 5);
}

#[rstest]
fn exists_query(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // Simple exists query
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
        paradedb.exists('rating')
    "#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // Non fast field should fail
    match r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
        paradedb.exists('description')
    "#
    .execute_result(&mut conn)
    {
        Err(err) => assert!(err.to_string().contains("not a fast field")),
        _ => panic!("exists() over non-fast field should fail"),
    }

    // Exists with boolean query
    "INSERT INTO paradedb.bm25_search (id, description, rating) VALUES (42, 'shoes', NULL)"
        .execute(&mut conn);

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
        paradedb.boolean(
            must => ARRAY[
                paradedb.exists('rating'),
                paradedb.parse('description:shoes')
            ]
        )
    "#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 3);
}

#[rstest]
fn more_like_this_raw(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id SERIAL PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (flavour) VALUES 
        ('apple'),
        ('banana'), 
        ('cherry'), 
        ('banana split');
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (
            key_field='id',
            text_fields='{
                "flavour": { "stored": true }
            }'
        );
    "#
    .execute(&mut conn);

    match r#"
    SELECT id, flavour FROM test_more_like_this_table WHERE test_more_like_this_table @@@ 
        paradedb.more_like_this();
    "#
    .fetch_result::<()>(&mut conn)
    {
        Err(err) => {
            assert_eq!(err
            .to_string()
            , "error returned from database: more_like_this must be called with either document_id or document_fields")
        }
        _ => panic!("document_id or document_fields validation failed"),
    }

    let rows: Vec<(i32, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ paradedb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        document_fields => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);

    let rows: Vec<(i32, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ paradedb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        document_id => 2
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_empty(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id SERIAL PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (flavour) VALUES 
        ('apple'),
        ('banana'), 
        ('cherry'), 
        ('banana split');
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (
            key_field='id',
            text_fields='{
                "flavour": { "stored": true }
            }'
        );
    "#
    .execute(&mut conn);

    match r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ paradedb.more_like_this()
    ORDER BY id;
    "#
    .fetch_result::<()>(&mut conn)
    {
        Err(err) => {
            assert_eq!(err
            .to_string()
            , "error returned from database: more_like_this must be called with either document_id or document_fields")
        }
        _ => panic!("document_id or document_fields validation failed"),
    }
}

#[rstest]
fn more_like_this_text(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id SERIAL PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (flavour) VALUES 
        ('apple'),
        ('banana'), 
        ('cherry'), 
        ('banana split');
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (
            key_field='id',
            text_fields='{
                "flavour": { "stored": true }
            }'
        );
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ paradedb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        document_fields => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_boolean_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id BOOLEAN PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES 
        (true, 'apple'),
        (false, 'banana')
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (
            key_field='id',
            text_fields='{
                "flavour": { "stored": true }
            }'
        );
    "#
    .execute(&mut conn);

    let rows: Vec<(bool, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ 
    paradedb.more_like_this(
       min_doc_frequency => 0,
       min_term_frequency => 0,
       document_fields => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 1);
}

#[rstest]
fn more_like_this_uuid_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id UUID PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES 
        ('f159c89e-2162-48cd-85e3-e42b71d2ecd0', 'apple'),
        ('38bf27a0-1aa8-42cd-9cb0-993025e0b8d0', 'banana'), 
        ('b5faacc0-9eba-441a-81f8-820b46a3b57e', 'cherry'), 
        ('eb833eb6-c598-4042-b84a-0045828fceea', 'banana split');
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (
            key_field='id',
            text_fields='{
                "flavour": { "stored": true }
            }'
        );
    "#
    .execute(&mut conn);

    let rows: Vec<(uuid::Uuid, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ paradedb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        document_fields => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_i64_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id BIGINT PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES 
        (1, 'apple'),
        (2, 'banana'), 
        (3, 'cherry'), 
        (4, 'banana split');
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (
            key_field='id',
            text_fields='{
                "flavour": { "stored": true }
            }'
        );
    "#
    .execute(&mut conn);

    let rows: Vec<(i64, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ 
    paradedb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        document_fields => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_i32_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id INT PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES 
        (1, 'apple'),
        (2, 'banana'), 
        (3, 'cherry'), 
        (4, 'banana split');
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (
            key_field='id',
            text_fields='{
                "flavour": { "stored": true }
            }'
        );
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ paradedb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        document_fields => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_i16_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id SMALLINT PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES 
        (1, 'apple'),
        (2, 'banana'), 
        (3, 'cherry'), 
        (4, 'banana split');
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (
            key_field='id',
            text_fields='{
                "flavour": { "stored": true }
            }'
        );
    "#
    .execute(&mut conn);

    let rows: Vec<(i16, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ paradedb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        document_fields => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_f32_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id FLOAT4 PRIMARY KEY,
        flavour TEXT
    );

    INSERT INTO test_more_like_this_table (id, flavour) VALUES 
        (1.1, 'apple'),
        (2.2, 'banana'),
        (3.3, 'cherry'),
        (4.4, 'banana split');
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (
            key_field='id',
            text_fields='{
                "flavour": { "stored": true }
            }'
        );
    "#
    .execute(&mut conn);

    let rows: Vec<(f32, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ paradedb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        document_fields => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_f64_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
    id FLOAT8 PRIMARY KEY,
    flavour TEXT
    );
    INSERT INTO test_more_like_this_table (id, flavour) VALUES 
        (1.1, 'apple'),
        (2.2, 'banana'), 
        (3.3, 'cherry'), 
        (4.4, 'banana split');
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (
            key_field='id',
            text_fields='{
                "flavour": { "stored": true }
            }'
        );
    "#
    .execute(&mut conn);

    let rows: Vec<(f64, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ 
    paradedb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        document_fields => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_numeric_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
    id NUMERIC PRIMARY KEY,
    flavour TEXT
    );
    
    INSERT INTO test_more_like_this_table (id, flavour) VALUES 
        (1.1, 'apple'),
        (2.2, 'banana'), 
        (3.3, 'cherry'), 
        (4.4, 'banana split');
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (
            key_field='id',
            text_fields='{
                "flavour": { "stored": true }
            }'
        );
    "#
    .execute(&mut conn);

    let rows: Vec<(f64, String)> = r#"
    SELECT CAST(id AS FLOAT8), flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ paradedb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        document_fields => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_date_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
    id DATE PRIMARY KEY,
    flavour TEXT
    );
    INSERT INTO test_more_like_this_table (id, flavour) VALUES 
        ('2023-05-03', 'apple'),
        ('2023-05-04', 'banana'), 
        ('2023-05-05', 'cherry'), 
        ('2023-05-06', 'banana split');
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (
            key_field='id',
            text_fields='{
                "flavour": { "stored": true }
            }'
        );
    "#
    .execute(&mut conn);

    let rows: Vec<(String, String)> = r#"
    SELECT CAST(id AS TEXT), flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@  paradedb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        document_fields => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_time_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
    id TIME PRIMARY KEY,
    flavour TEXT
    );
    
    INSERT INTO test_more_like_this_table (id, flavour) VALUES 
        ('08:09:10', 'apple'),
        ('09:10:11', 'banana'), 
        ('10:11:12', 'cherry'), 
        ('11:12:13', 'banana split');
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (
            key_field='id',
            text_fields='{
                "flavour": { "stored": true }
            }'
        );
    "#
    .execute(&mut conn);

    let rows: Vec<(String, String)> = r#"
    SELECT CAST(id AS TEXT), flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ 
    paradedb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        document_fields => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_timestamp_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id TIMESTAMP PRIMARY KEY,
        flavour TEXT
    );
    
    INSERT INTO test_more_like_this_table (id, flavour) VALUES 
        ('2023-05-03 08:09:10', 'apple'),
        ('2023-05-04 09:10:11', 'banana'), 
        ('2023-05-05 10:11:12', 'cherry'), 
        ('2023-05-06 11:12:13', 'banana split');
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (
            key_field='id',
            text_fields='{
                "flavour": { "stored": true }
            }'
        );
    "#
    .execute(&mut conn);

    let rows: Vec<(String, String)> = r#"
    SELECT CAST(id AS TEXT), flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ 
    paradedb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        document_fields => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_timestamptz_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
    id TIMESTAMP WITH TIME ZONE PRIMARY KEY,
    flavour TEXT
    );
    
    INSERT INTO test_more_like_this_table (id, flavour) VALUES 
        ('2023-05-03 08:09:10 EST', 'apple'),
        ('2023-05-04 09:10:11 PST', 'banana'), 
        ('2023-05-05 10:11:12 MST', 'cherry'), 
        ('2023-05-06 11:12:13 CST', 'banana split');
    "#
    .execute(&mut conn);

    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (
            key_field='id',
            text_fields='{
                "flavour": { "stored": true }
            }'
        );
    "#
    .execute(&mut conn);

    let rows: Vec<(String, String)> = r#"
    SELECT CAST(id AS TEXT), flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ 
    paradedb.more_like_this(
        min_doc_frequency => 0,
        min_term_frequency => 0,
        document_fields => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn more_like_this_timetz_key(mut conn: PgConnection) {
    r#"
    CREATE TABLE test_more_like_this_table (
        id TIME WITH TIME ZONE PRIMARY KEY,
        flavour TEXT
    );
    INSERT INTO test_more_like_this_table (id, flavour) VALUES 
        ('08:09:10 EST',
        'apple'),
        ('09:10:11 PST', 'banana'),
        ('10:11:12 MST', 'cherry'),
        ('11:12:13 CST', 'banana split');
    "#
    .execute(&mut conn);
    r#"
        CREATE INDEX test_more_like_this_index on test_more_like_this_table USING bm25 (id, flavour)
        WITH (
            key_field='id',
            text_fields='{
                "flavour": { "stored": true }
            }'
        );
    "#
    .execute(&mut conn);

    let rows: Vec<(String, String)> = r#"
    SELECT CAST(id AS TEXT), flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ paradedb.more_like_this(
            min_doc_frequency => 0,
            min_term_frequency => 0,
            document_fields => '{"flavour": "banana"}'
    ) ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn fuzzy_phrase(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ paradedb.fuzzy_phrase(field => 'description', value => 'ruling shoeez')
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3, 4, 5]);

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ paradedb.fuzzy_phrase(
        field => 'description',
        value => 'ruling shoeez',
        match_all_terms => true
    ) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3]);

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ paradedb.fuzzy_phrase(field => 'description', value => 'ruling shoeez', distance => 1)
    ORDER BY id"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.id.len(), 0);
}

#[rstest]
fn parse_lenient(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // Default lenient should be false
    let result = r#"
    SELECT id FROM paradedb.bm25_search 
    WHERE paradedb.bm25_search.id @@@ paradedb.parse('shoes keyboard')
    ORDER BY id;
    "#
    .execute_result(&mut conn);
    assert!(result.is_err());

    // With lenient enabled
    let rows: Vec<(i32,)> = r#"
    SELECT id FROM paradedb.bm25_search 
    WHERE paradedb.bm25_search.id @@@ paradedb.parse('shoes keyboard', lenient => true)
    ORDER BY id;
    "#
    .fetch(&mut conn);
    assert_eq!(rows, vec![(1,), (2,), (3,), (4,), (5,)]);
}

#[rstest]
fn parse_conjunction(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let rows: Vec<(i32,)> = r#"
    SELECT id FROM paradedb.bm25_search 
    WHERE paradedb.bm25_search.id @@@ paradedb.parse('description:(shoes running)', conjunction_mode => true)
    ORDER BY id;
    "#.fetch(&mut conn);
    assert_eq!(rows, vec![(3,)]);
}

#[rstest]
fn parse_with_field_conjunction(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let rows: Vec<(i32,)> = r#"
    SELECT id FROM paradedb.bm25_search 
    WHERE paradedb.bm25_search.id @@@ paradedb.parse_with_field('description', 'shoes running', conjunction_mode => true)
    ORDER BY id;
    "#.fetch(&mut conn);
    assert_eq!(rows, vec![(3,)]);
}

#[rstest]
fn range_term(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
        schema_name => 'public',
        table_name => 'deliveries',
        table_type => 'Deliveries'
    );

    CREATE INDEX deliveries_idx ON deliveries
    USING bm25 (delivery_id, weights, quantities, prices, ship_dates, facility_arrival_times, delivery_times)
    WITH (key_field = 'delivery_id');
    "#
    .execute(&mut conn);

    // int4range
    let expected: Vec<(i32,)> =
        "SELECT delivery_id FROM deliveries WHERE weights @> 1 ORDER BY delivery_id"
            .fetch(&mut conn);
    let result: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE delivery_id @@@ paradedb.range_term('weights', 1) ORDER BY delivery_id".fetch(&mut conn);
    assert_eq!(result, expected);

    let expected: Vec<(i32,)> =
        "SELECT delivery_id FROM deliveries WHERE weights @> 13 ORDER BY delivery_id"
            .fetch(&mut conn);
    let result: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE delivery_id @@@ paradedb.range_term('weights', 13) ORDER BY delivery_id".fetch(&mut conn);
    assert_eq!(result, expected);

    // int8range
    let expected: Vec<(i32,)> =
        "SELECT delivery_id FROM deliveries WHERE quantities @> 17000::int8 ORDER BY delivery_id"
            .fetch(&mut conn);
    let result: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE delivery_id @@@ paradedb.range_term('quantities', 17000) ORDER BY delivery_id".fetch(&mut conn);
    assert_eq!(result, expected);

    // numrange
    let expected: Vec<(i32,)> =
        "SELECT delivery_id FROM deliveries WHERE prices @> 3.5 ORDER BY delivery_id"
            .fetch(&mut conn);
    let result: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE delivery_id @@@ paradedb.range_term('prices', 3.5) ORDER BY delivery_id".fetch(&mut conn);
    assert_eq!(result, expected);

    // daterange
    let expected: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE ship_dates @> '2023-03-07'::date ORDER BY delivery_id".fetch(&mut conn);
    let result: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE delivery_id @@@ paradedb.range_term('ship_dates', '2023-03-07'::date) ORDER BY delivery_id".fetch(&mut conn);
    assert_eq!(result, expected);

    let expected: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE ship_dates @> '2023-03-06'::date ORDER BY delivery_id".fetch(&mut conn);
    let result: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE delivery_id @@@ paradedb.range_term('ship_dates', '2023-03-06'::date) ORDER BY delivery_id".fetch(&mut conn);
    assert_eq!(result, expected);

    // tsrange
    let expected: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE facility_arrival_times @> '2024-05-01 14:00:00'::timestamp ORDER BY delivery_id".fetch(&mut conn);
    let result: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE delivery_id @@@ paradedb.range_term('facility_arrival_times', '2024-05-01 14:00:00'::timestamp) ORDER BY delivery_id".fetch(&mut conn);
    assert_eq!(result, expected);

    let expected: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE facility_arrival_times @> '2024-05-01 15:00:00'::timestamp ORDER BY delivery_id".fetch(&mut conn);
    let result: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE delivery_id @@@ paradedb.range_term('facility_arrival_times', '2024-05-01 15:00:00'::timestamp) ORDER BY delivery_id".fetch(&mut conn);
    assert_eq!(result, expected);

    // tstzrange
    let expected: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE delivery_times @> '2024-05-01 06:31:00-04'::timestamptz ORDER BY delivery_id".fetch(&mut conn);
    let result: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE delivery_id @@@ paradedb.range_term('delivery_times', '2024-05-01 06:31:00-04'::timestamptz) ORDER BY delivery_id".fetch(&mut conn);
    assert_eq!(result, expected);

    let expected: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE delivery_times @> '2024-05-01T11:30:00Z'::timestamptz ORDER BY delivery_id".fetch(&mut conn);
    let result: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE delivery_id @@@ paradedb.range_term('delivery_times', '2024-05-01T11:30:00Z'::timestamptz) ORDER BY delivery_id".fetch(&mut conn);
    assert_eq!(result, expected);
}

#[rstest]
async fn prepared_statement_replanning(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // ensure our plan doesn't change into a sequential scan after the 5th execution
    for _ in 0..10 {
        let _: Vec<i32> = sqlx::query("SELECT id FROM paradedb.bm25_search WHERE id @@@ paradedb.term('rating', $1) ORDER BY id")
            .bind(2)
            .fetch_all(&mut conn)
            .await
            .unwrap()
            .into_iter()
            .map(|row| row.get::<i32, _>("id"))
            .collect();
    }
}

#[rstest]
async fn direct_prepared_statement_replanning(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "PREPARE stmt(text) AS SELECT id FROM paradedb.bm25_search WHERE description @@@ $1"
        .execute(&mut conn);

    // ensure our plan doesn't change into a sequential scan after the 5th execution
    for _ in 0..10 {
        "EXECUTE stmt('keyboard')".fetch_one::<(i32,)>(&mut conn);
    }
}

#[rstest]
async fn direct_prepared_statement_replanning_custom_scan(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "PREPARE stmt(text) AS SELECT paradedb.score(id), id FROM paradedb.bm25_search WHERE description @@@ $1 ORDER BY score desc LIMIT 10"
        .execute(&mut conn);

    // ensure our plan doesn't change into a sequential scan after the 5th execution
    for _ in 0..10 {
        let (score, id) = "EXECUTE stmt('keyboard')".fetch_one::<(f32, i32)>(&mut conn);
        assert_eq!((score, id), (3.2668595, 2))
    }
}
