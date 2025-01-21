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

use crate::api::Cardinality;
use crate::postgres::customscan::builders::custom_path::OrderByStyle;
use crate::postgres::customscan::builders::custom_path::SortDirection;
use crate::postgres::customscan::pdbscan::qual_inspect::Qual;
use pgrx::{pg_sys, PgList};

#[derive(Default, Debug)]
pub struct PrivateData {
    heaprelid: Option<pg_sys::Oid>,
    indexrelid: Option<pg_sys::Oid>,
    range_table_index: Option<pg_sys::Index>,
    quals: Option<*mut pg_sys::List>,
    limit: Option<usize>,
    sort_field: Option<String>,
    sort_direction: Option<SortDirection>,
    var_attname_lookup: Option<*mut pg_sys::List>,
    maybe_ff: bool,
    segment_count: usize,
}

impl From<*mut pg_sys::List> for PrivateData {
    fn from(list: *mut pg_sys::List) -> Self {
        unsafe { deserialize::deserialize(list) }
    }
}

impl From<PrivateData> for *mut pg_sys::List {
    fn from(value: PrivateData) -> Self {
        unsafe { serialize::serialize(value).into_pg() }
    }
}

//
// setter functions
//

impl PrivateData {
    pub fn set_heaprelid(&mut self, oid: pg_sys::Oid) {
        self.heaprelid = Some(oid);
    }

    pub fn set_indexrelid(&mut self, oid: pg_sys::Oid) {
        self.indexrelid = Some(oid);
    }

    pub fn set_range_table_index(&mut self, rti: pg_sys::Index) {
        self.range_table_index = Some(rti);
    }

    pub fn set_quals(&mut self, quals: Qual) {
        let serialized: PgList<pg_sys::Node> = quals.into();
        self.quals = Some(serialized.into_pg().cast())
    }

    pub fn set_limit(&mut self, limit: Option<Cardinality>) {
        self.limit = limit.map(|l| l.round() as usize);
    }

    pub fn set_sort_direction(&mut self, sort_direction: Option<SortDirection>) {
        self.sort_direction = sort_direction;
    }

    pub fn set_sort_info(&mut self, pathkey: &Option<OrderByStyle>) {
        if let Some(style) = pathkey {
            match style {
                OrderByStyle::Score(_) => {}
                OrderByStyle::Field(_, name) => self.sort_field = Some(name.clone()),
            }
            self.sort_direction = Some(style.direction())
        }
    }

    pub fn set_var_attname_lookup(&mut self, var_attname_lookup: *mut pg_sys::List) {
        self.var_attname_lookup = Some(var_attname_lookup);
    }

    pub fn set_maybe_ff(&mut self, maybe: bool) {
        self.maybe_ff = maybe;
    }

    pub fn set_segment_count(&mut self, segment_count: usize) {
        self.segment_count = segment_count;
    }
}

//
// getter functions
//

impl PrivateData {
    pub fn heaprelid(&self) -> Option<pg_sys::Oid> {
        self.heaprelid
    }

    pub fn indexrelid(&self) -> Option<pg_sys::Oid> {
        self.indexrelid
    }

    pub fn range_table_index(&self) -> Option<pg_sys::Index> {
        self.range_table_index
    }

    pub fn quals(&self) -> Option<Qual> {
        self.quals
            .map(|ri| unsafe { Qual::from(PgList::<pg_sys::Node>::from_pg(ri)) })
    }

    pub fn limit(&self) -> Option<usize> {
        self.limit
    }

    pub fn sort_field(&self) -> Option<String> {
        self.sort_field.clone()
    }

    pub fn sort_direction(&self) -> Option<SortDirection> {
        self.sort_direction
    }

    pub fn var_attname_lookup(&self) -> Option<PgList<pg_sys::Node>> {
        self.var_attname_lookup
            .map(|list| unsafe { PgList::from_pg(list) })
    }

    pub fn maybe_ff(&self) -> bool {
        self.maybe_ff
    }

    pub fn segment_count(&self) -> usize {
        self.segment_count
    }
}

#[allow(non_snake_case)]
pub mod serialize {
    use crate::api::{AsCStr, AsInt};
    use crate::postgres::customscan::builders::custom_path::SortDirection;
    use crate::postgres::customscan::pdbscan::privdat::PrivateData;
    use pgrx::pg_sys::{AsPgCStr, Node};
    use pgrx::{pg_sys, PgList};
    use std::fmt::Display;
    use std::str::FromStr;

    pub trait AsValueNode: Sized {
        fn as_value_node(&self) -> *mut pg_sys::Node;

        fn from_value_node(node: *mut pg_sys::Node) -> Option<Self>;
    }

    impl AsValueNode for i32 {
        fn as_value_node(&self) -> *mut Node {
            unsafe { pg_sys::makeInteger(*self).cast() }
        }

        fn from_value_node(node: *mut Node) -> Option<Self> {
            unsafe { node.as_int() }
        }
    }

    impl AsValueNode for u32 {
        fn as_value_node(&self) -> *mut Node {
            unsafe { makeString(Some(&format!("{self}"))) }
        }

        fn from_value_node(node: *mut Node) -> Option<Self> {
            unsafe { Self::from_str(node.as_c_str()?.to_str().ok()?).ok() }
        }
    }

    impl AsValueNode for usize {
        fn as_value_node(&self) -> *mut Node {
            unsafe { makeString(Some(&format!("{self}"))) }
        }

        fn from_value_node(node: *mut Node) -> Option<Self> {
            unsafe { Self::from_str(node.as_c_str()?.to_str().ok()?).ok() }
        }
    }

