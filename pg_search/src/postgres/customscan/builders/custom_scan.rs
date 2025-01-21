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

use crate::postgres::customscan::CustomScan;
use pgrx::{node_to_string, pg_sys, PgList};
use std::fmt::{Debug, Formatter};

pub struct Args {
    pub root: *mut pg_sys::PlannerInfo,
    pub rel: *mut pg_sys::RelOptInfo,
    pub best_path: *mut pg_sys::CustomPath,
    pub tlist: PgList<pg_sys::TargetEntry>,
    pub clauses: *mut pg_sys::List,
    pub custom_plans: *mut pg_sys::List,
}

impl Debug for Args {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Args")
            .field("root", &self.root)
            .field("rel", &self.rel)
            .field("best_path", &self.best_path)
            .field(
                "tlist",
                &self
                    .tlist
                    .iter_ptr()
                    .map(|te| unsafe { node_to_string(te.cast()) }.unwrap_or("<null>"))
                    .collect::<Vec<_>>(),
            )
            .field("clauses", &self.clauses)
            .field("custom_plans", &self.custom_plans)
            .finish()
    }
}

pub struct CustomScanBuilder<P: Into<*mut pg_sys::List> + From<*mut pg_sys::List> + Default> {
    args: Args,

    custom_scan_node: pg_sys::CustomScan,
    custom_private: P,
}

impl<P: Into<*mut pg_sys::List> + From<*mut pg_sys::List> + Default> CustomScanBuilder<P> {
    pub fn new<CS: CustomScan>(
        root: *mut pg_sys::PlannerInfo,
        rel: *mut pg_sys::RelOptInfo,
        best_path: *mut pg_sys::CustomPath,
        tlist: *mut pg_sys::List,
        clauses: *mut pg_sys::List,
        custom_plans: *mut pg_sys::List,
    ) -> Self {
        let scan = pg_sys::CustomScan {
            flags: unsafe { (*best_path).flags },
            custom_private: unsafe { *best_path }.custom_private,
            custom_plans,
            methods: CS::custom_scan_methods(),
            scan: pg_sys::Scan {
                plan: pg_sys::Plan {
                    type_: pg_sys::NodeTag::T_CustomScan,
                    targetlist: tlist,
                    startup_cost: unsafe { (*best_path).path.startup_cost },
                    total_cost: unsafe { (*best_path).path.total_cost },
                    plan_rows: unsafe { (*best_path).path.rows },
                    parallel_aware: unsafe { (*best_path).path.parallel_aware },
                    parallel_safe: unsafe { (*best_path).path.parallel_safe },
                    ..Default::default()
                },
                scanrelid: unsafe { *rel }.relid,
            },
            ..Default::default()
        };

        let custom_private = P::from(scan.custom_private);
        CustomScanBuilder {
            args: Args {
                root,
                rel,
                best_path,
                tlist: unsafe { PgList::from_pg(tlist) },
                clauses,
                custom_plans,
            },
            custom_scan_node: scan,
            custom_private,
        }
    }

    pub fn args(&self) -> &Args {
        &self.args
    }

    pub fn custom_private(&self) -> &P {
        &self.custom_private
    }

    pub fn custom_private_mut(&mut self) -> &mut P {
        &mut self.custom_private
    }

    pub fn build(self) -> pg_sys::CustomScan {
        let mut node = self.custom_scan_node;
        node.custom_private = self.custom_private.into();
        node
    }
}
