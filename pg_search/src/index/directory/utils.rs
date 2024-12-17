use crate::index::channel::NeedWal;
use crate::postgres::storage::block::{
    DeleteMetaEntry, DirectoryEntry, LinkedList, MVCCEntry, PgItem, SegmentMetaEntry,
    DELETE_METAS_START, DIRECTORY_START, SCHEMA_START, SEGMENT_METAS_START, SETTINGS_START,
};
use crate::postgres::storage::utils::{BM25Buffer, BM25BufferCache};
use crate::postgres::storage::{LinkedBytesList, LinkedItemList};
use anyhow::{anyhow, bail, Result};
use pgrx::pg_sys;
#[cfg(any(test, feature = "pg_test"))]
use pgrx::pg_test;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tantivy::{
    index::{DeleteMeta, IndexSettings, InnerSegmentMeta, SegmentId, SegmentMetaInventory},
    schema::Schema,
    Directory, IndexMeta, Opstamp,
};

// Converts a SegmentID + SegmentComponent into a PathBuf
pub struct SegmentComponentPath(pub PathBuf);
pub struct SegmentComponentId(pub SegmentId);

impl TryFrom<SegmentComponentPath> for SegmentComponentId {
    type Error = anyhow::Error;

    fn try_from(val: SegmentComponentPath) -> Result<Self, Self::Error> {
        let path_str = val
            .0
            .to_str()
            .ok_or_else(|| anyhow!("Invalid segment path: {:?}", val.0.to_str().unwrap()))?;
        if let Some(pos) = path_str.find('.') {
            Ok(SegmentComponentId(SegmentId::from_uuid_string(
                &path_str[..pos],
            )?))
        } else {
            bail!("Invalid segment path: {}", path_str);
        }
    }
}

pub trait DirectoryLookup {
    // Required methods
    fn relation_oid(&self) -> pg_sys::Oid;

    fn need_wal(&self) -> NeedWal;

    // Provided methods
    unsafe fn directory_lookup(
        &self,
        path: &Path,
    ) -> Result<(DirectoryEntry, pg_sys::BlockNumber, pg_sys::OffsetNumber)> {
        let directory = LinkedItemList::<DirectoryEntry>::open(
            self.relation_oid(),
            DIRECTORY_START,
            self.need_wal(),
        );
        let result = directory.lookup(path, |opaque, path| opaque.path == *path)?;
        Ok(result)
    }
}

pub trait BlockDirectory: Directory + DirectoryLookup {
    fn box_clone(&self) -> Box<dyn BlockDirectory>;
}

impl<T> BlockDirectory for T
where
    T: Directory + DirectoryLookup + Clone + 'static,
{
    fn box_clone(&self) -> Box<dyn BlockDirectory> {
        Box::new(self.clone())
    }
}

pub unsafe fn list_managed_files(relation_oid: pg_sys::Oid) -> tantivy::Result<HashSet<PathBuf>> {
    let cache = BM25BufferCache::open(relation_oid);
    let segment_components =
        LinkedItemList::<DirectoryEntry>::open(relation_oid, DIRECTORY_START, false);
    let mut blockno = segment_components.get_start_blockno();
    let mut files = HashSet::new();

    while blockno != pg_sys::InvalidBlockNumber {
        let buffer = cache.get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_SHARE));
        let page = pg_sys::BufferGetPage(buffer);
        let mut offsetno = pg_sys::FirstOffsetNumber;
        let max_offset = pg_sys::PageGetMaxOffsetNumber(page);

        while offsetno <= max_offset {
            let item_id = pg_sys::PageGetItemId(page, offsetno);
            let item = DirectoryEntry::from(PgItem(
                pg_sys::PageGetItem(page, item_id),
                (*item_id).lp_len() as pg_sys::Size,
            ));
            files.insert(item.path.clone());
            offsetno += 1;
        }

        blockno = buffer.next_blockno();
        pg_sys::UnlockReleaseBuffer(buffer);
    }

    Ok(files)
}

pub fn save_schema(
    relation_oid: pg_sys::Oid,
    tantivy_schema: &Schema,
    need_wal: NeedWal,
) -> Result<()> {
    let mut schema = LinkedBytesList::open(relation_oid, SCHEMA_START, need_wal);
    if schema.is_empty() {
        let bytes = serde_json::to_vec(tantivy_schema)?;
        unsafe { schema.write(&bytes)? };
    }
    Ok(())
}

