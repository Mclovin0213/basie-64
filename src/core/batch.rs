use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

/// Batch operation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BatchOp {
    Encode,
    Decode,
}

impl fmt::Display for BatchOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BatchOp::Encode => write!(f, "encode"),
            BatchOp::Decode => write!(f, "decode"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchSourceKind {
    Directory,
    Files,
}

/// Source selection for a batch operation.
#[derive(Debug, Clone)]
pub enum BatchSource {
    Directory {
        root: PathBuf,
        depth_limit: Option<usize>,
    },
    Files {
        root: PathBuf,
        files: Vec<PathBuf>,
    },
}

impl BatchSource {
    pub fn directory(root: PathBuf, depth_limit: Option<usize>) -> Self {
        Self::Directory { root, depth_limit }
    }

    pub fn files(files: Vec<PathBuf>) -> Self {
        let mut iter = files.iter();
        let root = iter
            .next()
            .and_then(|first| first.parent().map(Path::to_path_buf))
            .map(|mut root| {
                for path in iter {
                    root = common_parent(&root, path);
                }
                root
            })
            .unwrap_or_default();
        Self::Files { root, files }
    }

    pub fn root(&self) -> &Path {
        match self {
            BatchSource::Directory { root, .. } | BatchSource::Files { root, .. } => root,
        }
    }

    pub fn kind(&self) -> BatchSourceKind {
        match self {
            BatchSource::Directory { .. } => BatchSourceKind::Directory,
            BatchSource::Files { .. } => BatchSourceKind::Files,
        }
    }

    pub fn selection_count(&self) -> usize {
        match self {
            BatchSource::Directory { .. } => 1,
            BatchSource::Files { files, .. } => files.len(),
        }
    }
}

/// Status of a single file in a batch operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum BatchStatus {
    Ok { input_size: u64, output_size: u64 },
    Skipped { reason: String },
    Error { error: String },
}


/// Result of processing a single file in a batch operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchFileResult {
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    #[serde(flatten)]
    pub status: BatchStatus,
}

/// Complete result of a batch operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    pub operation: BatchOp,
    pub processed: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub skipped: usize,
    pub files: Vec<BatchFileResult>,
    pub output_dir: Option<PathBuf>,
    pub timestamp: String,
}

impl BatchResult {
    pub fn succeeded(&self) -> usize {
        self.succeeded
    }

    pub fn failed(&self) -> usize {
        self.failed
    }

    pub fn processed(&self) -> usize {
        self.processed
    }

    pub fn skipped(&self) -> usize {
        self.skipped
    }

    /// Generate a JSON manifest string.
    pub fn manifest_json(&self) -> Result<String, serde_json::Error> {
        let manifest = BatchManifest::from_result(self);
        serde_json::to_string_pretty(&manifest)
    }
}

/// Manifest format for export (matches the spec format).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchManifest {
    pub version: usize,
    pub timestamp: String,
    pub operation: String,
    pub files: Vec<BatchManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchManifestEntry {
    pub input: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl BatchManifest {
    fn from_result(result: &BatchResult) -> Self {
        let files = result
            .files
            .iter()
            .map(|f| {
                let (status_str, input_size, output_size, error) = match &f.status {
                    BatchStatus::Ok {
                        input_size,
                        output_size,
                    } => (
                        "ok".to_string(),
                        Some(*input_size),
                        Some(*output_size),
                        None,
                    ),
                    BatchStatus::Skipped { reason } => {
                        ("skipped".to_string(), None, None, Some(reason.clone()))
                    }
                    BatchStatus::Error { error } => {
                        ("error".to_string(), None, None, Some(error.clone()))
                    }
                };

                BatchManifestEntry {
                    input: path_string(&f.input),
                    output: f.output.as_ref().map(|path| path_string(path)),
                    status: status_str,
                    input_size,
                    output_size,
                    error,
                }
            })
            .collect();

        BatchManifest {
            version: 1,
            timestamp: result.timestamp.clone(),
            operation: result.operation.to_string(),
            files,
        }
    }
}

/// Configuration for a batch operation.
#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub source: BatchSource,
    pub output_dir: Option<PathBuf>,
    pub operation: BatchOp,
    pub decode_b64_only: bool,
}

