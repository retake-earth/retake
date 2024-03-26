use async_std::task;
use core::ffi::c_char;
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::datafusion::catalog::CatalogProvider;
use deltalake::datafusion::common::arrow::datatypes::{DataType, Field, Schema as ArrowSchema};
use deltalake::datafusion::sql::TableReference;
use pgrx::*;

use std::sync::Arc;

use crate::datafusion::directory::ParadeDirectory;
use crate::datafusion::schema::ParadeSchemaProvider;
use crate::datafusion::session::Session;
use crate::datafusion::table::{DatafusionTable, RESERVED_TID_FIELD};
use crate::errors::{NotSupported, ParadeError};
use crate::storage::metadata::PgMetadata;

#[pg_guard]
#[cfg(any(feature = "pg12", feature = "pg13", feature = "pg14", feature = "pg15"))]
pub extern "C" fn deltalake_relation_set_new_filenode(
    rel: pg_sys::Relation,
    newrnode: *const pg_sys::RelFileNode,
    persistence: c_char,
    freezeXid: *mut pg_sys::TransactionId,
    minmulti: *mut pg_sys::MultiXactId,
) {
    unsafe {
        let srel = pg_sys::RelationCreateStorage(*newrnode, persistence);
        rel.init_metadata(srel);

        *freezeXid = pg_sys::RecentXmin;
        *minmulti = pg_sys::GetOldestMultiXactId();
        pg_sys::smgrclose(srel);
    }

    task::block_on(create_deltalake_file_node(rel, persistence)).unwrap_or_else(|err| {
        panic!("{}", err);
    });
}

#[pg_guard]
#[cfg(feature = "pg16")]
pub extern "C" fn deltalake_relation_set_new_filelocator(
    rel: pg_sys::Relation,
    newrlocator: *const pg_sys::RelFileLocator,
    persistence: c_char,
    freezeXid: *mut pg_sys::TransactionId,
    minmulti: *mut pg_sys::MultiXactId,
) {
    unsafe {
        let srel = pg_sys::RelationCreateStorage(*newrlocator, persistence, true);
        rel.init_metadata(srel);

        *freezeXid = pg_sys::RecentXmin;
        *minmulti = pg_sys::GetOldestMultiXactId();
        pg_sys::smgrclose(srel);
    }

    task::block_on(create_deltalake_file_node(rel, persistence)).unwrap_or_else(|err| {
        panic!("{}", err);
    });
}

#[inline]
async fn create_deltalake_file_node(
    rel: pg_sys::Relation,
    persistence: c_char,
) -> Result<(), ParadeError> {
    let pg_relation = unsafe { PgRelation::from_pg(rel) };

    match persistence as u8 {
        pg_sys::RELPERSISTENCE_TEMP => Err(NotSupported::TempTable.into()),
        _ => {
            let table_name = pg_relation.name().to_string();
            let schema_name = pg_relation.namespace().to_string();
            let table_path = pg_relation.table_path()?;
            let catalog_name = Session::catalog_name()?;

            Session::with_catalog(|catalog| {
                if catalog.schema(&schema_name).is_none() {
                    let schema_provider =
                        Arc::new(task::block_on(ParadeSchemaProvider::try_new(&schema_name))?);

                    catalog.register_schema(&schema_name, schema_provider)?;
                }

                Ok(())
            })?;

            let schema_name = pg_relation.namespace().to_string();
            let table_exists = Session::with_session_context(|context| {
                Box::pin(async move {
                    let reference = TableReference::full(catalog_name, schema_name, table_name);
                    Ok(context.table_exist(reference)?)
                })
            })?;

            // If the table already exists, then this function is being called as part of another
            // operation like VACUUM FULL or TRUNCATE and we don't want to create any new files
            if table_exists {
                return Ok(());
            }

            ParadeDirectory::create_schema_path(
                Session::catalog_oid()?,
                pg_relation.namespace_oid(),
            )?;

            let schema_name = pg_relation.namespace().to_string();

            Session::with_tables(&schema_name, |mut tables| {
                Box::pin(async move {
                    let arrow_schema = Arc::new(ArrowSchema::try_merge(vec![
                        pg_relation.arrow_schema()?,
                        ArrowSchema::new(vec![Field::new(
                            RESERVED_TID_FIELD,
                            DataType::Int64,
                            false,
                        )]),
                    ])?);

                    tables.create(&table_path, arrow_schema.clone()).await?;
                    // Write an empty batch to the table so that a Parquet file is written
                    let batch = RecordBatch::new_empty(arrow_schema.clone());
                    let mut delta_table = tables.alter_schema(&table_path, batch).await?;

                    delta_table.update().await?;
                    tables.register(&table_path, delta_table)
                })
            })
        }
    }
}
