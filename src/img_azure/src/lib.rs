use std::io::ErrorKind;
use serde_json::Value;
use crate::azure_api::{AzureRequest, AzureResponse};

pub mod azure_api;

pub async fn get_response_by_path(path_str: &str) -> Result<AzureResponse, ErrorKind> {
    let mut request = AzureRequest::new("4d7bd39a70c249eebd19f5b8d62f5d7b", vec!["tags", "caption"]);
    request.set_img(path_str).unwrap();
    let response = request.send_request().await;
    if response.is_err() {
        println!("{:?}", response);
        return Err(ErrorKind::InvalidData)
    }
    let response = response.unwrap();
    let response_copy = response.json::<Value>().await.unwrap();
    let response_struct: Result<AzureResponse, ErrorKind> = AzureResponse::try_from(response_copy.clone());
    response_struct
}