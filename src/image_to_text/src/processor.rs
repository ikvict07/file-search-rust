use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use base64::encode;
use futures::future::join_all;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::{json, Value};
use tokio::time::sleep;
use yup_oauth2::{AccessToken, ServiceAccountKey};
use yup_oauth2 as oauth2;

pub enum ImageProcessor {
    Label(LabelProcessor),
    Caption(CaptionProcessor),
    Vision(VisionProcessor),
}

impl ImageProcessor {
    pub async fn new_label(dir: String, secret_json_path: String) -> ImageProcessor {
        let secret = Self::read_secret(&secret_json_path)
            .await
            .expect(format!("{} not found", secret_json_path).as_str());

        let auth = oauth2::ServiceAccountAuthenticator::builder(secret)
            .build()
            .await
            .expect("ServiceAccountAuthenticator::builder");

        let token = auth
            .token(&["https://www.googleapis.com/auth/cloud-vision"])
            .await
            .expect("auth.token");

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", token.token().unwrap())).unwrap());


        ImageProcessor::Label(LabelProcessor {
            dir: dir,
            secret: token,
            headers: headers,
        })
    }
    pub async fn new_caption(dir: String, secret_json_path: String, samples: u32) -> ImageProcessor {
        let secret = Self::read_secret(&secret_json_path)
            .await
            .expect(format!("{} not found", secret_json_path).as_str());

        let auth = oauth2::ServiceAccountAuthenticator::builder(secret)
            .build()
            .await
            .expect("ServiceAccountAuthenticator::builder");

        let token = auth
            .token(&["https://www.googleapis.com/auth/cloud-platform"])
            .await
            .expect("auth.token");

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", token.token().unwrap())).unwrap());

        ImageProcessor::Caption(CaptionProcessor {
            dir: dir,
            secret: token,
            headers: headers,
            samples: samples,
        })
    }

    pub async fn new_vision(dir: String, secret_json_path: String) -> Self {
        let secret = Self::read_secret(&secret_json_path)
            .await
            .expect(format!("{} not found", secret_json_path).as_str());

        let auth = oauth2::ServiceAccountAuthenticator::builder(secret)
            .build()
            .await
            .expect("ServiceAccountAuthenticator::builder");

        let token = auth
            .token(&["https://www.googleapis.com/auth/cloud-language", "https://www.googleapis.com/auth/cloud-platform"])
            .await
            .expect("auth.token");

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", token.token().unwrap())).unwrap());

        ImageProcessor::Vision(VisionProcessor {
            dir: dir,
            secret: token,
            headers: headers,
        })
    }

    async fn read_secret(secret_json_path: &String) -> std::io::Result<ServiceAccountKey> {
        let secret = oauth2::read_service_account_key(secret_json_path)
            .await;
        secret
    }
}

pub struct CaptionProcessor {
    dir: String,
    secret: AccessToken,
    headers: HeaderMap,
    samples: u32,
}

pub struct LabelProcessor {
    dir: String,
    secret: AccessToken,
    headers: HeaderMap,
}

pub struct VisionProcessor {
    dir: String,
    secret: AccessToken,
    headers: HeaderMap,
}

impl LabelProcessor {
    pub async fn process (
        &self,
        callback: Arc<Mutex<dyn Fn(Value) -> Result<(), Box<dyn Error>> + Send + 'static>>,
    ) -> Result<(), Box<dyn Error>> {
        let client = reqwest::Client::new();

        let mut tasks = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let mut file = File::open(path)?;
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)?;
                let encoded = encode(&buffer);

                let body = json!({
                "requests": [
                    {
                        "image": {
                            "content": encoded
                        },
                        "features": [
                            {
                                "maxResults": 30,
                                "type": "LABEL_DETECTION"
                            }
                        ]
                    }
                ]
            });

                let client = client.clone();
                let headers = self.headers.clone();

                let client_clone = client.clone();

                let callback_clone = callback.clone();

