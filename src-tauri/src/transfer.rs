//! 传输引擎：上传（分片/断点/重试）与下载（流式/Range 断点），只由用户手动触发。

use crate::error::{AppError, Result};
use crate::model::{now_millis, TransferItem};
use crate::pds::{calc_chunk, calc_part_count};
use crate::state::Shared;
use futures_util::StreamExt;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;

pub const EVT_UPDATE: &str = "transfer://update";
pub const EVT_LIST: &str = "transfer://list";

#[derive(Debug, Clone)]
pub struct UploadCtx {
    pub file_id: String,
    pub upload_id: String,
    pub cloud_name: String,
    pub chunk: u64,
    pub part_count: u32,
}

struct Meter {
    last_at: Instant,
    last_bytes: u64,
    speed: f64,
    last_emit: Instant,
}

impl Default for Meter {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            last_at: now,
            last_bytes: 0,
            speed: 0.0,
            last_emit: now,
        }
    }
}

pub struct Task {
    pub id: String,
    pub item: Mutex<TransferItem>,
    pub cancel: AtomicBool,
    pub upload_ctx: Mutex<Option<UploadCtx>>,
    pub part_urls: Mutex<HashMap<u32, String>>,
    pub resumed: AtomicBool,
    meter: Mutex<Meter>,
}

