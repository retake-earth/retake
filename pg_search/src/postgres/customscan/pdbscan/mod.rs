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

mod qual_inspect;

use crate::api::operator::{anyelement_jsonb_opoid, estimate_selectivity};
use crate::globals::WriterGlobal;
use crate::index::state::SearchResults;
use crate::index::SearchIndex;
use crate::postgres::customscan::builders::custom_path::{CustomPathBuilder, Flags};
use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::builders::custom_state::{
    CustomScanStateBuilder, CustomScanStateWrapper,
};
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::pdbscan::qual_inspect::{extract_quals, Qual};
use crate::postgres::customscan::{node, CustomScan, CustomScanState};
use crate::postgres::options::SearchIndexCreateOptions;
use crate::postgres::rel_get_bm25_index;
use crate::postgres::utils::{relfilenode_from_pg_relation, VisibilityChecker};
use crate::schema::SearchConfig;
use crate::writer::WriterDirectory;
use crate::{DEFAULT_STARTUP_COST, GUCS, UNKNOWN_SELECTIVITY};
use pgrx::{is_a, name_data_to_str, pg_sys, IntoDatum, PgList, PgRelation, PgTupleDesc};
use shared::gucs::GlobalGucSettings;
use std::ffi::CStr;
use std::ptr::{addr_of, addr_of_mut};

#[derive(Default)]
pub struct PdbScan;

#[derive(Default)]
pub struct PdbScanState {
    snapshot: Option<pg_sys::Snapshot>,
    heaprel: Option<pg_sys::Relation>,
    index_name: String,
    index_oid: pg_sys::Oid,
    index_uuid: String,
    key_field: String,
    search_config: SearchConfig,
    search_results: SearchResults,

    visibility_checker: Option<VisibilityChecker>,
    score_field_indices: Vec<usize>,
}

impl CustomScanState for PdbScanState {}

impl PdbScanState {
    #[inline(always)]
    pub fn need_scores(&self) -> bool {
        !self.score_field_indices.is_empty()
    }

    #[inline(always)]
    pub fn snapshot(&self) -> pg_sys::Snapshot {
        self.snapshot.unwrap()
    }

    #[inline(always)]
    pub fn heaprel(&self) -> pg_sys::Relation {
        self.heaprel.unwrap()
    }

    #[inline(always)]
    pub fn heaprelid(&self) -> pg_sys::Oid {
        unsafe { (*self.heaprel()).rd_id }
    }

    #[inline(always)]
    pub fn heaprelname(&self) -> &str {
        unsafe { name_data_to_str(&(*(*self.heaprel()).rd_rel).relname) }
    }

    #[inline(always)]
    pub fn heaptupdesc(&self) -> pg_sys::TupleDesc {
        unsafe { (*self.heaprel()).rd_att }
    }

    #[inline(always)]
    pub fn visibility_checker(&mut self) -> &mut VisibilityChecker {
        self.visibility_checker.as_mut().unwrap()
    }
}

struct PrivateData(PgList<pg_sys::Node>);

impl PrivateData {
    fn heaprelid(&self) -> Option<pg_sys::Oid> {
        unsafe {
            Some(pg_sys::Oid::from(
                (*node::<pg_sys::Integer>(self.0.get_ptr(0)?.cast(), pg_sys::NodeTag::T_Integer)?)
                    .ival as u32,
            ))
        }
    }

    fn indexrelid(&self) -> Option<pg_sys::Oid> {
        unsafe {
            Some(pg_sys::Oid::from(
                (*node::<pg_sys::Integer>(self.0.get_ptr(1)?.cast(), pg_sys::NodeTag::T_Integer)?)
                    .ival as u32,
            ))
        }
    }

    fn quals(&self) -> Option<Qual> {
        let base_restrict_info = self.0.get_ptr(2)?;
        unsafe { extract_quals(base_restrict_info, anyelement_jsonb_opoid()) }
    }
}

impl CustomScan for PdbScan {
    const NAME: &'static CStr = c"ParadeDB Scan";
    type State = PdbScanState;

