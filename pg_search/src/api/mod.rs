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

pub mod config;
pub mod index;
pub mod operator;
pub mod tokenize;

#[macro_export]
macro_rules! nodecast {
    ($type_:ident, $kind:ident, $node:expr) => {{
        let node = $node;
        pgrx::is_a(node.cast(), pgrx::pg_sys::NodeTag::$kind)
            .then(|| node.cast::<pgrx::pg_sys::$type_>())
    }};

    ($type_:ident, $kind:ident, $node:expr, true) => {{
        let node = $node;
        (node.is_null() || pgrx::is_a(node.cast(), pgrx::pg_sys::NodeTag::$kind))
            .then(|| node.cast::<pgrx::pg_sys::$type_>())
    }};
}

// came to life in pg15
pub type Cardinality = f64;

pub trait AsInt {
    unsafe fn as_int(&self) -> Option<i32>;
}

#[allow(dead_code)]
pub trait AsBool {
    unsafe fn as_bool(&self) -> Option<bool>;
}

pub trait AsCStr {
    unsafe fn as_c_str(&self) -> Option<&std::ffi::CStr>;
}

#[cfg(any(feature = "pg13", feature = "pg14"))]
impl AsInt for *mut pgrx::pg_sys::Node {
    unsafe fn as_int(&self) -> Option<i32> {
        let node = nodecast!(Value, T_Integer, *self)?;
        Some((*node).val.ival)
    }
}

#[cfg(not(any(feature = "pg13", feature = "pg14")))]
impl AsInt for *mut pgrx::pg_sys::Node {
    unsafe fn as_int(&self) -> Option<i32> {
        let node = nodecast!(Integer, T_Integer, *self)?;
        Some((*node).ival)
    }
}

#[cfg(any(feature = "pg13", feature = "pg14"))]
impl AsBool for *mut pgrx::pg_sys::Node {
    unsafe fn as_bool(&self) -> Option<bool> {
        let node = nodecast!(Value, T_Integer, *self)?;
        Some((*node).val.ival != 0)
    }
}

#[cfg(not(any(feature = "pg13", feature = "pg14")))]
impl AsBool for *mut pgrx::pg_sys::Node {
    unsafe fn as_bool(&self) -> Option<bool> {
        let node = nodecast!(Boolean, T_Boolean, *self)?;
        Some((*node).boolval)
    }
}

#[cfg(any(feature = "pg13", feature = "pg14"))]
impl AsCStr for *mut pgrx::pg_sys::Node {
    unsafe fn as_c_str(&self) -> Option<&std::ffi::CStr> {
        let node = nodecast!(Value, T_String, *self)?;
        Some(std::ffi::CStr::from_ptr((*node).val.str_))
    }
}

#[cfg(not(any(feature = "pg13", feature = "pg14")))]
impl AsCStr for *mut pgrx::pg_sys::Node {
    unsafe fn as_c_str(&self) -> Option<&std::ffi::CStr> {
        let node = nodecast!(String, T_String, *self)?;
        Some(std::ffi::CStr::from_ptr((*node).sval))
    }
}
