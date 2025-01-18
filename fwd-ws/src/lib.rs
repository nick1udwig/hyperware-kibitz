use std::collections::HashMap;

use crate::kinode::process::fwd_ws::{
    ConnectionType, Request as FwdWsRequest, Response as FwdWsResponse, State,
};
use kinode_process_lib::logging::{error, info, init_logging, Level};
use kinode_process_lib::{
    await_message, call_init, get_blob, get_state,
    homepage::add_to_homepage,
    http::{
        client::{open_ws_connection, send_ws_client_push},
        server::{
            send_response, HttpBindingConfig, HttpServer, HttpServerAction, HttpServerRequest,
            StatusCode, WsBindingConfig, WsMessageType,
        },
    },
    println, set_state, Address, LazyLoadBlob, Message, Request, Response,
};

wit_bindgen::generate!({
    path: "target/wit",
    world: "fwd-ws-template-dot-os-v0",
    generate_unused_types: true,
    additional_derives: [serde::Deserialize, serde::Serialize, process_macros::SerdeJsonInto],
});

const HTTP_API_PATH: &str = "/api";
const WS_PATH: &str = "/";

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct ProcessState {
    partner: Option<String>,
    connection: ConnectionType,
    ws_url: Option<String>,
    ws_channel: Option<u32>,
}

impl Default for ProcessState {
    fn default() -> Self {
        Self {
            partner: None,
            connection: ConnectionType::None,
            ws_url: None,
            ws_channel: None,
        }
    }
}

impl ProcessState {
    fn restore() -> anyhow::Result<Self> {
        Ok(if let Some(state) = get_state() {
            serde_json::from_slice(&state)?
        } else {
            Self::default()
        })
    }

    fn save(&self) -> anyhow::Result<()> {
        set_state(&serde_json::to_vec(self)?);
        Ok(())
    }

    fn to_public_state(&self) -> State {
        State {
            partner: self.partner.clone(),
            connection: self.connection.clone(),
            ws_url: self.ws_url.clone(),
        }
    }
}

fn make_http_server_address(our: &Address) -> Address {
    Address::from((our.node(), "http-server", "distro", "sys"))
}

fn make_http_client_address(our: &Address) -> Address {
    Address::from((our.node(), "http-client", "distro", "sys"))
}

fn handle_http_server_request(
    our: &Address,
    body: &[u8],
    server: &mut HttpServer,
    state: &mut ProcessState,
) -> anyhow::Result<()> {
    let request = serde_json::from_slice::<HttpServerRequest>(body)?;

    match request {
        HttpServerRequest::WebSocketOpen {
            ref path,
            channel_id,
        } => {
            if !(path == WS_PATH || path == HTTP_API_PATH)
                || matches!(state.connection, ConnectionType::ToWsServer)
            {
                return Ok(());
            }
            info!("WebSocket client connected on channel {}", channel_id);
            state.connection = ConnectionType::ToWsClient;
            state.ws_channel = Some(channel_id);
            state.save()?;
            server.handle_websocket_open(path, channel_id);
        }

        HttpServerRequest::WebSocketClose(channel_id) => {
            if state.ws_channel != Some(channel_id) {
                return Ok(());
            }
            info!("WebSocket client disconnected");
            state.connection = ConnectionType::None;
            state.ws_channel = None;
            state.save()?;
            server.handle_websocket_close(channel_id);
        }

        HttpServerRequest::WebSocketPush { channel_id, .. } => {
            if state.ws_channel != Some(channel_id) {
                return Ok(());
            }
            if let Some(blob) = get_blob() {
                // Only forward if we have a partner
                if let Some(ref partner) = state.partner {
                    Request::new()
                        .target((partner, "fwd-ws", "fwd-ws", "nick.kino"))
                        .body(FwdWsRequest::Forward(String::from_utf8(blob.bytes)?))
                        .send()?;
                }
            }
        }

        HttpServerRequest::Http(request) => {
            println!("fwd-ws httpserver: {}", request.method().unwrap().as_str());
            match request.method().unwrap().as_str() {
                "GET" => {
                    println!("fwd-ws httpserver: in get");
                    let headers = HashMap::from([(
                        "Content-Type".to_string(),
                        "application/json".to_string(),
                    )]);
                    send_response(
                        StatusCode::OK,
                        Some(headers),
                        serde_json::to_vec(&state.to_public_state())?,
                    );
                }
                "PUT" => {
                    if let Some(blob) = get_blob() {
                        handle_request_message(our, our, &blob.bytes, false, state)?;
                        send_response(StatusCode::OK, None, vec![]);
                    } else {
                        send_response(StatusCode::BAD_REQUEST, None, vec![]);
                    }
                }
                _ => send_response(StatusCode::METHOD_NOT_ALLOWED, None, vec![]),
            }
        }
    }

    Ok(())
}

