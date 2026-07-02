use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use sha2::{Sha256, Digest};
use futures_util::StreamExt;
use tokio::task::JoinSet;
use crate::types::{Lockfile, RuntimeLock};
use crate::cache::regenerate_shims_cache;

pub struct FileCleanupGuard {
    pub path: PathBuf,
    pub active: bool,
}

impl FileCleanupGuard {
    pub fn new(path: PathBuf) -> Self {
        Self { path, active: true }
    }
    
    pub fn deactivate(&mut self) {
        self.active = false;
    }
}

impl Drop for FileCleanupGuard {
    fn drop(&mut self) {
        if self.active {
            if self.path.exists() {
                let _ = fs::remove_file(&self.path);
            }
        }
    }
}

pub struct DirCleanupGuard {
    pub path: PathBuf,
    pub active: bool,
}

impl DirCleanupGuard {
    pub fn new(path: PathBuf) -> Self {
        Self { path, active: true }
    }
    
    pub fn deactivate(&mut self) {
        self.active = false;
    }
}

impl Drop for DirCleanupGuard {
    fn drop(&mut self) {
        if self.active {
            if self.path.exists() {
                let _ = fs::remove_dir_all(&self.path);
            }
        }
    }
}

pub fn compute_sha256(path: &Path) -> Result<String, String> {
    let mut file = File::open(path)
        .map_err(|e| format!("Failed to open file for hashing: {}", e))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];
    loop {
        let count = file.read(&mut buffer)
            .map_err(|e| format!("Failed to read file for hashing: {}", e))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub async fn download_runtime(
    lock: &RuntimeLock,
    cache_dir: &Path,
) -> Result<PathBuf, String> {
    let dest_dir = cache_dir.join(&lock.name).join(&lock.version);
    fs::create_dir_all(&dest_dir)
        .map_err(|e| format!("Failed to create destination directory: {}", e))?;
    
    let filename = lock.url.split('/').last().unwrap_or("archive");
    let dest_path = dest_dir.join(filename);
    
    if dest_path.exists() {
        if let Ok(hash) = compute_sha256(&dest_path) {
            if hash == lock.sha256 {
                return Ok(dest_path);
            }
        }
    }
    
    let mut file_guard = FileCleanupGuard::new(dest_path.clone());
    
    let client = reqwest::Client::new();
    let response = client.get(&lock.url)
        .send()
        .await
        .map_err(|e| format!("Failed to send download request: {}", e))?;
        
    if !response.status().is_success() {
        return Err(format!("Download failed with status: {}", response.status()));
    }
    
    let mut file = File::create(&dest_path)
        .map_err(|e| format!("Failed to create local archive: {}", e))?;
        
    let mut stream = response.bytes_stream();
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| format!("Error during stream chunk: {}", e))?;
        file.write_all(&chunk)
            .map_err(|e| format!("Failed to write to file: {}", e))?;
    }
    
    file.sync_all().map_err(|e| format!("Failed to sync file: {}", e))?;
    drop(file);
    
    let computed_hash = compute_sha256(&dest_path)?;
    if computed_hash != lock.sha256 {
        return Err(format!(
            "SHA-256 mismatch for {}: expected {}, got {}",
            lock.name, lock.sha256, computed_hash
        ));
    }
    
    file_guard.deactivate();
    Ok(dest_path)
}

pub trait Extractor: Send + Sync {
    fn extract(&self, archive: &Path, dest: &Path) -> Result<(), String>;
}

pub fn check_path_traversal(dest: &Path, entry_path: &Path) -> Result<PathBuf, String> {
    let dest_canon = dest.canonicalize().unwrap_or_else(|_| dest.to_path_buf());
    
    let mut normalized_entry = PathBuf::new();
    for component in entry_path.components() {
        match component {
            std::path::Component::ParentDir => {
                if !normalized_entry.pop() {
                    return Err(format!("Path traversal attempt detected (parent escape): {:?}", entry_path));
                }
            }
            std::path::Component::Normal(c) => {
                normalized_entry.push(c);
            }
            std::path::Component::CurDir => {}
            std::path::Component::Prefix(_) | std::path::Component::RootDir => {
                return Err(format!("Absolute path or prefix in archive: {:?}", entry_path));
            }
        }
    }
    
    let outpath = dest_canon.join(normalized_entry);
    
    if !outpath.starts_with(&dest_canon) {
        return Err(format!("Path traversal attempt detected (out of bounds): {:?}", entry_path));
    }
    
    Ok(outpath)
}