impl Task {
    pub fn new(
        kind: &str,
        name: String,
        local_path: String,
        size: u64,
        remote_file_id: Option<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            item: Mutex::new(TransferItem {
                id: String::new(), // 下面统一回填
                kind: kind.to_string(),
                name,
                local_path,
                size,
                transferred: 0,
                state: "queued".to_string(),
                speed_bps: 0,
                error: None,
                remote_file_id,
                created_at: now_millis(),
                finished_at: None,
            }),
            cancel: AtomicBool::new(false),
            upload_ctx: Mutex::new(None),
            part_urls: Mutex::new(HashMap::new()),
            resumed: AtomicBool::new(false),
            meter: Mutex::new(Meter::default()),
        }
        .with_id()
    }

    fn with_id(self) -> Self {
        let id = self.id.clone();
        self.item.lock().unwrap().id = id;
        self
    }

    pub fn from_history(item: TransferItem) -> Self {
        let id = if item.id.is_empty() {
            uuid::Uuid::new_v4().to_string()
        } else {
            item.id.clone()
        };
        Self {
            id: id.clone(),
            item: Mutex::new(item),
            cancel: AtomicBool::new(false),
            upload_ctx: Mutex::new(None),
            part_urls: Mutex::new(HashMap::new()),
            resumed: AtomicBool::new(false),
            meter: Mutex::new(Meter::default()),
        }
    }

    pub fn snapshot(&self) -> TransferItem {
        self.item.lock().unwrap().clone()
    }

    pub fn is_active(&self) -> bool {
        let s = self.item.lock().unwrap().state.clone();
        s == "running" || s == "queued"
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    /// 取消信号 future（用于 select! 中断网络等待）
    pub async fn cancelled(&self) {
        while !self.is_cancelled() {
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }

    pub fn mark_running(&self) {
        let mut item = self.item.lock().unwrap();
        if item.state == "queued" || item.state == "error" || item.state == "canceled" {
            item.state = "running".into();
            item.error = None;
            item.finished_at = None;
        }
        *self.meter.lock().unwrap() = Meter::default();
    }

    pub fn set_state(&self, state: &str) {
        let mut item = self.item.lock().unwrap();
        item.state = state.to_string();
    }

    pub fn set_error(&self, message: String) {
        let mut item = self.item.lock().unwrap();
        item.state = "error".into();
        item.error = Some(message);
        item.speed_bps = 0;
        item.finished_at = Some(now_millis());
    }

    pub fn finish(&self, state: &str) {
        let mut item = self.item.lock().unwrap();
        item.state = state.to_string();
        item.speed_bps = 0;
        item.finished_at = Some(now_millis());
    }

    pub fn set_name(&self, name: &str) {
        if !name.is_empty() {
            self.item.lock().unwrap().name = name.to_string();
        }
    }

    pub fn set_local_path(&self, path: &Path) {
        self.item.lock().unwrap().local_path = path.to_string_lossy().to_string();
    }

    pub fn set_remote_id(&self, id: &str) {
        if !id.is_empty() {
            self.item.lock().unwrap().remote_file_id = Some(id.to_string());
        }
    }

    pub fn set_size(&self, size: u64) {
        if size > 0 {
            self.item.lock().unwrap().size = size;
        }
    }

    /// 更新进度并节流计算速度；返回 Some(speed) 表示这次应该推送给前端
    pub fn set_progress(&self, bytes: u64) -> Option<u64> {
        let mut item = self.item.lock().unwrap();
        item.transferred = bytes;
        let now = Instant::now();
        let mut m = self.meter.lock().unwrap();
        let dt = now.duration_since(m.last_at).as_secs_f64();
        if dt >= 0.1 {
            let delta = bytes.saturating_sub(m.last_bytes) as f64;
            let inst = delta / dt;
            m.speed = if m.speed <= 0.0 {
                inst
            } else {
                m.speed * 0.7 + inst * 0.3
            };
            m.last_at = now;
            m.last_bytes = bytes;
        }
        item.speed_bps = m.speed.max(0.0) as u64;
        if now.duration_since(m.last_emit) >= Duration::from_millis(180) {
            m.last_emit = now;
            Some(item.speed_bps)
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------- 对外入口

impl Shared {
    /// 手动触发上传；返回任务 id 列表
    pub fn start_uploads(self: &Arc<Self>, paths: Vec<String>) -> Vec<String> {
        let mut ids = Vec::new();
        for p in paths {
            let path = PathBuf::from(&p);
            let name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| p.clone());
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            let task = Arc::new(Task::new("upload", name, p.clone(), size, None));
            ids.push(task.id.clone());
            self.push_task(task.clone());
            let shared = self.clone();
            tauri::async_runtime::spawn(async move {
                run_upload(shared, task).await;
            });
        }
        self.emit_list();
        ids
    }

    /// 手动触发下载；files 为 (file_id, name, size)
    pub fn start_downloads(
        self: &Arc<Self>,
        files: Vec<(String, String, u64)>,
    ) -> Vec<String> {
        let settings = self.settings();
        let dir = settings.download_dir.clone();
        let mut ids = Vec::new();
        for (file_id, name, size) in files {
            let dest = unique_download_path(Path::new(&dir), &name);
            let task = Arc::new(Task::new(
                "download",
                name.clone(),
                dest.to_string_lossy().to_string(),
                size,
                Some(file_id),
            ));
            ids.push(task.id.clone());
            self.push_task(task.clone());
            let shared = self.clone();
            tauri::async_runtime::spawn(async move {
                run_download(shared, task).await;
            });
        }
        self.emit_list();
        ids
    }

    pub fn cancel_transfer(&self, id: &str) -> bool {
        if let Some(task) = self.find_task(id) {
            if task.is_active() {
                task.cancel();
                return true;
            }
        }
        false
    }

    pub fn retry_transfer(self: &Arc<Self>, id: &str) -> Result<String> {
        let task = self
            .find_task(id)
            .ok_or_else(|| AppError::msg("任务不存在"))?;
        if task.is_active() {
            return Err(AppError::msg("任务正在进行中"));
        }
        task.cancel.store(false, Ordering::SeqCst);
        {
            let mut item = task.item.lock().unwrap();
            item.state = "queued".into();
            item.error = None;
            item.speed_bps = 0;
            item.finished_at = None;
            if item.kind == "upload" {
                item.transferred = 0;
            }
        }
        *task.meter.lock().unwrap() = Meter::default();
        task.resumed.store(
            matches!(task.snapshot().kind.as_str(), "upload") && task.upload_ctx.lock().unwrap().is_some(),
            Ordering::SeqCst,
        );
        self.emit_item(&task.snapshot());
        let shared = self.clone();
        let t2 = task.clone();
        tauri::async_runtime::spawn(async move {
            if t2.snapshot().kind == "upload" {
                run_upload(shared, t2).await;
            } else {
                run_download(shared, t2).await;
            }
        });
        Ok(task.id.clone())
    }
}

async fn run_upload(shared: Arc<Shared>, task: Arc<Task>) {
    let sem = shared.sem.read().unwrap().clone();
    let permit = sem.acquire_owned().await.ok();
    if task.is_cancelled() {
        task.finish("canceled");
        shared.emit_item(&task.snapshot());
        return;
    }
    task.mark_running();
    shared.emit_item(&task.snapshot());
    let result = upload_inner(&shared, &task).await;
    drop(permit);
    if result.is_ok() {
        task.set_progress(task.snapshot().size);
        task.finish("success");
    } else if task.is_cancelled() {
        task.finish("canceled");
    } else {
        let msg = result.err().map(|e| e.to_string()).unwrap_or_default();
        task.set_error(msg);
    }
    shared.emit_item(&task.snapshot());
    shared.persist_history();
    shared.emit_list();
}

async fn run_download(shared: Arc<Shared>, task: Arc<Task>) {
    let sem = shared.sem.read().unwrap().clone();
    let permit = sem.acquire_owned().await.ok();
    if task.is_cancelled() {
        task.finish("canceled");
        shared.emit_item(&task.snapshot());
        return;
    }
    task.mark_running();
    shared.emit_item(&task.snapshot());
    let result = download_inner(&shared, &task).await;
    drop(permit);
    if result.is_ok() {
        task.finish("success");
    } else if task.is_cancelled() {
        task.finish("canceled");
    } else {
        let msg = result.err().map(|e| e.to_string()).unwrap_or_default();
        task.set_error(msg);
    }
    shared.emit_item(&task.snapshot());
    shared.persist_history();
    shared.emit_list();
}

// ---------------------------------------------------------------- 上传实现

async fn upload_inner(shared: &Arc<Shared>, task: &Arc<Task>) -> Result<()> {
    for attempt in 0..3 {
        match upload_attempt(shared, task).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                if attempt < 2 && is_session_invalid(&e) {
                    // 上传会话失效：清空会话，重新创建后自动再试
                    *task.upload_ctx.lock().unwrap() = None;
                    task.part_urls.lock().unwrap().clear();
                    continue;
                }
                return Err(e);
            }
        }
    }
    Err(AppError::msg("上传失败，请点击重试"))
}

fn is_session_invalid(e: &AppError) -> bool {
    match e {
        AppError::Api { code, message, .. } => {
            code.contains("NotFound.UploadId")
                || code.contains("NoSuchUpload")
                || message.contains("upload is not exist")
                || message.contains("已失效")
        }
        _ => false,
    }
}

async fn upload_attempt(shared: &Arc<Shared>, task: &Arc<Task>) -> Result<()> {
    let settings = shared.settings();
    let client = shared.client()?;
    let item = task.snapshot();
    let path = PathBuf::from(&item.local_path);
    let meta = tokio::fs::metadata(&path)
        .await
        .map_err(|e| AppError::msg(format!("无法读取本地文件：{e}")))?;
    if meta.is_dir() {
        return Err(AppError::msg("暂不支持上传文件夹，请选择单个文件"));
    }
    let size = meta.len();
    task.set_size(size);
    let chunk = calc_chunk(size, settings.chunk_size_mb);
    let part_count = calc_part_count(size, chunk);
    let local_name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "未命名文件".into());

    let folder = client
        .ensure_transfer_folder(&settings.transfer_folder_name)
        .await?;

    // 1) 复用或创建上传会话
    let mut existing = task.upload_ctx.lock().unwrap().clone();
    if let Some(c) = &existing {
        if c.chunk != chunk || c.part_count != part_count || c.cloud_name != local_name {
            existing = None;
        }
    }
    let ctx = match existing {
        Some(c) => {
            task.resumed.store(true, Ordering::SeqCst);
            c
        }
        None => {
            let cu = client
                .create_upload(&folder.file_id, &local_name, size, part_count)
                .await?;
            if cu.rapid_upload || cu.exist {
                task.set_remote_id(&cu.file_id);
                if let Some(n) = &cu.file_name {
                    task.set_name(n);
                }
                task.set_progress(size);
                return Ok(());
            }
            let c = UploadCtx {
                file_id: cu.file_id.clone(),
                upload_id: cu.upload_id.clone(),
                cloud_name: cu.file_name.clone().unwrap_or_else(|| local_name.clone()),
                chunk,
                part_count,
            };
            *task.upload_ctx.lock().unwrap() = Some(c.clone());
            let mut urls = task.part_urls.lock().unwrap();
            urls.clear();
            for p in &cu.part_info_list {
                if p.part_number > 0 && !p.upload_url.is_empty() {
                    urls.insert(p.part_number, p.upload_url.clone());
                }
            }
            c
        }
    };
    task.set_remote_id(&ctx.file_id);
    task.set_name(&ctx.cloud_name);

    // 2) 断点续传：查询已完成分片
    let mut uploaded: HashSet<u32> = HashSet::new();
    if task.resumed.swap(false, Ordering::SeqCst) {
        let parts = client
            .list_uploaded_parts(&ctx.file_id, &ctx.upload_id)
            .await?;
        uploaded = parts.into_iter().collect();
        task.part_urls.lock().unwrap().clear();
    }

    if part_count == 0 {
        // 0 字节文件：直接完成
        client.complete_upload(&ctx.file_id, &ctx.upload_id).await?;
        task.set_progress(0);
        return Ok(());
    }

    // 3) 待传分片 + 批量换取上传地址
    let missing: Vec<u32> = (1..=part_count)
        .filter(|n| !uploaded.contains(n))
        .collect();
    if !missing.is_empty() {
        let urls = client
            .get_upload_urls(&ctx.file_id, &ctx.upload_id, &missing)
            .await?;
        *task.part_urls.lock().unwrap() = urls;
    }
    let done = Arc::new(AtomicU64::new(uploaded_bytes(
        &uploaded,
        part_count,
        chunk,
        size,
    )));
    task.set_progress(done.load(Ordering::SeqCst));
    shared.emit_item(&task.snapshot());

    // 4) 并发上传分片
    let concurrency = settings.parallel_parts.clamp(1, 4) as usize;
    let sem = Arc::new(tokio::sync::Semaphore::new(concurrency));
    let mut join = tokio::task::JoinSet::new();
    for n in missing {
        let client = client.clone();
        let path = path.clone();
        let urls = Arc::new(Mutex::new(task.part_urls.lock().unwrap().clone()));
        let task2 = task.clone();
        let sem2 = sem.clone();
        let done2 = done.clone();
        let shared2 = shared.clone();
        let file_id = ctx.file_id.clone();
        let upload_id = ctx.upload_id.clone();
        join.spawn(async move {
            let _permit = sem2.acquire_owned().await.ok();
            if task2.is_cancelled() {
                return Err(AppError::Cancelled);
            }
            let offset = (n as u64 - 1) * chunk;
            let len = part_len(n, part_count, chunk, size) as usize;
            let read_path = path.clone();
            let data =
                tokio::task::spawn_blocking(move || read_part(&read_path, offset, len))
                    .await
                    .map_err(|e| AppError::msg(format!("读取分片失败：{e}")))??;
            let mut attempt = 0u32;
            loop {
                if task2.is_cancelled() {
                    return Err(AppError::Cancelled);
                }
                let mut url = { urls.lock().unwrap().get(&n).cloned() };
                if url.is_none() {
                    let m = client
                        .get_upload_urls(&file_id, &upload_id, std::slice::from_ref(&n))
                        .await?;
                    url = m.get(&n).cloned();
                }
                let url = match url {
                    Some(u) => u,
                    None => return Err(AppError::msg(format!("第 {n} 片没有拿到上传地址"))),
                };
                let result = tokio::select! {
                    r = client.put_part(&url, data.clone()) => r,
                    _ = task2.cancelled() => Err(AppError::Cancelled),
                };
                match result {
                    Ok(()) => break,
                    Err(AppError::Cancelled) => return Err(AppError::Cancelled),
                    Err(e) => {
                        attempt += 1;
                        if attempt >= 4 {
                            return Err(e);
                        }
                        if e.is_upload_url_expired() {
                            let m = client
                                .get_upload_urls(&file_id, &upload_id, std::slice::from_ref(&n))
                                .await?;
                            if let Some(u) = m.get(&n) {
                                urls.lock().unwrap().insert(n, u.clone());
                            }
                        } else if e.is_retryable() {
                            tokio::time::sleep(backoff(attempt)).await;
                        } else {
                            return Err(e);
                        }
                    }
                }
            }
            let cur = done2.fetch_add(len as u64, Ordering::SeqCst) + len as u64;
            if task2.set_progress(cur).is_some() {
                shared2.emit_item(&task2.snapshot());
            }
            Ok::<(), AppError>(())
        });
    }
    while let Some(res) = join.join_next().await {
        match res {
            Ok(r) => r?,
            Err(e) => return Err(AppError::msg(format!("分片任务异常：{e}"))),
        }
    }

    // 5) 完成上传并校验大小
    let v = client.complete_upload(&ctx.file_id, &ctx.upload_id).await?;
    let got_size = match v.get("size") {
        Some(serde_json::Value::Number(n)) => n.as_u64(),
        Some(serde_json::Value::String(s)) => s.parse::<u64>().ok(),
        _ => None,
    };
    if let Some(gs) = got_size {
        if gs != size {
            return Err(AppError::msg(format!(
                "云端文件大小不一致（本地 {size} / 云端 {gs}），请重试"
            )));
        }
    }
    task.set_progress(size);
    Ok(())
}

