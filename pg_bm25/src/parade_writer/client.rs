use crate::parade_index::index::ParadeIndex;
use crate::WriterInitError;
use crate::{
    json::builder::JsonBuilder,
    parade_writer::{ParadeWriterRequest, ParadeWriterResponse},
};
use pgrx::{log, PGRXSharedMemory};
use std::{error::Error, net::SocketAddr};
use tantivy::schema::Field;

#[derive(Copy, Clone, Default)]
pub struct ParadeWriterClient {
    addr: Option<SocketAddr>,
    error: Option<WriterInitError>,
}

impl ParadeWriterClient {
    pub fn set_addr(&mut self, addr: SocketAddr) {
        self.addr = Some(addr);
    }

    pub fn set_error(&mut self, err: WriterInitError) {
        self.error = Some(err);
    }

    fn send_request(
        &self,
        request: ParadeWriterRequest,
    ) -> Result<ParadeWriterResponse, Box<dyn Error>> {
        let addr = match self.addr {
            // If there's no addr, the server hasn't started yet.
            // We won't send the shutdown request,but it is up to the insert worker
            // to handle this case by checking for SIGTERM right before starting its server.
            None => match request {
                ParadeWriterRequest::Shutdown => {
                    log!("pg_bm25 shutdown worker skipped sending signal to insert worker");
                    return Ok(ParadeWriterResponse::Ok);
                }
                // If it wasn't a shutdown request, then we have a problem if the server has not
                // been started. Return an error.
                req => {
                    return Err(format!(
                        "pg_bm25 writer not yet initialized, but received request: {req:?}"
                    )
                    .into())
                }
            },
            Some(addr) => addr,
        };

        let bytes: Vec<u8> = request.into();
        let client = reqwest::blocking::Client::new();
        let response = client.post(format!("http://{addr}")).body(bytes).send()?;
        let response_body = response.bytes()?;
        ParadeWriterResponse::try_from(response_body.to_vec().as_slice()).map_err(|e| e.into())
    }

    fn get_data_directory(name: &str) -> String {
        unsafe {
            let option_name_cstr =
                std::ffi::CString::new("data_directory").expect("failed to create CString");
            let data_dir_str = String::from_utf8(
                std::ffi::CStr::from_ptr(pgrx::pg_sys::GetConfigOptionByName(
                    option_name_cstr.as_ptr(),
                    std::ptr::null_mut(),
                    true,
                ))
                .to_bytes()
                .to_vec(),
            )
            .expect("Failed to convert C string to Rust string");

            format!("{}/{}/{}", data_dir_str, "paradedb", name)
        }
    }

    pub fn insert(&self, index_name: &str, json_builder: JsonBuilder) {
        let data_directory = Self::get_data_directory(index_name);
        let response = self
            .send_request(ParadeWriterRequest::Insert(
                data_directory.clone(),
                json_builder,
            ))
            .expect("error while sending insert request}");

        match response {
            ParadeWriterResponse::Ok => {}
            error => {
                panic!("unexpected error while inserting into index at {data_directory}: {error:?}")
            }
        };
    }

    pub fn delete(&self, index_name: &str, ctid_field: Field, ctid_values: Vec<u64>) {
        let data_directory = Self::get_data_directory(index_name);
        let response = self
            .send_request(ParadeWriterRequest::Delete(
                data_directory.clone(),
                ctid_field,
                ctid_values,
            ))
            .expect("error while sending delete request}");

        match response {
            ParadeWriterResponse::Ok => {}
            error => {
                panic!("unexpected error while deleting from index at {data_directory}: {error:?}")
            }
        };
    }

    pub fn commit(&self, index_name: &str) {
        let data_directory = Self::get_data_directory(index_name);
        let response = self
            .send_request(ParadeWriterRequest::Commit(data_directory.clone()))
            .expect("error while sending commit request}");

        match response {
            ParadeWriterResponse::Ok => {}
            error => {
                panic!("unexpected error while committing to index at {data_directory}: {error:?}")
            }
        };
    }

    pub fn vacuum(&self, index_name: &str) {
        let data_directory = Self::get_data_directory(index_name);
        let response = self
            .send_request(ParadeWriterRequest::Vacuum(data_directory.clone()))
            .expect("error while sending commit request}");

        match response {
            ParadeWriterResponse::Ok => {}
            error => {
                panic!("unexpected error while vacuuming index at {data_directory}: {error:?}")
            }
        };
    }

    pub fn drop_index(&self, index_name: &str) {
        // The background worker will delete any file path we give it as part of its cleanup.
        // Here we define the paths we need gone.

        let mut paths_to_delete = Vec::new();
        let data_directory = Self::get_data_directory(index_name);
        let field_configs_file = ParadeIndex::get_field_configs_path(&data_directory);
        let tantivy_writer_lock = format!("{data_directory}/.tantivy-writer.lock");
        let tantivy_meta_lock = format!("{data_directory}/.tantivy-meta.lock");

        // The background worker will correctly order paths for safe deletion, so order
        // here doesn't matter.
        paths_to_delete.push(tantivy_writer_lock);
        paths_to_delete.push(tantivy_meta_lock);
        paths_to_delete.push(field_configs_file);
        paths_to_delete.push(data_directory.clone());

        let response = self
            .send_request(ParadeWriterRequest::DropIndex(
                data_directory.clone(),
                paths_to_delete,
            ))
            .expect("error while sending drop index request}");

        match response {
            ParadeWriterResponse::Ok => {}
            error => {
                panic!("unexpected error while dropping index at {data_directory}: {error:?}")
            }
        };
    }

    pub fn shutdown(&self) -> Result<(), Box<dyn Error>> {
        self.send_request(ParadeWriterRequest::Shutdown)?;
        Ok(())
    }
}

unsafe impl PGRXSharedMemory for ParadeWriterClient {}
