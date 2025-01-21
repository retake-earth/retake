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

// ---------------------------------------------------------------
// BM25 block storage layout
// A block storage representation of a Tantivy index
// ---------------------------------------------------------------

// ---------------------------------------------------------------
// Lock blocks
// We burn the first two blocks to use as locks for merge and
// vacuum cleanup
// ---------------------------------------------------------------

// +-------------------------------------------------------------+
// |                          Merge Lock                         |
// +-------------------------------------------------------------+
// | Empty                                                       |
// +-------------------------------------------------------------+

// +-------------------------------------------------------------+
// |                         Cleanup Lock                        |
// +-------------------------------------------------------------+
// | Empty                                                       |
// +-------------------------------------------------------------+

// ---------------------------------------------------------------
// Metadata blocks
// These blocks are created at index build
// Their block numbers should never change
// ---------------------------------------------------------------

// +-------------------------------------------------------------+
// |                          Schema Block                       |
// +-------------------------------------------------------------+
// | Serialized Tantivy Schema                                   |
// +-------------------------------------------------------------+
// | LP_SPECIAL                                                  |
// | [next_blockno: BlockNumber, xmax: TransactionId]            |
// +-------------------------------------------------------------+

// +-------------------------------------------------------------+
// |                        Settings Block                       |
// +-------------------------------------------------------------+
// | Serialized Tantivy IndexSettings                            |
// +-------------------------------------------------------------+
// | LP_SPECIAL                                                  |
// | [next_blockno: BlockNumber, xmax: TransactionId]            |
// +-------------------------------------------------------------+

// +-------------------------------------------------------------+
// |                 Segment Meta Entries Block                  |
// +-------------------------------------------------------------+
// | Serialized SegmentMetaEntry Items                           |
// | [SegmentMetaEntry]                                          |
// | [SegmentMetaEntry]                                          |
// | [SegmentMetaEntry]                                          |
// | ...                                                         |
// +-------------------------------------------------------------+
// | LP_SPECIAL                                                  |
// | [next_blockno: BlockNumber, xmax: TransactionId]            |
// +-------------------------------------------------------------+

// ---------------------------------------------------------------
// Remaining blocks: Segment component blocks
// These blocks are created when Tantivy writes new segments
// Can be picked up by vacuum
// ---------------------------------------------------------------

// +-------------------------------------------------------------+
// |                 Segment Component <uuid.ext>                |
// +-------------------------------------------------------------+
// | Serialized SegmentComponent data                            |
// +-------------------------------------------------------------+
// | LP_SPECIAL                                                  |
// | [next_blockno: BlockNumber, xmax: TransactionId]            |
// +-------------------------------------------------------------+

pub mod block;
mod blocklist;
pub mod buffer;
pub mod linked_bytes;
pub mod linked_items;
pub mod utils;

pub use self::linked_bytes::LinkedBytesList;
pub use self::linked_items::LinkedItemList;