fn uploaded_bytes(uploaded: &HashSet<u32>, part_count: u32, chunk: u64, size: u64) -> u64 {
    uploaded
        .iter()
        .map(|n| part_len(*n, part_count, chunk, size))
        .sum()
}

fn part_len(n: u32, part_count: u32, chunk: u64, size: u64) -> u64 {
    if n >= part_count {
        size.saturating_sub((part_count as u64 - 1) * chunk)
    } else {
        chunk
    }
}

fn read_part(path: &Path, offset: u64, len: usize) -> std::io::Result<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(path)?;
    f.seek(SeekFrom::Start(offset))?;
    let mut buf = vec![0u8; len];
    f.read_exact(&mut buf)?;
    Ok(buf)
}

fn backoff(attempt: u32) -> Duration {
    let ms = 400u64 * 2u64.pow(attempt.min(4));
    Duration::from_millis(ms.min(8000))
}

// ---------------------------------------------------------------- 下载实现

async fn download_inner(shared: &Arc<Shared>, task: &Arc<Task>) -> Result<()> {
    let settings = shared.settings();
    let client = shared.client()?;
    let item = task.snapshot();
    let file_id = item
        .remote_file_id
        .clone()
        .ok_or_else(|| AppError::msg("任务缺少云端文件 ID"))?;
    let size = item.size;
    let dir = PathBuf::from(&settings.download_dir);
    tokio::fs::create_dir_all(&dir).await?;
    let final_path = unique_download_path(&dir, &item.name);
    let tmp = tmp_path_for(&final_path);

    let mut last_err: Option<AppError> = None;
    for attempt in 0..3 {
        if task.is_cancelled() {
            return Err(AppError::Cancelled);
        }
        let resume_from = tokio::fs::metadata(&tmp).await.map(|m| m.len()).unwrap_or(0);
        if size > 0 && resume_from >= size {
            tokio::fs::rename(&tmp, &final_path).await?;
            task.set_local_path(&final_path);
            task.set_progress(size);
            return Ok(());
        }
        let transferred = if size > 0 { resume_from.min(size) } else { 0 };
        task.set_progress(transferred);

        let info = match client.get_download_url(&file_id).await {
            Ok(i) => i,
            Err(e) => {
                last_err = Some(e);
                tokio::time::sleep(backoff(attempt)).await;
                continue;
            }
        };
        let mut req = client.http().get(&info.url);
        if transferred > 0 {
            req = req.header(reqwest::header::RANGE, format!("bytes={transferred}-"));
        }
        let resp = tokio::select! {
            r = req.send() => r?,
            _ = task.cancelled() => return Err(AppError::Cancelled),
        };
        let status = resp.status();
        if status.as_u16() == 416 {
            // Range 失效（云端文件变了）：从头开始
            let _ = tokio::fs::remove_file(&tmp).await;
            continue;
        }
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            last_err = Some(AppError::msg(format!(
                "下载失败（HTTP {}）：{}",
                status.as_u16(),
                text.chars().take(160).collect::<String>()
            )));
            tokio::time::sleep(backoff(attempt)).await;
            continue;
        }
        let append = transferred > 0 && status.as_u16() == 206;
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(append)
            .truncate(!append)
            .open(&tmp)
            .await?;
        let mut stream = resp.bytes_stream();
        let mut local = transferred;
        let mut stream_error: Option<AppError> = None;
        loop {
            let next = tokio::select! {
                n = stream.next() => n,
                _ = task.cancelled() => return Err(AppError::Cancelled),
            };
            match next {
                Some(Ok(bytes)) => {
                    if let Err(e) = file.write_all(&bytes).await {
                        stream_error = Some(AppError::Io(e));
                        break;
                    }
                    local += bytes.len() as u64;
                    if task.set_progress(local).is_some() {
                        shared.emit_item(&task.snapshot());
                    }
                }
                Some(Err(e)) => {
                    stream_error = Some(AppError::Http(e));
                    break;
                }
                None => break,
            }
        }
        let _ = file.flush().await;
        drop(file);
        if let Some(e) = stream_error {
            last_err = Some(e);
            if attempt < 2 {
                tokio::time::sleep(backoff(attempt)).await;
                continue;
            }
            return Err(last_err.unwrap());
        }
        if size > 0 && local < size {
            last_err = Some(AppError::msg(format!(
                "下载中断（{local}/{size} 字节），点击重试可续传"
            )));
            if attempt < 2 {
                continue;
            }
            return Err(last_err.unwrap());
        }
        tokio::fs::rename(&tmp, &final_path).await?;
        task.set_local_path(&final_path);
        task.set_progress(if size > 0 { size } else { local });
        return Ok(());
    }
    Err(last_err.unwrap_or_else(|| AppError::msg("下载失败，请重试")))
}

