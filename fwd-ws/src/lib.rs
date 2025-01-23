use std::collections::HashMap;

use crate::kinode::process::fwd_ws::{
    ConnectionType, Request as FwdWsRequest, Response as FwdWsResponse, State,
};
use kinode_process_lib::logging::{error, info, init_logging, Level};
use kinode_process_lib::{
    await_message, call_init, get_blob, get_state,
    homepage::add_to_homepage,
    http::{
        client::{open_ws_connection, send_ws_client_push, HttpClientRequest},
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
const DEFAULT_WS_URL: &str = "ws://localhost:10125";

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct ProcessState {
    partner: Option<String>,
    connection: ConnectionType,
    ws_url: Option<String>,
    ws_channel: Option<u32>,
    #[serde(skip)]
    pending_message: Option<String>,
    #[serde(skip)]
    pending_partner_message: Option<String>,
}

impl Default for ProcessState {
    fn default() -> Self {
        Self {
            partner: None,
            connection: ConnectionType::None,
            ws_url: None,
            ws_channel: None,
            pending_message: None,
            pending_partner_message: None,
        }
    }
}

impl ProcessState {
    fn restore() -> anyhow::Result<Self> {
        let restored = if let Some(state) = get_state() {
            let mut state: Self = serde_json::from_slice(&state)?;

            // If we have a WebSocket server connection and a channel
            state.try_reconnect_to_server()?;

            state
        } else {
            Self::default()
        };
        Ok(restored)
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

    fn try_reconnect_to_server(&mut self) -> anyhow::Result<()> {
        if !matches!(self.connection, ConnectionType::ToWsServer) {
            return Ok(());
        }

        // Only attempt reconnect if we don't have a valid channel
        if self.ws_channel.is_some() {
            return Ok(());
        }

        let url = &self
            .ws_url
            .as_ref()
            .map(|url| url.clone())
            .unwrap_or_else(|| DEFAULT_WS_URL.to_string());

        // Create new connection
        let channel_id = rand::random();
        match open_ws_connection(url.clone(), None, channel_id) {
            Ok(_) => {
                self.ws_channel = Some(channel_id);
                self.save()?;
                info!("Successfully reconnected to WebSocket server");
            }
            Err(e) => {
                info!("Failed to reconnect to WebSocket server: {}", e);
                self.ws_channel = None;
                self.save()?;
            }
        }
        Ok(())
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

            // Send any pending partner messages
            if let Some(ref message) = state.pending_partner_message {
                Request::new()
                    .target(&make_http_server_address(our))
                    .body(serde_json::to_vec(&HttpServerAction::WebSocketPush {
                        channel_id,
                        message_type: WsMessageType::Text,
                    })?)
                    .blob(LazyLoadBlob {
                        mime: Some("text/plain".to_string()),
                        bytes: message.clone().into_bytes(),
                    })
                    .send()?;
                state.pending_partner_message = None;
            }
            state.save()?;
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
            // request from client (kibitz fe):
            //  forward to our partner over the Kinet
            if state.ws_channel != Some(channel_id) {
                return Ok(());
            }
            if let Some(blob) = get_blob() {
                let msg = String::from_utf8(blob.bytes)?;
                if let Some(ref partner) = state.partner {
                    Request::new()
                        .target((partner, "fwd-ws", "kibitz", "nick.kino"))
                        .body(FwdWsRequest::Forward(msg))
                        .send()?;
                } else {
                    // Store message if no partner set
                    state.pending_message = Some(msg);
                    state.save()?;
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
            // If partner is being set, send any pending messages
            if state.partner.is_some() {
                if let Some(ref msg) = state.pending_message {
                    if let Some(ref partner) = state.partner {
                        Request::new()
                            .target((partner, "fwd-ws", "kibitz", "nick.kino"))
                            .body(FwdWsRequest::Forward(msg.clone()))
                            .send()?;
                        state.pending_message = None;
                    }
                }
            }
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
            if message.is_empty() {
                return Ok(());
            }
            // Only handle if from partner
            if state.partner.as_ref().map_or(false, |p| p == &source.node) {
                if let Some(channel_id) = state.ws_channel {
                    match state.connection {
                        ConnectionType::ToWsServer => {
                            // we're connected to a WS server: the ws-mcp
                            //  send the message to the ws-mcp to be fulfilled
                            send_ws_client_push(
                                channel_id,
                                WsMessageType::Text,
                                LazyLoadBlob {
                                    mime: Some("text/plain".to_string()),
                                    bytes: message.clone().into_bytes(),
                                },
                            );
                        }
                        ConnectionType::ToWsClient => {
                            // we're connected to kibitz:
                            //  send the message to kibitz
                            let http_server = make_http_server_address(our);
                            Request::new()
                                .target(&http_server)
                                .body(serde_json::to_vec(&HttpServerAction::WebSocketPush {
                                    channel_id,
                                    message_type: WsMessageType::Text,
                                })?)
                                .blob(LazyLoadBlob {
                                    mime: Some("text/plain".to_string()),
                                    bytes: message.clone().into_bytes(),
                                })
                                .send()?;
                        }
                        ConnectionType::None => {
                            // Store message if no WS connection
                            state.pending_partner_message = Some(message);
                            state.save()?;
                        }
                    }
                } else {
                    // Store message if no WS channel
                    state.pending_partner_message = Some(message);
                    state.save()?;
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
        let request = serde_json::from_slice::<HttpClientRequest>(body)?;
        if let HttpClientRequest::WebSocketClose { .. } = request {
            state.try_reconnect_to_server()?;
        } else {
            // Its a WebSocketPush:
            //  Handle WebSocket client message
            if let Some(blob) = get_blob() {
                if let Some(ref partner) = state.partner {
                    Request::new()
                        .target((partner, "fwd-ws", "kibitz", "nick.kino"))
                        .body(FwdWsRequest::Forward(String::from_utf8(blob.bytes)?))
                        .send()?;
                }
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

    // Only try to connect to default URL if we're not already connected
    if matches!(state.connection, ConnectionType::None) && state.ws_channel.is_none() {
        if let Err(_) = handle_request_message(
            &our,
            &our,
            &serde_json::to_vec(&FwdWsRequest::ConnectToServer(DEFAULT_WS_URL.to_string()))
                .unwrap(),
            false,
            &mut state,
        ) {
            info!("couldn't connect to default WS url: {DEFAULT_WS_URL}");
        }
    }

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
