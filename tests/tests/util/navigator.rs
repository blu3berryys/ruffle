use crate::util::runner::TestLogBackend;
use ruffle_core::backend::log::LogBackend;
use ruffle_core::backend::navigator::{
    fetch_path, resolve_url_with_relative_base_path, ErrorResponse, NavigationMethod,
    NavigatorBackend, NullExecutor, NullSpawner, OwnedFuture, Request, SuccessResponse,
};
use ruffle_core::indexmap::IndexMap;
use ruffle_core::loader::Error;
use ruffle_core::socket::{SocketAction, SocketHandle};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use url::{ParseError, Url};

/// A `NavigatorBackend` used by tests that supports logging fetch requests.
///
/// This can be used by tests that fetch data to verify that the request is correct.
pub struct TestNavigatorBackend {
    spawner: NullSpawner,
    relative_base_path: PathBuf,
    log: Option<TestLogBackend>,
}

impl TestNavigatorBackend {
    pub fn new(
        path: &Path,
        executor: &NullExecutor,
        log: Option<TestLogBackend>,
    ) -> Result<Self, std::io::Error> {
        Ok(Self {
            spawner: executor.spawner(),
            relative_base_path: path.canonicalize()?,
            log,
        })
    }
}

impl NavigatorBackend for TestNavigatorBackend {
    fn navigate_to_url(
        &self,
        url: &str,
        target: &str,
        vars_method: Option<(NavigationMethod, IndexMap<String, String>)>,
    ) {
        // Log request.
        if let Some(log) = &self.log {
            log.avm_trace("Navigator::navigate_to_url:");
            log.avm_trace(&format!("  URL: {}", url));
            log.avm_trace(&format!("  Target: {}", target));
            if let Some((method, vars)) = vars_method {
                log.avm_trace(&format!("  Method: {}", method));
                for (key, value) in vars {
                    log.avm_trace(&format!("  Param: {}={}", key, value));
                }
            }
        }
    }

    fn fetch(&self, request: Request) -> OwnedFuture<SuccessResponse, ErrorResponse> {
        // Log request.
        if let Some(log) = &self.log {
            log.avm_trace("Navigator::fetch:");
            log.avm_trace(&format!("  URL: {}", request.url()));
            log.avm_trace(&format!("  Method: {}", request.method()));
            let headers = request.headers();
            if !headers.is_empty() {
                log.avm_trace(&format!(
                    "  Headers:\n{}",
                    headers
                        .iter()
                        .map(|(key, val)| format!("{key}: {val}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                ))
            }
            if let Some((body, mime_type)) = request.body() {
                log.avm_trace(&format!("  Mime-Type: {}", mime_type));
                if mime_type == "application/x-www-form-urlencoded" {
                    log.avm_trace(&format!("  Body: {}", String::from_utf8_lossy(body)));
                } else {
                    log.avm_trace(&format!("  Body: ({} bytes)", body.len()));
                }
            }
        }

        fetch_path(self, "TestNavigatorBackend", request.url())
    }

    fn resolve_url(&self, url: &str) -> Result<Url, ParseError> {
        resolve_url_with_relative_base_path(self, self.relative_base_path.clone(), url)
    }

    fn spawn_future(&mut self, future: OwnedFuture<(), Error>) {
        self.spawner.spawn_local(future);
    }

    fn pre_process_url(&self, url: Url) -> Url {
        url
    }

    fn connect_socket(
        &mut self,
        host: String,
        port: u16,
        handle: SocketHandle,
        _receiver: Receiver<Vec<u8>>,
        sender: Sender<SocketAction>,
    ) {
        if let Some(log) = &self.log {
            log.avm_trace("Navigator::connect_socket");
            log.avm_trace(&format!("    Host: {}; Port: {}", host, port));
        }
        sender
            .send(SocketAction::Connect(handle, false))
            .expect("working channel send");
    }
}
