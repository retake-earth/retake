/// Rust implementations of Postgres functions in src/include/utils/rel.h
/// related to Write-Ahead Logging (WAL).
///
/// This can be contributed to pgrx.
use pgrx::*;
use std::mem::size_of;

static INVALID_SUBTRANSACTION_ID: pg_sys::SubTransactionId = 0;

unsafe fn xlog_is_needed() -> bool {
    pg_sys::wal_level >= pg_sys::WalLevel_WAL_LEVEL_REPLICA as i32
}

unsafe fn relation_is_permanent(rel: pg_sys::Relation) -> bool {
    (*(*rel).rd_rel).relpersistence == pg_sys::RELPERSISTENCE_PERMANENT as i8
}

unsafe fn page_xlog_recptr_set(mut ptr: pg_sys::PageXLogRecPtr, lsn: pg_sys::XLogRecPtr) {
    ptr.xlogid = (lsn >> 32) as u32;
    ptr.xrecoff = lsn as u32;
}

unsafe fn page_xlog_recptr_get(mut ptr: pg_sys::PageXLogRecPtr) -> pg_sys::XLogRecPtr {
    (ptr.xlogid as u64) << 32 | ptr.xrecoff as pg_sys::XLogRecPtr
}

/// # Safety
/// This function is unsafe because it calls pg_sys functions
pub unsafe fn relation_needs_wal(rel: pg_sys::Relation) -> bool {
    // #define RelationNeedsWAL(relation)							        \
    // (RelationIsPermanent(relation) && (XLogIsNeeded() ||				    \
    //   (relation->rd_createSubid == InvalidSubTransactionId &&			\
    //    relation->rd_firstRelfilelocatorSubid == InvalidSubTransactionId)))
    relation_is_permanent(rel)
        && (xlog_is_needed()
            || ((*rel).rd_createSubid == INVALID_SUBTRANSACTION_ID
                && (*rel).rd_firstRelfilelocatorSubid == INVALID_SUBTRANSACTION_ID))
}

/// # Safety
/// This function is unsafe because it calls pg_sys functions
pub unsafe fn page_get_lsn(page: pg_sys::Page) -> pg_sys::XLogRecPtr {
    // static inline XLogRecPtr
    // PageGetLSN(Page page)
    // {
    //     return PageXLogRecPtrGet(((PageHeader) page)->pd_lsn);
    // }
    let page_header = page as *mut pg_sys::PageHeaderData;
    page_xlog_recptr_get((*page_header).pd_lsn)
}

/// # Safety
/// This function is unsafe because it calls pg_sys functions
pub unsafe fn page_set_lsn(page: pg_sys::Page, lsn: pg_sys::XLogRecPtr) {
    // static inline void
    // PageSetLSN(Page page, XLogRecPtr lsn)
    // {
    //     PageXLogRecPtrSet(((PageHeader) page)->pd_lsn, lsn);
    // }
    let page_header = page as *mut pg_sys::PageHeaderData;
    page_xlog_recptr_set((*page_header).pd_lsn, lsn);
}

/// # Safety
/// This function is unsafe because it calls pg_sys functions
pub unsafe fn xlog_rec_get_info(record: *mut pg_sys::XLogReaderState) -> u8 {
    (*(*record).record).header.xl_info
}

/// # Safety
/// This function is unsafe because it calls pg_sys functions
pub unsafe fn xlog_rec_get_data(record: *mut pg_sys::XLogReaderState) -> *mut i8 {
    (*(*record).record).main_data
}