    fn callback(mut builder: CustomPathBuilder) -> Option<pg_sys::CustomPath> {
        if !GUCS.enable_custom_scan() {
            return None;
        }

        unsafe {
            if builder.base_restrict_info().is_empty() {
                return None;
            }
            let rte = builder.args().rte();

            // first, we only work on plain relations
            if rte.rtekind != pg_sys::RTEKind::RTE_RELATION {
                return None;
            }
            let relkind = pg_sys::get_rel_relkind(rte.relid) as u8;
            if relkind != pg_sys::RELKIND_RELATION && relkind != pg_sys::RELKIND_MATVIEW {
                return None;
            }

            // and that relation must have a `USING bm25` index
            let (table, bm25_index) = rel_get_bm25_index(rte.relid)?;

            // TODO:  need to see if we can detect that scores are necessary here.  Need to know
            //        up front because if so, we gotta be able to answer the query -- nobody else can
            //  hint:  probably look at `builder.path_target()` to figure this out

            //
            // look for quals we can support
            //
            if let Some(quals) = extract_quals(
                builder.base_restrict_info().as_ptr().cast(),
                anyelement_jsonb_opoid(),
            ) {
                builder = builder
                    .add_private_data(pg_sys::makeInteger(table.oid().as_u32() as _).cast())
                    .add_private_data(pg_sys::makeInteger(bm25_index.oid().as_u32() as _).cast());

                let restrict_info = builder.base_restrict_info();

                let selectivity = if restrict_info.len() == 1 {
                    // we can use the norm_selec that already happened
                    (*restrict_info.get_ptr(0).unwrap()).norm_selec
                } else {
                    // ask the index
                    let search_config = SearchConfig::from(quals);
                    estimate_selectivity(
                        table.oid(),
                        relfilenode_from_pg_relation(&bm25_index),
                        &search_config,
                    )
                    .unwrap_or(UNKNOWN_SELECTIVITY)
                };

                let reltuples = table.reltuples().unwrap_or(1.0) as f64;
                let rows = (reltuples * selectivity).max(1.0);
                let startup_cost = DEFAULT_STARTUP_COST;
                let total_cost =
                    startup_cost + selectivity * reltuples * (pg_sys::cpu_index_tuple_cost);

                let cpu_run_cost = {
                    #[allow(non_upper_case_globals)]
                    const per_tuple: f64 = 4.0; // TODO:  this is a curious value -- I picked it out of the air!

                    let cpu_run_cost = pg_sys::cpu_tuple_cost + per_tuple;
                    cpu_run_cost + rows * per_tuple
                };

                builder = builder.set_rows(rows);
                builder = builder.set_startup_cost(startup_cost);
                builder = builder.set_total_cost(total_cost + cpu_run_cost);

                builder = builder.add_private_data(restrict_info.into_pg().cast());
                builder = builder.set_flag(Flags::Projection);

                return Some(builder.build());
            }
        }

        None
    }

    fn plan_custom_path(builder: CustomScanBuilder) -> pgrx::pg_sys::CustomScan {
        builder.build()
    }

    fn create_custom_scan_state(
        mut builder: CustomScanStateBuilder<Self>,
    ) -> *mut CustomScanStateWrapper<Self> {
        unsafe {
            let private_data = PrivateData(builder.private_data());
            let heaprelid = private_data
                .heaprelid()
                .expect("heaprelid should have a value");
            let indexrelid = private_data
                .indexrelid()
                .expect("indexrelid should have a value");

            {
                let indexrel = PgRelation::with_lock(indexrelid, pg_sys::AccessShareLock as _);
                let ops = indexrel.rd_options as *mut SearchIndexCreateOptions;
                let uuid = (*ops)
                    .get_uuid()
                    .expect("`USING bm25` index should have a value `uuid` option");
                let key_field = (*ops)
                    .get_key_field()
                    .expect("`USING bm25` index should have a valued `key_field` option")
                    .0;

                builder.custom_state().index_oid = indexrel.oid();
                builder.custom_state().index_name = indexrel.name().to_string();
                builder.custom_state().index_uuid = uuid;
                builder.custom_state().key_field = key_field;
            }

            let heaprel = pg_sys::relation_open(heaprelid, pg_sys::AccessShareLock as _);
            let tupdesc = PgTupleDesc::from_pg_unchecked((*heaprel).rd_att);

            let quals = private_data.quals().expect("should have a Qual structure");

            builder.custom_state().search_config = SearchConfig::from(quals);
            builder.custom_state().heaprel = Some(heaprel);
            builder.custom_state().snapshot = Some(pg_sys::GetActiveSnapshot());
            builder.custom_state().visibility_checker = Some(VisibilityChecker::with_rel_and_snap(
                heaprel,
                pg_sys::GetActiveSnapshot(),
            ));

            let score_funcoid = score_support::score_funcoid();
            for (i, entry) in builder.target_list().iter_ptr().enumerate() {
                let entry = entry.as_ref().expect("`TargetEntry` should not be null");
                let expr = entry.expr;

                if is_a(expr.cast(), pg_sys::NodeTag::T_FuncExpr) {
                    let func: *mut pg_sys::FuncExpr = expr.cast();
                    if (*func).funcid == score_funcoid {
                        builder.custom_state().score_field_indices.push(i);
                    }
                }
            }

            builder.build()
        }
    }

