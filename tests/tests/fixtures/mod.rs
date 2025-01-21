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

#![allow(dead_code)]
#![allow(unused_imports)]

pub mod db;
pub mod tables;
pub mod utils;

use async_std::task::block_on;
use rstest::*;
use sqlx::{self, PgConnection};

pub use crate::fixtures::db::*;
pub use crate::fixtures::tables::*;

#[fixture]
pub fn database() -> Db {
    block_on(async { Db::new().await })
}

#[fixture]
pub fn conn(database: Db) -> PgConnection {
    block_on(async {
        let mut conn = database.connection().await;

        // You can hijack a test run to debug, like so:
        // let mut conn = <PgConnection as sqlx::Connection>::connect(
        //     "postgres://neilhansen@localhost:5432/postgres",
        // )
        // .await
        // .unwrap();

        sqlx::query("CREATE EXTENSION pg_search;")
            .execute(&mut conn)
            .await
            .expect("could not create extension pg_search");
        conn
    })
}