pub struct ZipExtractor;
impl Extractor for ZipExtractor {
    fn extract(&self, archive: &Path, dest: &Path) -> Result<(), String> {
        let file = File::open(archive)
            .map_err(|e| format!("Failed to open zip archive: {}", e))?;
        let mut zip_archive = zip::ZipArchive::new(file)
            .map_err(|e| format!("Failed to parse zip archive: {}", e))?;
            
        for i in 0..zip_archive.len() {
            let mut file = zip_archive.by_index(i)
                .map_err(|e| format!("Failed to get zip entry: {}", e))?;
            
            let raw_name = file.name();
            let entry_path = Path::new(raw_name);
            let outpath = check_path_traversal(dest, entry_path)?;
            
            if raw_name.ends_with('/') {
                fs::create_dir_all(&outpath)
                    .map_err(|e| format!("Failed to create directory: {}", e))?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        fs::create_dir_all(p)
                            .map_err(|e| format!("Failed to create directory: {}", e))?;
                    }
                }
                let mut outfile = File::create(&outpath)
                    .map_err(|e| format!("Failed to create extracted file: {}", e))?;
                io::copy(&mut file, &mut outfile)
                    .map_err(|e| format!("Failed to write extracted file: {}", e))?;
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = file.unix_mode() {
                    fs::set_permissions(&outpath, fs::Permissions::from_mode(mode)).ok();
                }
            }
        }
        Ok(())
    }
}

pub struct TarGzExtractor;
impl Extractor for TarGzExtractor {
    fn extract(&self, archive: &Path, dest: &Path) -> Result<(), String> {
        let file = File::open(archive)
            .map_err(|e| format!("Failed to open tar.gz archive: {}", e))?;
        let tar_gz = flate2::read::GzDecoder::new(file);
        let mut tar_archive = tar::Archive::new(tar_gz);
        
        let entries = tar_archive.entries()
            .map_err(|e| format!("Failed to get tar.gz entries: {}", e))?;
            
        for entry_res in entries {
            let mut entry = entry_res
                .map_err(|e| format!("Failed to read tar.gz entry: {}", e))?;
            
            let entry_path = entry.path()
                .map_err(|e| format!("Failed to get tar.gz entry path: {}", e))?
                .to_path_buf();
                
            let outpath = check_path_traversal(dest, &entry_path)?;
            
            if entry.header().entry_type().is_dir() {
                fs::create_dir_all(&outpath)
                    .map_err(|e| format!("Failed to create directory: {}", e))?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        fs::create_dir_all(p)
                            .map_err(|e| format!("Failed to create directory: {}", e))?;
                    }
                }
                let mut outfile = File::create(&outpath)
                    .map_err(|e| format!("Failed to create extracted file: {}", e))?;
                io::copy(&mut entry, &mut outfile)
                    .map_err(|e| format!("Failed to write extracted file: {}", e))?;
            }
        }
        Ok(())
    }
}

pub struct TarXzExtractor;
impl Extractor for TarXzExtractor {
    fn extract(&self, archive: &Path, dest: &Path) -> Result<(), String> {
        let file = File::open(archive)
            .map_err(|e| format!("Failed to open tar.xz archive: {}", e))?;
        let tar_xz = xz2::read::XzDecoder::new(file);
        let mut tar_archive = tar::Archive::new(tar_xz);
        
        let entries = tar_archive.entries()
            .map_err(|e| format!("Failed to get tar.xz entries: {}", e))?;
            
        for entry_res in entries {
            let mut entry = entry_res
                .map_err(|e| format!("Failed to read tar.xz entry: {}", e))?;
            
            let entry_path = entry.path()
                .map_err(|e| format!("Failed to get tar.xz entry path: {}", e))?
                .to_path_buf();
                
            let outpath = check_path_traversal(dest, &entry_path)?;
            
            if entry.header().entry_type().is_dir() {
                fs::create_dir_all(&outpath)
                    .map_err(|e| format!("Failed to create directory: {}", e))?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        fs::create_dir_all(p)
                            .map_err(|e| format!("Failed to create directory: {}", e))?;
                    }
                }
                let mut outfile = File::create(&outpath)
                    .map_err(|e| format!("Failed to create extracted file: {}", e))?;
                io::copy(&mut entry, &mut outfile)
                    .map_err(|e| format!("Failed to write extracted file: {}", e))?;
            }
        }
        Ok(())
    }
}

