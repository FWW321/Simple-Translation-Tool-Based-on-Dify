mod api;
mod config;
mod file_operations;

use crate::config::{ConfigData, load_config_from_file, load_api_config, APIConfig};
use crate::file_operations::{
    read_file_content, write_json_overwrite, write_txt_append, write_txt_overwrite,
    check_file_exists, get_filename, remove_extension, LazyFileReader, CONFIG_DIR, TERM_DIR, TRANSLATION_DIR
};
use api::{run_workflow, Input, RequestData};
use serde_json::Value;
use std::io::{self, Write};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::sync::mpsc::Sender;

#[tokio::main]
async fn main() {
    let input_file_path = get_input_file_path();
    if !check_file_exists(&input_file_path) {
        println!("文件不存在: {}", input_file_path);
        return;
    }

    let input_file_name = get_filename(&input_file_path).unwrap();
    let input_file_base_name = remove_extension(&input_file_name);

    let config_data = load_config_from_file(&input_file_path).unwrap_or_else(|| create_default_config());
    let config_data = Arc::new(config_data);
    let term = get_term_file_path(&input_file_base_name);

    let output_key = get_output_key();
    let num_lines = get_num_lines();
    let task_num = get_task_num();

    let (tx, rx) = mpsc::channel::<(usize, usize, Result<Value, String>)>(1024);

    let reader = Arc::new(Mutex::new(LazyFileReader::new(&input_file_path, num_lines, config_data.history_lines).await.unwrap()));
    let api_config = Arc::new(get_api_config().unwrap());

    let handles = spawn_translation_tasks(
        task_num,
        Arc::clone(&api_config),
        Arc::clone(&config_data),
        Arc::clone(&term),
        Arc::clone(&reader),
        tx.clone()
    ).await;

    process_results(task_num, tx, rx, num_lines, &output_key, &input_file_base_name, &config_data, &term).await;

    for handle in handles {
        handle.await.unwrap();
    }
}

fn get_input_file_path() -> String {
    let mut input_file_path = String::new();
    print!("请输入文件名: ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut input_file_path).unwrap();
    input_file_path.trim().trim_matches('"').to_string()
}

fn get_num_lines() -> usize {
    let mut num_lines = String::new();
    print!("请输入num_lines: ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut num_lines).unwrap();
    num_lines.trim().parse().unwrap()
}

fn get_task_num() -> usize {
    let mut task_num = String::new();
    print!("请输入task_num: ");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut task_num).unwrap();
    task_num.trim().parse().unwrap()
}

fn get_output_key() -> String {
    let mut output_key = String::new();
    println!("请输入变量名作为输出(默认为output):");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut output_key).unwrap();

    let output_key = output_key.trim();
    if output_key.is_empty() {
        "output".to_string()
    } else {
        output_key.to_string()
    }
}

fn create_default_config() -> ConfigData {
    let target_lang = get_input_string("请输入target_lang: ");
    let source_lang = get_input_string("请输入source_lang: ");
    ConfigData {
        target_lang,
        source_lang,
        history_lines: 0,
    }
}

fn get_api_config() -> Result<APIConfig, String> {
    let config_path = format!("{}/user.yaml", CONFIG_DIR);
    let api_config = load_api_config(&config_path).map_err(|e| e.to_string())?;
    Ok(api_config)
}

