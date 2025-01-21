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

use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::CustomScan;
use pgrx::{pg_guard, pg_sys, PgMemoryContexts};

/// Convert a custom path to a finished plan. The return value will generally be a CustomScan object,
/// which the callback must allocate and initialize. See Section 61.2 for more details.
#[pg_guard]
pub extern "C" fn plan_custom_path<CS: CustomScan>(
    root: *mut pg_sys::PlannerInfo,
    rel: *mut pg_sys::RelOptInfo,
    best_path: *mut pg_sys::CustomPath,
    tlist: *mut pg_sys::List,
    clauses: *mut pg_sys::List,
    custom_plans: *mut pg_sys::List,
) -> *mut pg_sys::Plan {
    let mut planner_cxt = PgMemoryContexts::CurrentMemoryContext;
    let builder = CustomScanBuilder::new::<CS>(root, rel, best_path, tlist, clauses, custom_plans);

    let scan = CS::plan_custom_path(builder);
    planner_cxt.leak_and_drop_on_delete(scan).cast()
}

/// This callback is called while converting a path parameterized by the top-most parent of the
/// given child relation child_rel to be parameterized by the child relation. The callback is used
/// to reparameterize any paths or translate any expression nodes saved in the given custom_private
/// member of a CustomPath. The callback may use reparameterize_path_by_child,
/// adjust_appendrel_attrs or adjust_appendrel_attrs_multilevel as required.
#[pg_guard]
pub extern "C" fn reparameterize_custom_path_by_child<CS: CustomScan>(
    root: *mut pg_sys::PlannerInfo,
    custom_private: *mut pg_sys::List,
    child_rel: *mut pg_sys::RelOptInfo,
) -> *mut pg_sys::List {
    todo!("reparameterize_custom_path_by_child")
}
