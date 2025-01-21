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
fn boolean_tree(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
   '{
        "boolean": {
            "should": [
                {"parse": {"query_string": "description:shoes"}},
                {"phrase_prefix": {"field": "description", "phrases": ["book"]}},
                {"term": {"field": "description", "value": "speaker"}},
                {"fuzzy_term": {"field": "description", "value": "wolo", "transposition_cost_one": false, "distance": 1, "prefix": true}}
            ]
        }
    }'::jsonb ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3, 4, 5, 7, 10, 32, 33, 34, 37, 39, 41]);
}

#[rstest]
fn fuzzy_term(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ '{"fuzzy_term": {"field": "category", "value": "elector", "prefix": true}}'::jsonb
    ORDER BY id"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2, 12, 22, 32], "wrong results");

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
    '{"term": {"field": "category", "value": "electornics"}}'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert!(columns.is_empty(), "without fuzzy field should be empty");

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
        '{
            "fuzzy_term": {
                "field": "description",
                "value": "keybaord",
                "transposition_cost_one": false,
                "distance": 1,
                "prefix": true
            }
        }'::jsonb ORDER BY id"#
        .fetch_collect(&mut conn);
    assert!(
        columns.is_empty(),
        "transposition_cost_one false should be empty"
    );

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
        '{
            "fuzzy_term": {
                "field": "description",
                "value": "keybaord",
                "transposition_cost_one": true,
                "distance": 1,
                "prefix": true
            }
        }'::jsonb ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(
        columns.id,
        vec![1, 2],
        "incorrect transposition_cost_one true"
    );

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
        '{
            "fuzzy_term": {
                "field": "description",
                "value": "keybaord",
                "prefix": true
            }
        }'::jsonb ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2], "incorrect defaults");
}

#[rstest]
fn single_queries(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // All
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
    '{"all": null}'::jsonb ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // Boost
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
    '{"boost": {"query": {"all": null}, "factor": 1.5}}'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // ConstScore
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ '{"const_score": {"query": {"all": null}, "score": 3.9}}'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // DisjunctionMax
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
    '{"disjunction_max": {"disjuncts": [{"parse": {"query_string": "description:shoes"}}]}}'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 3);

    // Empty
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ '{"empty": null}'::jsonb ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 0);

    // FuzzyTerm
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ '{
        "fuzzy_term": {
            "field": "description",
            "value": "wolo",
            "transposition_cost_one": false,
            "distance": 1,
            "prefix": true
        }
    }'::jsonb ORDER BY ID"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 4);

    // Parse
    let columns: SimpleProductsTableVec = r#"
        SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
        '{"parse": {"query_string": "description:teddy"}}'::jsonb ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // PhrasePrefix
    let columns: SimpleProductsTableVec = r#"
        SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
        '{"phrase_prefix": {"field": "description", "phrases": ["har"]}}'::jsonb
        ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // Phrase with invalid term list
    match r#"
        SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
        '{"phrase": {"field": "description", "phrases": ["robot"]}}'::jsonb
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
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ '{
        "phrase": {
            "field": "description",
            "phrases": ["robot", "building", "kit"]
        }
    }'::jsonb ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // Range
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
        '{
            "range": {
                "field": "last_updated_date",
                "lower_bound": {"included": "2023-05-01T00:00:00.000000Z"},
                "upper_bound": {"included": "2023-05-03T00:00:00.000000Z"},
                "is_datetime": true
            }
        }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 7);

    // Regex
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ '{
        "regex": {
            "field": "description",
            "pattern": "(hardcover|plush|leather|running|wireless)"
        }
    }'::jsonb ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 5);

    // Term
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ '{"term": {"field": "description", "value": "shoes"}}'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 3);

    // Term with no field (should search all columns)
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ '{"term": {"value": "shoes"}}'::jsonb ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 3);

    // TermSet
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ '{
        "term_set": {
            "terms": [
                {"field": "description", "value": "shoes", "is_datetime": false},
                {"field": "description", "value": "novel", "is_datetime": false}
            ]
        }
    }'::jsonb ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 5);
}