impl BatchConfig {
    pub fn source_root(&self) -> &Path {
        self.source.root()
    }
}

#[derive(Debug, Clone, Default)]
pub struct BatchProgress {
    pub total: usize,
    pub processed: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub skipped: usize,
    pub current: Option<PathBuf>,
}

impl BatchProgress {
    fn from_total(total: usize) -> Self {
        Self {
            total,
            ..Self::default()
        }
    }
}

/// Preview of what a batch operation would process (shown before confirmation).
#[derive(Debug, Clone)]
pub struct BatchPreview {
    pub root: PathBuf,
    pub source_kind: BatchSourceKind,
    pub selection_count: usize,
    pub file_count: usize,
    pub eligible_count: usize,
    pub total_size: u64,
    pub operation: BatchOp,
}

#[derive(Debug, Clone)]
struct BatchWorkItem {
    input_path: PathBuf,
    input_relative: PathBuf,
    output_path: Option<PathBuf>,
    output_relative: Option<PathBuf>,
    skip_reason: Option<String>,
}

#[derive(Debug, Clone)]
enum BatchPlanEntry {
    Work(BatchWorkItem),
    Immediate(BatchFileResult),
}

#[derive(Debug, Clone)]
struct BatchPlan {
    entries: Vec<BatchPlanEntry>,
}

impl BatchPlan {
    fn total(&self) -> usize {
        self.entries.len()
    }

    fn preview(&self) -> (usize, usize, u64) {
        let mut file_count = 0usize;
        let mut eligible_count = 0usize;
        let mut total_size = 0u64;

        for entry in &self.entries {
            match entry {
                BatchPlanEntry::Work(item) => {
                    file_count += 1;
                    total_size += fs::metadata(&item.input_path)
                        .map(|meta| meta.len())
                        .unwrap_or(0);
                    if item.skip_reason.is_none() {
                        eligible_count += 1;
                    }
                }
                BatchPlanEntry::Immediate(_) => {
                    file_count += 1;
                }
            }
        }

        (file_count, eligible_count, total_size)
    }
}

/// Encode a single file's content to Base64.
pub fn encode_file_bytes(bytes: &[u8]) -> String {
    general_purpose::STANDARD.encode(bytes)
}

/// Decode a single file's content from Base64.
pub fn decode_file_bytes(b64_content: &str) -> Result<Vec<u8>, String> {
    let trimmed = b64_content.trim();
    general_purpose::STANDARD
        .decode(trimmed)
        .map_err(|e| format!("Invalid Base64: {}", e))
}

/// Scan a batch config and return a preview of what would be processed.
pub fn preview_batch(config: &BatchConfig) -> BatchPreview {
    let plan = build_batch_plan(config);
    let (file_count, eligible_count, total_size) = plan.preview();
    BatchPreview {
        root: config.source_root().to_path_buf(),
        source_kind: config.source.kind(),
        selection_count: config.source.selection_count(),
        file_count,
        eligible_count,
        total_size,
        operation: config.operation,
    }
}

/// Process a batch operation while reporting incremental progress.
pub fn process_batch_with_progress<F>(config: &BatchConfig, mut on_progress: F) -> BatchResult
where
    F: FnMut(BatchProgress),
{
    let plan = build_batch_plan(config);
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| chrono_like_timestamp(d.as_secs()))
        .unwrap_or_else(|_| "unknown".to_string());

    let mut progress = BatchProgress::from_total(plan.total());
    let mut results = Vec::with_capacity(plan.total());

    for entry in plan.entries {
        let result = match entry {
            BatchPlanEntry::Immediate(result) => {
                progress.current = Some(result.input.clone());
                result
            }
            BatchPlanEntry::Work(item) => {
                progress.current = Some(item.input_relative.clone());
                execute_work_item(config, item)
            }
        };

        progress.processed += 1;
        match &result.status {
            BatchStatus::Ok { .. } => progress.succeeded += 1,
            BatchStatus::Skipped { .. } => progress.skipped += 1,
            BatchStatus::Error { .. } => progress.failed += 1,
        }
        results.push(result);
        on_progress(progress.clone());
    }

    BatchResult {
        operation: config.operation,
        processed: progress.processed,
        succeeded: progress.succeeded,
        failed: progress.failed,
        skipped: progress.skipped,
        files: results,
        output_dir: config.output_dir.clone(),
        timestamp,
    }
}

