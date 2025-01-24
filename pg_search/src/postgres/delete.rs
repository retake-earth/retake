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

use pgrx::{pg_sys::ItemPointerData, *};
use tantivy::Term;

use super::storage::block::CLEANUP_LOCK;
use crate::index::fast_fields_helper::FFType;
use crate::index::merge_policy::MergeLock;
use crate::index::reader::index::SearchIndexReader;
use crate::index::writer::index::SearchIndexWriter;
use crate::index::{BlockDirectoryType, WriterResources};
use crate::postgres::storage::buffer::BufferManager;

#[pg_guard]
pub unsafe extern "C" fn ambulkdelete(
    info: *mut pg_sys::IndexVacuumInfo,
    stats: *mut pg_sys::IndexBulkDeleteResult,
    callback: pg_sys::IndexBulkDeleteCallback,
    callback_state: *mut ::std::os::raw::c_void,
) -> *mut pg_sys::IndexBulkDeleteResult {
    let info = PgBox::from_pg(info);
    let mut stats = PgBox::from_pg(stats);
    let index_relation = PgRelation::from_pg(info.index);
    let callback =
        callback.expect("the ambulkdelete() callback should be a valid function pointer");
    let callback = move |ctid_val: u64| {
        let mut ctid = ItemPointerData::default();
        crate::postgres::utils::u64_to_item_pointer(ctid_val, &mut ctid);
        callback(&mut ctid, callback_state)
    };

    let merge_lock = MergeLock::acquire_for_delete(index_relation.oid());
    let mut writer = SearchIndexWriter::open(
        &index_relation,
        BlockDirectoryType::BulkDelete,
        WriterResources::Vacuum,
    )
    .expect("ambulkdelete: should be able to open a SearchIndexWriter");
    let reader = SearchIndexReader::open(&index_relation, BlockDirectoryType::BulkDelete, false)
        .expect("ambulkdelete: should be able to open a SearchIndexReader");

    let ctid_field = writer.get_ctid_field();
    let mut did_delete = false;

    for segment_reader in reader.searcher().segment_readers() {
        let ctid_ff = FFType::new_ctid(segment_reader.fast_fields());

        for doc_id in 0..segment_reader.max_doc() {
            if doc_id % 100 == 0 {
                // NB:  when `IsInParallelMode()` is true, it seems there's always a pending interrupt
                // so we just don't bother checking for pending interrupts in that situation.  This
                // is in the case of a parallel vacuum
                if pg_sys::InterruptPending != 0 && !pg_sys::IsInParallelMode() {
                    drop(merge_lock);

                    // we think there's a pending interrupt, so this should raise a cancel query ERROR
                    pg_sys::vacuum_delay_point();

                    // if we got here then, ultimately, CHECK_FOR_INTERRUPTS() (which is called via
                    // vacuum_delay_point()) didn't actually interrupt us, and we're just DOA now
                    // because we've already dropped our merge_lock
                    unreachable!("ambulkdelete: detected interrupt but wasn't cancelled");
                }
                pg_sys::vacuum_delay_point();
            }

            let ctid = ctid_ff.as_u64(doc_id).expect("ctid should be present");
            if callback(ctid) {
                did_delete = true;
                writer
                    .delete_term(Term::from_field_u64(ctid_field, ctid))
                    .expect("ambulkdelete: deleting ctid Term should succeed");
            }
        }
    }

    // this won't merge as the `WriterResources::Vacuum` uses `AllowedMergePolicy::None`
    writer
        .commit()
        .expect("ambulkdelete: commit should succeed");

    // we're done evaluating docs and no longer need to hold the merge_lock.
    //
    // In fact, holding it while we potentially acquire the CLEANUP_LOCK below can cause deadlocks
    // across backends due to lock inversion issues
    drop(merge_lock);

    if stats.is_null() {
        stats = unsafe {
            PgBox::from_pg(
                pg_sys::palloc0(std::mem::size_of::<pg_sys::IndexBulkDeleteResult>()).cast(),
            )
        };
        stats.pages_deleted = 0;
    }

    // As soon as ambulkdelete returns, Postgres will update the visibility map
    // This can cause concurrent scans that have just read ctids, which are dead but
    // are about to be marked visible, to return wrong results. To guard against this,
    // we acquire a cleanup lock that guarantees that there are no pins on the index,
    // which means that all concurrent scans have completed.
    //
    // This lock is then immediately released, allowing any queued scans to continue, seeing
    // the new deleted state of the docs we just marked as deleted
    if did_delete {
        let mut bman = BufferManager::new(index_relation.oid());
        let cleanup_lock =
            bman.get_buffer_for_cleanup(CLEANUP_LOCK, pg_sys::ReadBufferMode::RBM_NORMAL as _);
        drop(cleanup_lock);
    }

    stats.into_pg()
}