// ---------------------------------------------------------------- 工具

pub fn sanitize_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c == '/' || c == '\\' || c == ':' || c == '*' || c == '?' || c == '"' || c == '<'
                || c == '>' || c == '|' || c == '\0'
            {
                '_'
            } else {
                c
            }
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.').to_string();
    if trimmed.is_empty() {
        "未命名文件".into()
    } else {
        trimmed
    }
}

pub fn unique_download_path(dir: &Path, name: &str) -> PathBuf {
    let safe = sanitize_name(name);
    let candidate = dir.join(&safe);
    if !candidate.exists() && !tmp_path_for(&candidate).exists() {
        return candidate;
    }
    let p = Path::new(&safe);
    let stem = p
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| safe.clone());
    let ext = p
        .extension()
        .map(|s| format!(".{}", s.to_string_lossy()))
        .unwrap_or_default();
    for i in 1..1000 {
        let candidate = dir.join(format!("{stem} ({i}){ext}"));
        if !candidate.exists() && !tmp_path_for(&candidate).exists() {
            return candidate;
        }
    }
    dir.join(format!("{stem}-{}{ext}", now_millis()))
}

fn tmp_path_for(final_path: &Path) -> PathBuf {
    let mut s = final_path.as_os_str().to_owned();
    s.push(".hpart");
    PathBuf::from(s)
}