    impl AsValueNode for pg_sys::Oid {
        fn as_value_node(&self) -> *mut Node {
            unsafe { makeString(Some(&format!("{}", self.as_u32()))) }
        }
        fn from_value_node(node: *mut Node) -> Option<Self> {
            let as_u32 = unsafe { u32::from_str(node.as_c_str()?.to_str().ok()?).ok() }?;
            Some(pg_sys::Oid::from(as_u32))
        }
    }

    impl AsValueNode for SortDirection {
        fn as_value_node(&self) -> *mut Node {
            unsafe { makeInteger(Some(*self as i32)) }
        }

        fn from_value_node(node: *mut Node) -> Option<Self> {
            unsafe {
                let integer = node.as_int()? as i32;
                if integer == SortDirection::Asc as i32 {
                    Some(Self::Asc)
                } else if integer == SortDirection::Desc as i32 {
                    Some(Self::Desc)
                } else if integer == SortDirection::None as i32 {
                    Some(Self::None)
                } else {
                    None
                }
            }
        }
    }

    pub unsafe fn makeInteger<T: AsValueNode>(input: Option<T>) -> *mut pg_sys::Node {
        unwrapOrNull(input.map(|i| i.as_value_node()))
    }

    pub unsafe fn makeString<T: Display>(input: Option<T>) -> *mut pg_sys::Node {
        unwrapOrNull(
            input.map(|s| pg_sys::makeString(s.to_string().as_pg_cstr()).cast::<pg_sys::Node>()),
        )
    }

    #[allow(dead_code)]
    pub unsafe fn makeBoolean<T: Into<bool>>(input: Option<T>) -> *mut pg_sys::Node {
        #[cfg(any(feature = "pg13", feature = "pg14"))]
        {
            unwrapOrNull(
                input.map(|b| {
                    pg_sys::makeInteger(if b.into() { 1 } else { 0 }).cast::<pg_sys::Node>()
                }),
            )
        }

        #[cfg(not(any(feature = "pg13", feature = "pg14")))]
        {
            unwrapOrNull(input.map(|b| pg_sys::makeBoolean(b.into()).cast::<pg_sys::Node>()))
        }
    }

    unsafe fn unwrapOrNull(node: Option<*mut pg_sys::Node>) -> *mut pg_sys::Node {
        node.unwrap_or_else(|| {
            pg_sys::makeNullConst(pg_sys::OIDOID, -1, pg_sys::Oid::INVALID).cast::<pg_sys::Node>()
        })
    }

    pub unsafe fn serialize(privdat: PrivateData) -> PgList<pg_sys::Node> {
        let mut ser = PgList::new();

        ser.push(makeInteger(privdat.heaprelid));
        ser.push(makeInteger(privdat.indexrelid));
        ser.push(makeInteger(privdat.range_table_index));
        ser.push(unwrapOrNull(privdat.quals.map(|l| l.cast())));
        ser.push(makeString(privdat.limit));
        ser.push(makeString(privdat.sort_field));
        ser.push(makeInteger(privdat.sort_direction));
        ser.push(unwrapOrNull(
            privdat.var_attname_lookup.map(|v| v.cast::<pg_sys::Node>()),
        ));
        ser.push(makeBoolean(Some(privdat.maybe_ff)));
        ser.push(makeString(Some(privdat.segment_count)));
        ser
    }
}

#[allow(non_snake_case)]
pub mod deserialize {
    use crate::api::{AsBool, AsCStr};
    use crate::nodecast;
    use crate::postgres::customscan::pdbscan::privdat::serialize::AsValueNode;
    use crate::postgres::customscan::pdbscan::privdat::PrivateData;
    use pgrx::{pg_sys, PgList};
    use std::str::FromStr;

    pub unsafe fn decodeInteger<T: AsValueNode>(node: *mut pg_sys::Node) -> Option<T> {
        T::from_value_node(node)
    }

    pub unsafe fn decodeString<T: FromStr>(node: *mut pg_sys::Node) -> Option<T> {
        node.as_c_str().map(|i| {
            let s = i.to_str().expect("string node should be valid utf8");
            T::from_str(s)
                .ok()
                .expect("value should parse from a String")
        })
    }

    #[allow(dead_code)]
    pub unsafe fn decodeBoolean<T: From<bool>>(node: *mut pg_sys::Node) -> Option<T> {
        node.as_bool().map(|b| b.into())
    }

    pub unsafe fn deserialize(input: *mut pg_sys::List) -> PrivateData {
        let input = PgList::<pg_sys::Node>::from_pg(input);
        PrivateData {
            heaprelid: input.get_ptr(0).and_then(|n| decodeInteger(n)),
            indexrelid: input.get_ptr(1).and_then(|n| decodeInteger(n)),
            range_table_index: input.get_ptr(2).and_then(|n| decodeInteger(n)),
            quals: input.get_ptr(3).and_then(|n| nodecast!(List, T_List, n)),
            limit: input.get_ptr(4).and_then(|n| decodeString(n)),
            sort_field: input.get_ptr(5).and_then(|n| decodeString(n)),
            sort_direction: input.get_ptr(6).and_then(|n| decodeInteger(n)),
            var_attname_lookup: input
                .get_ptr(7)
                .and_then(|n| nodecast!(List, T_List, n, true)),
            maybe_ff: input
                .get_ptr(8)
                .and_then(|n| decodeBoolean(n))
                .unwrap_or_default(),
            segment_count: input.get_ptr(9).and_then(|n| decodeString(n)).unwrap_or(0),
        }
    }
}