pub fn save_settings(
    relation_oid: pg_sys::Oid,
    tantivy_settings: &IndexSettings,
    need_wal: NeedWal,
) -> Result<()> {
    let mut settings = LinkedBytesList::open(relation_oid, SETTINGS_START, need_wal);
    if settings.is_empty() {
        let bytes = serde_json::to_vec(tantivy_settings)?;
        unsafe { settings.write(&bytes)? };
    }
    Ok(())
}

pub fn get_deleted_ids(meta: &IndexMeta, previous_meta: &IndexMeta) -> HashSet<SegmentId> {
    let meta_ids = meta.segments.iter().map(|s| s.id()).collect::<HashSet<_>>();
    let empty_ids = meta
        .segments
        .iter()
        .filter(|s| s.num_docs() == 0)
        .map(|s| s.id())
        .collect::<HashSet<_>>();
    let merged_ids = previous_meta
        .segments
        .iter()
        .filter(|s| !meta_ids.contains(&s.id()))
        .map(|s| s.id())
        .collect::<HashSet<_>>();

    empty_ids.union(&merged_ids).cloned().collect()
}

pub unsafe fn save_delete_metas(
    relation_oid: pg_sys::Oid,
    meta: &IndexMeta,
    opstamp: Opstamp,
    need_wal: NeedWal,
) -> Result<()> {
    let mut delete_metas =
        LinkedItemList::<DeleteMetaEntry>::open(relation_oid, DELETE_METAS_START, need_wal);

    let new_entries = meta
        .segments
        .iter()
        .filter(|segment| {
            if let Some(delete_opstamp) = segment.delete_opstamp() {
                delete_opstamp == opstamp
            } else {
                false
            }
        })
        .map(|segment| DeleteMetaEntry {
            segment_id: segment.id(),
            num_deleted_docs: segment.num_deleted_docs(),
            opstamp: segment.delete_opstamp().expect("expected delete opstamp"),
            xmax: pg_sys::InvalidTransactionId,
        })
        .collect::<Vec<_>>();

    delete_metas.add_items(new_entries)
}

pub unsafe fn delete_unused_metas(
    relation_oid: pg_sys::Oid,
    deleted_ids: &HashSet<SegmentId>,
    xmax: pg_sys::TransactionId,
    need_wal: NeedWal,
) {
    let mut segment_metas =
        LinkedItemList::<SegmentMetaEntry>::open(relation_oid, SEGMENT_METAS_START, need_wal);
    let mut blockno = segment_metas.get_start_blockno();
    let bman = segment_metas.buffer_manager();
    unsafe {
        while blockno != pg_sys::InvalidBlockNumber {
            let mut buffer = bman.get_buffer_mut(blockno);
            let mut page = buffer.page_mut();
            let max_offset = page.max_offset_number();
            let mut offsetno = pg_sys::FirstOffsetNumber;

            while offsetno <= max_offset {
                let item_id = page.get_item_id(offsetno);
                let item = page.get_item(item_id);
                let entry = SegmentMetaEntry::from(PgItem(item, (*item_id).lp_len() as _));

                if deleted_ids.contains(&entry.segment_id) && !entry.deleted() {
                    let entry_with_xmax = SegmentMetaEntry {
                        xmax,
                        ..entry.clone()
                    };
                    let PgItem(item, size) = entry_with_xmax.clone().into();
                    let did_replace = page.replace_item(offsetno, item, size);
                    assert!(did_replace);
                }
                offsetno += 1;
            }

            blockno = buffer.next_blockno();
        }
    }
}

pub unsafe fn save_new_metas(
    relation_oid: pg_sys::Oid,
    meta: &IndexMeta,
    previous_meta: &IndexMeta,
    xmin: pg_sys::TransactionId,
    opstamp: Opstamp,
    need_wal: NeedWal,
) -> Result<()> {
    let previous_ids = previous_meta
        .segments
        .iter()
        .map(|s| s.id())
        .collect::<HashSet<_>>();
    let mut segment_metas =
        LinkedItemList::<SegmentMetaEntry>::open(relation_oid, SEGMENT_METAS_START, need_wal);

    let new_entries = meta
        .segments
        .iter()
        .filter(|s| !previous_ids.contains(&s.id()) && s.num_docs() > 0)
        .map(|s| SegmentMetaEntry {
            segment_id: s.id(),
            max_doc: s.max_doc(),
            opstamp,
            xmin,
            xmax: pg_sys::InvalidTransactionId,
        })
        .collect::<Vec<_>>();

    segment_metas.add_items(new_entries)
}

