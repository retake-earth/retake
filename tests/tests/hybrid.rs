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
use sqlx::{types::BigDecimal, PgConnection};

#[rstest]
fn hybrid_deprecated(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    );

    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category, rating, in_stock, created_at, metadata)
    WITH (
        key_field = 'id',
        text_fields = '{"description": {}, "category": {}}',
        numeric_fields = '{"rating": {}}',
        boolean_fields = '{"in_stock": {}}',
        datetime_fields = '{"created_at": {}}',
        json_fields = '{"metadata": {}}'
    );

    CREATE EXTENSION vector;
    ALTER TABLE mock_items ADD COLUMN embedding vector(3);

    UPDATE mock_items m
    SET embedding = ('[' ||
        ((m.id + 1) % 10 + 1)::integer || ',' ||
        ((m.id + 2) % 10 + 1)::integer || ',' ||
        ((m.id + 3) % 10 + 1)::integer || ']')::vector;
    "#
    .execute(&mut conn);

    let rows: Vec<(i32, BigDecimal)> = r#"
    WITH semantic_search AS (
        SELECT id, RANK () OVER (ORDER BY embedding <=> '[1,2,3]') AS rank
        FROM mock_items ORDER BY embedding <=> '[1,2,3]' LIMIT 20
    ),
    bm25_search AS (
        SELECT id, RANK () OVER (ORDER BY paradedb.score(id) DESC) as rank
        FROM mock_items WHERE description @@@ 'keyboard' LIMIT 20
    )
    SELECT
        COALESCE(semantic_search.id, bm25_search.id) AS id,
        (COALESCE(1.0 / (60 + semantic_search.rank), 0.0) * 0.1) +
        (COALESCE(1.0 / (60 + bm25_search.rank), 0.0) * 0.9) AS score
    FROM semantic_search
    FULL OUTER JOIN bm25_search ON semantic_search.id = bm25_search.id
    JOIN mock_items ON mock_items.id = COALESCE(semantic_search.id, bm25_search.id)
    ORDER BY score DESC
    LIMIT 5
    "#
    .fetch(&mut conn);

    assert_eq!(
        rows.into_iter().map(|t| t.0).collect::<Vec<_>>(),
        vec![2, 1, 19, 9, 29]
    );
}

#[rstest]
#[allow(clippy::excessive_precision)]
fn reciprocal_rank_fusion(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    r#"
    CREATE EXTENSION vector;
    ALTER TABLE paradedb.bm25_search ADD COLUMN embedding vector(3);

    UPDATE paradedb.bm25_search m
    SET embedding = ('[' ||
    ((m.id + 1) % 10 + 1)::integer || ',' ||
    ((m.id + 2) % 10 + 1)::integer || ',' ||
    ((m.id + 3) % 10 + 1)::integer || ']')::vector;

    CREATE INDEX on paradedb.bm25_search
    USING hnsw (embedding vector_l2_ops)"#
        .execute(&mut conn);

    let columns: Vec<(i32, f32, String)> = r#"
    WITH semantic AS (
        SELECT id, RANK () OVER (ORDER BY embedding <=> '[1,2,3]') AS rank
        FROM paradedb.bm25_search
        ORDER BY embedding <=> '[1,2,3]'
        LIMIT 20
    ),
    bm25 AS (
        SELECT id, RANK () OVER (ORDER BY paradedb.score(id) DESC) as rank
        FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' LIMIT 20
    )
    SELECT
        COALESCE(semantic.id, bm25.id) AS id,
        (COALESCE(1.0 / (60 + semantic.rank), 0.0) +
        COALESCE(1.0 / (60 + bm25.rank), 0.0))::REAL AS score,
        paradedb.bm25_search.description
    FROM semantic
    FULL OUTER JOIN bm25 ON semantic.id = bm25.id
    JOIN paradedb.bm25_search ON paradedb.bm25_search.id = COALESCE(semantic.id, bm25.id)
    ORDER BY score DESC
    LIMIT 5;
    "#
    .fetch(&mut conn);

    assert_eq!(
        columns[0],
        (
            1,
            0.03062178588125292193,
            "Ergonomic metal keyboard".to_string()
        )
    );
    assert_eq!(
        columns[1],
        (2, 0.02990695613646433318, "Plastic Keyboard".to_string())
    );
    assert_eq!(
        columns[2],
        (
            19,
            0.01639344262295081967,
            "Artistic ceramic vase".to_string()
        )
    );
    assert_eq!(
        columns[3],
        (9, 0.01639344262295081967, "Modern wall clock".to_string())
    );
    assert_eq!(
        columns[4],
        (
            29,
            0.01639344262295081967,
            "Designer wall paintings".to_string()
        )
    );
}
