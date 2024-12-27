use std::io::{self, Error, ErrorKind, Read};
use std::path::Path;

use serde::Serialize;
use serde_json;
use serde_yaml;
use tokio::{self, fs};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncWriteExt, AsyncBufReadExt, BufReader};
use toml;

pub const CONFIG_DIR: &str = "config";
pub const TRANSLATION_DIR: &str = "translation";
pub const TERM_DIR: &str = "term";

pub struct LazyFileReader {
    reader: BufReader<File>,
    chunk_size: usize,
    buffer: Vec<String>,
    call_count: usize,
    read_count: usize
}

impl LazyFileReader {
    pub async fn new(file_path: &str, chunk_size: usize, skip_lines: usize) -> io::Result<Self> {
        let file = File::open(file_path).await?;
        let mut reader = BufReader::new(file);

        let mut line = String::new();
        for _ in 0..skip_lines {
            reader.read_line(&mut line).await?;
            line.clear();
        }

        Ok(LazyFileReader {
            reader,
            chunk_size,
            buffer: Vec::new(),
            call_count: 0,
            read_count: 0
        })
    }

    pub async fn read_next_chunk(&mut self) -> io::Result<Option<String>> {
        self.call_count += 1;
        loop {
            self.read_count += 1;
            self.buffer.clear();
            let mut lines_read = 0;

            while lines_read < self.chunk_size {
                let mut line = String::new();
                let bytes_read = self.reader.read_line(&mut line).await?;

                if bytes_read == 0 {
                    break;
                }

                line = line.trim_end().to_string();
                self.buffer.push(line);
                lines_read += 1;
            }

            if self.buffer.is_empty() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "文件已读取完毕"));
            }

            if !self.buffer.iter().all(|line| line.trim().is_empty()) {
                return Ok(Some(self.buffer.join("\n")));
            }
        }
    }





    pub fn get_call_count(&self) -> usize {
        self.call_count
    }

    pub fn get_read_count(&self) -> usize {
        self.read_count
    }
}

pub async fn write_txt_overwrite(folder: &str, filename: &str, content: &str) -> io::Result<()> {
    let folder_path = Path::new(folder);
    if !folder_path.exists() {
        fs::create_dir_all(folder_path).await?;
    }

    let path = folder_path.join(filename);
    let mut file = File::create(path).await?;
    file.write_all(content.as_bytes()).await?;

    Ok(())
}

pub async fn write_txt_append(folder: &str, filename: &str, content: &str) -> io::Result<()> {
    let folder_path = Path::new(folder);
    if !folder_path.exists() {
        fs::create_dir_all(folder_path).await?;
    }

    let path = folder_path.join(filename);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path).await?;
    file.write_all(content.as_bytes()).await?;

    file.write_all(b"\n").await?;

    Ok(())
}

pub async fn write_json_overwrite<T: Serialize>(folder: &str, filename: &str, content: &T) -> io::Result<()> {
    let folder_path = Path::new(folder);
    if !folder_path.exists() {
        fs::create_dir_all(folder_path).await?;
    }

    let path = folder_path.join(filename);
    let mut file = File::create(path).await?;
    let json_data = serde_json::to_string(content)?;
    file.write_all(json_data.as_bytes()).await?;

    Ok(())
}

pub fn check_file_exists(file_path: &str) -> bool {
    let path = Path::new(file_path);

    path.exists()
}

pub fn read_file_content(file_path: &str) -> io::Result<String> {
    let path = Path::new(file_path);
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    let content = read_file(file_path)?;

    match extension {
        "txt" => Ok(content),
        "json" => parse_json(&content),
        "toml" => parse_toml(&content),
        "yaml" | "yml" => parse_yaml(&content),
        _ => Err(io::Error::new(io::ErrorKind::InvalidData, "不支持的文件类型")),
    }
}

fn read_file(file_path: &str) -> io::Result<String> {
    let mut file = std::fs::File::open(file_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

fn parse_json(content: &str) -> io::Result<String> {
    match serde_json::from_str::<serde_json::Value>(content) {
        Ok(_) => Ok(content.to_string()),
        Err(e) => Err(Error::new(ErrorKind::InvalidData, e)),
    }
}

fn parse_toml(content: &str) -> io::Result<String> {
    match toml::de::from_str::<toml::Value>(content) {
        Ok(_) => Ok(content.to_string()),
        Err(e) => Err(Error::new(ErrorKind::InvalidData, e)),
    }
}

fn parse_yaml(content: &str) -> io::Result<String> {
    match serde_yaml::from_str::<serde_yaml::Value>(content) {
        Ok(_) => Ok(content.to_string()),
        Err(e) => Err(Error::new(ErrorKind::InvalidData, e)),
    }
}

pub fn remove_extension(file_name: &str) -> String {
    let path = Path::new(file_name);
    path.file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| file_name.to_string())
}

pub fn get_filename(file_path: &str) -> Result<String, Error> {
    let path = Path::new(file_path);

    match path.file_name() {
        Some(name) => Ok(name.to_string_lossy().to_string()),
        None => Err(Error::new(ErrorKind::NotFound, "未找到文件名")),
    }
}