pub unsafe fn delete_unused_directory_entries(
    relation_oid: pg_sys::Oid,
    deleted_ids: &HashSet<SegmentId>,
    xmax: pg_sys::TransactionId,
    need_wal: NeedWal,
) {
    let mut directory =
        LinkedItemList::<DirectoryEntry>::open(relation_oid, DIRECTORY_START, need_wal);
    let mut blockno = directory.get_start_blockno();
    let bman = directory.buffer_manager();

    while blockno != pg_sys::InvalidBlockNumber {
        let mut buffer = bman.get_buffer_mut(blockno);
        let mut page = buffer.page_mut();
        let max_offset = page.max_offset_number();
        let mut offsetno = pg_sys::FirstOffsetNumber;

        while offsetno <= max_offset {
            let item_id = page.get_item_id(offsetno);
            let item = page.get_item(item_id);
            let entry = DirectoryEntry::from(PgItem(item, (*item_id).lp_len() as _));
            let SegmentComponentId(entry_segment_id) = SegmentComponentPath(entry.path.clone())
                .try_into()
                .unwrap_or_else(|_| panic!("{:?} should be valid", entry.path.clone()));

            if deleted_ids.contains(&entry_segment_id) && !entry.deleted() {
                let entry_with_xmax = DirectoryEntry {
                    xmax,
                    ..entry.clone()
                };
                let PgItem(item, size) = entry_with_xmax.clone().into();
                let did_replace = page.replace_item(offsetno, item, size);
                assert!(did_replace);

                // Delete the corresponding segment component
                let mut segment_component = LinkedBytesList::open(relation_oid, entry.start, true);
                segment_component.mark_deleted();
            }
            offsetno += 1;
        }

        blockno = buffer.next_blockno();
    }
}

pub unsafe fn delete_unused_delete_metas(
    relation_oid: pg_sys::Oid,
    deleted_ids: &HashSet<SegmentId>,
    xmax: pg_sys::TransactionId,
    need_wal: NeedWal,
) {
    let mut delete_metas =
        LinkedItemList::<DeleteMetaEntry>::open(relation_oid, DELETE_METAS_START, need_wal);
    let mut blockno = delete_metas.get_start_blockno();
    let bman = delete_metas.buffer_manager();

    while blockno != pg_sys::InvalidBlockNumber {
        let mut buffer = bman.get_buffer_mut(blockno);
        let mut page = buffer.page_mut();
        let max_offset = page.max_offset_number();
        let mut offsetno = pg_sys::FirstOffsetNumber;

        while offsetno <= max_offset {
            let item_id = page.get_item_id(offsetno);
            let item = page.get_item(item_id);
            let entry = DeleteMetaEntry::from(PgItem(item, (*item_id).lp_len() as _));

            if deleted_ids.contains(&entry.segment_id) && !entry.deleted() {
                let entry_with_xmax = DeleteMetaEntry {
                    xmax,
                    ..entry.clone()
                };
                let PgItem(item, size) = entry_with_xmax.clone().into();
                let did_replace = page.replace_item(offsetno, item, size);
                assert!(did_replace);
            }
            offsetno += 1;
        }

        blockno = buffer.next_blockno();
    }
}

