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

#![cfg(feature = "icu")]

mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn test_icu_arabic_tokenizer(mut conn: PgConnection) {
    IcuArabicPostsTable::setup().execute(&mut conn);
    r#"
    CREATE INDEX idx_arabic ON icu_arabic_posts 
    USING bm25 (id, author, title, message)
    WITH (
        key_field = 'id', 
        text_fields = '{"author": {"tokenizer": {"type": "icu"}}, "title": {"tokenizer": {"type": "icu"}}, "message": {"tokenizer": {"type": "icu"}}}'
    );"#
    .execute(&mut conn);

    let columns: IcuArabicPostsTableVec =
        r#"SELECT * FROM icu_arabic_posts WHERE icu_arabic_posts @@@ 'author:"محمد"' ORDER BY id"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![2]);

    let columns: IcuArabicPostsTableVec =
        r#"SELECT * FROM icu_arabic_posts WHERE icu_arabic_posts @@@ 'title:"السوق"' ORDER BY id"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![2]);

    let columns: IcuArabicPostsTableVec =
        r#"SELECT * FROM icu_arabic_posts WHERE icu_arabic_posts @@@ 'message:"في"' ORDER BY id"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2, 3]);
}

#[rstest]
fn test_icu_amharic_tokenizer(mut conn: PgConnection) {
    IcuAmharicPostsTable::setup().execute(&mut conn);
    r#"
    CREATE INDEX idx_amharic ON icu_amharic_posts 
    USING bm25 (id, author, title, message)
    WITH (
        key_field = 'id', 
        text_fields = '{"author": {"tokenizer": {"type": "icu"}}, "title": {"tokenizer": {"type": "icu"}}, "message": {"tokenizer": {"type": "icu"}}}'
    );"#
    .execute(&mut conn);

    let columns: IcuAmharicPostsTableVec =
        r#"SELECT * FROM icu_amharic_posts WHERE icu_amharic_posts @@@ 'author:"አለም"' ORDER BY id"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3]);

    let columns: IcuAmharicPostsTableVec =
        r#"SELECT * FROM icu_amharic_posts WHERE icu_amharic_posts @@@ 'title:"ለመማር"' ORDER BY id"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3]);

    let columns: IcuAmharicPostsTableVec =
        r#"SELECT * FROM icu_amharic_posts WHERE icu_amharic_posts @@@ 'message:"ዝናብ"' ORDER BY id"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2]);
}

#[rstest]
fn test_icu_greek_tokenizer(mut conn: PgConnection) {
    IcuGreekPostsTable::setup().execute(&mut conn);
    r#"
    CREATE INDEX idx_greek ON icu_greek_posts 
    USING bm25 (id, author, title, message)
    WITH (
        key_field = 'id', 
        text_fields = '{"author": {"tokenizer": {"type": "icu"}}, "title": {"tokenizer": {"type": "icu"}}, "message": {"tokenizer": {"type": "icu"}}}'
    );"#
    .execute(&mut conn);

    let columns: IcuGreekPostsTableVec =
        r#"SELECT * FROM icu_greek_posts WHERE icu_greek_posts @@@ 'author:"Σοφία"' ORDER BY id"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![2]);

    let columns: IcuGreekPostsTableVec =
        r#"SELECT * FROM icu_greek_posts WHERE icu_greek_posts @@@ 'title:"επιτυχία"' ORDER BY id"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3]);

    let columns: IcuGreekPostsTableVec =
        r#"SELECT * FROM icu_greek_posts WHERE icu_greek_posts @@@ 'message:"συμβουλές"' ORDER BY id"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3]);
}

#[rstest]
fn test_icu_czech_tokenizer(mut conn: PgConnection) {
    IcuCzechPostsTable::setup().execute(&mut conn);
    r#"
    CREATE INDEX idx_czech ON icu_czech_posts 
    USING bm25 (id, author, title, message)
    WITH (
        key_field = 'id', 
        text_fields = '{"author": {"tokenizer": {"type": "icu"}}, "title": {"tokenizer": {"type": "icu"}}, "message": {"tokenizer": {"type": "icu"}}}'
    );"#
    .execute(&mut conn);

    let columns: IcuCzechPostsTableVec =
        r#"SELECT * FROM icu_czech_posts WHERE icu_czech_posts @@@ 'author:"Tomáš"' ORDER BY id"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1]);

    let columns: IcuCzechPostsTableVec =
        r#"SELECT * FROM icu_czech_posts WHERE icu_czech_posts @@@ 'title:"zdravý"' ORDER BY id"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![2]);

    let columns: IcuCzechPostsTableVec =
        r#"SELECT * FROM icu_czech_posts WHERE icu_czech_posts @@@ 'message:"velký"~100' ORDER BY id"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3]);
}

#[rstest]
fn test_icu_czech_content_tokenizer(mut conn: PgConnection) {
    IcuCzechPostsTable::setup().execute(&mut conn);
    r#"
    CREATE INDEX idx_czech_content ON icu_czech_posts 
    USING bm25 (id, message)
    WITH (
        key_field = 'id', 
        text_fields = '{"message": {"tokenizer": {"type": "icu"}}}'
    );"#
    .execute(&mut conn);

    let columns: IcuCzechPostsTableVec = r#"
        SELECT * FROM icu_czech_posts
        WHERE icu_czech_posts @@@ paradedb.phrase(
            field => 'message',
            phrases => ARRAY['šla', 'sbírat']
        ) ORDER BY id;"#
        .fetch_collect(&mut conn);

    assert_eq!(columns.id, vec![1]);
}

#[rstest]
fn test_icu_snippet(mut conn: PgConnection) {
    IcuArabicPostsTable::setup().execute(&mut conn);
    r#"
    CREATE INDEX idx_arabic ON icu_arabic_posts 
    USING bm25 (id, author, title, message)
    WITH (
        key_field = 'id', 
        text_fields = '{"author": {"tokenizer": {"type": "icu"}}, "title": {"tokenizer": {"type": "icu"}}, "message": {"tokenizer": {"type": "icu"}}}'
    );"#
    .execute(&mut conn);

    let columns: Vec<(i32, String)> =
        r#"SELECT id, paradedb.snippet(title) FROM icu_arabic_posts WHERE title @@@ 'السوق' "#
            .fetch(&mut conn);
    assert_eq!(
        columns,
        vec![(2, "رحلة إلى <b>السوق</b> مع أبي".to_string())]
    );
}
