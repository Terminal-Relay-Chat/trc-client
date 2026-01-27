use std::error::Error;

use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use serde::{Serialize, Deserialize};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::{self};
use tokio_tungstenite::{WebSocketStream, MaybeTlsStream};
use tokio::net::{TcpStream};
use std::sync::Arc;
use tokio::select;
use futures_util::stream::SplitStream;
use super::Message;

pub async fn do_socket_connection(base_ip: &String, secure: &bool, token: &String, messages: Arc<Mutex<Vec<Message>>>) -> Result<(), tokio_tungstenite::tungstenite::Error> {
    let target_url = sock_url(base_ip, secure);
    let (mut ws_stream, _) = tokio_tungstenite::connect_async(&target_url).await.expect(&format!("Failed to connect to: {}", target_url));

    /* authenticate */

    // the first request is assumed to be the token
    ws_stream.send(tungstenite::Message::Text(token.into())).await?;

    // if authentication is failed the socket will close, otherwise we will recieve a message in
    // text
    if let Some(Ok(tungstenite::Message::Text(_good_response))) = ws_stream.next().await {
        print!("successfully connected to websocket");
    } else {
        panic!("unable to authenticate with websocket");
    }

    //TODO replace this with the handle_sock_send() fn
    ws_stream.send(tungstenite::Message::Text("general".into())).await?;   

    /* handle socket */
    let (tx, rx) = ws_stream.split();
    let (tx, rx) = (Arc::new(Mutex::new(tx)), Arc::new(Mutex::new(rx)));

    select! {
        res = handle_sock_recv(rx, messages) => {},
        // res = handle_sock_send() => {},
    }

    print!("disconnected.");
    std::process::exit(0);
}

async fn handle_sock_send() {}
async fn handle_sock_recv(ws: Arc<Mutex<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>>, messages: Arc<Mutex<Vec<Message>>>) -> Result<(), Box<dyn Error>> {
    while let Some(msg) = ws.lock().await.next().await {
        match msg? {
            tungstenite::Message::Text(raw) => {
                messages.lock().await.push(Message { sender: String::new(), content: raw.to_string() });
            },
            _ => {}
        }
    }

    Ok(())
}


#[derive(Debug, Serialize, Deserialize)]
struct TokenFetchResponse {
    error: bool,
    value: String
}

pub async fn get_token(base_ip: &String, secure: &bool) -> String {
    let target = api_url(base_ip, secure, "login");
    let client = reqwest::Client::new();

    //TODO: use real credentials and not test ones
    let body = json!({
        "handle": "test",
        "password": "test"
    });

    let res: TokenFetchResponse = client.post(target).json(&body)
        .send().await.unwrap()
        .json().await.unwrap();
    
    // crash on error
    if res.error {
        panic!("there was an error retrieving your token")
    }

    res.value
}

/// return the formatted api url. path is expected to be either empty or a firstfolder/second/third
/// formatted string
pub fn api_url(base_ip: &String, secure: &bool, path: &str) -> String {
    let prefix = match secure {
        true => "https",
        false => "http"
    };
    
    format!("{}://{}:3000/api/{}", prefix, base_ip, path)
}

/// return the formatted socket url. 
pub fn sock_url<'a>(base_ip: &String, secure: &bool) -> String {
    let prefix = match secure {
        true => "wss",
        false => "ws"
    };
    
    format!("{}://{}:3001/", prefix, base_ip)

}


pub async fn send_message(token: String, message: String, channel: &str, base_ip: &String, secure: &bool) {
    let target = api_url(base_ip, secure, &format!("messages/{}", channel));
    let client =  reqwest::Client::new();
    client.post(target)
        .body(message)
        .header("x-auth-token", token)
        .send()
        .await
        .unwrap(); // ok to call unwrap because if this fails, there is something wrong with a
                   // variety of things.
}