    fn explain_custom_scan(
        state: &CustomScanStateWrapper<Self>,
        ancestors: *mut pgrx::pg_sys::List,
        explainer: &mut Explainer,
    ) {
        let tupdesc = unsafe { PgTupleDesc::from_pg_unchecked(state.projection_tupdesc()) };

        explainer.add_text("Table", state.custom_state.heaprelname());
        explainer.add_text("Index", &state.custom_state.index_name);

        let query = &state.custom_state.search_config.query;
        let pretty_json = if explainer.is_verbose() {
            serde_json::to_string_pretty(&query)
        } else {
            serde_json::to_string(&query)
        }
        .expect("query should serialize to json");
        explainer.add_text("Tantivy Query", &pretty_json);
    }

    fn begin_custom_scan(
        state: &mut CustomScanStateWrapper<Self>,
        estate: *mut pg_sys::EState,
        eflags: i32,
    ) {
        unsafe {
            let tupdesc = state.custom_state.heaptupdesc();

            pg_sys::ExecInitScanTupleSlot(
                estate,
                addr_of_mut!(state.csstate.ss),
                tupdesc,
                pg_sys::table_slot_callbacks(state.custom_state.heaprel()),
            );
            pg_sys::ExecInitResultTypeTL(addr_of_mut!(state.csstate.ss.ps));
            pg_sys::ExecAssignProjectionInfo(
                addr_of_mut!(state.csstate.ss.ps),
                (*state.csstate.ss.ss_ScanTupleSlot).tts_tupleDescriptor,
            );
        }

        PdbScan::rescan_custom_scan(state)
    }

    fn exec_custom_scan(state: &mut CustomScanStateWrapper<Self>) -> *mut pg_sys::TupleTableSlot {
        loop {
            match state.custom_state.search_results.next() {
                // we've returned all the matching results
                None => return std::ptr::null_mut(),

                // need to fetch the returned ctid from the heap and perform projection
                Some((scored, _)) => {
                    let scanslot = state.scanslot();
                    let bslot = state.scanslot() as *mut pg_sys::BufferHeapTupleTableSlot;
                    let heaprelid = state.custom_state.heaprelid();
                    let slot = state.custom_state.visibility_checker().exec_if_visible(
                        scored.ctid,
                        move |htup, buffer| unsafe {
                            #[allow(improper_ctypes)]
                            extern "C" {
                                fn ExecStoreBufferHeapTuple(
                                    tuple: pg_sys::HeapTuple,
                                    slot: *mut pg_sys::TupleTableSlot,
                                    buffer: pg_sys::Buffer,
                                ) -> *mut pg_sys::TupleTableSlot;
                            }

                            (*bslot).base.base.tts_tableOid = heaprelid;
                            (*bslot).base.tupdata = htup;
                            (*bslot).base.tupdata.t_self = (*htup.t_data).t_ctid;
                            ExecStoreBufferHeapTuple(
                                addr_of_mut!((*bslot).base.tupdata),
                                bslot.cast(),
                                buffer,
                            )
                        },
                    );

                    match slot {
                        // project the slot and return it
                        Some(slot) => unsafe {
                            let bslot = slot as *mut pg_sys::BufferHeapTupleTableSlot;
                            state.set_projection_scanslot(slot);
                            let slot = pg_sys::ExecProject(state.projection_info());

                            if state.custom_state.need_scores() {
                                let values = std::slice::from_raw_parts_mut(
                                    (*slot).tts_values,
                                    (*slot).tts_nvalid as usize,
                                );
                                let nulls = std::slice::from_raw_parts_mut(
                                    (*slot).tts_isnull,
                                    (*slot).tts_nvalid as usize,
                                );

                                for i in &state.custom_state.score_field_indices {
                                    let i = *i;

                                    if i < (*slot).tts_nvalid as usize {
                                        values[i] = scored.bm25.into_datum().unwrap();
                                        nulls[i] = false;
                                    }
                                }
                            }
                            return slot;
                        },

                        // ctid isn't visible, move to the next one
                        None => continue,
                    }
                }
            }
        }
    }

