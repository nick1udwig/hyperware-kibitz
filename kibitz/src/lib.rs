use kinode_process_lib::logging::{info, init_logging, Level};
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

const ICON: &str = include_str!("icon");

call_init!(init);
fn init(our: Address) {
    init_logging(&our, Level::DEBUG, Level::INFO, None, None).unwrap();
    info!("begin");

    let mut server = HttpServer::new(5);

    server
        .serve_ui(&our, "kibitz-ui", vec!["/"], HttpBindingConfig::default())
        .expect("failed to serve UI");

    add_to_homepage("Kibitz", Some(ICON), Some(""), None);

    loop {
        let _ = await_message();
    }
}
