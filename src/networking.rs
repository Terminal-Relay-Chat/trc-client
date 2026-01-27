use serde_json::json;
use serde::{Serialize, Deserialize};

async fn do_socket_connection() {
    
}

async fn handle_sock() {

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
    
    format!("{}://{}/api/{}", prefix, base_ip, path)
}

/// return the formatted socket url. 
pub fn sock_url<'a>(base_ip: &String, secure: &bool) -> String {
    let prefix = match secure {
        true => "wss",
        false => "ws"
    };
    
    format!("{}://{}/", prefix, base_ip)

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
