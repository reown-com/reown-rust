use {
    crate::ClientId,
    gloo_timers::future::TimeoutFuture,
    std::{sync::Arc, time::Duration},
    walletconnect_sdk::{
        client::{websocket::Client, ConnectionOptions},
        rpc::domain::Topic,
    },
    wasm_bindgen::JsValue,
    web_sys::console,
};

// Helper function to set text in the result div
pub fn set_result_text(div: &str, text: &str) -> Result<(), JsValue> {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let result_div = document
        .get_element_by_id(div)
        .expect("should have result element");

    result_div.set_inner_html("");

    let text_element = document.create_element("p")?;
    text_element.set_inner_html(text);
    result_div.append_child(&text_element)?;
    Ok(())
}

pub async fn subscribe_topic(id: ClientId, client: &Client, topic: Topic) -> bool {
    let msg = format!("{topic}");
    match client.subscribe(topic).await {
        Ok(_) => {
            let div = format!("{}topic", id);
            let _ = set_result_text(&div, &msg);
            true
        }
        Err(e) => {
            let div = id.div("error");
            let _ = set_result_text(&div, &format!("failed to subscribe {e}"));
            false
        }
    }
}

pub async fn connect(id: &str, client: &Client, opts: &ConnectionOptions) {
    match client.connect(opts).await {
        Ok(_) => {
            console::log_1(&"WebSocket connection successful".into());
        }
        Err(e) => {
            let error_msg = format!("WebSocket connection failed: {:?}", e);
            let div = format!("{}error", id);
            let _ = set_result_text(&div, error_msg.as_str());
        }
    }
}

pub async fn publish(id: ClientId, client: Client, topic: Topic) {
    for i in 1..9 {
        let msg = format!("{i}");
        if let Err(e) = client
            .publish(
                topic.clone(),
                Arc::from(msg.as_str()),
                None,
                0,
                Duration::from_secs(60),
                false,
            )
            .await
        {
            let error_msg = format!("Failed  message send {e}");
            let _ = set_result_text(&id.div("error"), &error_msg);
            return;
        }
        TimeoutFuture::new(2000).await;
    }
}
