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

use fixtures::db::Query;
use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn test_subselect(mut conn: PgConnection) {
    r#"
        CREATE TABLE test_subselect(id serial8, t text);
        INSERT INTO test_subselect(t) VALUES ('this is a test');

        CREATE INDEX test_subselect_idx ON test_subselect
        USING bm25 (id, t)
        WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    let (id,) = r#"
        select id from (select random(), * from (select random(), t, id from test_subselect) x) test_subselect 
        where id @@@ 't:test';"#
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(id, 1);
}

#[rstest]
fn test_cte(mut conn: PgConnection) {
    r#"
        CREATE TABLE test_cte(id serial8, t text);
        INSERT INTO test_cte(t) VALUES ('beer wine cheese');
        INSERT INTO test_cte(t) VALUES ('beer cheese');

        CREATE INDEX test_cte_idx ON test_cte
        USING bm25 (id, t)
        WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    let (id,) = r#"
        with my_cte as (select * from test_cte)
        select * from my_cte a inner join my_cte b on a.id = b.id
        where a.id @@@ 't:beer' and b.id @@@ 't:cheese' order by a.id;"#
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(id, 1);
}

#[rstest]
fn test_cte2(mut conn: PgConnection) {
    r#"
        CREATE TABLE test_cte(id serial8, t text);
        INSERT INTO test_cte(t) VALUES ('beer wine cheese');
        INSERT INTO test_cte(t) VALUES ('beer cheese');

        CREATE INDEX test_cte_idx ON test_cte
        USING bm25 (id, t)
        WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    let (id,) = r#"
        with my_cte as (select * from test_cte)
        select * from my_cte where id @@@ 't:beer' order by id;"#
        .fetch_one::<(i64,)>(&mut conn);
    assert_eq!(id, 1);
}

#[rstest]
fn test_plain_relation(mut conn: PgConnection) {
    r#"
        CREATE TABLE test_plain_relation(id serial8, t text);
        INSERT INTO test_plain_relation(t) VALUES ('beer wine cheese');

        CREATE INDEX test_plain_relation_idx ON test_plain_relation
        USING bm25 (id, t)
        WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    let (id,) =
        "select id from test_plain_relation where id @@@ 't:beer'".fetch_one::<(i64,)>(&mut conn);
    assert_eq!(id, 1);
}
