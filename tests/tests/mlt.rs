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
fn mlt_enables_scoring_issue1747(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let (id,) = "
    SELECT id FROM paradedb.bm25_search WHERE id @@@ paradedb.more_like_this(
        document_id => 3,
        min_term_frequency => 1
    ) ORDER BY id LIMIT 1"
        .fetch_one::<(i32,)>(&mut conn);
    assert_eq!(id, 3);
}

#[rstest]
fn mlt_scoring_nested(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // Boolean must
    let results: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE id @@@ 
    paradedb.boolean(
        must => paradedb.more_like_this(
            min_doc_frequency => 2,
            min_term_frequency => 1,
            document_fields => '{"description": "keyboard"}'
        )
    )
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(results.id, [1, 2]);

    // Boolean must_not
    let results: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE id @@@ 
    paradedb.boolean(
        must_not => paradedb.more_like_this(
            min_doc_frequency => 2,
            min_term_frequency => 1,
            document_fields => '{"description": "keyboard"}'
        )
    )
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert!(results.is_empty());

    // Boolean should
    let results: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE id @@@ 
    paradedb.boolean(
        should => paradedb.more_like_this(
            min_doc_frequency => 2,
            min_term_frequency => 1,
            document_fields => '{"description": "keyboard"}'
        )
    )
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(results.id, [1, 2]);

    // Boost
    let results: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE id @@@ 
    paradedb.boost(
        factor => 1.5,
        query => paradedb.more_like_this(
            min_doc_frequency => 2,
            min_term_frequency => 1,
            document_fields => '{"description": "keyboard"}'
        )
    )
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(results.id, [1, 2]);

    // ConstScore
    let results: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE id @@@ 
    paradedb.const_score(
        score => 5,
        query => paradedb.more_like_this(
            min_doc_frequency => 2,
            min_term_frequency => 1,
            document_fields => '{"description": "keyboard"}'
        )
    )
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(results.id, [1, 2]);

    // DisjunctionMax
    let results: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE id @@@ 
    paradedb.disjunction_max(
        disjuncts => ARRAY[
            paradedb.more_like_this(
                min_doc_frequency => 2,
                min_term_frequency => 1,
                document_fields => '{"description": "keyboard"}'
            ), 
            paradedb.more_like_this(
                min_doc_frequency => 2,
                min_term_frequency => 1,
                document_fields => '{"description": "shoes"}'
            )            
        ]
    )
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(results.id, [1, 2, 3, 4, 5]);

    // Multiple nested
    let results: SimpleProductsTableVec = r#"
    SELECT * FROM paradedb.bm25_search
    WHERE id @@@ 
    paradedb.boolean(
        must_not => paradedb.parse('description:plastic'),
        should => paradedb.disjunction_max(
            disjuncts => ARRAY[
                paradedb.boost(
                    factor => 3,
                    query => paradedb.more_like_this(
                        min_doc_frequency => 2,
                        min_term_frequency => 1,
                        document_fields => '{"description": "keyboard"}'
                    ) 
                ),
                paradedb.more_like_this(
                    min_doc_frequency => 2,
                    min_term_frequency => 1,
                    document_fields => '{"description": "shoes"}'
                )            
            ]
        )
    )
    ORDER BY id
    "#
    .fetch_collect(&mut conn);
    assert_eq!(results.id, [1, 3, 4, 5]);
}