/// 供命令层使用：读取任务快照并推送
pub fn emit_snapshot(shared: &Shared, task: &Task) {
    shared.emit_item(&task.snapshot());
    shared.emit_list();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn part_lengths() {
        let size = 10 * 1024 * 1024 + 123;
        let chunk = calc_chunk(size, 8);
        let parts = calc_part_count(size, chunk);
        let total: u64 = (1..=parts).map(|n| part_len(n, parts, chunk, size)).sum();
        assert_eq!(total, size);
        assert_eq!(part_len(1, parts, chunk, size), chunk);
    }

    async fn wait_finished(shared: &Arc<Shared>, id: &str) -> TransferItem {
        for _ in 0..1800 {
            if let Some(t) = shared.find_task(id) {
                let s = t.snapshot();
                if s.state == "success" || s.state == "error" || s.state == "canceled" {
                    return s;
                }
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("任务超时: {id}");
    }

    /// 传输引擎端到端真机测试（上传→下载→字节比对）：
    /// `cargo test --lib -- --ignored live_engine`
    #[tokio::test]
    #[ignore]
    async fn live_engine_upload_and_download() {
        let tmp_dir = std::env::temp_dir().join(format!("hulian-cfg-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp_dir).unwrap();
        let shared = Shared::new_headless(tmp_dir);

        let mut acct = crate::config::import_from_cli().expect("需要本机 aliyun CLI 配置");
        if acct.drive_id.trim().is_empty() {
            let tmp_client = crate::pds::PdsClient::new(&acct).unwrap();
            let drives = tmp_client.list_drives().await.unwrap();
            acct.drive_id = drives
                .first()
                .expect("没有可用空间")
                .drive_id
                .clone();
        }
        let mut cfg = shared.config.read().unwrap().clone();
        cfg.account = Some(acct);
        cfg.settings.transfer_folder_name = "文件互传".into();
        cfg.settings.download_dir = std::env::temp_dir()
            .join("hulian-engine-dl")
            .to_string_lossy()
            .to_string();
        cfg.settings.chunk_size_mb = 4;
        cfg.settings.parallel_parts = 1;
        shared.reconfigure(cfg).expect("配置保存失败");

        let payload: Vec<u8> = (0..3 * 1024 * 1024 + 77).map(|i| (i % 249) as u8).collect();
        let src = std::env::temp_dir().join(format!("hulian-src-{}.bin", uuid::Uuid::new_v4()));
        std::fs::write(&src, &payload).unwrap();
        let name = src.file_name().unwrap().to_string_lossy().to_string();

        let ids = shared.start_uploads(vec![src.to_string_lossy().to_string()]);
        assert_eq!(ids.len(), 1);
        let item = wait_finished(&shared, &ids[0]).await;
        assert_eq!(item.state, "success", "上传失败: {:?}", item.error);
        assert_eq!(item.transferred, payload.len() as u64);
        let remote_id = item.remote_file_id.clone().expect("缺少 remote_file_id");

        let dl_ids = shared.start_downloads(vec![(
            remote_id,
            name.clone(),
            payload.len() as u64,
        )]);
        assert_eq!(dl_ids.len(), 1);
        let dl = wait_finished(&shared, &dl_ids[0]).await;
        assert_eq!(dl.state, "success", "下载失败: {:?}", dl.error);
        let got = std::fs::read(&dl.local_path).expect("读取下载文件失败");
        assert_eq!(got.len(), payload.len(), "下载长度不一致");
        assert_eq!(got, payload, "下载内容不一致");

        println!(
            "✅ 传输引擎端到端通过：上传 {} 字节 → 下载字节一致 → {}",
            payload.len(),
            dl.local_path
        );
        let _ = std::fs::remove_file(&src);
        let _ = std::fs::remove_file(&dl.local_path);
    }

    #[test]
    fn sanitize_and_unique() {
        assert_eq!(sanitize_name("a/b:c*d?.txt"), "a_b_c_d_.txt");
        assert_eq!(sanitize_name("   "), "未命名文件");
        let dir = std::env::temp_dir().join(format!("hulian-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), b"x").unwrap();
        let p = unique_download_path(&dir, "a.txt");
        assert_eq!(p.file_name().unwrap().to_string_lossy(), "a (1).txt");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
