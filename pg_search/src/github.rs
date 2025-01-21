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

use pgrx::*;
use url::{form_urlencoded, Url};

const BASE_GITHUB_URL: &str = "https://github.com";

#[pg_extern(sql = r#"
    DO $$
    BEGIN
    IF NOT EXISTS (
        SELECT 1 
        FROM pg_namespace 
        WHERE nspname = 'paradedb'
    ) THEN
        CREATE SCHEMA paradedb;
    END IF;
    IF NOT EXISTS (SELECT FROM pg_proc p
        JOIN pg_namespace n ON p.pronamespace = n.oid
        WHERE n.nspname = 'paradedb' AND p.proname = 'help') THEN
            CREATE OR REPLACE FUNCTION paradedb.help(subject TEXT, body TEXT) RETURNS TEXT
            STRICT
            LANGUAGE c 
            AS '@MODULE_PATHNAME@', '@FUNCTION_NAME@';
        END IF;
    END $$;
"#)]
pub fn help(subject: &str, body: &str) -> String {
    let mut url = Url::parse(BASE_GITHUB_URL).expect("Failed to parse GitHub URL");
    url.set_path("/orgs/paradedb/discussions/new");

    let query = form_urlencoded::Serializer::new(String::new())
        .append_pair("category", "q-a")
        .append_pair("title", subject)
        .append_pair("body", body)
        .finish();

    url.set_query(Some(&query));

    format!("To file this ticket, visit {}", String::from(url))
}
