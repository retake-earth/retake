use async_std::sync::Mutex;
use async_std::task;
use deltalake::datafusion::execution::runtime_env::{RuntimeConfig, RuntimeEnv};
use deltalake::datafusion::prelude::{SessionConfig, SessionContext};
use once_cell::sync::Lazy;
use parking_lot::{RwLock, RwLockWriteGuard};
use pgrx::*;
use std::any::type_name;
use std::ffi::{CStr, CString};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::datafusion::catalog::{ParadeCatalog, ParadeCatalogList};
use crate::datafusion::directory::ParadeDirectory;
use crate::datafusion::schema::ParadeSchemaProvider;
use crate::datafusion::table::Tables;
use crate::errors::{NotFound, ParadeError};

static SESSION_CACHE: Lazy<Arc<RwLock<Option<SessionContext>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

pub struct Session;

impl<'a> Session {
    pub fn with_session_context<F, R>(f: F) -> Result<R, ParadeError>
    where
        F: FnOnce(&SessionContext) -> Result<R, ParadeError>,
    {
        let context_lock = SESSION_CACHE.read();
        let context = match context_lock.as_ref() {
            Some(context) => context.clone(),
            None => {
                drop(context_lock);
                Self::init(Self::catalog_oid()?)?
            }
        };

        f(&context)
    }

    pub fn with_catalog<F, R>(f: F) -> Result<R, ParadeError>
    where
        F: FnOnce(&ParadeCatalog) -> Result<R, ParadeError>,
    {
        let context_lock = SESSION_CACHE.read();
        let context = match context_lock.as_ref() {
            Some(context) => context.clone(),
            None => {
                drop(context_lock);
                Self::init(Self::catalog_oid()?)?
            }
        };

        let catalog_provider = context
            .catalog(&Self::catalog_name()?)
            .ok_or(NotFound::Catalog(Self::catalog_name()?.to_string()))?;

        let parade_catalog = catalog_provider
            .as_any()
            .downcast_ref::<ParadeCatalog>()
            .ok_or(NotFound::Value(type_name::<ParadeCatalog>().to_string()))?;

        f(parade_catalog)
    }

    pub fn with_schema_provider<F, R>(schema_name: &str, f: F) -> Result<R, ParadeError>
    where
        F: for<'b> FnOnce(
            &'b ParadeSchemaProvider,
        ) -> Pin<Box<dyn Future<Output = Result<R, ParadeError>> + 'b>>,
    {
        let context_lock = SESSION_CACHE.read();
        let context = match context_lock.as_ref() {
            Some(context) => context.clone(),
            None => {
                drop(context_lock);
                Self::init(Self::catalog_oid()?)?
            }
        };

        let schema_provider = context
            .catalog(&Self::catalog_name()?)
            .ok_or(NotFound::Catalog(Self::catalog_name()?.to_string()))?
            .schema(schema_name)
            .ok_or(NotFound::Schema(schema_name.to_string()))?;

        let parade_provider = schema_provider
            .as_any()
            .downcast_ref::<ParadeSchemaProvider>()
            .ok_or(NotFound::Value(
                type_name::<ParadeSchemaProvider>().to_string(),
            ))?;

        task::block_on(f(parade_provider))
    }

    pub fn with_tables<F, Fut, R>(schema_name: &str, f: F) -> Result<R, ParadeError>
    where
        F: FnOnce(Arc<Mutex<Tables>>) -> Fut,
        Fut: Future<Output = Result<R, ParadeError>>,
    {
        let tables = Self::with_schema_provider(schema_name, |provider| {
            Box::pin(async move { provider.tables() })
        })?;

        task::block_on(f(tables))
    }

    pub fn with_write_lock<F, R>(f: F) -> Result<R, ParadeError>
    where
        F: FnOnce(RwLockWriteGuard<'a, Option<SessionContext>>) -> Result<R, ParadeError>,
    {
        let context_lock = SESSION_CACHE.write();
        f(context_lock)
    }

    pub fn init(catalog_oid: pg_sys::Oid) -> Result<SessionContext, ParadeError> {
        let preload_libraries = unsafe {
            CStr::from_ptr(pg_sys::GetConfigOptionByName(
                CString::new("shared_preload_libraries")?.as_ptr(),
                std::ptr::null_mut(),
                true,
            ))
            .to_str()?
        };

        if !preload_libraries.contains("pg_analytics") {
            return Err(ParadeError::SharedPreload);
        }

        let session_config = SessionConfig::from_env()?.with_information_schema(true);

        let rn_config = RuntimeConfig::new();
        let runtime_env = RuntimeEnv::new(rn_config)?;

        Self::with_write_lock(|mut context_lock| {
            let mut context =
                SessionContext::new_with_config_rt(session_config, Arc::new(runtime_env));

            // Create schema directory if it doesn't exist
            ParadeDirectory::create_catalog_path(catalog_oid)?;

            // Register catalog list
            context.register_catalog_list(Arc::new(ParadeCatalogList::try_new()?));

            // Create and register catalog
            let catalog = ParadeCatalog::try_new()?;
            task::block_on(catalog.init())?;
            context.register_catalog(&Self::catalog_name()?, Arc::new(catalog));

            // Set context
            *context_lock = Some(context.clone());

            Ok(context)
        })
    }

    pub fn catalog_name() -> Result<String, ParadeError> {
        let database_name = unsafe { pg_sys::get_database_name(Self::catalog_oid()?) };
        if database_name.is_null() {
            return Err(NotFound::Database(Self::catalog_oid()?.as_u32().to_string()).into());
        }

        Ok(unsafe { CStr::from_ptr(database_name).to_str()?.to_owned() })
    }

    pub fn catalog_oid() -> Result<pg_sys::Oid, ParadeError> {
        Ok(unsafe { pg_sys::MyDatabaseId })
    }
}
