pub mod processor;

use std::error::Error;
use futures::future::join_all;
use std::fs;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, AUTHORIZATION};
use std::fs::File;
use std::future::Future;
use std::io::Read;
use std::sync::{Arc, Mutex};
use base64::encode;
use reqwest::Response;
use serde_json::{json, Value};
use yup_oauth2 as oauth2;



pub async fn apply_for_labels (
    dir: &str,
    callback: Arc<Mutex<dyn Fn(Value) -> Result<(), Box<dyn Error>> + Send + 'static>>,
) -> Result<(), Box<dyn Error>> {
    let secret = oauth2::read_service_account_key("client_secret.json")
        .await
        .expect("client_secret.json");

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
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", token.token().unwrap()))?);
    let callback = Arc::new(callback);
    let mut tasks = Vec::new();

    let client = reqwest::Client::new();


    for entry in fs::read_dir(dir)? {
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
                                "type": "LABEL_DETECTION"
                            }
                        ]
                    }
                ]
            });

            let client = client.clone();
            let headers = headers.clone();

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
                            },
                            Err(e) => eprintln!("Ошибка при разборе JSON ответа: {}", e),
                        }
                    },
                    Err(e) => eprintln!("Ошибка отправки запроса: {}", e),
                }
            });
            tasks.push(task);
        }
    }

    join_all(tasks).await;

    Ok(())
}


pub async fn apply_for_caption (
    dir: &str,
    callback: Arc<Mutex<dyn Fn(Value) -> Result<(), Box<dyn Error>> + Send + 'static>>,
) -> Result<(), Box<dyn Error>>
{
    let secret = oauth2::read_service_account_key("client_secret.json")
        .await
        .expect("client_secret.json");

    let auth = oauth2::ServiceAccountAuthenticator::builder(secret)
        .build()
        .await
        .expect("ServiceAccountAuthenticator::builder");

    let token = auth
        .token(&["https://www.googleapis.com/auth/cloud-platform"])
        .await
        .expect("auth.token");

    let mut buffer = Vec::new();


    let client = reqwest::Client::new();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", token.token().unwrap()))?);

    let callback = Arc::new(callback);
    let mut tasks = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let mut file = File::open(path)?;
            buffer = Vec::new();
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
                    "sampleCount": 1,
                    "language": "en"
                }
            });

            let client = client.clone();
            let headers = headers.clone();
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
                            },
                            Err(e) => eprintln!("Ошибка при разборе JSON ответа: {}", e),
                        }
                    },
                    Err(e) => eprintln!("Ошибка отправки запроса: {}", e),
                }
            });
            tasks.push(task);
        }
    }

    join_all(tasks).await; // Дожидаемся выполнения всех задач

    Ok(())
}

use image::{GenericImageView, GenericImage, imageops::FilterType};

fn resize_image(path: &str) -> image::ImageResult<()> {
    let img = image::open(path)?;

    let (width, height) = img.dimensions();
    let new_dimensions = if width > height {
        (600, 600 * height / width)
    } else {
        (600 * width / height, 600)
    };

    let new_img = img.resize_exact(new_dimensions.0, new_dimensions.1, FilterType::Nearest);
    new_img.save("resized_image.jpg")?;

    Ok(())
}