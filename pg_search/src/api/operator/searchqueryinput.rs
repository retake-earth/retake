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
use super::{
    anyelement_query_input_opoid, anyelement_query_input_procoid,
    make_search_query_input_opexpr_node,
};
use crate::api::operator::{estimate_selectivity, find_var_relation, ReturnedNodePointer};
use crate::gucs::per_tuple_cost;
use crate::index::fast_fields_helper::FFHelper;
use crate::index::reader::index::SearchIndexReader;
use crate::index::BlockDirectoryType;
use crate::postgres::types::TantivyValue;
use crate::postgres::utils::locate_bm25_index;
use crate::query::SearchQueryInput;
use crate::{nodecast, UNKNOWN_SELECTIVITY};
use pgrx::{
    check_for_interrupts, pg_extern, pg_func_extra, pg_sys, AnyElement, FromDatum, Internal,
    PgList, PgOid, PgRelation,
};
use rustc_hash::FxHashSet;
use std::ptr::NonNull;

#[pg_extern(immutable, parallel_safe, cost = 1000000000)]
pub fn search_with_query_input(
    element: AnyElement,
    query: SearchQueryInput,
    fcinfo: pg_sys::FunctionCallInfo,
) -> bool {
    let default_hash_set = || {
        let index_oid = {
            // We don't have access to the index oid here, so we don't know what index to use.
            // That means we're going to need to rely on the query being correctly wrapped
            // with the WithIndex when it is rewritten with our custom operator.
            match query {
                SearchQueryInput::WithIndex { oid, .. } => oid,
                _ => panic!("the SearchQueryInput must be wrapped in a WithIndex variant"),
            }
        };
        let index_relation = unsafe {
            PgRelation::with_lock(index_oid, pg_sys::AccessShareLock as pg_sys::LOCKMODE)
        };
        let search_reader =
            SearchIndexReader::open(&index_relation, BlockDirectoryType::Mvcc, false)
                .expect("search_with_query_input: should be able to open a SearchIndexReader");

        let key_field = search_reader.key_field();
        let key_field_name = key_field.name.0;
        let key_field_type = key_field.type_.into();
        let fast_fields = FFHelper::with_fields(
            &search_reader,
            &[(key_field_name.clone(), key_field_type).into()],
        );
        let top_docs = search_reader.search(query.contains_more_like_this(), false, &query, None);
        let mut hs = FxHashSet::default();
        for (_, doc_address) in top_docs {
            check_for_interrupts!();
            hs.insert(
                fast_fields
                    .value(0, doc_address)
                    .expect("key_field value should not be null"),
            );
        }

        (key_field_name, hs)
    };

    let cached = unsafe { pg_func_extra(fcinfo, default_hash_set) };
    let key_field = &cached.0;
    let hash_set = &cached.1;

    let key_field_value = match unsafe {
        TantivyValue::try_from_datum(element.datum(), PgOid::from_untagged(element.oid()))
    } {
        Err(err) => panic!(
            "no value present in key_field {} in tuple: {err}",
            key_field
        ),
        Ok(value) => value,
    };

    hash_set.contains(&key_field_value)
}

#[pg_extern(immutable, parallel_safe)]
pub unsafe fn query_input_support(arg: Internal) -> ReturnedNodePointer {
    let datum = match arg.unwrap() {
        Some(d) => d,
        None => return ReturnedNodePointer(None),
    };

    if let Some(node) = query_input_support_request_simplify(datum) {
        return node;
    }

    if let Some(node) = search_query_input_request_cost(datum) {
        return node;
    }

    ReturnedNodePointer(None)
}