fn get_input_string(prompt: &str) -> String {
    let mut input = String::new();
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn get_term_file_path(input_file_base_name: &str) -> Arc<String> {
    let mut term = get_input_string("请输入术语表路径(默认term): ");
    if term.is_empty() {
        term = format!("{}\\{}_term.txt", TERM_DIR, input_file_base_name);
    }

    if check_file_exists(&term) {
        Arc::new(read_file_content(&term).unwrap())
    } else {
        Arc::new(String::new())
    }
}

async fn spawn_translation_tasks(
    task_num: usize,
    api_config: Arc<APIConfig>,
    config_data: Arc<ConfigData>,
    term: Arc<String>,
    reader: Arc<Mutex<LazyFileReader>>,
    tx: Sender<(usize, usize, Result<Value, String>)>
) -> Vec<tokio::task::JoinHandle<()>> {
    let mut handles = Vec::new();

    for i in 0..task_num {
        println!("正在创建工作流{}...\n", i);
        let config_data = Arc::clone(&config_data);
        let term = Arc::clone(&term);
        let reader = Arc::clone(&reader);
        let tx = tx.clone();
        let api_config = Arc::clone(&api_config);

        let handle = tokio::spawn(create_task(i, api_config, config_data, term, reader, tx));
        handles.push(handle);
    }

    handles
}

async fn create_task(
    task_id: usize,
    api_config: Arc<APIConfig>,
    config_data: Arc<ConfigData>,
    term: Arc<String>,
    reader: Arc<Mutex<LazyFileReader>>,
    tx: Sender<(usize, usize, Result<Value, String>)>
) -> () {
    loop {
        println!("工作流{}正在读取下一块数据...\n", task_id);
        let (chunk, count, read_count);
        {
            let mut reader = reader.lock().await;
            chunk = reader.read_next_chunk().await;
            count = reader.get_call_count();
            read_count = reader.get_read_count();
        }

        if let Ok(Some(value)) = chunk {
            let result = process_task(&config_data, &api_config, &term, value).await;
            tx.send((count, read_count, result)).await.unwrap();
        } else {
            println!("工作流{}已结束\n", task_id);
            tx.send((0, 0, Err("文件结束".to_string()))).await.unwrap();
            break;
        }
    }
}

async fn process_task(config_data: &Arc<ConfigData>, api_config: &Arc<APIConfig>, term: &Arc<String>, value: String) -> Result<Value, String> {
    let user_id = "fww";
    let response_mode = "streaming";
    let input = Input::new(&config_data.target_lang, value, &config_data.source_lang, &term);
    let request_data = RequestData::new(input, response_mode, user_id);
    let result = run_workflow(&api_config.api_key, &api_config.base_url, &request_data).await;

    match result {
        Ok(Some(outputs)) => Ok(outputs),
        Ok(None) => Err("返回结果为空".to_string()),
        Err(err) => Err(format!("请求出错: {}", err)),
    }
}

async fn process_results(
    task_num: usize,
    tx: Sender<(usize, usize, Result<Value, String>)>,
    mut rx: mpsc::Receiver<(usize, usize, Result<Value, String>)>,
    num_lines: usize,
    output_key: &str,
    input_file_base_name: &str,
    config_data: &ConfigData,
    term: &Arc<String>
) {
    let mut received = 0;
    let mut end = 0;

    loop {
        if end == task_num && rx.is_empty() {
            drop(tx);
            break;
        }

        if let Some((count, read_count, result)) = rx.recv().await {
            handle_message(count, read_count, result, &mut received, &mut end, num_lines, output_key, input_file_base_name, config_data, term, &tx).await;
        }
    }
}

async fn handle_message(
    count: usize,
    read_count: usize,
    result: Result<Value, String>,
    received: &mut usize,
    end: &mut usize,
    num_lines: usize,
    output_key: &str,
    input_file_base_name: &str,
    config_data: &ConfigData,
    term: &Arc<String>,
    tx: &Sender<(usize, usize, Result<Value, String>)>
) {
    if count == 0 {
        *end += 1;
    } else if count == *received + 1 {
        process_normal_result(count, read_count, result, output_key, input_file_base_name, config_data, term, num_lines).await;
        *received += 1;
    } else {
        tx.send((count, read_count, result)).await.unwrap();
    }
}

async fn process_normal_result(
    count: usize,
    read_count: usize,
    result: Result<Value, String>,
    output_key: &str,
    input_file_base_name: &str,
    config_data: &ConfigData,
    term: &Arc<String>,
    num_lines: usize
) {
    if let Ok(data) = result {
        let translation = data.get(output_key).unwrap().as_str().unwrap();
        write_translation_to_file(input_file_base_name, config_data, translation).await;
        write_term_if_needed(term, input_file_base_name).await;
        update_config_data(config_data, input_file_base_name, config_data.history_lines + read_count * num_lines).await;
        println!("chunk {} 已返回结果", count);
    } else {
        println!("chunk {} 未返回结果", count);
    }
}

async fn write_translation_to_file(
    input_file_base_name: &str,
    config_data: &ConfigData,
    translation: &str
) {
    write_txt_append(
        TRANSLATION_DIR,
        &format!("{}_{}2{}.txt", input_file_base_name, config_data.source_lang, config_data.target_lang),
        translation,
    ).await.unwrap();
}


async fn write_term_if_needed(term: &Arc<String>, input_file_base_name: &str) {
    if !term.is_empty() {
        write_txt_overwrite(TERM_DIR, &format!("{}_term.txt", input_file_base_name), &term).await.unwrap();
    }
}

async fn update_config_data(config_data: &ConfigData, file_name: &str, history_lines: usize) {
    let new_config_data = ConfigData {
        target_lang: config_data.target_lang.clone(),
        source_lang: config_data.source_lang.clone(),
        history_lines,
    };
    write_json_overwrite(CONFIG_DIR, &format!("{}.json", file_name), &new_config_data).await.unwrap();
}
