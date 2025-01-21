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
use pgrx::{pg_sys, PgList, PgMemoryContexts};
use std::fmt::{Debug, Formatter};
use std::ptr::addr_of_mut;

pub struct Args {
    pub cscan: *mut pg_sys::CustomScan,
}

#[repr(C)]
pub struct CustomScanStateWrapper<CS: CustomScan> {
    pub csstate: pg_sys::CustomScanState,
    custom_state: CS::State,
}

impl<CS: CustomScan> Debug for CustomScanStateWrapper<CS>
where
    CS::State: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(&format!(
            "CustomScanStateWrapper<{}>",
            std::any::type_name::<CS>()
        ))
        .field("state_type", &std::any::type_name::<CS::State>())
        .field("csstate", &self.csstate)
        .field("custom_state", &self.custom_state)
        .finish()
    }
}

impl<CS: CustomScan> CustomScanStateWrapper<CS> {
    #[inline(always)]
    pub fn planstate(&mut self) -> *mut pg_sys::PlanState {
        addr_of_mut!(self.csstate.ss.ps)
    }

    #[inline(always)]
    pub fn custom_state(&self) -> &CS::State {
        &self.custom_state
    }

    #[inline(always)]
    pub fn custom_state_mut(&mut self) -> &mut CS::State {
        &mut self.custom_state
    }

    #[inline(always)]
    pub fn scanslot(&self) -> *mut pg_sys::TupleTableSlot {
        self.csstate.ss.ss_ScanTupleSlot
    }

    #[inline(always)]
    pub fn projection_info(&self) -> *mut pg_sys::ProjectionInfo {
        self.csstate.ss.ps.ps_ProjInfo
    }
}

pub struct CustomScanStateBuilder<CS: CustomScan, P: From<*mut pg_sys::List>> {
    args: Args,

    custom_state: CS::State,
    custom_private: P,
}

impl<CS: CustomScan, P: From<*mut pg_sys::List>> CustomScanStateBuilder<CS, P> {
    pub fn new(cscan: *mut pg_sys::CustomScan) -> Self {
        Self {
            args: Args { cscan },

            custom_state: CS::State::default(),
            custom_private: unsafe { P::from((*cscan).custom_private) },
        }
    }

    pub fn args(&self) -> &Args {
        &self.args
    }

    pub fn custom_private(&self) -> &P {
        &self.custom_private
    }

    pub fn custom_state(&mut self) -> &mut CS::State {
        &mut self.custom_state
    }

    pub fn target_list(&self) -> PgList<pg_sys::TargetEntry> {
        unsafe { PgList::from_pg((*self.args.cscan).scan.plan.targetlist) }
    }

    pub fn build(self) -> *mut CustomScanStateWrapper<CS> {
        unsafe {
            PgMemoryContexts::CurrentMemoryContext.leak_and_drop_on_delete(CustomScanStateWrapper {
                csstate: pg_sys::CustomScanState {
                    ss: pg_sys::ScanState {
                        ps: pg_sys::PlanState {
                            type_: pg_sys::NodeTag::T_CustomScanState,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    flags: (*self.args.cscan).flags,
                    custom_ps: std::ptr::null_mut(),
                    pscan_len: 0,
                    methods: CS::exec_methods(),
                    #[cfg(any(feature = "pg16", feature = "pg17"))]
                    slotOps: std::ptr::null_mut(),
                },
                custom_state: self.custom_state,
            })
        }
    }
}
