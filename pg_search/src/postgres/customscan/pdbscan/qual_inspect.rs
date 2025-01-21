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

use crate::nodecast;
use crate::postgres::customscan::pdbscan::privdat::deserialize::decodeString;
use crate::postgres::customscan::pdbscan::privdat::serialize::{
    makeInteger, makeString, AsValueNode,
};
use crate::query::SearchQueryInput;
use pgrx::{node_to_string, pg_sys, FromDatum, PgList};

#[derive(Debug, Clone)]
pub enum Qual {
    Ignore,
    OperatorExpression {
        var: *mut pg_sys::Var,
        opno: pg_sys::Oid,
        val: *mut pg_sys::Const,
    },
    And(Vec<Qual>),
    Or(Vec<Qual>),
    Not(Box<Qual>),
}

impl From<&Qual> for SearchQueryInput {
    fn from(value: &Qual) -> Self {
        match value {
            Qual::Ignore => SearchQueryInput::All,
            Qual::OperatorExpression { val, .. } => unsafe {
                SearchQueryInput::from_datum((**val).constvalue, (**val).constisnull)
                    .expect("rhs of @@@ operator Qual must not be null")
            },

            Qual::And(quals) => {
                let must = quals.iter().map(SearchQueryInput::from).collect::<Vec<_>>();

                match must.len() {
                    0 => panic!("Qual::And should have at least one item"),
                    1 => must.into_iter().next().unwrap(),
                    _ => SearchQueryInput::Boolean {
                        must,
                        should: Default::default(),
                        must_not: Default::default(),
                    },
                }
            }
            Qual::Or(quals) => {
                let should = quals.iter().map(SearchQueryInput::from).collect::<Vec<_>>();

                match should.len() {
                    0 => panic!("Qual::Or should have at least one item"),
                    1 => should.into_iter().next().unwrap(),
                    _ => SearchQueryInput::Boolean {
                        must: Default::default(),
                        should,
                        must_not: Default::default(),
                    },
                }
            }
            Qual::Not(qual) => {
                let must_not = vec![SearchQueryInput::from(qual.as_ref())];

                SearchQueryInput::Boolean {
                    must: Default::default(),
                    should: Default::default(),
                    must_not,
                }
            }
        }
    }
}

impl From<Qual> for PgList<pg_sys::Node> {
    fn from(value: Qual) -> Self {
        unsafe {
            let mut list = PgList::new();

            match value {
                Qual::Ignore => list.push(makeString(Some("IGNORE"))),
                Qual::OperatorExpression { var, opno, val } => {
                    list.push(makeString(Some("OPERATOR_EXPRESSION")));
                    list.push(var.cast());
                    list.push(makeInteger(Some(opno)));
                    list.push(val.cast());
                }
                Qual::And(quals) => {
                    list.push(makeString(Some("AND")));
                    list.push(makeInteger(Some(quals.len())));
                    for qual in quals {
                        let or: PgList<pg_sys::Node> = qual.into();
                        list.push(or.into_pg().cast());
                    }
                }
                Qual::Or(quals) => {
                    list.push(makeString(Some("OR")));
                    list.push(makeInteger(Some(quals.len())));
                    for qual in quals {
                        let and: PgList<pg_sys::Node> = qual.into();
                        list.push(and.into_pg().cast());
                    }
                }
                Qual::Not(not) => {
                    list.push(makeString(Some("NOT")));
                    let not: PgList<pg_sys::Node> = (*not).into();
                    list.push(not.into_pg().cast());
                }
            }

            list
        }
    }
}

