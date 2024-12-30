// Copyright (c) 2023-2024 Retake, Inc.
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

use crate::gucs;
use crate::postgres::customscan::builders::custom_path::{CustomPathBuilder, Flags};
use crate::postgres::customscan::CustomScan;
use once_cell::sync::Lazy;
use pgrx::{pg_guard, pg_sys, PgMemoryContexts};
use rustc_hash::FxHashMap;
use std::collections::hash_map::Entry;

pub fn register_rel_pathlist<CS: CustomScan + 'static>(_: CS) {
    unsafe {
        static mut PREV_HOOKS: Lazy<
            FxHashMap<std::any::TypeId, pg_sys::set_rel_pathlist_hook_type>,
        > = Lazy::new(Default::default);

        #[pg_guard]
        extern "C" fn __priv_callback<CS: CustomScan + 'static>(
            root: *mut pg_sys::PlannerInfo,
            rel: *mut pg_sys::RelOptInfo,
            rti: pg_sys::Index,
            rte: *mut pg_sys::RangeTblEntry,
        ) {
            unsafe {
                #[allow(static_mut_refs)]
                if let Some(Some(prev_hook)) = PREV_HOOKS.get(&std::any::TypeId::of::<CS>()) {
                    (*prev_hook)(root, rel, rti, rte);
                }

                paradedb_rel_pathlist_callback::<CS>(root, rel, rti, rte);
            }
        }

        #[allow(static_mut_refs)]
        match PREV_HOOKS.entry(std::any::TypeId::of::<CS>()) {
            Entry::Occupied(_) => panic!("{} is already registered", std::any::type_name::<CS>()),
            Entry::Vacant(entry) => entry.insert(pg_sys::set_rel_pathlist_hook),
        };

        pg_sys::set_rel_pathlist_hook = Some(__priv_callback::<CS>);

        pg_sys::RegisterCustomScanMethods(CS::custom_scan_methods())
    }
}

/// Although this hook function can be used to examine, modify, or remove paths generated by the
/// core system, a custom scan provider will typically confine itself to generating CustomPath
/// objects and adding them to rel using add_path. The custom scan provider is responsible for
/// initializing the CustomPath object, which is declared like this:
#[pg_guard]
pub extern "C" fn paradedb_rel_pathlist_callback<CS: CustomScan>(
    root: *mut pg_sys::PlannerInfo,
    rel: *mut pg_sys::RelOptInfo,
    rti: pg_sys::Index,
    rte: *mut pg_sys::RangeTblEntry,
) {
    unsafe {
        if !gucs::enable_custom_scan() {
            return;
        }

        if let Some(mut path) = CS::callback(CustomPathBuilder::new::<CS>(root, rel, rti, rte)) {
            let forced = path.flags & Flags::Force as u32 != 0;
            path.flags ^= Flags::Force as u32; // make sure to clear this flag because it's special to us

            let custom_path = PgMemoryContexts::CurrentMemoryContext
                .copy_ptr_into(&mut path, std::mem::size_of_val(&path));

            if forced {
                // remove all the existing possible paths
                (*rel).pathlist = std::ptr::null_mut();
            }

            // add this path for consideration
            pg_sys::add_path(rel, custom_path.cast());
        }
    }
}
