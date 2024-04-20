use std::error::Error;
use std::io::{ErrorKind, Read};
use std::time::Duration;
use reqwest::{Response};
use serde::Deserialize;
use serde_json::Value;

pub struct AzureRequest {
    client: reqwest::Client,
    headers: reqwest::header::HeaderMap,
    img: Vec<u8>,
    request_adress: String,
}

impl AzureRequest {
    pub fn new(key: &str, args: Vec<&str>) -> Self {
        let client = reqwest::Client::new();
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Ocp-Apim-Subscription-Key", reqwest::header::HeaderValue::from_str(key).unwrap());
        headers.insert("Content-Type", reqwest::header::HeaderValue::from_str("application/octet-stream").unwrap());

        AzureRequest {
            client,
            headers,
            img: Vec::new(),
            request_adress: "https://file-search-rust-paid.cognitiveservices.azure.com/computervision/imageanalysis:analyze?api-version=2024-02-01&features=".to_string() + args.join(",").as_str(),
        }
    }

    pub fn set_img(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        match std::fs::File::open(path)?.read_to_end(&mut self.img) {
            Ok(_) => Ok(()),
            Err(e) => Err(Box::new(e)),
        }
    }

    pub async fn send_request(&self) -> Result<Response, Box<dyn std::error::Error>> {
        let response = self.client.post(&self.request_adress)
            .headers(self.headers.clone())
            .body(self.img.clone())
            .timeout(Duration::from_secs(10))
            .send()
            .await;
        match response {
            Ok(response) => Ok(response),
            Err(e) => Err(Box::new(e)),
        }
    }
}
#[derive(Debug, Deserialize)]
pub struct Label {
    pub name: String,
    pub score: f64,
}
impl Label {
    pub fn new(name: String, score: f64) -> Self {
        Label {
            name,
            score,
        }
    }
}

impl From<&Value> for Label {
    fn from(value: &Value) -> Self {
        let name = value["name"].as_str().unwrap().to_string();
        let score = value["confidence"].as_f64().unwrap();
        Label {
            name,
            score,
        }
    }
}
#[derive(Debug, Deserialize)]
pub struct AzureResponse {
    pub caption: String,
    pub labels: Vec<Label>,
}

impl TryFrom<Value> for AzureResponse {
    type Error = ErrorKind;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let caption = value["captionResult"]["text"].as_str();

        if caption.is_none() {
            println!("No caption found");
            println!("{:?}", value);
            return Err(Self::Error::from(ErrorKind::NotFound));
        }
        let caption = caption.unwrap().to_string();
        let mut labels = Vec::new();


        let label = value["tagsResult"].get("values");
        if label.is_none() {
            println!("No labels found");
            return Err(Self::Error::from(ErrorKind::NotFound));
        }
        let label_it = label.unwrap().as_array().unwrap();

        for label in label_it {
            labels.push(Label::new(label.get("name").unwrap().as_str().unwrap().to_string(), label.get("confidence").unwrap().as_f64().unwrap()));
        }

        Ok(AzureResponse {
            caption,
            labels,
        })
    }
}

