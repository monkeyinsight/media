use reqwest::Client;
use reqwest::redirect;
use regex::Regex;
use serde::Serialize;
use std::fs::{write,read_to_string};

#[derive(Debug,Serialize)]
pub struct TwitchChannel {
    pub channel: String,
    pub title: String,
    pub thumb: String,
    pub link: String
}

pub async fn get_subscriptions() -> Vec<String> {
    let file_contents = read_to_string("twitch.txt").unwrap();
    let list: Vec<&str> = file_contents.split(';').collect();
    return list.iter().map(|&x| x.to_owned()).collect();
}

pub async fn add(channel: &str) -> Result<(), &'static str> {
    let mut subscriptions = self::get_subscriptions().await;
    match subscriptions.contains(&channel.to_owned()) {
        true => Ok(()),
        false => {
            subscriptions.push(channel.to_owned());
            match write("twitch.txt", subscriptions.join(";")) {
                Ok(_) => Ok(()),
                Err(_) => Err("Error writing file")
            }
        }
    }
}

pub async fn remove(channel: &str) -> Result<(), &'static str> {
    let mut subscriptions = self::get_subscriptions().await;
    subscriptions.retain(|ch| ch != channel);
    match write("twitch.txt", subscriptions.join(";")) {
        Ok(_) => Ok(()),
        Err(_) => Err("Error writing file")
    }
}

pub async fn get_status(channel: String) -> Result<TwitchChannel, &'static str>  {
    let client = Client::new();

    let response = client.get(format!("https://www.twitch.tv/{}", channel))
        .send()
        .await
        .unwrap();

    match response.status() {
        reqwest::StatusCode::OK => {
            match response.text().await {
                Ok(parsed) => {
                    let re = Regex::new(r#"isLiveBroadcast":true}"#).unwrap();
                    let status = re.find(&parsed);
                    match status {
                        Some(_m) => {
                            let re = Regex::new("name=\"description\" content=\"(.+?)\"").unwrap();
                            let title = match re.captures(&parsed) {
                                Some(x) => x.get(1).unwrap().as_str().to_owned(),
                                None => "".to_owned()
                            };
                            let re = Regex::new(r#""(\S[^"]+?)"],"uploadDate"#).unwrap();
                            let thumb = match re.captures(&parsed) {
                                Some(x) => x.get(1).unwrap().as_str().to_owned(),
                                None => "".to_owned()
                            };

                            let client = Client::builder().redirect(redirect::Policy::none()).build().unwrap();
                            let pic_status = client.get(&thumb)
                                .send()
                                .await
                                .unwrap();
                            println!("{} {}", channel, pic_status.status());
                            if pic_status.status() != reqwest::StatusCode::OK {
                                return Err("Offline");
                            }

                            let output: TwitchChannel = TwitchChannel {
                                channel: channel.to_owned(),
                                title,
                                thumb,
                                link: format!("https://embed.twitch.tv/?channel={}&parent=localhost&muted=false&layout=video", channel)
                            };

                            return Ok(output);
                        },
                        None => Err("Offline")
                    }
                },
                Err(_) => Err("Request is not successful")
            }
        }
        _other => Err("Request is not successful")
    }
}