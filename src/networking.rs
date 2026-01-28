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

const LOGIN_LOCATION: &'static str = "~/.trclogin";


#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(dead_code)]
pub enum UpdateType {
    MESSAGE,
    SYSTEM, // SYSTEM is for commands or responses to requests from a client
    ERROR,
}
#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct SocketMessage {
    pub message_type: UpdateType,
    pub content: String,
    pub sender: Option<User>
}


#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum UserMode {
    User,
    Bot
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum UserPermissions {
    User, // basic things: join channels, read/write to those channels
    Moderator, // `/kick` people, `/ban` people of lower ranks
    Admin, // highest permission. Assumed owner or extremely trusted member 
}

/// the publicly available information for a given user that should be stored in state
/// password is only used in the login process
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)] // PartialEq for testing convinience.
                                                           // See token.rs tests if you change
                                                           // this.
pub struct User {
    pub user_type: UserMode,
    pub permission_level: UserPermissions,
    pub username: String,
    pub handle: String,
    pub provider_site: Option<String>, // this is so people can know how to DM them
    pub banned: bool, // for while the user is stored in memory
}


pub async fn do_socket_connection(base_ip: &String, secure: &bool, token: &String, messages: Arc<Mutex<Vec<SocketMessage>>>) -> Result<(), tokio_tungstenite::tungstenite::Error> {
    let target_url = sock_url(base_ip, secure);
    let (ws_stream, _) = tokio_tungstenite::connect_async(&target_url).await.expect(&format!("Failed to connect to: {}", target_url));
    let (tx, rx) = ws_stream.split();
    let (tx, rx) = (Arc::new(Mutex::new(tx)), Arc::new(Mutex::new(rx)));

    /* authenticate */

    // the first request is assumed to be the token
    tx.lock().await.send(tungstenite::Message::Text(token.into())).await?;

    // if authentication is failed the socket will close, otherwise we will recieve a message in
    // text
    if let Some(Ok(tungstenite::Message::Text(_good_response))) = rx.lock().await.next().await {
        print!("successfully connected to websocket");
    } else {
        panic!("unable to authenticate with websocket");
    }

    //TODO replace this with the handle_sock_send() fn
    tx.lock().await.send(tungstenite::Message::Text("general".into())).await?;   

    /* handle socket */
    
    select! {
        _res = handle_sock_recv(rx, messages) => {},
        // res = handle_sock_send() => {}, //TODO
    }

    print!("disconnected.");
    std::process::exit(0);
}

/// The skeleton for a socket update. This is used to differentiate between MESSAGE updates and
/// other types.
#[derive(Debug, Serialize, Deserialize)]
struct RawSocketUpdate {
    pub message_type: UpdateType,
}


async fn handle_sock_send() {}
async fn handle_sock_recv(ws: Arc<Mutex<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>>, messages: Arc<Mutex<Vec<SocketMessage>>>) -> Result<(), Box<dyn Error>> {
    while let Some(msg) = ws.lock().await.next().await {
        match msg? {
            tungstenite::Message::Text(raw) => {
                let rawsockupdate: RawSocketUpdate = serde_json::from_str(&raw.as_str())?;

                match rawsockupdate.message_type {
                    UpdateType::MESSAGE => {
                        let deserialized: SocketMessage = serde_json::from_str(&raw)?;
                        messages.lock().await.push(deserialized);
                    },
                    UpdateType::ERROR => {
                        //TODO
                        panic!("hit an error from the server")
                    },
                    UpdateType::SYSTEM => {
                        //TODO things like switching channels
                    }
                }

            },
            _ => {}
        }
    }
    print!("bork");
    Ok(())
}


#[derive(Debug, Serialize, Deserialize)]
struct TokenFetchResponse {
    error: bool,
    value: String
}

#[derive(Debug, Serialize, Deserialize)]
struct Login {
    handle: String,
    password: String
}

pub async fn get_token(base_ip: &String, secure: &bool) -> String {
    use std::fs;

    let target = api_url(base_ip, secure, "login");
    let client = reqwest::Client::new();

    //TODO: use real credentials and not test ones
    let body = {
        let login_config = fs::read_to_string(LOGIN_LOCATION).expect("needed a config file");
        let _validation = serde_json::from_str::<Login>(&login_config).expect("invalid login");
        login_config
    };

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