fn handle_request_message(
    our: &Address,
    source: &Address,
    body: &[u8],
    should_respond: bool,
    state: &mut ProcessState,
) -> anyhow::Result<()> {
    println!("fwdreq...");
    let request: FwdWsRequest = body.try_into()?;
    println!("fwdreq: {:?}", request);
    match request {
        FwdWsRequest::SetPartner(partner) => {
            state.partner = partner;
            state.save()?;
            if should_respond {
                Response::new().body(FwdWsResponse::Ok).send()?;
            }
        }

        FwdWsRequest::ConnectToServer(url) => {
            if !matches!(state.connection, ConnectionType::None) {
                if should_respond {
                    Response::new()
                        .body(FwdWsResponse::Err("Already connected".to_string()))
                        .send()?;
                }
                return Ok(());
            }

            let channel_id = rand::random(); // Simple unique ID generation
            if let Ok(_) = open_ws_connection(url.clone(), None, channel_id) {
                state.connection = ConnectionType::ToWsServer;
                state.ws_url = Some(url);
                state.ws_channel = Some(channel_id);
                state.save()?;
                if should_respond {
                    Response::new().body(FwdWsResponse::Ok).send()?;
                }
            } else {
                if should_respond {
                    Response::new()
                        .body(FwdWsResponse::Err("Failed to connect".to_string()))
                        .send()?;
                }
            }
        }

        FwdWsRequest::AcceptClients(endpoint) => {
            if !matches!(state.connection, ConnectionType::None) {
                Response::new()
                    .body(FwdWsResponse::Err("Already connected".to_string()))
                    .send()?;
                return Ok(());
            }

            state.ws_url = Some(endpoint);
            state.save()?;
            if should_respond {
                Response::new().body(FwdWsResponse::Ok).send()?;
            }
        }

        FwdWsRequest::Disconnect => {
            state.connection = ConnectionType::None;
            state.ws_url = None;
            state.ws_channel = None;
            state.save()?;
            if should_respond {
                Response::new().body(FwdWsResponse::Ok).send()?;
            }
        }

        FwdWsRequest::GetState => {
            if should_respond {
                Response::new()
                    .body(FwdWsResponse::GetState(state.to_public_state()))
                    .send()?;
            }
        }

        FwdWsRequest::Forward(message) => {
            // Only forward if from partner
            if state.partner.as_ref().map_or(false, |p| p == &source.node) {
                if let Some(channel_id) = state.ws_channel {
                    match state.connection {
                        ConnectionType::ToWsServer => {
                            send_ws_client_push(
                                channel_id,
                                WsMessageType::Text,
                                LazyLoadBlob {
                                    mime: Some("text/plain".to_string()),
                                    bytes: message.into_bytes(),
                                },
                            );
                        }
                        ConnectionType::ToWsClient => {
                            let http_server = make_http_server_address(our);
                            Request::new()
                                .target(&http_server)
                                .body(serde_json::to_vec(&HttpServerAction::WebSocketPush {
                                    channel_id,
                                    message_type: WsMessageType::Text,
                                })?)
                                .blob(LazyLoadBlob {
                                    mime: Some("text/plain".to_string()),
                                    bytes: message.into_bytes(),
                                })
                                .send()?;
                        }
                        ConnectionType::None => {
                            if should_respond {
                                Response::new()
                                    .body(FwdWsResponse::Err("Not connected".to_string()))
                                    .send()?;
                            }
                            return Ok(());
                        }
                    }
                }
            }
            if should_respond {
                Response::new().body(FwdWsResponse::Ok).send()?;
            }
        }
    }
    Ok(())
}

fn handle_message(
    our: &Address,
    message: &Message,
    server: &mut HttpServer,
    state: &mut ProcessState,
) -> anyhow::Result<()> {
    if !message.is_request() {
        return Ok(());
    }

    let body = message.body();
    let source = message.source();

    if source == &make_http_server_address(our) {
        handle_http_server_request(our, body, server, state)?;
    } else if source == &make_http_client_address(our) {
        // Handle WebSocket client message
        if let Some(blob) = get_blob() {
            if let Some(ref partner) = state.partner {
                Request::new()
                    .target((partner, "fwd-ws", "kibitz", "nick.kino"))
                    .body(FwdWsRequest::Forward(String::from_utf8(blob.bytes)?))
                    .send()?;
            }
        }
    } else {
        // Handle request from another node
        handle_request_message(our, source, body, true, state)?;
    }
    server.ws_push_all_channels(
        HTTP_API_PATH,
        WsMessageType::Text,
        LazyLoadBlob {
            mime: None,
            bytes: vec![],
        },
    );

    Ok(())
}

call_init!(init);
fn init(our: Address) {
    init_logging(&our, Level::DEBUG, Level::INFO, None, None).unwrap();
    info!("begin");

    let mut server = HttpServer::new(5);
    let mut state = ProcessState::restore().unwrap_or_default();

    // Serve static UI files at root
    server
        .serve_ui(&our, "ui", vec!["/"], HttpBindingConfig::default())
        .expect("failed to serve UI");

    // State API endpoint
    server
        .bind_http_path(HTTP_API_PATH, HttpBindingConfig::default())
        .expect("failed to bind API");

    // WebSocket endpoint for when acting as server
    server
        .bind_ws_path(HTTP_API_PATH, WsBindingConfig::default())
        .expect("failed to bind WS");
    server
        .bind_ws_path(WS_PATH, WsBindingConfig::default())
        .expect("failed to bind WS");

    add_to_homepage("fwd-ws", None, Some("index.html"), None);

    info!("initialized with state: {:?}", state);

    loop {
        match await_message() {
            Err(send_error) => error!("got SendError: {send_error}"),
            Ok(ref message) => match handle_message(&our, message, &mut server, &mut state) {
                Ok(_) => {}
                Err(e) => error!("got error while handling message: {e:?}"),
            },
        }
    }
}
