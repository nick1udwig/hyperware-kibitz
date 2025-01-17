use kinode_process_lib::logging::{error, info, init_logging, Level};
use kinode_process_lib::{
    await_message, call_init,
    homepage::add_to_homepage,
    http::server::{HttpBindingConfig, HttpServer},
    Address,
};

wit_bindgen::generate!({
    path: "target/wit",
    world: "process-v1",
    generate_unused_types: true,
    additional_derives: [serde::Deserialize, serde::Serialize, process_macros::SerdeJsonInto],
});

call_init!(init);
fn init(our: Address) {
    init_logging(&our, Level::DEBUG, Level::INFO, None, None).unwrap();
    info!("begin");

    let mut server = HttpServer::new(5);

    // Bind UI files to routes with index.html at "/"; API to /messages; WS to "/"
    server
        .serve_ui(&our, "ui", vec!["/"], HttpBindingConfig::default())
        .expect("failed to serve UI");

    add_to_homepage("Kibitz", None, Some("index.html"), None);

    loop {
        await_message();
    }
}
