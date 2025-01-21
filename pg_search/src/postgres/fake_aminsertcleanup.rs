//! This module fakes the behavior of pg17+'s `aminsertcleanup` by hooking the executor's "finish"
//! hook along with the "process utility hook".
#![allow(static_mut_refs)]

use crate::postgres::insert::{paradedb_aminsertcleanup, InsertState};
use pgrx::{pg_guard, pg_sys};

static mut PENDING_TANTIVY_COMMIT: Option<InsertState> = None;

#[inline]
pub unsafe fn get_pending_insert_state() -> Option<&'static mut InsertState> {
    PENDING_TANTIVY_COMMIT.as_mut()
}

#[inline]
pub unsafe fn set_pending_insert_state(insert_state: InsertState) {
    assert!(
        PENDING_TANTIVY_COMMIT.is_none(),
        "already have a pending tantivy commit state"
    );
    PENDING_TANTIVY_COMMIT = Some(insert_state);
}

#[inline]
pub unsafe fn reset_pending_insert_state() {
    PENDING_TANTIVY_COMMIT = None;
}

pub unsafe fn register() {
    static mut PREV_PROCESS_UTILITY_HOOK: pg_sys::ProcessUtility_hook_type = None;
    static mut PREV_EXECUTOR_FINISH_HOOK: pg_sys::ExecutorFinish_hook_type = None;

    PREV_PROCESS_UTILITY_HOOK = pg_sys::ProcessUtility_hook;
    pg_sys::ProcessUtility_hook = Some(process_utility_hook);

    PREV_EXECUTOR_FINISH_HOOK = pg_sys::ExecutorFinish_hook;
    pg_sys::ExecutorFinish_hook = Some(executor_finish_hook);

    #[cfg(not(feature = "pg13"))]
    #[allow(clippy::too_many_arguments)]
    #[rustfmt::skip]
    #[pg_guard]
    unsafe extern "C" fn process_utility_hook(
        pstmt: *mut pg_sys::PlannedStmt,
        query_string: *const ::core::ffi::c_char,
        read_only_tree: bool,
        context: pg_sys::ProcessUtilityContext::Type,
        params: pg_sys::ParamListInfo,
        query_env: *mut pg_sys::QueryEnvironment,
        dest: *mut pg_sys::DestReceiver,
        qc: *mut pg_sys::QueryCompletion,
    ) {
        if let Some(prev_hook) = PREV_PROCESS_UTILITY_HOOK {
            prev_hook(pstmt, query_string, read_only_tree, context, params, query_env, dest, qc);
        } else {
            pg_sys::standard_ProcessUtility(pstmt, query_string, read_only_tree, context, params, query_env, dest, qc)
        }

        paradedb_aminsertcleanup(PENDING_TANTIVY_COMMIT.take().and_then(|state| state.writer));
    }

    #[cfg(feature = "pg13")]
    #[allow(clippy::too_many_arguments)]
    #[rustfmt::skip]
    #[pg_guard]
    unsafe extern "C" fn process_utility_hook(
        pstmt: *mut pg_sys::PlannedStmt,
        query_string: *const ::core::ffi::c_char,
        context: pg_sys::ProcessUtilityContext::Type,
        params: pg_sys::ParamListInfo,
        query_env: *mut pg_sys::QueryEnvironment,
        dest: *mut pg_sys::DestReceiver,
        qc: *mut pg_sys::QueryCompletion,
    ) {
        if let Some(prev_hook) = PREV_PROCESS_UTILITY_HOOK {
            prev_hook(pstmt, query_string, context, params, query_env, dest, qc);
        } else {
            pg_sys::standard_ProcessUtility(pstmt, query_string, context, params, query_env, dest, qc)
        }

        paradedb_aminsertcleanup(PENDING_TANTIVY_COMMIT.take().and_then(|state| state.writer));
    }

    #[pg_guard]
    unsafe extern "C" fn executor_finish_hook(query_desc: *mut pg_sys::QueryDesc) {
        paradedb_aminsertcleanup(PENDING_TANTIVY_COMMIT.take().and_then(|state| state.writer));

        if let Some(prev_hook) = PREV_EXECUTOR_FINISH_HOOK {
            prev_hook(query_desc);
        } else {
            pg_sys::standard_ExecutorFinish(query_desc);
        }
    }
}
