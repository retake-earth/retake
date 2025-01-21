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

use pgrx::{pg_guard, pg_sys};
use std::ptr::addr_of_mut;

#[derive(Debug)]
#[repr(transparent)]
pub struct Spinlock(pg_sys::slock_t);

impl Spinlock {
    #[inline(always)]
    pub fn acquire(&mut self) -> impl Drop {
        AcquiredSpinLock::new(self)
    }
}

#[repr(transparent)]
struct AcquiredSpinLock(*mut pg_sys::slock_t);

impl AcquiredSpinLock {
    fn new(lock: &mut Spinlock) -> Self {
        unsafe {
            let addr = addr_of_mut!(lock.0);
            pg_sys::SpinLockAcquire(addr);
            Self(addr)
        }
    }
}

impl Drop for AcquiredSpinLock {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe {
            pg_sys::SpinLockRelease(self.0);
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct Bm25ParallelScanState {
    mutex: Spinlock,
    remaining_segments: u32,
}

impl Bm25ParallelScanState {
    #[inline(always)]
    pub fn lock(&mut self) -> impl Drop {
        self.mutex.acquire()
    }
}

#[pg_guard]
pub unsafe extern "C" fn aminitparallelscan(target: *mut ::core::ffi::c_void) {
    let state = target.cast::<Bm25ParallelScanState>();
    pg_sys::SpinLockInit(addr_of_mut!((*state).mutex.0));
}

#[pg_guard]
pub unsafe extern "C" fn amparallelrescan(_scan: pg_sys::IndexScanDesc) {}

#[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
#[pg_guard]
pub unsafe extern "C" fn amestimateparallelscan() -> pg_sys::Size {
    size_of::<Bm25ParallelScanState>()
}

#[cfg(feature = "pg17")]
#[pg_guard]
pub unsafe extern "C" fn amestimateparallelscan(_nkeys: i32, _norderbys: i32) -> pg_sys::Size {
    size_of::<Bm25ParallelScanState>()
}

unsafe fn bm25_shared_state(
    scan: &pg_sys::IndexScanDescData,
) -> Option<&mut Bm25ParallelScanState> {
    if scan.parallel_scan.is_null() {
        None
    } else {
        scan.parallel_scan
            .cast::<std::ffi::c_void>()
            .add((*scan.parallel_scan).ps_offset)
            .cast::<Bm25ParallelScanState>()
            .as_mut()
    }
}

pub fn maybe_init_parallel_scan(
    scan: pg_sys::IndexScanDesc,
    searcher: &tantivy::Searcher,
) -> Option<i32> {
    if unsafe { (*scan).parallel_scan.is_null() } {
        // not a parallel scan, so there's nothing to initialize
        return None;
    }

    let state = get_bm25_scan_state(&scan)?;
    let worker_number = unsafe { pg_sys::ParallelWorkerNumber };
    let _mutex = state.lock();
    if worker_number == -1 {
        // ParallelWorkerNumber -1 is the main backend, which is where we'll set up
        // our shared memory information
        state.remaining_segments = searcher
            .segment_readers()
            .len()
            .try_into()
            .expect("should not have more than u32 index segments");
    }
    Some(worker_number)
}

pub fn maybe_claim_segment(scan: pg_sys::IndexScanDesc) -> Option<tantivy::SegmentOrdinal> {
    let state = get_bm25_scan_state(&scan)?;

    let _mutex = state.lock();
    if state.remaining_segments == 0 {
        // no more to claim
        None
    } else {
        // claim the next one
        state.remaining_segments -= 1;
        Some(state.remaining_segments)
    }
}

fn get_bm25_scan_state(scan: &pg_sys::IndexScanDesc) -> Option<&mut Bm25ParallelScanState> {
    unsafe {
        assert!(!scan.is_null());
        let scan = scan.as_mut().unwrap_unchecked();
        bm25_shared_state(scan)
    }
}