pub fn extract_archive(archive_path: &Path, extract_to: &Path) -> Result<(), String> {
    let path_str = archive_path.to_string_lossy();
    if path_str.ends_with(".zip") {
        ZipExtractor.extract(archive_path, extract_to)
    } else if path_str.ends_with(".tar.gz") || path_str.ends_with(".tgz") {
        TarGzExtractor.extract(archive_path, extract_to)
    } else if path_str.ends_with(".tar.xz") || path_str.ends_with(".txz") {
        TarXzExtractor.extract(archive_path, extract_to)
    } else {
        Err(format!("Unsupported archive format: {}", path_str))
    }
}

pub async fn install_runtime_transactional(
    lock: &RuntimeLock,
    workspace_root: &Path,
    cache_dir: &Path,
    operation_id: &str,
    event_bus: Option<&crate::event_bus::EventBus>,
) -> Result<Vec<crate::types::ChangeRecord>, String> {
    let target_dir = cache_dir.join(&lock.name).join(&lock.version);
    let target_extract_to = target_dir.join("extracted");

    // If already installed, skip
    if target_extract_to.exists() {
        if let Ok(mut entries) = fs::read_dir(&target_extract_to) {
            if entries.next().is_some() {
                return Ok(Vec::new());
            }
        }
    }

    // 1. Setup staging path
    let staging_dir = workspace_root.join(".forge").join("staging").join(operation_id).join(&lock.name).join(&lock.version);
    fs::create_dir_all(&staging_dir)
        .map_err(|e| format!("Failed to create staging dir: {}", e))?;

    let filename = lock.url.split('/').last().unwrap_or("archive");
    let staging_archive_path = staging_dir.join(filename);
    let staging_extract_to = staging_dir.join("extracted");

    if let Some(eb) = event_bus {
        let _ = eb.publish(crate::types::Event {
            timestamp: "2026-07-01T09:24:30-05:00".to_string(),
            operation_id: operation_id.to_string(),
            runtime: lock.name.clone(),
            phase: "Download".to_string(),
            status: crate::types::EventStatus::Started,
            message: Some(format!("Downloading {} v{}", lock.name, lock.version)),
        });
    }

    // Check if we can skip download (e.g. for offline unit testing or cached files)
    let mut need_download = true;
    if staging_archive_path.exists() {
        if let Ok(h) = compute_sha256(&staging_archive_path) {
            if h == lock.sha256 {
                need_download = false;
            }
        }
    }

    if need_download {
        if lock.url.starts_with("file://") {
            let mut url_path = lock.url.trim_start_matches("file://");
            if url_path.starts_with('/') && url_path.chars().nth(2) == Some(':') {
                url_path = &url_path[1..];
            }
            let src_path = Path::new(url_path);
            fs::copy(src_path, &staging_archive_path)
                .map_err(|e| format!("Failed to copy file from {}: {}", url_path, e))?;
        } else {
            let client = reqwest::Client::new();
            let response = client.get(&lock.url)
                .send()
                .await
                .map_err(|e| format!("Failed to send download request: {}", e))?;
                
            if !response.status().is_success() {
                return Err(format!("Download failed with status: {}", response.status()));
            }
            
            let mut file = File::create(&staging_archive_path)
                .map_err(|e| format!("Failed to create local staging archive: {}", e))?;
                
            let mut stream = response.bytes_stream();
            while let Some(chunk_result) = stream.next().await {
                let chunk = chunk_result.map_err(|e| format!("Error during stream chunk: {}", e))?;
                file.write_all(&chunk)
                    .map_err(|e| format!("Failed to write to file: {}", e))?;
            }
            file.sync_all().map_err(|e| format!("Failed to sync file: {}", e))?;
            drop(file);
        }

        let computed_hash = compute_sha256(&staging_archive_path)?;
        if computed_hash != lock.sha256 {
            let _ = fs::remove_dir_all(&staging_dir);
            return Err(format!(
                "SHA-256 mismatch for {}: expected {}, got {}",
                lock.name, lock.sha256, computed_hash
            ));
        }
    }

    if let Some(eb) = event_bus {
        let _ = eb.publish(crate::types::Event {
            timestamp: "2026-07-01T09:24:30-05:00".to_string(),
            operation_id: operation_id.to_string(),
            runtime: lock.name.clone(),
            phase: "Extract".to_string(),
            status: crate::types::EventStatus::Progress(50),
            message: Some(format!("Extracting {} v{}", lock.name, lock.version)),
        });
    }

    // 3. Extract to staging_extract_to
    fs::create_dir_all(&staging_extract_to)
        .map_err(|e| format!("Failed to create staging extract dir: {}", e))?;
    
    extract_archive(&staging_archive_path, &staging_extract_to)?;

    // Validation: make sure extraction was not empty
    let entries = fs::read_dir(&staging_extract_to)
        .map_err(|e| format!("Failed to read staging extract dir: {}", e))?;
    if entries.count() == 0 {
        let _ = fs::remove_dir_all(&staging_dir);
        return Err(format!("Extraction produced no files for {}", lock.name));
    }

    // 4. Commit (Promotion) with Backup/Rollback
    fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Failed to create final cache dir: {}", e))?;

    let backup_dir = workspace_root.join(".forge").join("backup").join(operation_id).join(&lock.name).join(&lock.version);
    let backup_extract_to = backup_dir.join("extracted");

    let mut backup_created = false;
    if target_extract_to.exists() {
        fs::create_dir_all(&backup_dir)
            .map_err(|e| format!("Failed to create backup dir: {}", e))?;
        fs::rename(&target_extract_to, &backup_extract_to)
            .map_err(|e| format!("Failed to move target to backup: {}", e))?;
        backup_created = true;
    }

    // Promote staging_extract_to to target_extract_to
    let promote_res = fs::rename(&staging_extract_to, &target_extract_to);
    if let Err(err) = promote_res {
        // Rollback: delete target_extract_to, restore backup if any, delete staging
        let _ = fs::remove_dir_all(&target_extract_to);
        if backup_created {
            let _ = fs::create_dir_all(target_extract_to.parent().unwrap()).ok();
            let _ = fs::rename(&backup_extract_to, &target_extract_to);
        }
        let _ = fs::remove_dir_all(&staging_dir);
        return Err(format!("Promotion failed: {}", err));
    }

    // Successful commit! Cleanup staging and backup
    let _ = fs::remove_dir_all(&staging_dir);
    if backup_created {
        let _ = fs::remove_dir_all(&backup_dir);
    }

    if let Some(eb) = event_bus {
        let _ = eb.publish(crate::types::Event {
            timestamp: "2026-07-01T09:24:30-05:00".to_string(),
            operation_id: operation_id.to_string(),
            runtime: lock.name.clone(),
            phase: "Commit".to_string(),
            status: crate::types::EventStatus::Finished,
            message: Some(format!("Successfully installed {} v{}", lock.name, lock.version)),
        });
    }

    let mut changes = Vec::new();
    changes.push(crate::types::ChangeRecord {
        path: target_extract_to.to_string_lossy().to_string(),
        action: "added".to_string(),
    });

    Ok(changes)
}

