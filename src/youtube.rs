use reqwest::Client;
use regex::Regex;
use serde::Serialize;
use std::fs::{write,read_to_string};

#[derive(Debug,Serialize)]
pub struct YoutubeVideo {
    pub title: String,
    pub thumb: String,
    pub link: String,
    pub time: String
}

#[derive(Debug,Serialize)]
pub struct YoutubeChannel {
    pub channel: String,
    pub videos: Vec<YoutubeVideo>
}

pub async fn get_subscriptions() -> Vec<String> {
    let file_contents = read_to_string("youtube.txt").unwrap();
    let list: Vec<&str> = file_contents.split(';').collect();
    return list.iter().map(|&x| x.to_owned()).collect();
}

pub async fn add(channel: &str) -> Result<(), &'static str> {
    let mut subscriptions = self::get_subscriptions().await;
    match subscriptions.contains(&channel.to_owned()) {
        true => Ok(()),
        false => {
            subscriptions.push(channel.to_owned());
            match write("youtube.txt", subscriptions.join(";")) {
                Ok(_) => Ok(()),
                Err(_) => Err("Error writing file")
            }
        }
    }
}

pub async fn remove(channel: &str) -> Result<(), &'static str> {
    let mut subscriptions = self::get_subscriptions().await;
    subscriptions.retain(|ch| ch != channel);
    match write("youtube.txt", subscriptions.join(";")) {
        Ok(_) => Ok(()),
        Err(_) => Err("Error writing file")
    }
}

pub async fn get_videos(channel: String) -> Result<YoutubeChannel, &'static str> {
    let client = Client::new();

    let response = client.get(format!("https://www.youtube.com/{}/videos", channel))
        .header("Cookie", "SOCS=CAESEwgDEgk1MTgwODc1MTcaAmVuIAEaBgiA5-OgBg")
        .send()
        .await
        .unwrap();

    match response.status() {
        reqwest::StatusCode::OK => {
            match response.text().await {
                Ok(parsed) => {
                    let re = Regex::new(r#""videoId":"([^"&?\s]{11})","thumbnail":.+?\[\{.+?\{"url":"(.+?)".+?\}\]\}.+?"title":\{"runs":\[\{"text":"(.+?)".+?"publishedTimeText":\{"simpleText":"(.+?)""#).unwrap();

                    let videos = re.captures_iter(&parsed)
                        .map(|x| {                            
                            let output = YoutubeVideo {
                                title: x.get(3).unwrap().as_str().to_owned(),
                                thumb: x.get(2).unwrap().as_str().to_owned(),
                                time: x.get(4).unwrap().as_str().to_owned(),
                                link: format!("https://youtube.com/watch?id={}", x.get(1).unwrap().as_str().to_owned())
                            };
                            return output;
                        })
                        .collect();

                    let output = YoutubeChannel {
                        channel: channel.to_string(),
                        videos
                    };

                    return Ok(output);
                },
                Err(_) => Err("Request is not successful")
            }
        }
        _other => Err("Request is not successful")
    }
}