impl From<PgList<pg_sys::Node>> for Qual {
    fn from(value: PgList<pg_sys::Node>) -> Self {
        fn inner(value: PgList<pg_sys::Node>) -> Option<Qual> {
            unsafe {
                let first = value.get_ptr(0)?;

                if let Some(type_) = decodeString::<String>(first) {
                    match type_.as_str() {
                        "IGNORE" => Some(Qual::Ignore),
                        "OPERATOR_EXPRESSION" => {
                            let (var, opno, val) = (
                                nodecast!(Var, T_Var, value.get_ptr(1)?)?,
                                pg_sys::Oid::from_value_node(value.get_ptr(2)?)?,
                                nodecast!(Const, T_Const, value.get_ptr(3)?)?,
                            );
                            Some(Qual::OperatorExpression { var, opno, val })
                        }
                        "AND" => {
                            let len = usize::from_value_node(value.get_ptr(1)?)?;
                            let mut quals = Vec::with_capacity(len);
                            for i in 2..value.len() {
                                let qual_list = PgList::<pg_sys::Node>::from_pg(nodecast!(
                                    List,
                                    T_List,
                                    value.get_ptr(i)?
                                )?);
                                quals.push(qual_list.into());
                            }
                            Some(Qual::And(quals))
                        }
                        "OR" => {
                            let len = usize::from_value_node(value.get_ptr(1)?)?;
                            let mut quals = Vec::with_capacity(len);
                            for i in 2..value.len() {
                                let qual_list = PgList::<pg_sys::Node>::from_pg(nodecast!(
                                    List,
                                    T_List,
                                    value.get_ptr(i)?
                                )?);
                                quals.push(qual_list.into());
                            }
                            Some(Qual::Or(quals))
                        }
                        "NOT" => {
                            let not_qual = PgList::<pg_sys::Node>::from_pg(nodecast!(
                                List,
                                T_List,
                                value.get_ptr(1)?
                            )?);
                            Some(Qual::Not(Box::new(not_qual.into())))
                        }
                        other => panic!("unexpected Qual list node: {other}"),
                    }
                } else {
                    panic!("malformed Qual list")
                }
            }
        }

        inner(value).expect("Qual list should not be empty")
    }
}

pub unsafe fn extract_quals(
    rti: pg_sys::Index,
    node: *mut pg_sys::Node,
    pdbopoid: pg_sys::Oid,
) -> Option<Qual> {
    match (*node).type_ {
        pg_sys::NodeTag::T_List => {
            let mut quals = list(rti, node.cast(), pdbopoid)?;
            if quals.len() == 1 {
                quals.pop()
            } else {
                Some(Qual::And(quals))
            }
        }

        pg_sys::NodeTag::T_RestrictInfo => {
            let ri = nodecast!(RestrictInfo, T_RestrictInfo, node)?;
            // if (*ri).num_base_rels > 1 {
            //     return None;
            // }
            let clause = if !(*ri).orclause.is_null() {
                (*ri).orclause
            } else {
                (*ri).clause
            };
            extract_quals(rti, clause.cast(), pdbopoid)
        }

        pg_sys::NodeTag::T_OpExpr => opexpr(rti, node, pdbopoid),

        pg_sys::NodeTag::T_BoolExpr => {
            let boolexpr = nodecast!(BoolExpr, T_BoolExpr, node)?;
            let args = PgList::<pg_sys::Node>::from_pg((*boolexpr).args);
            let mut quals = list(rti, (*boolexpr).args, pdbopoid)?;

            match (*boolexpr).boolop {
                pg_sys::BoolExprType::AND_EXPR => Some(Qual::And(quals)),
                pg_sys::BoolExprType::OR_EXPR => Some(Qual::Or(quals)),
                pg_sys::BoolExprType::NOT_EXPR => Some(Qual::Not(Box::new(quals.pop()?))),
                _ => panic!("unexpected `BoolExprType`: {}", (*boolexpr).boolop),
            }
        }

        // we don't understand this clause so we can't do anything
        _ => {
            // pgrx::warning!("unsupported qual node kind: {:?}", (*node).type_);
            None
        }
    }
}

unsafe fn list(
    rti: pg_sys::Index,
    list: *mut pg_sys::List,
    pdbopoid: pg_sys::Oid,
) -> Option<Vec<Qual>> {
    let args = PgList::<pg_sys::Node>::from_pg(list);
    let mut quals = Vec::new();
    for child in args.iter_ptr() {
        quals.push(extract_quals(rti, child, pdbopoid)?)
    }
    Some(quals)
}

unsafe fn opexpr(
    rti: pg_sys::Index,
    node: *mut pg_sys::Node,
    pdbopoid: pg_sys::Oid,
) -> Option<Qual> {
    let opexpr = nodecast!(OpExpr, T_OpExpr, node)?;
    let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);
    let (lhs, rhs) = (
        nodecast!(Var, T_Var, args.get_ptr(0)?),
        nodecast!(Const, T_Const, args.get_ptr(1)?),
    );

    if lhs.is_none() || rhs.is_none() {
        pgrx::debug1!(
            "unrecognized `OpExpr`: {}",
            node_to_string(opexpr.cast()).expect("node_to_string should not return null")
        );
        return None;
    }
    let (lhs, rhs) = (lhs?, rhs?);

    if (*opexpr).opno == pdbopoid {
        if (*lhs).varno as i32 != rti as i32 {
            Some(Qual::Ignore)
        } else {
            Some(Qual::OperatorExpression {
                var: lhs,
                opno: (*opexpr).opno,
                val: rhs,
            })
        }
    } else {
        None
    }
}