fn build_batch_plan(config: &BatchConfig) -> BatchPlan {
    let mut entries = Vec::new();

    match &config.source {
        BatchSource::Directory { root, depth_limit } => {
            let mut walker = WalkDir::new(root);
            if let Some(depth) = depth_limit {
                walker = walker.max_depth(*depth);
            }

            for entry in walker {
                match entry {
                    Ok(entry) => {
                        if !entry.file_type().is_file() {
                            continue;
                        }
                        entries.push(BatchPlanEntry::Work(build_work_item(
                            config,
                            root,
                            entry.path().to_path_buf(),
                        )));
                    }
                    Err(err) => {
                        entries.push(BatchPlanEntry::Immediate(BatchFileResult {
                            input: make_relative(err.path().unwrap_or(root.as_path()), root),
                            output: None,
                            status: BatchStatus::Error {
                                error: format!("Cannot traverse path: {}", err),
                            },
                        }));
                    }
                }
            }
        }
        BatchSource::Files { root, files } => {
            for path in files {
                match fs::metadata(path) {
                    Ok(meta) if meta.is_file() => {
                        entries.push(BatchPlanEntry::Work(build_work_item(
                            config,
                            root,
                            path.clone(),
                        )));
                    }
                    Ok(_) => entries.push(BatchPlanEntry::Immediate(BatchFileResult {
                        input: make_relative(path, root),
                        output: None,
                        status: BatchStatus::Error {
                            error: "Selected path is not a file".to_string(),
                        },
                    })),
                    Err(err) => entries.push(BatchPlanEntry::Immediate(BatchFileResult {
                        input: make_relative(path, root),
                        output: None,
                        status: BatchStatus::Error {
                            error: format!("Cannot read file metadata: {}", err),
                        },
                    })),
                }
            }
        }
    }

    BatchPlan { entries }
}

fn build_work_item(config: &BatchConfig, root: &Path, input_path: PathBuf) -> BatchWorkItem {
    let input_relative = make_relative(&input_path, root);
    let skip_reason = match config.operation {
        BatchOp::Decode
            if config.decode_b64_only
                && !input_path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("b64")) =>
        {
            Some("Not a .b64 file".to_string())
        }
        _ => None,
    };
    let output_relative = skip_reason
        .as_ref()
        .map(|_| None)
        .unwrap_or_else(|| Some(output_relative_for(&input_relative, config.operation)));
    let output_path = output_relative
        .as_ref()
        .map(|relative| output_path_for(config, &input_path, relative));

    BatchWorkItem {
        input_path,
        input_relative,
        output_path,
        output_relative,
        skip_reason,
    }
}

fn execute_work_item(config: &BatchConfig, item: BatchWorkItem) -> BatchFileResult {
    if let Some(reason) = item.skip_reason {
        return BatchFileResult {
            input: item.input_relative,
            output: item.output_relative,
            status: BatchStatus::Skipped { reason },
        };
    }

    let input_size = match fs::metadata(&item.input_path) {
        Ok(meta) => meta.len(),
        Err(err) => {
            return BatchFileResult {
                input: item.input_relative,
                output: item.output_relative,
                status: BatchStatus::Error {
                    error: format!("Cannot read file metadata: {}", err),
                },
            };
        }
    };

    let content = match fs::read(&item.input_path) {
        Ok(bytes) => bytes,
        Err(err) => {
            return BatchFileResult {
                input: item.input_relative,
                output: item.output_relative,
                status: BatchStatus::Error {
                    error: format!("Cannot read file: {}", err),
                },
            };
        }
    };

    let (output_bytes, output_size) = match config.operation {
        BatchOp::Encode => {
            let encoded = encode_file_bytes(&content);
            let output_size = encoded.len() as u64;
            (encoded.into_bytes(), output_size)
        }
        BatchOp::Decode => {
            let text = match String::from_utf8(content) {
                Ok(text) => text,
                Err(_) => {
                    return BatchFileResult {
                        input: item.input_relative,
                        output: item.output_relative,
                        status: BatchStatus::Error {
                            error: "File is not valid UTF-8 (required for decode)".to_string(),
                        },
                    };
                }
            };

            match decode_file_bytes(&text) {
                Ok(decoded) => {
                    let output_size = decoded.len() as u64;
                    (decoded, output_size)
                }
                Err(err) => {
                    return BatchFileResult {
                        input: item.input_relative,
                        output: item.output_relative,
                        status: BatchStatus::Error { error: err },
                    };
                }
            }
        }
    };

    let output_path = item.output_path.as_ref().expect("work item output path");
    if let Some(parent) = output_path.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            return BatchFileResult {
                input: item.input_relative,
                output: item.output_relative,
                status: BatchStatus::Error {
                    error: format!("Cannot create output directory: {}", err),
                },
            };
        }
    }

    match fs::write(output_path, output_bytes) {
        Ok(()) => BatchFileResult {
            input: item.input_relative,
            output: item.output_relative,
            status: BatchStatus::Ok {
                input_size,
                output_size,
            },
        },
        Err(err) => BatchFileResult {
            input: item.input_relative,
            output: item.output_relative,
            status: BatchStatus::Error {
                error: format!("Cannot write output: {}", err),
            },
        },
    }
}

