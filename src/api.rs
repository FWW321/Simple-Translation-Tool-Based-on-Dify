use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug)]
pub struct Input<'a> {
    target_lang: &'a str,
    source_text: String,
    source_lang: &'a str,
    term: &'a str,
}

impl<'a > Input<'a> {
    pub fn new(target_lang: &'a str, source_text: String, source_lang: &'a str, term: &'a str) -> Self {
        Input {
            target_lang,
            source_text,
            source_lang,
            term,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestData<'a> {
    inputs: Input<'a>,
    user: &'a str,
    response_mode: &'a str
}

impl<'a> RequestData<'a> {
    pub fn new(inputs: Input<'a>, response_mode: &'a str, user: &'a str) -> Self {
        RequestData {
            inputs,
            user,
            response_mode
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct WorkflowResponse {
    data: Option<WorkflowData>,
}

#[derive(Serialize, Deserialize, Debug)]
struct WorkflowData {
    outputs: Option<Value>,
}

pub async fn run_workflow<'a>(
    api_key: &str,
    base_url: &str,
    request_data: &RequestData<'a>
) -> Result<Option<Value>, String> {
    let url = format!("{}/v1/workflows/run", base_url);
    let client = Client::new();

    println!("工作流正在运行 {}\n", url);

    if let Err(e) = log_request_data(request_data) {
        return Err(e);
    }

    let mut response = send_post_request(&client, &url, api_key, request_data).await?;

    process_response(&mut response).await
}

fn log_request_data(request_data: &RequestData) -> Result<(), String> {
    let serialized_data = serde_json::to_string(request_data)
        .map_err(|e| e.to_string())?;

    println!("Sending: {}\n", serialized_data);
    Ok(())
}

async fn send_post_request<'a>(
    client: &Client,
    url: &str,
    api_key: &str,
    request_data: &RequestData<'a>
) -> Result<reqwest::Response, String> {
    client
        .post(url)
        .json(request_data)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())
}

async fn process_response(response: &mut reqwest::Response) -> Result<Option<Value>, String> {
    let mut buffer = Vec::new();

    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        buffer.extend_from_slice(chunk.as_ref());
        if let Some(pos) = find_event_data_position(&buffer) {
            let data = extract_data(&buffer, pos)?;
            if let Some(event_data) = process_event_data(data)? {
                return Ok(Some(event_data));
            }
            buffer.drain(..pos + 2);
        }
    }

    Ok(None)
}

fn find_event_data_position(buffer: &[u8]) -> Option<usize> {
    buffer.windows(2).position(|window| window == b"\n\n")
}

fn extract_data(buffer: &[u8], pos: usize) -> Result<&str, String> {
    let data = &buffer[..pos];
    std::str::from_utf8(data).map_err(|e| e.to_string())
}

fn process_event_data(data: &str) -> Result<Option<Value>, String> {
    if let Some(data_content) = data.strip_prefix("data: ") {
        let event_data = data_content.trim().to_string();
        println!("Received event data: {}\n", event_data);

        let json_data = serde_json::from_str::<Value>(&event_data)
            .map_err(|e| format!("event data转换失败: {}", e.to_string()))?;

        if let Some(event) = json_data.get("event").and_then(|e| e.as_str()) {
            if event == "workflow_finished" {
                if let Some(outputs) = json_data.get("data").and_then(|d| d.get("outputs")) {
                    println!("Workflow finished with outputs: {}\n", outputs);
                    return Ok(Some(outputs.clone()));
                }
            }
        }
    }
    Ok(None)
}
