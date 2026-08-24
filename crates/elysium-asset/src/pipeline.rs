//! Async asset pipeline — işleri kuyruğa alır, cook eder, .ely arşivine yazar.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};
use crate::{AssetType, AssetMetadata, ImportSettings, CompressionType, Cooker, ElyArchive};

// ─────────────────────────────────────────────────── Pipeline Job / Message

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineJob {
    pub source_path: PathBuf,
    pub destination_path: PathBuf,
    pub asset_type: AssetType,
    pub import_settings: ImportSettings,
}

#[derive(Debug)]
pub enum PipelineMessage {
    JobStarted(String),
    JobProgress(String, f32),
    JobCompleted(String, String), // (asset_id, output_path)
    JobFailed(String, String),    // (asset_id, error)
}

// ─────────────────────────────────────────────────── Pipeline Config

#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub num_threads: usize,
    pub output_directory: PathBuf,
    pub temp_directory: PathBuf,
    pub import_quality: f32,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            num_threads: num_cpus::get(),
            output_directory: PathBuf::from("./build/assets"),
            temp_directory: PathBuf::from("./temp"),
            import_quality: 1.0,
        }
    }
}

// ─────────────────────────────────────────────────── Asset Pipeline

pub struct AssetPipeline {
    pub config: PipelineConfig,
    pub cooker: Cooker,
    /// Mutex<HashMap> ile güvenli paylaşım sağlıyoruz.
    pub assets: Arc<Mutex<std::collections::HashMap<String, AssetMetadata>>>,
    job_sender: mpsc::UnboundedSender<PipelineJob>,
    job_receiver: Option<mpsc::UnboundedReceiver<PipelineJob>>,
    message_sender: mpsc::UnboundedSender<PipelineMessage>,
    pub message_receiver: Option<mpsc::UnboundedReceiver<PipelineMessage>>,
}

impl AssetPipeline {
    pub fn new(config: PipelineConfig) -> Self {
        let (job_tx, job_rx) = mpsc::unbounded_channel();
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        Self {
            cooker: Cooker,
            assets: Arc::new(Mutex::new(std::collections::HashMap::new())),
            config,
            job_sender: job_tx,
            job_receiver: Some(job_rx),
            message_sender: msg_tx,
            message_receiver: Some(msg_rx),
        }
    }

    /// İşi kuyruğa ekle.
    pub fn submit_job(&self, job: PipelineJob) -> Result<(), Box<dyn std::error::Error>> {
        self.job_sender.send(job).map_err(|e| e.to_string().into())
    }

    /// Tüm kuyruklanmış işleri işle.
    pub async fn run_pipeline(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::create_dir_all(&self.config.output_directory)?;
        std::fs::create_dir_all(&self.config.temp_directory)?;

        let mut rx = self.job_receiver.take().expect("pipeline zaten başlatılmış");
        while let Some(job) = rx.recv().await {
            if let Err(e) = self.process_job(job).await {
                tracing::error!("Pipeline işi başarısız: {}", e);
            }
        }
        Ok(())
    }

    async fn process_job(&self, job: PipelineJob) -> Result<(), Box<dyn std::error::Error>> {
        let asset_id = job
            .source_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let _ = self.message_sender.send(PipelineMessage::JobStarted(asset_id.clone()));

        let raw = tokio::fs::read(&job.source_path).await?;

        let cooked = match job.asset_type {
            AssetType::Mesh => self.cooker.cook_mesh(&raw)?,
            AssetType::Texture => self.cooker.cook_texture(&raw)?,
            AssetType::Material => self.cooker.cook_material(&raw)?,
            _ => raw,
        };

        let output_path = self.config.output_directory.join(format!("{}.ely", asset_id));
        tokio::fs::write(&output_path, &cooked).await?;

        // Metadata'yı Arc<Mutex<HashMap>>'e ekle.
        let meta = AssetMetadata {
            id: asset_id.clone(),
            asset_type: job.asset_type,
            path: job.source_path,
            dependencies: Vec::new(),
            import_settings: job.import_settings,
        };
        self.assets.lock().unwrap().insert(asset_id.clone(), meta);

        let _ = self.message_sender.send(PipelineMessage::JobCompleted(
            asset_id,
            output_path.to_string_lossy().into_owned(),
        ));
        Ok(())
    }

    /// Metadata haritasına bak.
    pub fn get_asset(&self, id: &str) -> Option<AssetMetadata> {
        self.assets.lock().unwrap().get(id).cloned()
    }

    /// Yüklü asset sayısı.
    pub fn asset_count(&self) -> usize {
        self.assets.lock().unwrap().len()
    }
}

impl Default for AssetPipeline {
    fn default() -> Self {
        Self::new(PipelineConfig::default())
    }
}