    fn shutdown_custom_scan(state: &mut CustomScanStateWrapper<Self>) {}

    fn end_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        // get the VisibilityChecker dropped
        state.custom_state.visibility_checker.take();

        if let Some(heaprel) = state.custom_state.heaprel.take() {
            unsafe {
                pg_sys::relation_close(heaprel, pg_sys::AccessShareLock as _);
            }
        }
    }

    fn rescan_custom_scan(state: &mut CustomScanStateWrapper<Self>) {
        let indexrelid = state.custom_state.index_oid.as_u32();
        let need_scores = state.custom_state.need_scores();
        let search_config = &mut state.custom_state.search_config;

        search_config.stable_sort = Some(false);
        search_config.need_scores = need_scores;

        // Create the index and scan state
        let directory = WriterDirectory::from_oids(
            crate::MyDatabaseId(),
            indexrelid,
            crate::postgres::utils::relfilenode_from_index_oid(indexrelid).as_u32(),
        );
        let search_index = SearchIndex::from_cache(&directory, &search_config.uuid)
            .unwrap_or_else(|err| panic!("error loading index from directory: {err}"));
        let writer_client = WriterGlobal::client();

        let search_state = search_index
            .search_state(&writer_client, search_config)
            .expect("`SearchState` should have been constructed correctly");

        state.custom_state.search_results =
            search_state.search_minimal(false, SearchIndex::executor());
    }
}

fn tts_ops_name(slot: *mut pg_sys::TupleTableSlot) -> &'static str {
    unsafe {
        if std::ptr::eq((*slot).tts_ops, addr_of!(pg_sys::TTSOpsVirtual)) {
            "Virtual"
        } else if std::ptr::eq((*slot).tts_ops, addr_of!(pg_sys::TTSOpsHeapTuple)) {
            "Heap"
        } else if std::ptr::eq((*slot).tts_ops, addr_of!(pg_sys::TTSOpsMinimalTuple)) {
            "Minimal"
        } else if std::ptr::eq((*slot).tts_ops, addr_of!(pg_sys::TTSOpsBufferHeapTuple)) {
            "BufferHeap"
        } else {
            "Unknown"
        }
    }
}

mod score_support {
    use pgrx::callconv::{Arg, ArgAbi};
    use pgrx::pgrx_sql_entity_graph::metadata::{
        ArgumentError, Returns, ReturnsError, SqlMapping, SqlTranslatable,
    };
    use pgrx::{direct_function_call, pg_extern, pg_sys, IntoDatum};

    pub struct OpaqueRecordArg;

    unsafe impl<'fcx> ArgAbi<'fcx> for OpaqueRecordArg {
        unsafe fn unbox_arg_unchecked(arg: Arg<'_, 'fcx>) -> Self {
            OpaqueRecordArg
        }
    }

    unsafe impl SqlTranslatable for OpaqueRecordArg {
        fn argument_sql() -> Result<SqlMapping, ArgumentError> {
            Ok(SqlMapping::As("record".into()))
        }

        fn return_sql() -> Result<Returns, ReturnsError> {
            Ok(Returns::One(SqlMapping::As("record".into())))
        }
    }

    #[pg_extern(name = "score", volatile, parallel_safe)]
    fn score_from_relation(_relation_reference: OpaqueRecordArg) -> f32 {
        f32::NAN
    }

    pub(super) fn score_funcoid() -> pg_sys::Oid {
        unsafe {
            direct_function_call::<pg_sys::Oid>(
                pg_sys::regprocedurein,
                &[c"paradedb.score(record)".into_datum()],
            )
            .expect("the `paradedb.score(record) type should exist")
        }
    }
}
