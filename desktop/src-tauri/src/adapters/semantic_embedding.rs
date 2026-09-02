use std::{
    fs::File,
    io::{BufReader, Read},
    path::{Path, PathBuf},
    sync::Mutex,
};

use fastembed::{
    InitOptionsUserDefined, Pooling, QuantizationMode, TextEmbedding, TokenizerFiles,
    UserDefinedEmbeddingModel,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::io::AsyncWriteExt;

pub const SEMANTIC_MODEL_ID: &str = "Qdrant/paraphrase-multilingual-MiniLM-L12-v2-onnx-Q";
pub const SEMANTIC_MODEL_REVISION: &str = "faf4aa4225822f3bc6376869cb1164e8e3feedd0";
pub const SEMANTIC_MODEL_VERSION: &str = "paraphrase-multilingual-minilm-l12-v2-q-faf4aa4";
pub const SEMANTIC_MODEL_LICENSE: &str = "Apache-2.0";
pub const SEMANTIC_MODEL_DIMENSIONS: usize = 384;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemanticModelPackFile {
    pub name: &'static str,
    pub size: u64,
    pub sha256: &'static str,
}

const MODEL_FILES: [SemanticModelPackFile; 6] = [
    SemanticModelPackFile {
        name: "model_optimized.onnx",
        size: 235_052_644,
        sha256: "634d0f66c29dc934c8fa72b8a4fe91dd4d420a22f1d82a241058d4316e659a99",
    },
    SemanticModelPackFile {
        name: "tokenizer.json",
        size: 17_083_009,
        sha256: "fa685fc160bbdbab64058d4fc91b60e62d207e8dc60b9af5c002c5ab946ded00",
    },
    SemanticModelPackFile {
        name: "config.json",
        size: 673,
        sha256: "c8ec081fdad2df991bf5abbf18418fec7a5cdaa421f60ffb060a30040b8c376f",
    },
    SemanticModelPackFile {
        name: "special_tokens_map.json",
        size: 964,
        sha256: "8c785abebea9ae3257b61681b4e6fd8365ceafde980c21970d001e834cf10835",
    },
    SemanticModelPackFile {
        name: "tokenizer_config.json",
        size: 1_416,
        sha256: "0666eebf692422757e1dddf3c9fb1ded73ba3dc726c5828671fc89e45bf3609f",
    },
    SemanticModelPackFile {
        name: "onnxruntime.dll",
        size: 14_186_016,
        sha256: "dec964ab1ee36cc9b0ae247d13b376627992fc57dec0454354017ab8fd84f1ea",
    },
];

const ORT_ARCHIVE_SIZE: u64 = 78_127_794;
const ORT_ARCHIVE_SHA256: &str = "0b38df9af21834e41e73d602d90db5cb06dbd1ca618948b8f1d66d607ac9f3cd";
const ORT_ARCHIVE_URLS: [&str; 2] = [
    "https://github.com/microsoft/onnxruntime/releases/download/v1.23.2/onnxruntime-win-x64-1.23.2.zip",
    "https://ghfast.top/https://github.com/microsoft/onnxruntime/releases/download/v1.23.2/onnxruntime-win-x64-1.23.2.zip",
];
const ORT_ARCHIVE_DLL_PATH: &str = "onnxruntime-win-x64-1.23.2/lib/onnxruntime.dll";
const MODEL_DOWNLOAD_BASE_URLS: [&str; 2] = ["https://huggingface.co", "https://hf-mirror.com"];
const VERIFICATION_MARKER: &str = ".mindscape-verified.json";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VerificationMarker {
    model_version: String,
    files: Vec<VerificationMarkerFile>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VerificationMarkerFile {
    name: String,
    size: u64,
    modified_unix_nanos: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum SemanticModelPackStatus {
    Missing {
        model_version: &'static str,
        missing_files: Vec<String>,
    },
    Corrupt {
        model_version: &'static str,
        invalid_files: Vec<String>,
    },
    Ready {
        model_version: &'static str,
        dimensions: usize,
        total_bytes: u64,
    },
}

#[derive(Debug, Clone)]
pub struct SemanticModelPack {
    root: PathBuf,
}

#[derive(Debug, Error)]
pub enum SemanticModelInstallError {
    #[error("semantic model download failed")]
    Download(#[from] reqwest::Error),
    #[error("all verified semantic model sources were unavailable: {0}")]
    AllSourcesUnavailable(&'static str),
    #[error("semantic model storage failed")]
    Storage(#[from] std::io::Error),
    #[error("semantic model file exceeded its declared size: {0}")]
    SizeLimit(&'static str),
    #[error("semantic model pack failed integrity verification")]
    Integrity,
    #[error("semantic model runtime archive could not be extracted")]
    Archive(#[from] zip::result::ZipError),
    #[error("semantic model installer task failed")]
    InstallerTask(#[from] tokio::task::JoinError),
}

#[derive(Debug, Error)]
pub enum SemanticEmbeddingError {
    #[error("semantic model pack is not ready")]
    PackNotReady,
    #[error("semantic model files could not be read")]
    Storage(#[from] std::io::Error),
    #[error("semantic model runtime could not be initialized")]
    RuntimeInitialization(#[source] fastembed::Error),
    #[error("ONNX Runtime could not be loaded")]
    RuntimeLibrary(#[source] ort::Error),
    #[error("semantic model runtime is unavailable")]
    RuntimeUnavailable,
    #[error("semantic model inference failed")]
    Inference(#[source] fastembed::Error),
    #[error("semantic model returned an invalid embedding")]
    InvalidEmbedding,
}

pub struct SemanticEmbedding {
    model: Mutex<TextEmbedding>,
}

impl SemanticEmbedding {
    pub fn load(pack: &SemanticModelPack) -> Result<Self, SemanticEmbeddingError> {
        if !matches!(pack.inspect(), SemanticModelPackStatus::Ready { .. }) {
            return Err(SemanticEmbeddingError::PackNotReady);
        }
        Self::load_verified(pack)
    }

    pub(crate) fn load_verified(pack: &SemanticModelPack) -> Result<Self, SemanticEmbeddingError> {
        ort::init_from(pack.root().join("onnxruntime.dll"))
            .map_err(SemanticEmbeddingError::RuntimeLibrary)?
            .commit();
        let read = |name: &str| std::fs::read(pack.root().join(name));
        let model = UserDefinedEmbeddingModel::new(
            read("model_optimized.onnx")?,
            TokenizerFiles {
                tokenizer_file: read("tokenizer.json")?,
                config_file: read("config.json")?,
                special_tokens_map_file: read("special_tokens_map.json")?,
                tokenizer_config_file: read("tokenizer_config.json")?,
            },
        )
        .with_pooling(Pooling::Mean)
        .with_quantization(QuantizationMode::Static);
        let model = TextEmbedding::try_new_from_user_defined(
            model,
            InitOptionsUserDefined::default().with_max_length(512),
        )
        .map_err(SemanticEmbeddingError::RuntimeInitialization)?;
        Ok(Self {
            model: Mutex::new(model),
        })
    }

    pub fn embed(&self, text: &str) -> Result<Vec<f32>, SemanticEmbeddingError> {
        self.embed_batch(&[text])?
            .pop()
            .ok_or(SemanticEmbeddingError::InvalidEmbedding)
    }

    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, SemanticEmbeddingError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let mut model = self
            .model
            .lock()
            .map_err(|_| SemanticEmbeddingError::RuntimeUnavailable)?;
        let result = model
            .embed(texts, Some(1))
            .map_err(SemanticEmbeddingError::Inference)?;
        if result.len() != texts.len()
            || result.iter().any(|embedding| {
                embedding.len() != SEMANTIC_MODEL_DIMENSIONS
                    || embedding.iter().any(|value| !value.is_finite())
                    || embedding.iter().all(|value| *value == 0.0)
            })
        {
            return Err(SemanticEmbeddingError::InvalidEmbedding);
        }
        Ok(result)
    }
}

impl SemanticModelPack {
    pub fn new(models_root: impl AsRef<Path>) -> Self {
        Self {
            root: models_root.as_ref().join(SEMANTIC_MODEL_VERSION),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn files() -> &'static [SemanticModelPackFile] {
        &MODEL_FILES
    }

    /// Inspects only local files. This method never creates directories or
    /// performs network access, so ordinary startup remains offline-safe.
    pub fn inspect(&self) -> SemanticModelPackStatus {
        let missing_files = MODEL_FILES
            .iter()
            .filter(|expected| !self.root.join(expected.name).is_file())
            .map(|expected| expected.name.to_owned())
            .collect::<Vec<_>>();
        if !missing_files.is_empty() {
            return SemanticModelPackStatus::Missing {
                model_version: SEMANTIC_MODEL_VERSION,
                missing_files,
            };
        }

        if marker_matches(&self.root) {
            return SemanticModelPackStatus::Ready {
                model_version: SEMANTIC_MODEL_VERSION,
                dimensions: SEMANTIC_MODEL_DIMENSIONS,
                total_bytes: MODEL_FILES.iter().map(|file| file.size).sum(),
            };
        }

        let invalid_files = MODEL_FILES
            .iter()
            .filter_map(|expected| {
                validate_file(&self.root.join(expected.name), expected)
                    .then_some(expected.name.to_owned())
            })
            .collect::<Vec<_>>();
        if !invalid_files.is_empty() {
            return SemanticModelPackStatus::Corrupt {
                model_version: SEMANTIC_MODEL_VERSION,
                invalid_files,
            };
        }

        SemanticModelPackStatus::Ready {
            model_version: SEMANTIC_MODEL_VERSION,
            dimensions: SEMANTIC_MODEL_DIMENSIONS,
            total_bytes: MODEL_FILES.iter().map(|file| file.size).sum(),
        }
    }

    /// Downloads the immutable, revision-pinned pack after an explicit user
    /// action. The active pack changes only after every integrity check passes.
    pub async fn install(&self) -> Result<SemanticModelPackStatus, SemanticModelInstallError> {
        let current_status = self.inspect();
        if matches!(current_status, SemanticModelPackStatus::Ready { .. }) {
            self.write_verification_marker()?;
            return Ok(current_status);
        }

        let models_root = self
            .root
            .parent()
            .expect("a semantic model pack always has a models parent");
        tokio::fs::create_dir_all(models_root).await?;
        let operation_id = uuid::Uuid::new_v4();
        let staging = models_root.join(format!(
            ".{SEMANTIC_MODEL_VERSION}.installing-{operation_id}"
        ));
        tokio::fs::create_dir(&staging).await?;

        if let Err(error) = self.download_into(&staging).await {
            let _ = tokio::fs::remove_dir_all(&staging).await;
            return Err(error);
        }

        let staged_pack = Self {
            root: staging.clone(),
        };
        let staged_status = staged_pack.inspect();
        if !matches!(staged_status, SemanticModelPackStatus::Ready { .. }) {
            let _ = tokio::fs::remove_dir_all(&staging).await;
            return Err(SemanticModelInstallError::Integrity);
        }
        staged_pack.write_verification_marker()?;

        if self.root.exists() {
            let quarantine =
                models_root.join(format!(".{SEMANTIC_MODEL_VERSION}.replaced-{operation_id}"));
            tokio::fs::rename(&self.root, quarantine).await?;
        }
        tokio::fs::rename(staging, &self.root).await?;
        Ok(staged_status)
    }

    async fn download_into(&self, staging: &Path) -> Result<(), SemanticModelInstallError> {
        let client = reqwest::Client::builder()
            .user_agent("MindScape semantic-model-installer/1")
            .connect_timeout(std::time::Duration::from_secs(8))
            .build()?;
        for expected in &MODEL_FILES[..5] {
            let mut downloaded = false;
            for base_url in MODEL_DOWNLOAD_BASE_URLS {
                let url = format!(
                    "{base_url}/{SEMANTIC_MODEL_ID}/resolve/{SEMANTIC_MODEL_REVISION}/{}",
                    expected.name
                );
                match download_checked(
                    &client,
                    &url,
                    &staging.join(expected.name),
                    expected.size,
                    expected.sha256,
                    expected.name,
                    std::time::Duration::from_secs(900),
                )
                .await
                {
                    Ok(()) => {
                        downloaded = true;
                        break;
                    }
                    Err(SemanticModelInstallError::Download(_)) => continue,
                    Err(error) => return Err(error),
                }
            }
            if !downloaded {
                return Err(SemanticModelInstallError::AllSourcesUnavailable(
                    expected.name,
                ));
            }
        }
        let archive = staging.join("onnxruntime.zip");
        let mut runtime_downloaded = false;
        for url in ORT_ARCHIVE_URLS {
            match download_checked(
                &client,
                url,
                &archive,
                ORT_ARCHIVE_SIZE,
                ORT_ARCHIVE_SHA256,
                "onnxruntime.zip",
                std::time::Duration::from_secs(90),
            )
            .await
            {
                Ok(()) => {
                    runtime_downloaded = true;
                    break;
                }
                Err(SemanticModelInstallError::Download(_)) => continue,
                Err(error) => return Err(error),
            }
        }
        if !runtime_downloaded {
            return Err(SemanticModelInstallError::AllSourcesUnavailable(
                "onnxruntime.zip",
            ));
        }
        let staging = staging.to_path_buf();
        tokio::task::spawn_blocking(move || extract_ort_runtime(&staging)).await??;
        tokio::fs::remove_file(archive).await?;
        Ok(())
    }

    fn write_verification_marker(&self) -> Result<(), SemanticModelInstallError> {
        let files = MODEL_FILES
            .iter()
            .map(|expected| {
                let metadata = std::fs::metadata(self.root.join(expected.name))?;
                let modified = metadata
                    .modified()?
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|_| {
                        std::io::Error::other("model file timestamp predates Unix epoch")
                    })?;
                let modified_unix_nanos = u64::try_from(modified.as_nanos())
                    .map_err(|_| std::io::Error::other("model file timestamp is out of range"))?;
                Ok(VerificationMarkerFile {
                    name: expected.name.into(),
                    size: metadata.len(),
                    modified_unix_nanos,
                })
            })
            .collect::<Result<Vec<_>, std::io::Error>>()?;
        let marker = serde_json::to_vec(&VerificationMarker {
            model_version: SEMANTIC_MODEL_VERSION.into(),
            files,
        })
        .map_err(|error| std::io::Error::other(error.to_string()))?;
        std::fs::write(self.root.join(VERIFICATION_MARKER), marker)?;
        Ok(())
    }
}

fn marker_matches(root: &Path) -> bool {
    let Ok(bytes) = std::fs::read(root.join(VERIFICATION_MARKER)) else {
        return false;
    };
    let Ok(marker) = serde_json::from_slice::<VerificationMarker>(&bytes) else {
        return false;
    };
    if marker.model_version != SEMANTIC_MODEL_VERSION || marker.files.len() != MODEL_FILES.len() {
        return false;
    }
    MODEL_FILES.iter().all(|expected| {
        let Some(recorded) = marker.files.iter().find(|file| file.name == expected.name) else {
            return false;
        };
        let Ok(metadata) = std::fs::metadata(root.join(expected.name)) else {
            return false;
        };
        let Ok(modified) = metadata.modified().and_then(|value| {
            value
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(std::io::Error::other)
        }) else {
            return false;
        };
        metadata.len() == expected.size
            && recorded.size == expected.size
            && u64::try_from(modified.as_nanos()).ok() == Some(recorded.modified_unix_nanos)
    })
}

async fn download_checked(
    client: &reqwest::Client,
    url: &str,
    destination: &Path,
    expected_size: u64,
    expected_sha256: &str,
    display_name: &'static str,
    timeout: std::time::Duration,
) -> Result<(), SemanticModelInstallError> {
    let response = client
        .get(url)
        .timeout(timeout)
        .send()
        .await?
        .error_for_status()?;
    if response
        .content_length()
        .is_some_and(|length| length != expected_size)
    {
        return Err(SemanticModelInstallError::Integrity);
    }
    let mut file = tokio::fs::File::create(destination).await?;
    let mut stream = response.bytes_stream();
    let mut written = 0_u64;
    let mut hasher = Sha256::new();
    while let Some(chunk) = futures_util::TryStreamExt::try_next(&mut stream).await? {
        written = written
            .checked_add(chunk.len() as u64)
            .ok_or(SemanticModelInstallError::SizeLimit(display_name))?;
        if written > expected_size {
            return Err(SemanticModelInstallError::SizeLimit(display_name));
        }
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
    }
    file.flush().await?;
    if written != expected_size || format!("{:x}", hasher.finalize()) != expected_sha256 {
        return Err(SemanticModelInstallError::Integrity);
    }
    Ok(())
}

fn extract_ort_runtime(staging: &Path) -> Result<(), SemanticModelInstallError> {
    let archive = File::open(staging.join("onnxruntime.zip"))?;
    let mut archive = zip::ZipArchive::new(archive)?;
    let mut runtime = archive.by_name(ORT_ARCHIVE_DLL_PATH)?;
    let mut destination = File::create(staging.join("onnxruntime.dll"))?;
    std::io::copy(&mut runtime, &mut destination)?;
    Ok(())
}

fn validate_file(path: &Path, expected: &SemanticModelPackFile) -> bool {
    let Ok(file) = File::open(path) else {
        return true;
    };
    let Ok(metadata) = file.metadata() else {
        return true;
    };
    if metadata.len() != expected.size {
        return true;
    }

    let mut reader = BufReader::with_capacity(1024 * 1024, file);
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => hasher.update(&buffer[..read]),
            Err(_) => return true,
        }
    }
    format!("{:x}", hasher.finalize()) != expected.sha256
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;
    use tempfile::TempDir;

    #[test]
    fn missing_pack_is_reported_without_creating_it() {
        let directory = TempDir::new().expect("temp directory");
        let pack = SemanticModelPack::new(directory.path().join("models"));

        let status = pack.inspect();

        assert!(matches!(status, SemanticModelPackStatus::Missing { .. }));
        assert!(!pack.root().exists());
    }

    #[test]
    fn present_but_invalid_pack_is_reported_as_corrupt() {
        let directory = TempDir::new().expect("temp directory");
        let pack = SemanticModelPack::new(directory.path().join("models"));
        std::fs::create_dir_all(pack.root()).expect("create pack");
        for file in SemanticModelPack::files() {
            std::fs::write(pack.root().join(file.name), b"invalid").expect("write fixture");
        }

        let status = pack.inspect();

        assert!(matches!(status, SemanticModelPackStatus::Corrupt { .. }));
    }

    #[tokio::test(flavor = "current_thread")]
    #[ignore = "downloads the fixed 330 MB production semantic model pack"]
    async fn production_model_recalls_chinese_paraphrases() {
        let cache = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("b4-semantic-quality-cache");
        let pack = SemanticModelPack::new(cache.join("models"));
        pack.install().await.expect("install fixed model pack");
        let started = Instant::now();
        let model = SemanticEmbedding::load_verified(&pack).expect("load semantic runtime");
        let texts = [
            "外面下雨需要带伞吗",
            "出门前请携带雨具，以免被淋湿",
            "SQLite 是一种嵌入式关系数据库",
            "怎样避免讨论结论在重启后消失",
            "将讨论结论写入本地持久化存储，应用重启后仍可恢复",
            "周末去公园散步",
            "轻量单机应用应该把结构化数据放在哪里",
        ];
        let embeddings = model.embed_batch(&texts).expect("embed quality matrix");

        assert!(cosine(&embeddings[0], &embeddings[1]) > cosine(&embeddings[0], &embeddings[2]));
        assert!(cosine(&embeddings[3], &embeddings[4]) > cosine(&embeddings[3], &embeddings[5]));
        assert!(cosine(&embeddings[6], &embeddings[2]) > cosine(&embeddings[6], &embeddings[1]));
        assert_eq!(embeddings[0].len(), SEMANTIC_MODEL_DIMENSIONS);
        eprintln!(
            "semantic inference elapsed_ms={}",
            started.elapsed().as_millis()
        );
    }

    fn cosine(left: &[f32], right: &[f32]) -> f32 {
        let dot = left.iter().zip(right).map(|(a, b)| a * b).sum::<f32>();
        let left_norm = left.iter().map(|value| value * value).sum::<f32>().sqrt();
        let right_norm = right.iter().map(|value| value * value).sum::<f32>().sqrt();
        dot / (left_norm * right_norm)
    }
}
