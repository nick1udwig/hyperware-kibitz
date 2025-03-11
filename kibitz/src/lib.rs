use hyperware_process_lib::http::server::{
    HttpBindingConfig, HttpResponse, HttpServer, HttpServerRequest,
};
use hyperware_process_lib::kv;
use hyperware_process_lib::logging::{info, init_logging, Level};
use hyperware_process_lib::{
    await_message, call_init, homepage::add_to_homepage, last_blob, Address, Response,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// HTTP status codes as u16
const HTTP_OK: u16 = 200;
const HTTP_BAD_REQUEST: u16 = 400;
const HTTP_SERVER_ERROR: u16 = 500;

const HTTP_API_PATH: &str = "/api/keys";

const DB_NAME: &str = "kibitz_api_keys";

const ICON: &str = include_str!("icon");

#[derive(Debug, Serialize, Deserialize)]
struct ApiKeys {
    keys: HashMap<String, String>,
}

impl Default for ApiKeys {
    fn default() -> Self {
        Self {
            keys: HashMap::new(),
        }
    }
}

wit_bindgen::generate!({
    path: "target/wit",
    world: "process-v1",
    generate_unused_types: true,
    additional_derives: [serde::Deserialize, serde::Serialize, process_macros::SerdeJsonInto],
});

call_init!(init);
fn init(our: Address) {
    init_logging(Level::DEBUG, Level::INFO, None, None, None).unwrap();
    info!("begin");

    let mut server = HttpServer::new(5);

    server
        .serve_ui("kibitz-ui", vec!["/"], HttpBindingConfig::default())
        .expect("failed to serve UI");
    server
        .bind_http_path(HTTP_API_PATH, HttpBindingConfig::default())
        .expect("failed to bind API");

    add_to_homepage("Kibitz", Some(ICON), Some(""), None);

    // Setup KV store
    let kv = kv::open(our.package_id(), DB_NAME, None).expect("failed to open kv db");
    let key = b"api_keys".to_vec();

    //Request::new()
    //    .target(("our", "kv", "distro", "sys"))
    //    .body(
    //        serde_json::to_vec(&KvRequest {
    //            package_id: our.package_id(),
    //            db: "kibitz_api_keys".to_string(),
    //            action: KvAction::Open,
    //        })
    //        .unwrap(),
    //    )
    //    .send()
    //    .unwrap();

    loop {
        match await_message() {
            Err(e) => {
                info!("Error receiving message: {:?}", e);
            }
            Ok(message) => {
                info!("got message from {:?}", message.source());
                let Ok(http_request) = serde_json::from_slice::<HttpServerRequest>(message.body())
                else {
                    info!("wasn't an HttpServerRequest");
                    continue;
                };
                let HttpServerRequest::Http(http_request) = http_request else {
                    info!("wasn't an HttpServerRequest::Http");
                    continue;
                };
                info!(
                    "got IncomingHttpRequest with method & path: {:?} & {:?}",
                    http_request.method().unwrap().as_str(),
                    http_request.path().unwrap().as_str()
                );
                match (
                    http_request.method().unwrap().as_str(),
                    http_request.path().unwrap().as_str(),
                ) {
                    ("GET", "/api/keys") => {
                        let api_keys: ApiKeys = kv.get(&key).unwrap_or_default();
                        // Get API keys from KV store
                        //let api_keys = match Request::new()
                        //    .target(("our", "kv", "distro", "sys"))
                        //    .body(
                        //        serde_json::to_vec(&KvRequest {
                        //            package_id: our.package_id(),
                        //            db: "kibitz_api_keys".to_string(),
                        //            action: KvAction::Get(b"api_keys".to_vec()),
                        //        })
                        //        .unwrap(),
                        //    )
                        //    .send_and_await_response(5)
                        //    .unwrap()
                        //{
                        //    Ok(resp) => {
                        //        if let Ok(KvResponse::Get(value)) =
                        //            serde_json::from_slice(resp.body())
                        //        {
                        //            serde_json::from_slice(&value).unwrap_or(ApiKeys {
                        //                keys: HashMap::new(),
                        //            })
                        //        } else {
                        //            ApiKeys {
                        //                keys: HashMap::new(),
                        //            }
                        //        }
                        //    }
                        //    Err(_) => ApiKeys {
                        //        keys: HashMap::new(),
                        //    },
                        //};

                        info!("GET /api/keys: {api_keys:?}");
                        Response::new()
                            .body(serde_json::to_vec(&HttpResponse::new(HTTP_OK)).unwrap())
                            .blob_bytes(serde_json::to_vec(&api_keys).unwrap())
                            .send()
                            .unwrap();
                    }
                    ("PUT", "/api/keys") => {
                        let Some(blob) = last_blob() else {
                            info!("PUT /api/keys: no blob");
                            Response::new()
                                .body(
                                    serde_json::to_vec(&HttpResponse::new(HTTP_BAD_REQUEST))
                                        .unwrap(),
                                )
                                .send()
                                .unwrap();
                            continue;
                        };

                        let Ok(new_keys) = serde_json::from_slice::<ApiKeys>(&blob.bytes) else {
                            info!("PUT /api/keys: improper format");
                            Response::new()
                                .body(
                                    serde_json::to_vec(&HttpResponse::new(HTTP_BAD_REQUEST))
                                        .unwrap(),
                                )
                                .send()
                                .unwrap();
                            continue;
                        };
                        info!("PUT /api/keys: {new_keys:?}");
                        match kv.set(&key, &new_keys, None) {
                            Ok(_) => {
                                info!("PUT /api/keys: succeeded");
                                Response::new()
                                    .body(serde_json::to_vec(&HttpResponse::new(HTTP_OK)).unwrap())
                                    .send()
                                    .unwrap();
                            }
                            Err(_) => {
                                info!("PUT /api/keys: failed");
                                Response::new()
                                    .body(
                                        serde_json::to_vec(&HttpResponse::new(HTTP_SERVER_ERROR))
                                            .unwrap(),
                                    )
                                    .send()
                                    .unwrap();
                            }
                        }
                        //// Store API keys in KV store
                        //let result = Request::new()
                        //    .target(("our", "kv", "distro", "sys"))
                        //    .body(
                        //        serde_json::to_vec(&KvRequest {
                        //            package_id: our.package_id(),
                        //            db: "kibitz_api_keys".to_string(),
                        //            action: KvAction::Set {
                        //                key: b"api_keys".to_vec(),
                        //                tx_id: None,
                        //            },
                        //        })
                        //        .unwrap(),
                        //    )
                        //    .blob(LazyLoadBlob {
                        //        mime: None,
                        //        bytes: serde_json::to_vec(&new_keys).unwrap(),
                        //    })
                        //    .send_and_await_response(5)
                        //    .unwrap();

                        //match result {
                        //    Ok(_) => {
                        //        Response::new()
                        //            .body(
                        //                serde_json::to_vec(&HttpResponse::new(HTTP_OK))
                        //                    .unwrap(),
                        //            )
                        //            .send()
                        //            .unwrap();
                        //    }
                        //    Err(_) => {
                        //        Response::new()
                        //            .body(
                        //                serde_json::to_vec(&HttpResponse::new(
                        //                    HTTP_SERVER_ERROR,
                        //                ))
                        //                .unwrap(),
                        //            )
                        //            .send()
                        //            .unwrap();
                        //    }
                        //}
                    }
                    _ => {
                        // Serve UI requests through the HttpServer
                        server
                            .serve_ui("kibitz-ui", vec!["/"], HttpBindingConfig::default())
                            .unwrap();
                    }
                }
            }
        }
    }
}