fn query_input_support_request_simplify(arg: pg_sys::Datum) -> Option<ReturnedNodePointer> {
    unsafe {
        let srs = nodecast!(
            SupportRequestSimplify,
            T_SupportRequestSimplify,
            arg.cast_mut_ptr::<pg_sys::Node>()
        )?;

        // Rewrite this node touse the @@@(key_field, paradedb.searchqueryinput) operator.
        // This involves converting the rhs of the operator into a SearchQueryInput.
        let mut input_args = PgList::<pg_sys::Node>::from_pg((*(*srs).fcall).args);
        let var = nodecast!(Var, T_Var, input_args.get_ptr(0)?)?;

        // NB:  there was a point where we only allowed a relation reference on the left of @@@
        // when the right side uses a builder function, but we've decided that also allowing a field
        // name is materially better as the relation reference might be an artificial ROW(...) whereas
        // a field will be a legitimate Var from which we can derive the physical table.
        // if (*var).varattno != 0 {
        //     panic!("the left side of the `@@@` operator must be a relation reference when the right side uses a builder function");
        // }

        let rhs = input_args.get_ptr(1)?;
        let query = nodecast!(Const, T_Const, rhs)
            .map(|const_| SearchQueryInput::from_datum((*const_).constvalue, (*const_).constisnull))
            .flatten();

        Some(make_search_query_input_opexpr_node(
            srs,
            &mut input_args,
            var,
            query,
            None,
            anyelement_query_input_opoid(),
            anyelement_query_input_procoid(),
        ))
    }
}

#[pg_extern(immutable, parallel_safe)]
pub fn query_input_restrict(
    planner_info: Internal, // <pg_sys::PlannerInfo>,
    operator_oid: pg_sys::Oid,
    args: Internal, // <pg_sys::List>,
    _var_relid: i32,
) -> f64 {
    fn inner_query_input(
        planner_info: Internal, // <pg_sys::PlannerInfo>,
        args: Internal,         // <pg_sys::List>,
    ) -> Option<f64> {
        unsafe {
            let info = planner_info.unwrap()?.cast_mut_ptr::<pg_sys::PlannerInfo>();
            let args =
                PgList::<pg_sys::Node>::from_pg(args.unwrap()?.cast_mut_ptr::<pg_sys::List>());

            let var = nodecast!(Var, T_Var, args.get_ptr(0)?)?;
            let const_ = nodecast!(Const, T_Const, args.get_ptr(1)?)?;

            let (heaprelid, _, _) = find_var_relation(var, info);
            let indexrel = locate_bm25_index(heaprelid)?;

            // In case a sequential scan gets triggered, we need a way to pass the index oid
            // to the scan function. It otherwise will not know which index to use.
            let search_query_input = SearchQueryInput::WithIndex {
                oid: indexrel.oid(),
                query: Box::new(SearchQueryInput::from_datum(
                    (*const_).constvalue,
                    (*const_).constisnull,
                )?),
            };

            estimate_selectivity(&indexrel, &search_query_input)
        }
    }

    assert!(operator_oid == anyelement_query_input_opoid());

    let mut selectivity = inner_query_input(planner_info, args).unwrap_or(UNKNOWN_SELECTIVITY);
    if selectivity > 1.0 {
        selectivity = UNKNOWN_SELECTIVITY;
    }

    selectivity
}

fn search_query_input_request_cost(arg: pg_sys::Datum) -> Option<ReturnedNodePointer> {
    unsafe {
        let src = nodecast!(
            SupportRequestCost,
            T_SupportRequestCost,
            arg.cast_mut_ptr::<pg_sys::Node>()
        )?;
        // our `search_with_*` functions are *incredibly* expensive.  So much so that
        // we really don't ever want Postgres to prefer them.
        //
        // The higher the `per_tuple` cost is here, the better.
        //
        // it can cost a lot to startup the `@@@` operator outside of an IndexScan because
        // ultimately we have to hash all the resulting ctids in memory.  For lack of a better
        // value, we say it costs as much as the `GUCS.per_tuple_cost()`.  This is an arbitrary
        // number that we've documented as needing to be big.
        (*src).startup = per_tuple_cost();

        // similarly, use the same GUC here.  Postgres will then add this into its per-tuple
        // cost evaluations for whatever scan it's considering using for the `@@@` operator.
        // our IAM provides more intelligent costs for the IndexScan situation.
        (*src).per_tuple = per_tuple_cost();

        Some(ReturnedNodePointer(NonNull::new(src.cast())))
    }
}
