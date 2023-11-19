use pgrx::{pg_sys::ItemPointerData, *};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::manager::get_current_executor_manager;
use crate::operator::scan_index;
use crate::parade_index::index::ParadeIndex;

#[pg_extern]
pub fn rank_bm25(ctid: Option<ItemPointerData>) -> f32 {
    match ctid {
        Some(ctid) => get_current_executor_manager()
            .get_score(ctid)
            .unwrap_or(0.0f32),
        None => 0.0f32,
    }
}

#[pg_extern]
pub fn highlight_bm25(
    ctid: Option<ItemPointerData>,
    index_name: String,
    field_name: String,
) -> String {
    let ctid = match ctid {
        Some(ctid) => ctid,
        _ => return "".into(),
    };
    let manager = get_current_executor_manager();
    let parade_index = ParadeIndex::from_index_name(index_name);
    let doc_address = manager
        .get_doc_address(ctid)
        .expect("could not lookup doc address in manager in highlight_bm25");
    let retrieved_doc = parade_index
        .searcher()
        .doc(doc_address)
        .expect("searcher could not retrieve doc by address in highlight_bm25");

    manager
        .get_highlight(&field_name, &retrieved_doc)
        .unwrap_or("".into())
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[pg_extern]
pub fn minmax_bm25(
    ctid: pg_sys::ItemPointerData,
    index_name: &str,
    query: &str,
    fcinfo: pg_sys::FunctionCallInfo,
) -> f32 {
    let indexrel =
        PgRelation::open_with_name_and_share_lock(index_name).expect("could not open index");
    let index_oid = indexrel.oid();
    let tid = Some(item_pointer_to_u64(ctid));

    match tid {
        Some(tid) => unsafe {
            let mut lookup_by_query = pg_func_extra(fcinfo, || {
                FxHashMap::<(pg_sys::Oid, Option<String>), FxHashSet<u64>>::default()
            });

            lookup_by_query
                .entry((index_oid, Some(String::from(query))))
                .or_insert_with(|| scan_index(query, index_oid))
                .contains(&tid);

            let max_score = get_current_executor_manager().get_max_score();
            let min_score = get_current_executor_manager().get_min_score();
            let raw_score = get_current_executor_manager()
                .get_score(ctid)
                .unwrap_or(0.0);

            if raw_score == 0.0 && min_score == max_score {
                return 0.0;
            }

            if min_score == max_score {
                return 1.0;
            }

            (raw_score - min_score) / (max_score - min_score)
        },
        None => 0.0,
    }
}

#[cfg(feature = "pg_test")]
#[pgrx::pg_schema]
mod tests {
    use pgrx::*;
    use shared::testing::SETUP_SQL;

    #[pg_test]
    fn test_rank_bm25() {
        Spi::run(SETUP_SQL).expect("failed to create index and table");
        let ctid = Spi::get_one::<pg_sys::ItemPointerData>(
            "SELECT ctid FROM one_republic_songs WHERE title = 'If I Lose Myself'",
        )
        .expect("could not get ctid");

        assert!(ctid.is_some());
        let ctid = ctid.unwrap();
        assert_eq!(ctid.ip_posid, 3);

        let query = "SELECT paradedb.rank_bm25(ctid) FROM one_republic_songs WHERE one_republic_songs @@@ 'lyrics:im AND description:song'";
        let rank = Spi::get_one::<f32>(query)
            .expect("failed to rank query")
            .unwrap();
        assert!(rank > 1.0);
    }

    #[pg_test]
    fn test_higlight() {
        Spi::run(SETUP_SQL).expect("failed to create index and table");

        let query = r#"
SELECT paradedb.highlight_bm25(ctid, 'idx_one_republic', 'lyrics')
FROM one_republic_songs
WHERE one_republic_songs @@@ 'lyrics:im:::max_num_chars=10';
        "#;

        let highlight = Spi::get_one::<&str>(query)
            .expect("failed to highlight lyrics")
            .unwrap();
        assert_eq!(highlight, "<b>Im</b> shaking");
    }

    //     #[pg_test]
    //     fn test_minmax() {
    //         Spi::run(SETUP_SQL).expect("failed to create index and table");

    //         let add_ratings = r#"
    //         ALTER TABLE one_republic_songs ADD COLUMN rating vector(7);

    //         WITH NumberedRows AS (
    //             SELECT ctid,
    //                 ROW_NUMBER() OVER () as row_num
    //             FROM one_republic_songs
    //         )
    //         UPDATE one_republic_songs m
    //         SET rating = ('[' ||
    //             ((n.row_num + 1) % 5 + 1)::integer || ',' ||
    //             ((n.row_num + 2) % 5 + 2)::integer || ',' ||
    //             ((n.row_num + 2) % 5 + 3)::integer || ',' ||
    //             ((n.row_num + 2) % 5 + 4)::integer || ',' ||
    //             ((n.row_num + 2) % 5 + 5)::integer || ',' ||
    //             ((n.row_num + 2) % 5 + 6)::integer || ',' ||
    //             ((n.row_num + 3) % 5 + 7)::integer || ']')::vector
    //         FROM NumberedRows n
    //         WHERE m.ctid = n.ctid;
    //         "#;
    //         Spi::run(add_ratings).expect("failed to add ratings column to table");

    //         let query = r#"
    // SELECT
    //     paradedb.weighted_mean(
    //         paradedb.minmax_bm25(ctid, 'idx_one_republic', 'lyrics:im AND description:desc'),
    //         1 - paradedb.minmax_norm(
    //           '[1,2,3]' <-> rating,
    //           MIN('[1,2,3]' <-> rating) OVER (),
    //           MAX('[1,2,3]' <-> rating) OVER ()
    //         ),
    //         ARRAY[0.8,0.2]
    //     ) as score_hybrid
    // FROM one_republic_songs
    // ORDER BY score_hybrid DESC
    // LIMIT 3;
    //         "#;

    //         let (score1, score2, score3) =
    //             Spi::get_three::<f32, f32, f32>(query).expect("failed to get min max");
    //         println!("{:#?}, {:#?}, {:#?}", score1, score2, score3);
    //     }
}
