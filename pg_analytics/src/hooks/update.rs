use async_std::task;
use deltalake::datafusion::logical_expr::LogicalPlan;
use pgrx::*;

use crate::datafusion::context::DatafusionContext;
use crate::errors::ParadeError;

pub fn update(
    rtable: *mut pg_sys::List,
    query_desc: PgBox<pg_sys::QueryDesc>,
    logical_plan: LogicalPlan,
) -> Result<(), ParadeError> {
    let elements = unsafe { (*rtable).elements };
    let rte = unsafe { (*elements.offset(0)).ptr_value as *mut pg_sys::RangeTblEntry };
    let relation = unsafe { pg_sys::RelationIdGetRelation((*rte).relid) };
    let pg_relation = unsafe { PgRelation::from_pg_owned(relation) };
    let table_name = pg_relation.name();
    let schema_name = pg_relation.namespace();

    let optimized_plan = DatafusionContext::with_session_context(|context| {
        Ok(context.state().optimize(&logical_plan)?)
    })?;

    if let LogicalPlan::Dml(dml_statement) = optimized_plan {
        info!("delete_metrics: {:?}", dml_statement.input.as_ref());
    } else {
        unreachable!()
    };

    // let delete_metrics = if let LogicalPlan::Dml(dml_statement) = optimized_plan {
    //     DatafusionContext::with_schema_provider(schema_name, |provider| {
    //         if let LogicalPlan::Filter(filter) = dml_statement.input.as_ref() {
    //             task::block_on(provider.delete(table_name, Some(filter.predicate.clone())))
    //         } else {
    //             task::block_on(provider.delete(table_name, None))
    //         }
    //     })?
    // } else {
    //     unreachable!()
    // };

    // if let Some(num_deleted) = delete_metrics.num_deleted_rows {
    //     unsafe {
    //         (*(*query_desc.clone().into_pg()).estate).es_processed = num_deleted as u64;
    //     }
    // }

    Ok(())
}
