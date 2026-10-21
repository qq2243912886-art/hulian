//! 应用共享状态：配置、云盘客户端、传输任务表、并发信号量。
//!
//! 为了可测试性，`app: Option<AppHandle>`：正常运行时带 AppHandle（用于发事件），
//! 无头测试时只保留配置目录与任务引擎。

use crate::config;
use crate::error::{AppError, Result};
use crate::model::{AppConfig, AppSettings, PdsAccount, TransferItem};
use crate::pds::PdsClient;
use crate::transfer::{Task, EVT_LIST};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use tauri::{AppHandle, Emitter};

pub struct Shared {
    pub app: Option<AppHandle>,
    pub config_dir: PathBuf,
    pub config: RwLock<AppConfig>,
    pub client: RwLock<Option<Arc<PdsClient>>>,
    /// 传输任务（新的在前）
    pub tasks: RwLock<Vec<Arc<Task>>>,
    pub sem: RwLock<Arc<tokio::sync::Semaphore>>,
    pub quitting: AtomicBool,
}

impl Shared {
    /// 正常运行入口
    pub fn new(app: AppHandle) -> Arc<Self> {
        let dir = config::config_dir(&app);
        let shared = Self::build(Some(app), dir);
        shared.load_history();
        shared
    }

    /// 无头模式（测试 / 自检）：不依赖窗口与事件
    pub fn new_headless(config_dir: PathBuf) -> Arc<Self> {
        let shared = Self::build(None, config_dir);
        shared.load_history();
        shared
    }

    fn build(app: Option<AppHandle>, config_dir: PathBuf) -> Arc<Self> {
        let cfg = config::load_from(&config_dir, app.as_ref());
        let client = cfg
            .account
            .as_ref()
            .and_then(|a| PdsClient::new(a).ok())
            .map(Arc::new);
        let sem = Arc::new(tokio::sync::Semaphore::new(
            cfg.settings.parallel_transfers.clamp(1, 6) as usize,
        ));
        Arc::new(Self {
            app,
            config_dir,
            config: RwLock::new(cfg),
            client: RwLock::new(client),
            tasks: RwLock::new(Vec::new()),
            sem: RwLock::new(sem),
            quitting: AtomicBool::new(false),
        })
    }

    fn load_history(self: &Arc<Self>) {
        let history = load_history(&self.config_dir);
        if history.is_empty() {
            return;
        }
        let mut tasks = self.tasks.write().unwrap();
        for item in history {
            tasks.push(Arc::new(Task::from_history(item)));
        }
    }

    pub fn settings(&self) -> AppSettings {
        self.config.read().unwrap().settings.clone()
    }

    pub fn account(&self) -> Option<PdsAccount> {
        self.config.read().unwrap().account.clone()
    }

    pub fn client(&self) -> Result<Arc<PdsClient>> {
        if let Some(c) = self.client.read().unwrap().as_ref() {
            return Ok(c.clone());
        }
        let acct = self.account().ok_or(AppError::NotConfigured)?;
        let c = Arc::new(PdsClient::new(&acct)?);
        *self.client.write().unwrap() = Some(c.clone());
        Ok(c)
    }

    /// 保存配置并重建客户端 / 信号量
    pub fn reconfigure(&self, cfg: AppConfig) -> Result<()> {
        config::save_to(&self.config_dir, &cfg)?;
        let client = match &cfg.account {
            Some(a) => Some(Arc::new(PdsClient::new(a)?)),
            None => None,
        };
        *self.client.write().unwrap() = client;
        *self.sem.write().unwrap() = Arc::new(tokio::sync::Semaphore::new(
            cfg.settings.parallel_transfers.clamp(1, 6) as usize,
        ));
        *self.config.write().unwrap() = cfg;
        Ok(())
    }

    pub fn snapshot(&self) -> Vec<TransferItem> {
        self.tasks
            .read()
            .unwrap()
            .iter()
            .map(|t| t.snapshot())
            .collect()
    }

    pub fn emit_item(&self, item: &TransferItem) {
        if let Some(app) = &self.app {
            let _ = app.emit(crate::transfer::EVT_UPDATE, item);
        }
    }

    pub fn emit_list(&self) {
        if let Some(app) = &self.app {
            let _ = app.emit(EVT_LIST, self.snapshot());
        }
    }

    /// 把已结束的任务写入历史文件（最多 100 条）
    pub fn persist_history(&self) {
        let items: Vec<TransferItem> = self
            .snapshot()
            .into_iter()
            .filter(|i| i.state != "running" && i.state != "queued")
            .take(100)
            .collect();
        save_history(&self.config_dir, &items);
    }

    pub fn push_task(&self, task: Arc<Task>) {
        self.tasks.write().unwrap().insert(0, task);
        self.emit_list();
    }

    pub fn find_task(&self, id: &str) -> Option<Arc<Task>> {
        self.tasks
            .read()
            .unwrap()
            .iter()
            .find(|t| t.id == id)
            .cloned()
    }

    pub fn remove_task(&self, id: &str) -> bool {
        let mut tasks = self.tasks.write().unwrap();
        let before = tasks.len();
        tasks.retain(|t| t.id != id || t.is_active());
        let removed = tasks.len() != before;
        drop(tasks);
        if removed {
            self.persist_history();
            self.emit_list();
        }
        removed
    }

    pub fn clear_finished(&self) {
        self.tasks.write().unwrap().retain(|t| t.is_active());
        self.persist_history();
        self.emit_list();
    }

    pub fn is_quitting(&self) -> bool {
        self.quitting.load(Ordering::SeqCst)
    }

    pub fn mark_quitting(&self) {
        self.quitting.store(true, Ordering::SeqCst);
    }

    pub fn history_path(&self) -> PathBuf {
        self.config_dir.join("transfers.json")
    }
}

pub fn load_history(config_dir: &Path) -> Vec<TransferItem> {
    let path = config_dir.join("transfers.json");
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<TransferItem>>(&text).unwrap_or_default()
}

pub fn save_history(config_dir: &Path, items: &[TransferItem]) {
    let path = config_dir.join("transfers.json");
    let _ = std::fs::create_dir_all(config_dir);
    if let Ok(text) = serde_json::to_string(items) {
        let _ = std::fs::write(path, text);
    }
}