pub unsafe fn load_metas(
    relation_oid: pg_sys::Oid,
    inventory: &SegmentMetaInventory,
    snapshot: pg_sys::Snapshot,
    solve_mvcc: bool,
) -> tantivy::Result<IndexMeta> {
    let cache = BM25BufferCache::open(relation_oid);
    let delete_metas =
        LinkedItemList::<DeleteMetaEntry>::open(relation_oid, DELETE_METAS_START, false);

    let mut delete_meta_entries = HashMap::new();
    let mut delete_meta_opstamps = HashMap::new();
    let mut blockno = delete_metas.get_start_blockno();

    while blockno != pg_sys::InvalidBlockNumber {
        let buffer = cache.get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_SHARE));
        let page = pg_sys::BufferGetPage(buffer);
        let mut offsetno = pg_sys::FirstOffsetNumber;
        let max_offset = pg_sys::PageGetMaxOffsetNumber(page);

        while offsetno <= max_offset {
            let item_id = pg_sys::PageGetItemId(page, offsetno);
            let entry = DeleteMetaEntry::from(PgItem(
                pg_sys::PageGetItem(page, item_id),
                (*item_id).lp_len() as pg_sys::Size,
            ));
            delete_meta_entries
                .entry(entry.segment_id)
                .and_modify(|existing: &mut DeleteMeta| {
                    if entry.opstamp > existing.opstamp {
                        *existing = DeleteMeta {
                            num_deleted_docs: entry.num_deleted_docs,
                            opstamp: entry.opstamp,
                        };
                    }
                })
                .or_insert(DeleteMeta {
                    num_deleted_docs: entry.num_deleted_docs,
                    opstamp: entry.opstamp,
                });
            delete_meta_opstamps
                .entry(entry.segment_id)
                .and_modify(|existing: &mut tantivy::Opstamp| {
                    if entry.opstamp > *existing {
                        *existing = entry.opstamp;
                    }
                })
                .or_insert(entry.opstamp);

            offsetno += 1;
        }

        blockno = buffer.next_blockno();
        pg_sys::UnlockReleaseBuffer(buffer);
    }

    let segment_metas =
        LinkedItemList::<SegmentMetaEntry>::open(relation_oid, SEGMENT_METAS_START, false);

    let heap_oid = unsafe { pg_sys::IndexGetRelation(relation_oid, false) };
    let heap_relation = unsafe { pg_sys::RelationIdGetRelation(heap_oid) };
    let mut alive_segments = vec![];
    let mut opstamp = 0;
    let mut blockno = segment_metas.get_start_blockno();

    while blockno != pg_sys::InvalidBlockNumber {
        let buffer = cache.get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_SHARE));
        let page = pg_sys::BufferGetPage(buffer);
        let mut offsetno = pg_sys::FirstOffsetNumber;
        let max_offset = pg_sys::PageGetMaxOffsetNumber(page);

        while offsetno <= max_offset {
            let item_id = pg_sys::PageGetItemId(page, offsetno);
            let entry = SegmentMetaEntry::from(PgItem(
                pg_sys::PageGetItem(page, item_id),
                (*item_id).lp_len() as pg_sys::Size,
            ));
            if entry.visible(snapshot)
                || (!solve_mvcc && !entry.recyclable(snapshot, heap_relation))
            {
                let deletes = delete_meta_entries.get(&entry.segment_id);
                let inner_segment_meta = InnerSegmentMeta {
                    max_doc: entry.max_doc,
                    segment_id: entry.segment_id,
                    deletes: deletes.cloned(),
                    include_temp_doc_store: Arc::new(AtomicBool::new(false)),
                };
                let segment_meta = inner_segment_meta.track(inventory);
                alive_segments.push(segment_meta);
                if entry.opstamp > opstamp {
                    opstamp = entry.opstamp;
                }
                if let Some(delete_opstamp) = delete_meta_opstamps.get(&entry.segment_id) {
                    if *delete_opstamp > opstamp {
                        opstamp = *delete_opstamp;
                    }
                }
            }
            offsetno += 1;
        }

        blockno = buffer.next_blockno();
        pg_sys::UnlockReleaseBuffer(buffer);
    }

    pg_sys::RelationClose(heap_relation);

    let schema = LinkedBytesList::open(relation_oid, SCHEMA_START, false);
    let settings = LinkedBytesList::open(relation_oid, SETTINGS_START, false);
    let deserialized_schema = serde_json::from_slice(&schema.read_all())?;
    let deserialized_settings = serde_json::from_slice(&settings.read_all())?;

    Ok(IndexMeta {
        segments: alive_segments,
        schema: deserialized_schema,
        index_settings: deserialized_settings,
        opstamp,
        payload: None,
    })
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use tantivy::index::SegmentId;

    #[pg_test]
    fn test_segment_component_path_to_id() {
        let path = SegmentComponentPath(PathBuf::from("00000000-0000-0000-0000-000000000000.ext"));
        let id = SegmentComponentId::try_from(path).unwrap();
        assert_eq!(
            id.0,
            SegmentId::from_uuid_string("00000000-0000-0000-0000-000000000000").unwrap()
        );

        let path = SegmentComponentPath(PathBuf::from(
            "00000000-0000-0000-0000-000000000000.123.del",
        ));
        let id = SegmentComponentId::try_from(path).unwrap();
        assert_eq!(
            id.0,
            SegmentId::from_uuid_string("00000000-0000-0000-0000-000000000000").unwrap()
        );
    }
}