                let task = tokio::spawn(async move {
                    match client_clone
                        .post("https://vision.googleapis.com/v1/images:annotate")
                        .headers(headers)
                        .json(&body)
                        .send()
                        .await {
                        Ok(response) => {
                            match response.json::<Value>().await {
                                Ok(json_body) => {
                                    let cb = callback_clone.lock().unwrap();
                                    cb(json_body).unwrap_or_else(|e| eprintln!("Ошибка в колбэке: {}", e));
                                }
                                Err(e) => eprintln!("Ошибка при разборе JSON ответа: {}", e),
                            }
                        }
                        Err(e) => eprintln!("Ошибка отправки запроса: {}", e),
                    }
                });
                tasks.push(task);
            }
        }

        join_all(tasks).await;

        Ok(())
    }
}

impl CaptionProcessor {
    pub async fn process(
        &self,
        callback: Arc<Mutex<dyn Fn(Value) -> Result<(), Box<dyn Error>> + Send + 'static>>,
    ) -> Result<(), Box<dyn Error>> {
        let client = reqwest::Client::new();

        let mut tasks = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let mut file = File::open(path)?;
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)?;
                let encoded = encode(&buffer);

                let body = json!({
                "instances": [
                    {
                        "image": {
                            "bytesBase64Encoded": encoded
                        }
                    }
                ],
                "parameters": {
                    "sampleCount": self.samples,
                    "language": "en"
                }
            });

                let client = client.clone();
                let headers = self.headers.clone();
                let client_clone = client.clone();
                let callback_clone = callback.clone();

                let task = tokio::spawn(async move {
                    match client_clone
                        .post(r"https://us-central1-aiplatform.googleapis.com/v1/projects/hopeful-breaker-419323/locations/us-central1/publishers/google/models/imagetext:predict")
                        .headers(headers)
                        .json(&body)
                        .send()
                        .await {
                        Ok(response) => {
                            match response.json::<Value>().await {
                                Ok(json_body) => {
                                    let cb = callback_clone.lock().unwrap();
                                    cb(json_body).unwrap_or_else(|e| eprintln!("Ошибка в колбэке: {}", e));
                                }
                                Err(e) => eprintln!("Ошибка при разборе JSON ответа: {}", e),
                            }
                        }
                        Err(e) => eprintln!("Ошибка отправки запроса: {}", e),
                    }
                });
                tasks.push(task);
            }
        }

        join_all(tasks).await;

        Ok(())
    }
}

impl VisionProcessor {
    pub async fn process(&self, callback: Arc<Mutex<fn(Value) -> Result<(), Box<dyn Error>>>>) -> Result<(), Box<dyn Error>> {
        let client = reqwest::Client::new();

        let mut tasks = Vec::new();

        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let mut file = File::open(path)?;
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)?;
                let encoded = encode(&buffer);

                let body = json!(

                    {
                        "instances": [
                        {
                            "image": {
                              "bytesBase64Encoded": encoded
                            }
                        }
                    ],
                        "parameters": {
                            "sampleCount": 1,
                            "language": "en"
                        }
                    }
                );

                let client = client.clone();
                let headers = self.headers.clone();

                let client_clone = client.clone();

                let task = tokio::spawn(async move {
                    match client_clone
                        .post("https://us-central1-aiplatform.googleapis.com/v1/projects/hopeful-breaker-419323/locations/us-central1/publishers/google/models/imagetext:predict")
                        .headers(headers)
                        .json(&body)
                        .send()
                        .await {
                        Ok(response) => {
                            match response.json::<Value>().await {
                                Ok(json_body) => {
                                    println!("{}", json_body);
                                }
                                Err(e) => eprintln!("Ошибка при разборе JSON ответа: {}", e),
                            }
                        }
                        Err(e) => eprintln!("Ошибка отправки запроса: {}", e),
                    }
                    sleep(Duration::from_millis(1600)).await;
                });
                tasks.push(task);
            }
        }

        Ok(())
    }
}