#[rstest]
fn single_queries_jsonb_build_object(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // All
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
    jsonb_build_object('all', null) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // Boost
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
    jsonb_build_object('boost', jsonb_build_object(
        'query', jsonb_build_object('all', null),
        'factor', 1.5)) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // ConstScore
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
    jsonb_build_object('const_score', jsonb_build_object(
        'query', jsonb_build_object('all', null),
        'score', 3.9)) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // DisjunctionMax
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
    jsonb_build_object('disjunction_max', jsonb_build_object(
        'disjuncts', jsonb_build_array(
            jsonb_build_object('parse', jsonb_build_object(
                'query_string', 'description:shoes'))))) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 3);

    // Empty
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
    jsonb_build_object('empty', null) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 0);

    // FuzzyTerm
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
    jsonb_build_object('fuzzy_term', jsonb_build_object(
        'field', 'description',
        'value', 'wolo',
        'transposition_cost_one', false,
        'distance', 1,
        'prefix', true)) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 4);

    // Parse
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
    jsonb_build_object('parse', jsonb_build_object(
        'query_string', 'description:teddy')) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // PhrasePrefix
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
    jsonb_build_object('phrase_prefix', jsonb_build_object(
        'field', 'description',
        'phrases', jsonb_build_array('har'))) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // Phrase with invalid term list
    match r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
    jsonb_build_object('phrase', jsonb_build_object(
        'field', 'description',
        'phrases', jsonb_build_array('robot'))) ORDER BY id"#
        .fetch_result::<SimpleProductsTable>(&mut conn)
    {
        Err(err) => assert!(err
            .to_string()
            .contains("required to have strictly more than one term")),
        _ => panic!("phrase prefix query should require multiple terms"),
    }

    // Phrase
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
    jsonb_build_object('phrase', jsonb_build_object(
        'field', 'description',
        'phrases', jsonb_build_array('robot', 'building', 'kit'))) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // Range
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
    jsonb_build_object('range', jsonb_build_object(
        'field', 'last_updated_date',
        'lower_bound', jsonb_build_object('included', '2023-05-01T00:00:00.000000Z'),
        'upper_bound', jsonb_build_object('included', '2023-05-03T00:00:00.000000Z'),
        'is_datetime', true)) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 7);

    // Regex
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
    jsonb_build_object('regex', jsonb_build_object(
        'field', 'description',
        'pattern', '(hardcover|plush|leather|running|wireless)')) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 5);

    // Term
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
    jsonb_build_object('term', jsonb_build_object(
        'field', 'description',
        'value', 'shoes')) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 3);

    // Term with no field (should search all columns)
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
    jsonb_build_object('term', jsonb_build_object('value', 'shoes')) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 3);

    // TermSet
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
    jsonb_build_object('term_set', jsonb_build_object(
        'terms', jsonb_build_array(
            jsonb_build_array('description', 'shoes', false),
            jsonb_build_array('description', 'novel', false)))) ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 5);
}