fn output_relative_for(input_relative: &Path, operation: BatchOp) -> PathBuf {
    let file_name = input_relative
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let output_name = match operation {
        BatchOp::Encode => format!("{}.b64", file_name),
        BatchOp::Decode => file_name
            .strip_suffix(".b64")
            .map(str::to_string)
            .unwrap_or_else(|| format!("{}.decoded", file_name)),
    };

    let mut path = input_relative.to_path_buf();
    path.set_file_name(output_name);
    path
}

fn output_path_for(config: &BatchConfig, input_path: &Path, output_relative: &Path) -> PathBuf {
    match &config.output_dir {
        Some(dir) => dir.join(output_relative),
        None => input_path.with_file_name(
            output_relative
                .file_name()
                .expect("output relative path must have a file name"),
        ),
    }
}

fn make_relative(path: &Path, root: &Path) -> PathBuf {
    path.strip_prefix(root)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| {
            path.file_name()
                .map(PathBuf::from)
                .unwrap_or_else(|| path.to_path_buf())
        })
}

fn common_parent(base: &Path, path: &Path) -> PathBuf {
    let other = path.parent().unwrap_or(path);
    let base_components: Vec<_> = base.components().collect();
    let other_components: Vec<_> = other.components().collect();
    let shared_len = base_components
        .iter()
        .zip(other_components.iter())
        .take_while(|(left, right)| left == right)
        .count();

    if shared_len == 0 {
        PathBuf::new()
    } else {
        base_components[..shared_len].iter().collect()
    }
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Simple ISO 8601-like timestamp from Unix seconds.
fn chrono_like_timestamp(secs: u64) -> String {
    let days_since_epoch = secs / 86_400;
    let time_of_day = secs % 86_400;
    let hours = time_of_day / 3_600;
    let minutes = (time_of_day % 3_600) / 60;
    let seconds = time_of_day % 60;

    let mut days = days_since_epoch as i64;
    let mut year = 1970i64;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }

    let month_lengths = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 0usize;
    for (index, month_len) in month_lengths.iter().enumerate() {
        if days < *month_len as i64 {
            month = index;
            break;
        }
        days -= *month_len as i64;
    }

    let day = days + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year,
        month + 1,
        day,
        hours,
        minutes,
        seconds
    )
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    fn setup_test_dir() -> (TempDir, PathBuf) {
        let tmp = TempDir::new().expect("create temp dir");
        let dir = tmp.path().to_path_buf();

        fs::write(dir.join("hello.txt"), "Hello, world!").expect("write text");
        fs::write(dir.join("data.json"), r#"{"key": "value"}"#).expect("write json");

        let nested_dir = dir.join("nested");
        fs::create_dir_all(&nested_dir).expect("create nested dir");
        fs::write(nested_dir.join("photo.png"), [1_u8, 2, 3, 4]).expect("write nested file");

        let b64_content = general_purpose::STANDARD.encode(b"decoded content");
        fs::write(dir.join("encoded.txt.b64"), &b64_content).expect("write b64 file");

        let nested_b64 = general_purpose::STANDARD.encode(b"nested decode");
        fs::write(nested_dir.join("child.txt.b64"), nested_b64).expect("write nested b64 file");

        (tmp, dir)
    }

    fn directory_config(
        root: &Path,
        operation: BatchOp,
        output_dir: Option<PathBuf>,
    ) -> BatchConfig {
        BatchConfig {
            source: BatchSource::directory(root.to_path_buf(), None),
            output_dir,
            operation,
            decode_b64_only: true,
        }
    }

    #[test]
    fn encode_file_bytes_roundtrip() {
        let original = b"test content";
        let encoded = encode_file_bytes(original);
        assert_eq!(
            general_purpose::STANDARD.decode(&encoded).unwrap(),
            original
        );
    }

    #[test]
    fn decode_file_bytes_valid() {
        let original = b"test content";
        let encoded = general_purpose::STANDARD.encode(original);
        let decoded = decode_file_bytes(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn decode_file_bytes_invalid() {
        let result = decode_file_bytes("not_validB64!!!");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid Base64"));
    }

    #[test]
    fn batch_encode_directory_preserves_relative_paths() {
        let (tmp, dir) = setup_test_dir();
        let output_dir = tmp.path().join("output");
        let config = directory_config(&dir, BatchOp::Encode, Some(output_dir.clone()));

        let result = process_batch_with_progress(&config, |_| {});

        assert_eq!(result.processed(), 5);
        assert_eq!(result.failed(), 0);
        assert!(output_dir.join("nested/photo.png.b64").exists());
        assert!(output_dir.join("hello.txt.b64").exists());
    }

    #[test]
    fn batch_decode_directory_skips_non_b64_and_preserves_tree() {
        let (tmp, dir) = setup_test_dir();
        let output_dir = tmp.path().join("decoded");
        let config = directory_config(&dir, BatchOp::Decode, Some(output_dir.clone()));

        let result = process_batch_with_progress(&config, |_| {});

        assert_eq!(result.processed(), 5);
        assert_eq!(result.succeeded(), 2);
        assert_eq!(result.skipped(), 3);
        assert!(output_dir.join("encoded.txt").exists());
        assert!(output_dir.join("nested/child.txt").exists());
    }

    #[test]
    fn duplicate_basenames_do_not_collide_in_output_dir() {
        let tmp = TempDir::new().expect("create temp dir");
        let root = tmp.path();
        let left = root.join("left");
        let right = root.join("right");
        fs::create_dir_all(&left).expect("create left");
        fs::create_dir_all(&right).expect("create right");
        fs::write(left.join("photo.png"), b"left").expect("write left");
        fs::write(right.join("photo.png"), b"right").expect("write right");

        let output_dir = root.join("out");
        let config = directory_config(root, BatchOp::Encode, Some(output_dir.clone()));
        let result = process_batch_with_progress(&config, |_| {});

        assert_eq!(result.succeeded(), 2);
        assert!(output_dir.join("left/photo.png.b64").exists());
        assert!(output_dir.join("right/photo.png.b64").exists());
    }

    #[test]
    fn explicit_file_selection_uses_common_parent_relative_paths() {
        let tmp = TempDir::new().expect("create temp dir");
        let root = tmp.path();
        let a = root.join("a");
        let b = root.join("b");
        fs::create_dir_all(&a).expect("create a");
        fs::create_dir_all(&b).expect("create b");
        let file_a = a.join("first.txt");
        let file_b = b.join("second.txt");
        fs::write(&file_a, b"a").expect("write a");
        fs::write(&file_b, b"b").expect("write b");

        let config = BatchConfig {
            source: BatchSource::files(vec![file_a.clone(), file_b.clone()]),
            output_dir: Some(root.join("out")),
            operation: BatchOp::Encode,
            decode_b64_only: true,
        };

        let result = process_batch_with_progress(&config, |_| {});
        assert_eq!(result.succeeded(), 2);
        assert_eq!(result.files[0].input, PathBuf::from("a/first.txt"));
        assert_eq!(result.files[1].input, PathBuf::from("b/second.txt"));
    }

    #[test]
    fn manifest_json_roundtrip_preserves_relative_paths() {
        let result = BatchResult {
            operation: BatchOp::Encode,
            processed: 2,
            succeeded: 1,
            failed: 1,
            skipped: 0,
            files: vec![
                BatchFileResult {
                    input: PathBuf::from("images/photo.png"),
                    output: Some(PathBuf::from("images/photo.png.b64")),
                    status: BatchStatus::Ok {
                        input_size: 45_231,
                        output_size: 60_308,
                    },
                },
                BatchFileResult {
                    input: PathBuf::from("notes.txt.b64"),
                    output: None,
                    status: BatchStatus::Error {
                        error: "Invalid Base64: ...".to_string(),
                    },
                },
            ],
            output_dir: Some(PathBuf::from("/output")),
            timestamp: "2026-04-11T12:00:00Z".to_string(),
        };

        let json = result.manifest_json().expect("serialize manifest");
        let manifest: BatchManifest = serde_json::from_str(&json).expect("deserialize manifest");

        assert_eq!(manifest.version, 1);
        assert_eq!(manifest.files[0].input, "images/photo.png");
        assert_eq!(
            manifest.files[0].output,
            Some("images/photo.png.b64".to_string())
        );
        assert_eq!(manifest.files[1].status, "error");
    }

    #[test]
    fn preview_batch_counts_total_and_eligible_files() {
        let (_tmp, dir) = setup_test_dir();
        let preview = preview_batch(&directory_config(&dir, BatchOp::Decode, None));

        assert_eq!(preview.file_count, 5);
        assert_eq!(preview.eligible_count, 2);
        assert_eq!(preview.source_kind, BatchSourceKind::Directory);
    }

    #[test]
    fn batch_progress_reports_incremental_updates() {
        let (_tmp, dir) = setup_test_dir();
        let mut progress_updates = Vec::new();

        let result = process_batch_with_progress(
            &directory_config(&dir, BatchOp::Decode, None),
            |progress| progress_updates.push(progress),
        );

        assert_eq!(progress_updates.len(), result.processed());
        assert_eq!(
            progress_updates.last().unwrap().processed,
            result.processed()
        );
        assert_eq!(
            progress_updates.last().unwrap().succeeded,
            result.succeeded()
        );
    }

    #[cfg(unix)]
    #[test]
    fn permission_errors_are_reported_in_results() {
        let tmp = TempDir::new().expect("create temp dir");
        let root = tmp.path().to_path_buf();
        let locked = root.join("locked");
        fs::create_dir_all(&locked).expect("create locked dir");
        fs::write(root.join("ok.txt"), b"ok").expect("write ok");
        fs::write(locked.join("secret.txt"), b"secret").expect("write secret");

        let mut perms = fs::metadata(&locked).expect("stat locked").permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&locked, perms.clone()).expect("lock dir");

        let result = process_batch_with_progress(
            &directory_config(&root, BatchOp::Encode, Some(root.join("out"))),
            |_| {},
        );

        let mut restore = perms;
        restore.set_mode(0o755);
        fs::set_permissions(&locked, restore).expect("unlock dir");

        assert!(result.failed() >= 1);
        assert!(result
            .files
            .iter()
            .any(|entry| matches!(entry.status, BatchStatus::Error { .. })));
    }

    #[test]
    fn timestamp_format_is_iso8601_like() {
        let ts = chrono_like_timestamp(0);
        assert_eq!(ts, "1970-01-01T00:00:00Z");

        let secs = calculate_epoch_secs(2026, 4, 11, 12, 30, 45);
        let ts = chrono_like_timestamp(secs);
        assert_eq!(ts, "2026-04-11T12:30:45Z");
    }

    fn calculate_epoch_secs(
        year: i64,
        month: usize,
        day: i64,
        hour: u64,
        min: u64,
        sec: u64,
    ) -> u64 {
        let mut days: i64 = 0;
        for current_year in 1970..year {
            days += if is_leap_year(current_year) { 366 } else { 365 };
        }

        let month_lengths = if is_leap_year(year) {
            [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        } else {
            [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        };

        for &month_len in month_lengths.iter().take(month - 1) {
            days += month_len as i64;
        }
        days += day - 1;

        (days as u64 * 86_400) + (hour * 3_600) + (min * 60) + sec
    }
}
