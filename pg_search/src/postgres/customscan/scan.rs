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

use crate::postgres::customscan::builders::custom_state::CustomScanStateBuilder;
use crate::postgres::customscan::CustomScan;
use pgrx::{pg_guard, pg_sys};

/// Allocate a CustomScanState for this CustomScan. The actual allocation will often be larger than
/// required for an ordinary CustomScanState, because many providers will wish to embed that as the
/// first field of a larger structure. The value returned must have the node tag and methods set
/// appropriately, but other fields should be left as zeroes at this stage; after ExecInitCustomScan
/// performs basic initialization, the BeginCustomScan callback will be invoked to give the custom
/// scan provider a chance to do whatever else is needed.
#[pg_guard]
pub extern "C" fn create_custom_scan_state<CS: CustomScan>(
    cscan: *mut pg_sys::CustomScan,
) -> *mut pg_sys::Node {
    let builder = CustomScanStateBuilder::new(cscan);
    CS::create_custom_scan_state(builder).cast()
}