#[rstest]
fn exists_query(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // Simple exists query
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
        '{"exists": {"field": "rating"}}'::jsonb
    "#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // Non fast field should fail
    match r#"
    SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 
        '{"exists": {"field": "description"}}'::jsonb
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
        '{
            "boolean": {
                "must": [
                    {"exists": {"field": "rating"}},
                    {"parse": {"query_string": "description:shoes"}}
                ]
            }
        }'::jsonb
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
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
    WITH (
            key_field='id',
            text_fields='{
                "flavour": { "stored": true }
            }'
        );
    "#
    .execute(&mut conn);

    // Missing keys should fail.
    match r#"
    SELECT id, flavour FROM test_more_like_this_table WHERE test_more_like_this_table @@@ 
        '{"more_like_this": {}}'::jsonb;
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

    // Conflicting keys should fail.
    match r#"
    SELECT id, flavour FROM test_more_like_this_table WHERE test_more_like_this_table @@@ 
        '{"more_like_this": {
            "document_id": 0,
            "document_fields": [["flavour", "banana"]]            
        }}'::jsonb;
    "#
    .fetch_result::<()>(&mut conn)
    {
        Err(err) => {
            assert_eq!(err
            .to_string()
            , "error returned from database: more_like_this must be called with only one of document_id or document_fields")
        }
        _ => panic!("document_id or document_fields validation failed"),
    }

    let rows: Vec<(i32, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document_fields": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);

    let rows: Vec<(i32, String)> = r#"
    SELECT id, flavour FROM test_more_like_this_table
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document_id": 2
        }
    }'::jsonb ORDER BY id;
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
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
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
    WHERE test_more_like_this_table @@@ '{"more_like_this": {}}'::jsonb
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
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
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
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document_fields": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
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
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
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
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document_fields": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
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
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
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
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document_fields": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
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
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
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
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document_fields": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
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
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
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
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document_fields": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
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
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
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
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document_fields": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
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
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
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
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document_fields": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
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
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
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
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document_fields": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
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
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
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
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document_fields": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
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
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
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
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document_fields": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
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
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
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
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document_fields": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
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
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
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
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document_fields": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
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
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
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
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document_fields": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
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
        ('08:09:10 EST', 'apple'),
        ('09:10:11 PST', 'banana'),
        ('10:11:12 MST', 'cherry'),
        ('11:12:13 CST', 'banana split');
    "#
    .execute(&mut conn);
    r#"
    CREATE INDEX test_more_like_this_index ON test_more_like_this_table USING bm25 (id, flavour)
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
    WHERE test_more_like_this_table @@@ '{
        "more_like_this": {
            "min_doc_frequency": 0,
            "min_term_frequency": 0,
            "document_fields": [["flavour", "banana"]]
        }
    }'::jsonb ORDER BY id;
    "#
    .fetch_collect(&mut conn);
    assert_eq!(rows.len(), 2);
}

