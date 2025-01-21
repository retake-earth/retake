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

use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::{wrap_custom_scan_state, CustomScan, MarkRestoreCapable};
use pgrx::{pg_guard, pg_sys};

/// Complete initialization of the supplied CustomScanState. Standard fields have been initialized
/// by ExecInitCustomScan, but any private fields should be initialized here.
#[pg_guard]
pub extern "C" fn begin_custom_scan<CS: CustomScan>(
    node: *mut pg_sys::CustomScanState,
    estate: *mut pg_sys::EState,
    eflags: i32,
) {
    unsafe { CS::begin_custom_scan(wrap_custom_scan_state(node).as_mut(), estate, eflags) }
}

/// Fetch the next scan tuple. If any tuples remain, it should fill ps_ResultTupleSlot with the next
/// tuple in the current scan direction, and then return the tuple slot. If not, NULL or an empty
/// slot should be returned.
#[pg_guard]
pub extern "C" fn exec_custom_scan<CS: CustomScan>(
    node: *mut pg_sys::CustomScanState,
) -> *mut pg_sys::TupleTableSlot {
    let mut custom_state = wrap_custom_scan_state::<CS>(node);
    unsafe { CS::exec_custom_scan(custom_state.as_mut()) }
}

/// Clean up any private data associated with the CustomScanState. This method is required, but it
/// does not need to do anything if there is no associated data or it will be cleaned up automatically.
#[pg_guard]
pub extern "C" fn end_custom_scan<CS: CustomScan>(node: *mut pg_sys::CustomScanState) {
    let mut custom_state = wrap_custom_scan_state(node);
    unsafe { CS::end_custom_scan(custom_state.as_mut()) }
}

/// Rewind the current scan to the beginning and prepare to rescan the relation.
#[pg_guard]
pub extern "C" fn rescan_custom_scan<CS: CustomScan>(node: *mut pg_sys::CustomScanState) {
    let mut custom_state = wrap_custom_scan_state(node);
    unsafe { CS::rescan_custom_scan(custom_state.as_mut()) }
}

/// Save the current scan position so that it can subsequently be restored by the RestrPosCustomScan
/// callback. This callback is optional, and need only be supplied if the CUSTOMPATH_SUPPORT_MARK_RESTORE
/// flag is set.
#[pg_guard]
pub extern "C" fn mark_pos_custom_scan<CS: CustomScan + MarkRestoreCapable>(
    node: *mut pg_sys::CustomScanState,
) {
    let mut custom_state = wrap_custom_scan_state(node);
    unsafe { CS::mark_pos_custom_scan(custom_state.as_mut()) }
}

/// Restore the previous scan position as saved by the MarkPosCustomScan callback. This callback is
/// optional, and need only be supplied if the CUSTOMPATH_SUPPORT_MARK_RESTORE flag is set.
#[pg_guard]
pub extern "C" fn restr_pos_custom_scan<CS: CustomScan + MarkRestoreCapable>(
    node: *mut pg_sys::CustomScanState,
) {
    let mut custom_state = wrap_custom_scan_state(node);
    unsafe { CS::restr_pos_custom_scan(custom_state.as_mut()) }
}

/// Release resources when it is anticipated the node will not be executed to completion. This is
/// not called in all cases; sometimes, EndCustomScan may be called without this function having
/// been called first. Since the DSM segment used by parallel query is destroyed just after this
/// callback is invoked, custom scan providers that wish to take some action before the DSM segment
/// goes away should implement this method.
#[pg_guard]
pub extern "C" fn shutdown_custom_scan<CS: CustomScan>(node: *mut pg_sys::CustomScanState) {
    let mut custom_state = wrap_custom_scan_state(node);
    unsafe { CS::shutdown_custom_scan(custom_state.as_mut()) }
}

/// Output additional information for EXPLAIN of a custom-scan plan node. This callback is optional.
/// Common data stored in the ScanState, such as the target list and scan relation, will be shown
/// even without this callback, but the callback allows the display of additional, private state.
#[pg_guard]
pub extern "C" fn explain_custom_scan<CS: CustomScan>(
    node: *mut pg_sys::CustomScanState,
    ancestors: *mut pg_sys::List,
    es: *mut pg_sys::ExplainState,
) {
    let custom_state = wrap_custom_scan_state::<CS>(node);
    unsafe {
        CS::explain_custom_scan(
            custom_state.as_ref(),
            ancestors,
            &mut Explainer::new(es).expect("`ExplainState` should not be null"),
        )
    }
}
