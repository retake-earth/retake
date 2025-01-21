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

use std::time::Instant;

use anyhow::Result;
use fixtures::*;
use futures::future::join_all;
use pretty_assertions::assert_eq;
use rand::Rng;
use rstest::*;
use tokio::join;

/// This test targets the locking functionality between Tantivy writers.
/// With no locking implemented, a high number of concurrent writers will
/// cause in an error when they all try to commit to the index at once.
#[rstest]
#[tokio::test]
async fn test_simultaneous_commits_with_bm25(database: Db) -> Result<()> {
    let mut conn1 = database.connection().await;

    // Create table once using any of the connections.
    r#"CREATE EXTENSION pg_search;

    CREATE TABLE concurrent_items (
      id SERIAL PRIMARY KEY,
      description TEXT,
      category VARCHAR(255),
      created_at TIMESTAMP DEFAULT now()
    );

    CREATE INDEX concurrent_items_bm25 ON public.concurrent_items
    USING bm25 (id, description)
    WITH (
        key_field = 'id',
        text_fields = '{
            "description": {}
        }'
    );
    "#
    .execute(&mut conn1);

    // Dynamically generate at least 100 rows for each connection
    let mut rng = rand::thread_rng();
    let categories = [
        "Category 1",
        "Category 2",
        "Category 3",
        "Category 4",
        "Category 5",
    ];

    for i in 0..5 {
        let random_category = categories[rng.gen_range(0..categories.len())];

        // Create new connections for this iteration and store them in a vector
        let mut connections = vec![];
        for _ in 0..50 {
            connections.push(database.connection().await);
        }

        let mut futures = vec![];
        for (n, mut conn) in connections.into_iter().enumerate() {
            let query = format!(
                "INSERT INTO concurrent_items (description, category)
                 VALUES ('Item {i} from conn{n}', '{random_category}')"
            );
            // Move the connection into the future, avoiding multiple borrows
            futures.push(async move { query.execute_async(&mut conn).await });
        }

        // Await all the futures for this iteration
        join_all(futures).await;
    }

    // Verify the number of rows in each database
    let rows1: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM concurrent_items")
        .fetch_one(&mut conn1)
        .await?;

    assert_eq!(rows1, 250);

    Ok(())
}

#[rstest]
#[tokio::test]
async fn test_statement_level_locking(database: Db) -> Result<()> {
    let mut conn = database.connection().await;

    // Create tables and indexes
    r#"CREATE EXTENSION pg_search;
    CREATE TABLE index_a (
      id SERIAL PRIMARY KEY,
      content TEXT
    );
    CREATE TABLE index_b (
      id SERIAL PRIMARY KEY,
      content TEXT
    );

    CREATE INDEX index_a_bm25 ON public.index_a
    USING bm25 (id, content)
    WITH (
        key_field = 'id',
        text_fields = '{
            "content": {}
        }'
    );

    CREATE INDEX index_b_bm25 ON public.index_b
    USING bm25 (id, content)
    WITH (
        key_field = 'id',
        text_fields = '{
            "content": {}
        }'
    );
    "#
    .execute(&mut conn);

    // Create two separate connections
    let mut conn_a = database.connection().await;
    let mut conn_b = database.connection().await;

    // Define the tasks for each connection
    let task_a = async move {
        "INSERT INTO index_a (content) VALUES ('Content A1');
         SELECT pg_sleep(3);
         INSERT INTO index_b (content) VALUES ('Content B1 from A');"
            .execute_async(&mut conn_a)
            .await;
    };

    let task_b = async move {
        "INSERT INTO index_b (content) VALUES ('Content B2');
         SELECT pg_sleep(3);
         INSERT INTO index_a (content) VALUES ('Content A2 from B');"
            .execute_async(&mut conn_b)
            .await;
    };

    // We're going to check a timer to ensure both of these queries,
    // which each sleep at query time, run concurrently.
    let start_time = Instant::now();

    // Run both tasks concurrently
    join!(task_a, task_b);

    // Stop the timer and assert that the duration is close to 5 seconds
    let duration = start_time.elapsed();
    assert!(
        duration.as_secs() >= 3 && duration.as_secs() < 5,
        "Expected duration to be around 3 seconds, but it took {:?}",
        duration
    );

    // Verify the results
    let count_a: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM index_a")
        .fetch_one(&mut conn)
        .await?;
    let count_b: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM index_b")
        .fetch_one(&mut conn)
        .await?;

    assert_eq!(count_a, 2, "Expected 2 rows in index_a");
    assert_eq!(count_b, 2, "Expected 2 rows in index_b");

    Ok(())
}
