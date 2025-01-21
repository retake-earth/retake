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

use anyhow::Result;
use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
async fn basic_reindex(mut conn: PgConnection) -> Result<()> {
    SimpleProductsTable::setup().execute(&mut conn);

    // Verify initial search works
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2]);

    // Perform REINDEX
    "REINDEX INDEX paradedb.bm25_search_bm25_index".execute(&mut conn);

    // Verify search still works after reindex
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2]);

    Ok(())
}

#[rstest]
async fn concurrent_reindex(mut conn: PgConnection) -> Result<()> {
    SimpleProductsTable::setup().execute(&mut conn);

    // Verify initial search
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2]);

    // Perform concurrent REINDEX
    "REINDEX INDEX CONCURRENTLY paradedb.bm25_search_bm25_index".execute(&mut conn);

    // Verify search still works after concurrent reindex
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2]);

    Ok(())
}

#[rstest]
async fn reindex_with_updates(mut conn: PgConnection) -> Result<()> {
    SimpleProductsTable::setup().execute(&mut conn);

    // Initial search
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2]);

    // Make some updates
    "UPDATE paradedb.bm25_search SET description = 'Mechanical keyboard' WHERE id = 1"
        .execute(&mut conn);
    "INSERT INTO paradedb.bm25_search (description, category, rating, in_stock, metadata, created_at, last_updated_date) VALUES ('Wireless keyboard', 'Electronics', 4, true, '{\"color\": \"black\"}', now(), current_date)".execute(&mut conn);

    // Verify updates are searchable
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2, 42]);

    // Perform REINDEX
    "REINDEX INDEX paradedb.bm25_search_bm25_index".execute(&mut conn);

    // Verify all updates are still searchable after reindex
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2, 42]);

    Ok(())
}

#[rstest]
async fn reindex_with_deletes(mut conn: PgConnection) -> Result<()> {
    SimpleProductsTable::setup().execute(&mut conn);

    // Initial search
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2]);

    // Delete some records
    "DELETE FROM paradedb.bm25_search WHERE id = 1".execute(&mut conn);

    // Verify delete is reflected in search
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![2]);

    // Perform REINDEX
    "REINDEX INDEX paradedb.bm25_search_bm25_index".execute(&mut conn);

    // Verify deleted records are still not searchable after reindex
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![2]);

    Ok(())
}

#[rstest]
async fn reindex_schema_validation(mut conn: PgConnection) -> Result<()> {
    SimpleProductsTable::setup().execute(&mut conn);

    // Get initial schema
    let initial_schema: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('paradedb.bm25_search_bm25_index') ORDER BY name"
            .fetch(&mut conn);

    // Perform REINDEX
    "REINDEX INDEX paradedb.bm25_search_bm25_index".execute(&mut conn);

    // Get schema after reindex
    let reindexed_schema: Vec<(String, String)> =
        "SELECT name, field_type FROM paradedb.schema('paradedb.bm25_search_bm25_index') ORDER BY name"
            .fetch(&mut conn);

    // Verify schema hasn't changed
    assert_eq!(initial_schema, reindexed_schema);

    Ok(())
}

#[rstest]
async fn reindex_partial_index(mut conn: PgConnection) -> Result<()> {
    "CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');"
        .execute(&mut conn);

    // Create a partial index
    r#"CREATE INDEX partial_idx ON paradedb.bm25_search
    USING bm25 (id, description, category)
    WITH (key_field='id')
    WHERE category = 'Electronics'"#
        .execute(&mut conn);

    // Initial search
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2]);

    // Perform REINDEX
    "REINDEX INDEX paradedb.partial_idx".execute(&mut conn);

    // Verify partial index still works correctly after reindex
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2]);

    Ok(())
}

#[rstest]
async fn concurrent_reindex_with_updates(mut conn: PgConnection) -> Result<()> {
    SimpleProductsTable::setup().execute(&mut conn);

    // Initial search
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2]);

    // Start concurrent reindex
    "REINDEX INDEX CONCURRENTLY paradedb.bm25_search_bm25_index".execute(&mut conn);

    // Make updates during reindex
    "UPDATE paradedb.bm25_search SET description = 'Mechanical keyboard' WHERE id = 1"
        .execute(&mut conn);
    "INSERT INTO paradedb.bm25_search (description, category, rating, in_stock, metadata, created_at, last_updated_date) VALUES ('Wireless keyboard', 'Electronics', 4, true, '{\"color\": \"black\"}', now(), current_date)".execute(&mut conn);

    // Verify all updates are searchable after concurrent reindex
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2, 42]);

    Ok(())
}

#[rstest]
async fn reindex_table(mut conn: PgConnection) -> Result<()> {
    SimpleProductsTable::setup().execute(&mut conn);

    // Initial search
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2]);

    // Reindex entire table
    "REINDEX TABLE paradedb.bm25_search".execute(&mut conn);

    // Verify search still works
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2]);

    Ok(())
}

#[rstest]
async fn concurrent_index_creation(mut conn: PgConnection) -> Result<()> {
    SimpleProductsTable::setup().execute(&mut conn);

    // Create a second index concurrently
    r#"CREATE INDEX CONCURRENTLY bm25_search_bm25_index_2 ON paradedb.bm25_search
    USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date)
    WITH (
        key_field='id',
        text_fields='{
            "description": {"tokenizer": {"type": "en_stem"}},
            "category": {}
        }',
        numeric_fields='{"rating": {}}',
        boolean_fields='{"in_stock": {}}',
        json_fields='{"metadata": {}}',
        datetime_fields='{"created_at": {}, "last_updated_date": {}}'
    )"#.execute(&mut conn);

    // Query using the new index
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE id @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2]);

    // Drop the original index
    "DROP INDEX paradedb.bm25_search_bm25_index".execute(&mut conn);

    // Verify the new index still works
    let columns: SimpleProductsTableVec =
        "SELECT * FROM paradedb.bm25_search WHERE id @@@ 'description:keyboard' ORDER BY id"
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2]);

    Ok(())
}