#[rstest]
fn fuzzy_phrase(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ '{
        "fuzzy_phrase": {
            "field": "description",
            "value": "ruling shoeez"
        }
    }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3, 4, 5]);

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ '{
        "fuzzy_phrase": {
            "field": "description",
            "value": "ruling shoeez",
            "match_all_terms": true
        }
    }'::jsonb ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3]);

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE bm25_search @@@ '{
        "fuzzy_phrase": {
            "field": "description",
            "value": "ruling shoeez",
            "distance": 1
        }
    }'::jsonb
    ORDER BY id"#
        .fetch_collect(&mut conn);
    assert_eq!(columns.id.len(), 0);
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
    WITH (key_field='delivery_id');
    "#
    .execute(&mut conn);

    // int4range
    let expected: Vec<(i32,)> =
        "SELECT delivery_id FROM deliveries WHERE weights @> 1 ORDER BY delivery_id"
            .fetch(&mut conn);
    let result: Vec<(i32,)> = r#"SELECT delivery_id FROM deliveries WHERE delivery_id @@@ '{"range_term": {"field": "weights", "value": 1}}'::jsonb ORDER BY delivery_id"#.fetch(&mut conn);
    assert_eq!(result, expected);

    let expected: Vec<(i32,)> =
        "SELECT delivery_id FROM deliveries WHERE weights @> 13 ORDER BY delivery_id"
            .fetch(&mut conn);
    let result: Vec<(i32,)> = r#"SELECT delivery_id FROM deliveries WHERE delivery_id @@@ '{"range_term": {"field": "weights", "value": 13}}'::jsonb ORDER BY delivery_id"#.fetch(&mut conn);
    assert_eq!(result, expected);

    // int8range
    let expected: Vec<(i32,)> =
        "SELECT delivery_id FROM deliveries WHERE quantities @> 17000::int8 ORDER BY delivery_id"
            .fetch(&mut conn);
    let result: Vec<(i32,)> = r#"SELECT delivery_id FROM deliveries WHERE delivery_id @@@ '{"range_term": {"field": "quantities", "value": 17000}}'::jsonb ORDER BY delivery_id"#.fetch(&mut conn);
    assert_eq!(result, expected);

    // numrange
    let expected: Vec<(i32,)> =
        "SELECT delivery_id FROM deliveries WHERE prices @> 3.5 ORDER BY delivery_id"
            .fetch(&mut conn);
    let result: Vec<(i32,)> = r#"SELECT delivery_id FROM deliveries WHERE delivery_id @@@ '{"range_term": {"field": "prices", "value": 3.5}}'::jsonb ORDER BY delivery_id"#.fetch(&mut conn);
    assert_eq!(result, expected);

    // daterange
    let expected: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE ship_dates @> '2023-03-07'::date ORDER BY delivery_id".fetch(&mut conn);
    let result: Vec<(i32,)> = r#"SELECT delivery_id FROM deliveries WHERE delivery_id @@@ '{"range_term": {"field": "ship_dates", "value": "2023-03-07T00:00:00.000000Z", "is_datetime": true}}'::jsonb ORDER BY delivery_id"#.fetch(&mut conn);
    assert_eq!(result, expected);

    let expected: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE ship_dates @> '2023-03-06'::date ORDER BY delivery_id".fetch(&mut conn);
    let result: Vec<(i32,)> = r#"SELECT delivery_id FROM deliveries WHERE delivery_id @@@ '{"range_term": {"field": "ship_dates", "value": "2023-03-06T00:00:00.000000Z", "is_datetime": true}}'::jsonb ORDER BY delivery_id"#.fetch(&mut conn);
    assert_eq!(result, expected);

    // tsrange
    let expected: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE facility_arrival_times @> '2024-05-01 14:00:00'::timestamp ORDER BY delivery_id".fetch(&mut conn);
    let result: Vec<(i32,)> = r#"SELECT delivery_id FROM deliveries WHERE delivery_id @@@ '{"range_term": {"field": "facility_arrival_times", "value": "2024-05-01T14:00:00.000000Z", "is_datetime": true}}'::jsonb ORDER BY delivery_id"#.fetch(&mut conn);
    assert_eq!(result, expected);

    let expected: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE facility_arrival_times @> '2024-05-01 15:00:00'::timestamp ORDER BY delivery_id".fetch(&mut conn);
    let result: Vec<(i32,)> = r#"SELECT delivery_id FROM deliveries WHERE delivery_id @@@ '{"range_term": {"field": "facility_arrival_times", "value": "2024-05-01T15:00:00.000000Z", "is_datetime": true}}'::jsonb ORDER BY delivery_id"#.fetch(&mut conn);
    assert_eq!(result, expected);

    // tstzrange
    let expected: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE delivery_times @> '2024-05-01 06:31:00-04'::timestamptz ORDER BY delivery_id".fetch(&mut conn);
    let result: Vec<(i32,)> = r#"SELECT delivery_id FROM deliveries WHERE delivery_id @@@ '{"range_term": {"field": "delivery_times", "value": "2024-05-01T10:31:00.000000Z", "is_datetime": true}}'::jsonb ORDER BY delivery_id"#.fetch(&mut conn);
    assert_eq!(result, expected);

    let expected: Vec<(i32,)> = "SELECT delivery_id FROM deliveries WHERE delivery_times @> '2024-05-01T11:30:00Z'::timestamptz ORDER BY delivery_id".fetch(&mut conn);
    let result: Vec<(i32,)> = r#"SELECT delivery_id FROM deliveries WHERE delivery_id @@@ '{"range_term": {"field": "delivery_times", "value": "2024-05-01T11:30:00.000000Z", "is_datetime": true}}'::jsonb ORDER BY delivery_id"#.fetch(&mut conn);
    assert_eq!(result, expected);
}

#[rstest]
fn parse_error(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let result = r#"
    SELECT id FROM paradedb.bm25_search WHERE bm25_search @@@ 
    '{"all": {}}'::jsonb ORDER BY id"#
        .fetch_result::<(i32,)>(&mut conn);

    match result {
        Err(err) => assert_eq!(
            err.to_string(),
            r#"error returned from database: error parsing search query input json at "all": invalid type: map, pass null as value for "all""#
        ),
        _ => {
            panic!("search input query variant with no fields should not be able to receive a map")
        }
    }
}