pub async fn install_runtimes(
    lockfile: &Lockfile,
    cache_dir: &Path,
) -> Result<(), String> {
    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let workspace_root = crate::manifest::find_forge_toml(&current_dir)
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| current_dir.clone());
    
    let operation_id = format!(
        "op-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );

    let mut join_set = JoinSet::new();
    for runtime in &lockfile.runtimes {
        let r = runtime.clone();
        let w = workspace_root.clone();
        let c = cache_dir.to_path_buf();
        let op_id = operation_id.clone();
        join_set.spawn(async move {
            install_runtime_transactional(&r, &w, &c, &op_id, None).await
        });
    }

    while let Some(res) = join_set.join_next().await {
        match res {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                join_set.abort_all();
                return Err(e);
            }
            Err(e) => {
                join_set.abort_all();
                return Err(format!("Task join error: {}", e));
            }
        }
    }

    // Decouple shims cache regeneration: run only upon successful commit
    let _ = regenerate_shims_cache(lockfile, cache_dir, &workspace_root);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RuntimeLock;

    #[tokio::test]
    async fn test_installer_successful_install() {
        let temp_dir = std::env::temp_dir().join("forge_installer_success_test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let workspace_root = temp_dir.join("workspace");
        let cache_dir = temp_dir.join("cache");
        fs::create_dir_all(&workspace_root).unwrap();
        fs::create_dir_all(&cache_dir).unwrap();

        // Create a source archive that we can copy using file://
        let src_archive = temp_dir.join("node.zip");
        {
            let file = File::create(&src_archive).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            zip.start_file("hello.txt", zip::write::FileOptions::default()).unwrap();
            zip.write_all(b"hello world").unwrap();
            zip.finish().unwrap();
        }

        let sha = compute_sha256(&src_archive).unwrap();

        let lock = RuntimeLock {
            name: "node".to_string(),
            version: "20.10.0".to_string(),
            platform: "windows".to_string(),
            arch: "x86_64".to_string(),
            url: format!("file://{}", src_archive.to_string_lossy().replace('\\', "/")),
            size: 123,
            sha256: sha,
            emulation: None,
        };

        let res = install_runtime_transactional(&lock, &workspace_root, &cache_dir, "op-test-success", None).await;
        assert!(res.is_ok());
        let changes = res.unwrap();
        assert_eq!(changes.len(), 1);

        // Verify target extracted contains the file
        let target_file = cache_dir.join("node").join("20.10.0").join("extracted").join("hello.txt");
        assert!(target_file.exists());
        let content = fs::read_to_string(target_file).unwrap();
        assert_eq!(content, "hello world");
    }

    #[tokio::test]
    async fn test_installer_hash_mismatch_rollback() {
        let temp_dir = std::env::temp_dir().join("forge_installer_mismatch_test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let workspace_root = temp_dir.join("workspace");
        let cache_dir = temp_dir.join("cache");
        fs::create_dir_all(&workspace_root).unwrap();
        fs::create_dir_all(&cache_dir).unwrap();

        let src_archive = temp_dir.join("node.zip");
        {
            let file = File::create(&src_archive).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            zip.start_file("hello.txt", zip::write::FileOptions::default()).unwrap();
            zip.write_all(b"hello world").unwrap();
            zip.finish().unwrap();
        }

        let lock = RuntimeLock {
            name: "node".to_string(),
            version: "20.10.0".to_string(),
            platform: "windows".to_string(),
            arch: "x86_64".to_string(),
            url: format!("file://{}", src_archive.to_string_lossy().replace('\\', "/")),
            size: 123,
            sha256: "incorrect_sha_value".to_string(),
            emulation: None,
        };

        let res = install_runtime_transactional(&lock, &workspace_root, &cache_dir, "op-test-mismatch", None).await;
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("SHA-256 mismatch"));

        // Verify staging directory is cleaned up
        let staging_dir = workspace_root.join(".forge").join("staging").join("op-test-mismatch").join("node").join("20.10.0");
        assert!(!staging_dir.exists());
    }

    #[tokio::test]
    async fn test_installer_validation_failure_rollback() {
        let temp_dir = std::env::temp_dir().join("forge_installer_validation_test");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let workspace_root = temp_dir.join("workspace");
        let cache_dir = temp_dir.join("cache");
        fs::create_dir_all(&workspace_root).unwrap();
        fs::create_dir_all(&cache_dir).unwrap();

        let src_archive = temp_dir.join("node.zip");
        {
            let file = File::create(&src_archive).unwrap();
            let _zip = zip::ZipWriter::new(file); // empty zip
        }

        let sha = compute_sha256(&src_archive).unwrap();

        let lock = RuntimeLock {
            name: "node".to_string(),
            version: "20.10.0".to_string(),
            platform: "windows".to_string(),
            arch: "x86_64".to_string(),
            url: format!("file://{}", src_archive.to_string_lossy().replace('\\', "/")),
            size: 123,
            sha256: sha,
            emulation: None,
        };

        let res = install_runtime_transactional(&lock, &workspace_root, &cache_dir, "op-test-validation", None).await;
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Extraction produced no files"));

        // Verify staging cleaned up
        let staging_dir = workspace_root.join(".forge").join("staging").join("op-test-validation").join("node").join("20.10.0");
        assert!(!staging_dir.exists());
    }
}
