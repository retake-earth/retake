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

#[rustfmt::skip]
#[rstest]
fn manual_vacuum(mut conn: PgConnection) {
    fn count_func(conn: &mut PgConnection) -> i64 {
        "select count(*)::bigint from sadvac WHERE sadvac @@@ 'data:test';".fetch_one::<(i64,)>(conn).0
    }
    
    // originally, this test uncovered a problem at ROW_COUNT=103, but now that the problem is
    // fixed, we'll do a bunch more rows
    const ROW_COUNT:i64 = 10_000;

   "drop table if exists sadvac cascade;
    drop schema if exists idxsadvac cascade;

    create table sadvac
        (
            id   serial8,
            data text
        );
    alter table sadvac set (autovacuum_enabled = 'off');".execute(&mut conn);

    format!("insert into sadvac (data) select 'this is a test ' || x from generate_series(1, {ROW_COUNT}) x;").execute(&mut conn);

    "
    CREATE INDEX idxsadvac ON public.sadvac
    USING bm25 (id, data)
    WITH (key_field = 'id');
    ".execute(&mut conn);
    assert_eq!(count_func(&mut conn), ROW_COUNT, "post create index");

    "update sadvac set id = id;".execute(&mut conn);
    assert_eq!(count_func(&mut conn), ROW_COUNT, "post first update");

    "vacuum sadvac;".execute(&mut conn);
    assert_eq!(count_func(&mut conn), ROW_COUNT, "post vacuum");

    // it's here, after a vacuum, that this would fail
    // for me it fails at i=103
    "update sadvac set id = id;".execute(&mut conn);
    assert_eq!(count_func(&mut conn), ROW_COUNT, "post update after vacuum");
}
