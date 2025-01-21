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
async fn score_bm25_after_delete(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "DELETE FROM paradedb.bm25_search WHERE id = 3 OR id = 4".execute(&mut conn);

    let rows: Vec<(i32,)> = "
    SELECT id, paradedb.score(id) FROM paradedb.bm25_search
    WHERE bm25_search @@@ 'description:shoes' ORDER BY score DESC"
        .fetch_collect(&mut conn);
    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    assert_eq!(ids, [5]);
}

#[rstest]
async fn snippet_after_delete(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "DELETE FROM paradedb.bm25_search WHERE id = 3 OR id = 4".execute(&mut conn);

    let rows: Vec<(i32,)> = "
    SELECT id, paradedb.snippet(description) FROM paradedb.bm25_search
    WHERE description @@@ 'shoes' ORDER BY id"
        .fetch_collect(&mut conn);
    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    assert_eq!(ids, [5]);
}

#[rstest]
async fn score_bm25_after_update(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "UPDATE paradedb.bm25_search SET description = 'leather sandals' WHERE id = 3"
        .execute(&mut conn);

    let rows: Vec<(i32,)> =
        "SELECT id, paradedb.score(id) FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:sandals' ORDER BY score DESC"
            .fetch_collect(&mut conn);
    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    assert_eq!(ids, [3]);

    let rows: Vec<(i32,)> =
        "SELECT id, paradedb.score(id) FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:shoes' ORDER BY score DESC"
            .fetch_collect(&mut conn);
    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    assert_eq!(ids, [5, 4]);
}

#[rstest]
async fn snippet_after_update(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "UPDATE paradedb.bm25_search SET description = 'leather sandals' WHERE id = 3"
        .execute(&mut conn);

    let rows: Vec<(i32,)> = "
        SELECT id, paradedb.snippet(description) FROM paradedb.bm25_search
        WHERE description @@@ 'sandals' ORDER BY id"
        .fetch_collect(&mut conn);
    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    assert_eq!(ids, [3]);

    let rows: Vec<(i32,)> = "
        SELECT id, paradedb.snippet(description) FROM paradedb.bm25_search
        WHERE description @@@ 'shoes' ORDER BY id"
        .fetch_collect(&mut conn);
    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    assert_eq!(ids, [4, 5]);
}

#[rstest]
async fn score_bm25_after_rollback(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    "DELETE FROM paradedb.bm25_search WHERE id = 3".execute(&mut conn);

    "BEGIN".execute(&mut conn);
    "DELETE FROM paradedb.bm25_search WHERE id = 4".execute(&mut conn);
    let rows: Vec<(i32,)> =
        "SELECT id, paradedb.score(id) FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:shoes' ORDER BY score DESC"
            .fetch_collect(&mut conn);
    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    assert_eq!(ids, [5]);

    "ROLLBACK".execute(&mut conn);

    let rows: Vec<(i32,)> =
        "SELECT id, paradedb.score(id) FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:shoes' ORDER BY score DESC"
            .fetch_collect(&mut conn);
    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    assert_eq!(ids, [5, 4]);
}

#[rstest]
async fn snippet_after_rollback(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    "DELETE FROM paradedb.bm25_search WHERE id = 3".execute(&mut conn);

    "BEGIN".execute(&mut conn);
    "DELETE FROM paradedb.bm25_search WHERE id = 4".execute(&mut conn);
    let rows: Vec<(i32,)> =
        "SELECT id, paradedb.snippet(description) FROM paradedb.bm25_search WHERE description @@@ 'shoes' ORDER BY id"
            .fetch_collect(&mut conn);
    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    assert_eq!(ids, [5]);

    "ROLLBACK".execute(&mut conn);

    let rows: Vec<(i32,)> =
        "SELECT id, paradedb.snippet(description) FROM paradedb.bm25_search WHERE description @@@ 'shoes' ORDER BY id"
            .fetch_collect(&mut conn);
    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    assert_eq!(ids, [4, 5]);
}

#[rstest]
async fn score_bm25_after_vacuum(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "DELETE FROM paradedb.bm25_search WHERE id = 4".execute(&mut conn);
    "VACUUM paradedb.bm25_search".execute(&mut conn);

    let rows: Vec<(i32,)> =
        "SELECT id, paradedb.score(id) FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:shoes' ORDER BY score DESC"
            .fetch_collect(&mut conn);
    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    assert_eq!(ids, [5, 3]);

    "VACUUM FULL paradedb.bm25_search".execute(&mut conn);

    let rows: Vec<(i32,)> =
        "SELECT id, paradedb.score(id) FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:shoes' ORDER BY score DESC"
            .fetch_collect(&mut conn);
    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    assert_eq!(ids, [5, 3]);
}

#[rstest]
async fn snippet_after_vacuum(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "DELETE FROM paradedb.bm25_search WHERE id = 4".execute(&mut conn);
    "VACUUM paradedb.bm25_search".execute(&mut conn);

    let rows: Vec<(i32,)> = "
    SELECT id, paradedb.snippet(description) FROM paradedb.bm25_search
    WHERE description @@@ 'description:shoes' ORDER BY id"
        .fetch_collect(&mut conn);
    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    assert_eq!(ids, [3, 5]);

    "VACUUM FULL paradedb.bm25_search".execute(&mut conn);

    let rows: Vec<(i32,)> = "
    SELECT id, paradedb.snippet(description) FROM paradedb.bm25_search
    WHERE description @@@ 'description:shoes' ORDER BY id"
        .fetch_collect(&mut conn);
    let ids: Vec<_> = rows.iter().map(|r| r.0).collect();
    assert_eq!(ids, [3, 5]);
}
