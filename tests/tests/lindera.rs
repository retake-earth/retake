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

#![allow(unused_variables, unused_imports)]
mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
async fn lindera_korean_tokenizer(mut conn: PgConnection) {
    r#"CREATE TABLE IF NOT EXISTS korean (
        id SERIAL PRIMARY KEY,
        author TEXT,
        title TEXT,
        message TEXT
    );

    INSERT INTO korean (author, title, message)
    VALUES
        ('김민준', '서울의 새로운 카페', '서울 중심부에 새로운 카페가 문을 열었습니다. 현대적인 디자인과 독특한 커피 선택이 특징입니다.'),
        ('이하은', '축구 경기 리뷰', '어제 열린 축구 경기에서 화려한 골이 터졌습니다. 마지막 순간의 반전이 경기의 하이라이트였습니다.'),
        ('박지후', '지역 축제 개최 소식', '이번 주말 지역 축제가 열립니다. 다양한 음식과 공연이 준비되어 있어 기대가 됩니다.');

        CREATE INDEX korean_idx ON korean
        USING bm25 (id, author, title, message)
        WITH (
            key_field = 'id',
            text_fields = '{
                "author": {
                    "tokenizer": {"type": "korean_lindera"},
                    "record": "position"
                },
                "title": {
                    "tokenizer": {"type": "korean_lindera"},
                    "record": "position"
                },
                "message": {
                    "tokenizer": {"type": "korean_lindera"},
                    "record": "position"
                }
            }'
        );
    "#
    .execute(&mut conn);

    let row: (i32,) = r#"SELECT id FROM korean WHERE korean @@@ 'author:김민준' ORDER BY id"#
        .fetch_one(&mut conn);
    assert_eq!(row.0, 1);

    let row: (i32,) =
        r#"SELECT id FROM korean WHERE korean @@@ 'title:"경기"' ORDER BY id"#.fetch_one(&mut conn);
    assert_eq!(row.0, 2);

    let row: (i32,) = r#"SELECT id FROM korean WHERE korean @@@ 'message:"지역 축제"' ORDER BY id"#
        .fetch_one(&mut conn);
    assert_eq!(row.0, 3);
}

#[rstest]
async fn lindera_chinese_tokenizer(mut conn: PgConnection) {
    r#"CREATE TABLE IF NOT EXISTS chinese (
        id SERIAL PRIMARY KEY,
        author TEXT,
        title TEXT,
        message TEXT
    );
    INSERT INTO chinese (author, title, message)
    VALUES
        ('李华', '北京的新餐馆', '北京市中心新开了一家餐馆，以其现代设计和独特的菜肴选择而闻名。'),
        ('张伟', '篮球比赛回顾', '昨日篮球比赛精彩纷呈，尤其是最后时刻的逆转成为了比赛的亮点。'),
        ('王芳', '本地文化节', '本周末将举行一个地方文化节，预计将有各种食物和表演。');

    CREATE INDEX chinese_idx ON chinese
    USING bm25 (id, author, title, message)
    WITH (
        key_field = 'id',
        text_fields = '{
            "author": {
                "tokenizer": {"type": "chinese_lindera"},
                "record": "position"
            },
            "title": {
                "tokenizer": {"type": "chinese_lindera"},
                "record": "position"
            },
            "message": {
                "tokenizer": {"type": "chinese_lindera"},
                "record": "position"
            }
        }'
    ); 
    "#
    .execute(&mut conn);

    let row: (i32,) =
        r#"SELECT id FROM chinese WHERE chinese @@@ 'author:华' ORDER BY id"#.fetch_one(&mut conn);
    assert_eq!(row.0, 1);

    let row: (i32,) =
        r#"SELECT id FROM chinese WHERE chinese @@@ 'title:北京' ORDER BY id"#.fetch_one(&mut conn);
    assert_eq!(row.0, 1);

    let row: (i32,) = r#"SELECT id FROM chinese WHERE chinese @@@ 'message:文化节' ORDER BY id"#
        .fetch_one(&mut conn);
    assert_eq!(row.0, 3);
}

#[rstest]
async fn lindera_japenese_tokenizer(mut conn: PgConnection) {
    r#"
    CREATE TABLE IF NOT EXISTS japanese (
        id SERIAL PRIMARY KEY,
        author TEXT,
        title TEXT,
        message TEXT
    );
    INSERT INTO japanese (author, title, message)
    VALUES
        ('佐藤健', '東京の新しいカフェ', '東京の中心部に新しいカフェがオープンしました。モダンなデザインとユニークなコーヒーが特徴です。'),
        ('鈴木一郎', 'サッカー試合レビュー', '昨日のサッカー試合では素晴らしいゴールが見られました。終了間際のドラマチックな展開がハイライトでした。'),
        ('高橋花子', '地元の祭り', '今週末に地元で祭りが開催されます。様々な食べ物とパフォーマンスが用意されています。');

    CREATE INDEX japanese_idx ON japanese
    USING bm25 (id, author, title, message)
    WITH (
        key_field = 'id',
        text_fields = '{
            "author": {
                "tokenizer": {"type": "japanese_lindera"},
                "record": "position"
            },
            "title": {
                "tokenizer": {"type": "japanese_lindera"},
                "record": "position"
            },
            "message": {
                "tokenizer": {"type": "japanese_lindera"},
                "record": "position"
            }
        }'
    );
    "#
    .execute(&mut conn);

    let row: (i32,) = r#"SELECT id FROM japanese WHERE japanese @@@ 'author:佐藤' ORDER BY id"#
        .fetch_one(&mut conn);
    assert_eq!(row.0, 1);

    let row: (i32,) = r#"SELECT id FROM japanese WHERE japanese @@@ 'title:サッカー' ORDER BY id"#
        .fetch_one(&mut conn);
    assert_eq!(row.0, 2);

    let row: (i32,) = r#"SELECT id FROM japanese WHERE japanese @@@ 'message:祭り' ORDER BY id"#
        .fetch_one(&mut conn);
    assert_eq!(row.0, 3);
}
