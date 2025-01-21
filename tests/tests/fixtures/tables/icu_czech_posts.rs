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

use soa_derive::StructOfArray;
use sqlx::FromRow;

#[derive(Debug, PartialEq, FromRow, StructOfArray, Default)]
pub struct IcuCzechPostsTable {
    pub id: i32,
    pub author: String,
    pub title: String,
    pub message: String,
}

impl IcuCzechPostsTable {
    pub fn setup() -> &'static str {
        ICU_CZECH_POSTS
    }
}

static ICU_CZECH_POSTS: &str = r#"
CREATE TABLE IF NOT EXISTS icu_czech_posts (
    id SERIAL PRIMARY KEY,
    author TEXT,
    title TEXT,
    message TEXT
);
INSERT INTO icu_czech_posts (author, title, message)
VALUES
    ('Tomáš', 'kouše sendvič', 'červená karkulka v lese šla sbírat dříví'),
    ('Eliška', 'zdravý banán', 'zpívat srdcem do světa'),
    ('Adéla', 'bylo nebylo', 've ztraceném tajném městě žil velký mág');
"#;
