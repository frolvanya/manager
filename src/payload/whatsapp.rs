use anyhow::Context;

use rocket::serde::{json::Json, Deserialize};

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct Data {
    entry: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
struct Entry {
    changes: Vec<Change>,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
struct Change {
    value: Value,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
struct Value {
    metadata: Metadata,
    messages: Option<Vec<Message>>,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
struct Metadata {
    phone_number_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
struct Message {
    from: String,
    text: Option<TextBody>,
}

#[derive(Debug, serde::Deserialize)]
struct TextBody {
    body: String,
}

#[derive(Debug, PartialEq, FromForm)]
pub struct Hub {
    mode: Option<String>,
    challenge: Option<String>,
    verify_token: Option<String>,
}

pub async fn send_message(phone_number_id: String, to: String, text: String) {
    let token = std::env::var("WHATSAPP_ACCESS_TOKEN")
        .context("Unable to retrieve `WHATSAPP_ACCESS_TOKEN` from env")
        .unwrap();

    let client = reqwest::Client::new();

    let url =
        format!("https://graph.facebook.com/v18.0/{phone_number_id}/messages?access_token={token}");

    let response = client
        .post(&url)
        .json(&serde_json::json!({
        "messaging_product": "whatsapp",
        "to": to,
            "text": { "body": text },
        }))
        .header("Content-Type", "application/json")
        .send()
        .await;

    match response {
        Ok(res) => {
            info!("Response was sent successfully: \n{:#?}", res);
        }
        Err(err) => {
            error!("Error sending message: {:?}", err);
        }
    }
}

#[get("/whatsapp-webhook?<hub>")]
pub fn get_webhook(hub: Hub) -> Option<(rocket::http::Status, String)> {
    let token = std::env::var("WHATSAPP_TOKEN")
        .context("Unable to retrieve `WHATSAPP_TOKEN` from env")
        .unwrap();

    if let Hub {
        mode: Some(parsed_mode),
        challenge: Some(parsed_challenge),
        verify_token: Some(parsed_token),
    } = hub
    {
        if parsed_mode == "subscribe" && parsed_token == token {
            info!("WHATSAPP WEBHOOK WAS VERIFIED");
            return Some((rocket::http::Status::Ok, parsed_challenge));
        }
    }

    None
}

#[post("/whatsapp-webhook", data = "<request>")]
pub async fn post_webhook(request: Json<Option<Data>>) -> rocket::http::Status {
    info!("\n{:#?}", request);

    if request.is_none() {
        return rocket::http::Status::NotFound;
    }

    if let Some(req) = &request.0 {
        if let Some(messages) = &req.entry[0].changes[0].value.messages {
            if let Some(text) = &messages[0].text {
                let phone_number_id = req.entry[0].changes[0]
                    .value
                    .metadata
                    .phone_number_id
                    .clone();
                let from = messages[0].from.clone();
                let text = text.body.clone();

                send_message(phone_number_id, from, text).await;
            }
        }
    }

    rocket::http::Status::Ok
}
