use std::fs::{self, File};
use std::path::Path;
use std::io::Write;
use sha2::{Sha256, Digest};
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use forge_core::{
    detect_platform, detect_arch, download_runtime, install_runtimes,
    ZipExtractor, TarGzExtractor, TarXzExtractor, Extractor, check_path_traversal,
    RuntimeLock, Lockfile, Operation,
};

async fn start_mock_server() -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = [0; 1024];
                let _ = socket.read(&mut buf).await;
                let body = "hello world";
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.flush().await;
            });
        }
    });
    (format!("http://{}", addr), handle)
}

fn create_test_zip(dest: &Path, file_name: &str, file_content: &str) {
    let file = File::create(dest).unwrap();
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::default();
    zip.start_file(file_name, options).unwrap();
    zip.write_all(file_content.as_bytes()).unwrap();
    zip.finish().unwrap();
}

fn create_test_tar_gz(dest: &Path, file_name: &str, file_content: &str) {
    let file = File::create(dest).unwrap();
    let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut tar = tar::Builder::new(enc);
    let mut header = tar::Header::new_gnu();
    header.set_size(file_content.len() as u64);
    header.set_mode(0o644);
    tar.append_data(&mut header, file_name, file_content.as_bytes()).unwrap();
    tar.finish().unwrap();
}

fn create_test_tar_xz(dest: &Path, file_name: &str, file_content: &str) {
    let file = File::create(dest).unwrap();
    let enc = xz2::write::XzEncoder::new(file, 6);
    let mut tar = tar::Builder::new(enc);
    let mut header = tar::Header::new_gnu();
    header.set_size(file_content.len() as u64);
    header.set_mode(0o644);
    tar.append_data(&mut header, file_name, file_content.as_bytes()).unwrap();
    tar.finish().unwrap();
}

#[tokio::test]
async fn test_download_sha_mismatch_and_deletion() {
    let (url, _server_handle) = start_mock_server().await;
    let temp_dir = std::env::temp_dir().join("forge_test_cache_integration");
    fs::create_dir_all(&temp_dir).unwrap();
    
    let mut lock = RuntimeLock {
        name: "test_tool_integration".to_string(),
        version: "1.0.0".to_string(),
        platform: detect_platform().to_string(),
        arch: detect_arch().to_string(),
        url,
        size: 11,
        sha256: "incorrect_sha256_here".to_string(),
        emulation: None,
    };
    
    let res = download_runtime(&lock, &temp_dir).await;
    assert!(res.is_err());
    
    let dest_dir = temp_dir.join(&lock.name).join(&lock.version);
    let filename = lock.url.split('/').last().unwrap_or("archive");
    let dest_path_real = dest_dir.join(filename);
    assert!(!dest_path_real.exists(), "File should be deleted on SHA mismatch");
    
    // Now test matching SHA
    let correct_hash = hex::encode(Sha256::digest(b"hello world"));
    lock.sha256 = correct_hash;
    
    let res_ok = download_runtime(&lock, &temp_dir).await;
    assert!(res_ok.is_ok());
    let downloaded_file = res_ok.unwrap();
    assert!(downloaded_file.exists());
    
    fs::remove_dir_all(&temp_dir).ok();
}

#[test]
fn test_standard_archives_extraction() {
    let temp_dir = std::env::temp_dir().join("standard_extract_test_integration");
    let archive_dir = std::env::temp_dir().join("standard_extract_archives_integration");
    fs::create_dir_all(&temp_dir).unwrap();
    fs::create_dir_all(&archive_dir).unwrap();

    // 1. ZIP
    let zip_path = archive_dir.join("good.zip");
    create_test_zip(&zip_path, "good_zip.txt", "content zip");
    let zip_dest = temp_dir.join("zip_out");
    fs::create_dir_all(&zip_dest).unwrap();
    ZipExtractor.extract(&zip_path, &zip_dest).unwrap();
    assert_eq!(fs::read_to_string(zip_dest.join("good_zip.txt")).unwrap(), "content zip");

    // 2. TarGz
    let tar_gz_path = archive_dir.join("good.tar.gz");
    create_test_tar_gz(&tar_gz_path, "good_targz.txt", "content targz");
    let targz_dest = temp_dir.join("targz_out");
    fs::create_dir_all(&targz_dest).unwrap();
    TarGzExtractor.extract(&tar_gz_path, &targz_dest).unwrap();
    assert_eq!(fs::read_to_string(targz_dest.join("good_targz.txt")).unwrap(), "content targz");

    // 3. TarXz
    let tar_xz_path = archive_dir.join("good.tar.xz");
    create_test_tar_xz(&tar_xz_path, "good_tarxz.txt", "content tarxz");
    let tarxz_dest = temp_dir.join("tarxz_out");
    fs::create_dir_all(&tarxz_dest).unwrap();
    TarXzExtractor.extract(&tar_xz_path, &tarxz_dest).unwrap();
    assert_eq!(fs::read_to_string(tarxz_dest.join("good_tarxz.txt")).unwrap(), "content tarxz");

    fs::remove_dir_all(&temp_dir).ok();
    fs::remove_dir_all(&archive_dir).ok();
}

#[test]
fn test_zip_slip_prevention() {
    let temp_dir = std::env::temp_dir().join("zip_slip_test_integration");
    let archive_dir = std::env::temp_dir().join("zip_slip_archives_integration");
    fs::create_dir_all(&temp_dir).unwrap();
    fs::create_dir_all(&archive_dir).unwrap();

    // Test Zip Slip using ZipExtractor
    let zip_path = archive_dir.join("bad.zip");
    create_test_zip(&zip_path, "../escaped.txt", "escaped content");

    let res = ZipExtractor.extract(&zip_path, &temp_dir);
    assert!(res.is_err(), "Zip Slip should return Err");
    assert!(!temp_dir.parent().unwrap().join("escaped.txt").exists());

    // Directly test check_path_traversal helper
    #[cfg(windows)]
    let dest = Path::new("C:\\allowed\\directory");
    #[cfg(not(windows))]
    let dest = Path::new("/allowed/directory");
    
    let path_traversal_1 = check_path_traversal(dest, Path::new("../escaped.txt"));
    assert!(path_traversal_1.is_err());

    let path_traversal_2 = check_path_traversal(dest, Path::new("foo/../../escaped.txt"));
    assert!(path_traversal_2.is_err());

    let path_traversal_3 = check_path_traversal(dest, Path::new("/escaped.txt"));
    assert!(path_traversal_3.is_err());

    let path_ok = check_path_traversal(dest, Path::new("subfolder/file.txt"));
    assert!(path_ok.is_ok());

    fs::remove_dir_all(&temp_dir).ok();
    fs::remove_dir_all(&archive_dir).ok();
}

#[tokio::test]
async fn test_parallel_download_and_abort() {
    let (url, _server_handle) = start_mock_server().await;
    let temp_dir = std::env::temp_dir().join("forge_parallel_test_integration");
    fs::create_dir_all(&temp_dir).unwrap();
    
    let good_hash = hex::encode(Sha256::digest(b"hello world"));
    
    let lockfile = Lockfile {
        runtimes: vec![
            RuntimeLock {
                name: "good_runtime".to_string(),
                version: "1.0.0".to_string(),
                platform: detect_platform().to_string(),
                arch: detect_arch().to_string(),
                url: url.clone(),
                size: 11,
                sha256: good_hash,
                emulation: None,
            },
            RuntimeLock {
                name: "bad_runtime".to_string(),
                version: "2.0.0".to_string(),
                platform: detect_platform().to_string(),
                arch: detect_arch().to_string(),
                url: url.clone(),
                size: 11,
                sha256: "bad_sha_hash".to_string(),
                emulation: None,
            },
        ],
    };
    
    let res = install_runtimes(&lockfile, &temp_dir).await;
    assert!(res.is_err());
    
    let bad_dest = temp_dir.join("bad_runtime").join("2.0.0").join("extracted");
    assert!(!bad_dest.exists());
    
    fs::remove_dir_all(&temp_dir).ok();
}

#[tokio::test]
async fn test_sync_idempotency_skipped() {
    let temp_dir = std::env::temp_dir().join("forge_sync_idempotency_test");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let workspace_root = temp_dir.join("workspace");
    let cache_dir = temp_dir.join("cache");
    fs::create_dir_all(&workspace_root).unwrap();
    fs::create_dir_all(&cache_dir).unwrap();

    let orig_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&workspace_root).unwrap();

    // 1. Create a mock zip file to use as a file:// target
    let src_archive = temp_dir.join("node.zip");
    {
        let file = File::create(&src_archive).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("hello.txt", zip::write::FileOptions::default()).unwrap();
        zip.write_all(b"hello world").unwrap();
        zip.finish().unwrap();
    }

    let sha = forge_core::installer::compute_sha256(&src_archive).unwrap();

    // 2. Set up workspace with forge.toml and forge.lock
    fs::write(
        workspace_root.join("forge.toml"),
        "[runtimes]\nnode = \"20.10.0\"\n",
    )
    .unwrap();

    let lockfile = Lockfile {
        runtimes: vec![RuntimeLock {
            name: "node".to_string(),
            version: "20.10.0".to_string(),
            platform: "windows".to_string(),
            arch: "x86_64".to_string(),
            url: format!("file://{}", src_archive.to_string_lossy().replace('\\', "/")),
            size: 123,
            sha256: sha,
            emulation: None,
        }],
    };

    let lock_content = toml::to_string_pretty(&lockfile).unwrap();
    fs::write(workspace_root.join("forge.lock"), lock_content).unwrap();

    // 3. Run SyncOperation first time -> should install (Success)
    let event_bus = forge_core::event_bus::EventBus::new(10);
    let mut ctx = forge_core::operations::Context::new(workspace_root.clone(), cache_dir.clone(), event_bus);
    
    let sync_op = forge_core::operations::SyncOperation;
    let plan1 = sync_op.plan(&ctx).unwrap();
    let res1 = sync_op.execute(&mut ctx, plan1).await.unwrap();
    assert_eq!(res1.status, forge_core::types::OperationStatus::Success);
    assert_eq!(res1.changes.len(), 1);

    // 4. Run SyncOperation second time -> should skip (Skipped)
    let plan2 = sync_op.plan(&ctx).unwrap();
    let res2 = sync_op.execute(&mut ctx, plan2).await.unwrap();
    assert_eq!(res2.status, forge_core::types::OperationStatus::Skipped);
    assert_eq!(res2.changes.len(), 0);

    std::env::set_current_dir(orig_dir).ok();
    fs::remove_dir_all(&temp_dir).ok();
}

#[tokio::test]
async fn test_e2e_lifecycle_state_transitions() {
    let temp_dir = std::env::temp_dir().join("forge_e2e_transitions_test");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let workspace_root = temp_dir.join("workspace");
    let cache_dir = temp_dir.join("cache");
    fs::create_dir_all(&workspace_root).unwrap();
    fs::create_dir_all(&cache_dir).unwrap();

    let orig_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(&workspace_root).unwrap();

    let event_bus = forge_core::event_bus::EventBus::new(10);
    
    // helper to get state in test
    let get_state = |wr: &Path, cd: &Path| -> forge_core::types::LifecycleState {
        let toml_path = wr.join("forge.toml");
        if !toml_path.exists() {
            return forge_core::types::LifecycleState::Uninitialized;
        }
        let config = match forge_core::manifest::load_config(&toml_path) {
            Ok(c) => c,
            Err(_) => return forge_core::types::LifecycleState::Broken,
        };
        let lock_path = wr.join("forge.lock");
        if !lock_path.exists() {
            return forge_core::types::LifecycleState::Initialized;
        }
        let lockfile = match forge_core::load_lockfile(&lock_path) {
            Ok(l) => l,
            Err(_) => return forge_core::types::LifecycleState::Broken,
        };
        let mut config_runtimes = config.runtimes.clone();
        for runtime in &lockfile.runtimes {
            if config_runtimes.remove(&runtime.name).is_none() {
                return forge_core::types::LifecycleState::Outdated;
            }
        }
        if !config_runtimes.is_empty() {
            return forge_core::types::LifecycleState::Outdated;
        }
        let mut any_missing = false;
        for runtime in &lockfile.runtimes {
            let target_dir = cd.join(&runtime.name).join(&runtime.version).join("extracted");
            if !target_dir.exists() {
                any_missing = true;
            }
        }
        if any_missing {
            return forge_core::types::LifecycleState::Locked;
        }
        let shims_cache = wr.join(".forge").join("shims.cache");
        if !shims_cache.exists() {
            return forge_core::types::LifecycleState::Synced;
        }
        forge_core::types::LifecycleState::Ready
    };

    // State 1: Uninitialized (no forge.toml)
    assert_eq!(get_state(&workspace_root, &cache_dir), forge_core::types::LifecycleState::Uninitialized);

    // State 2: Initialized (run InitOperation)
    let mut ctx = forge_core::operations::Context::new(workspace_root.clone(), cache_dir.clone(), event_bus.clone());
    let init_op = forge_core::operations::InitOperation;
    let init_plan = init_op.plan(&ctx).unwrap();
    init_op.execute(&mut ctx, init_plan).await.unwrap();
    assert_eq!(get_state(&workspace_root, &cache_dir), forge_core::types::LifecycleState::Initialized);

    // State 3: Locked (run LockOperation)
    // First mock registry entry or write configuration with no dependencies for simple test, or local zip lockfile
    fs::write(
        workspace_root.join("forge.toml"),
        "[runtimes]\nnode = \"20.10.0\"\n",
    )
    .unwrap();

    let src_archive = temp_dir.join("node.zip");
    {
        let file = File::create(&src_archive).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("hello.txt", zip::write::FileOptions::default()).unwrap();
        zip.write_all(b"hello world").unwrap();
        zip.finish().unwrap();
    }
    let sha = forge_core::installer::compute_sha256(&src_archive).unwrap();

    let lockfile = Lockfile {
        runtimes: vec![RuntimeLock {
            name: "node".to_string(),
            version: "20.10.0".to_string(),
            platform: "windows".to_string(),
            arch: "x86_64".to_string(),
            url: format!("file://{}", src_archive.to_string_lossy().replace('\\', "/")),
            size: 123,
            sha256: sha,
            emulation: None,
        }],
    };
    fs::write(workspace_root.join("forge.lock"), toml::to_string_pretty(&lockfile).unwrap()).unwrap();

    assert_eq!(get_state(&workspace_root, &cache_dir), forge_core::types::LifecycleState::Locked);

    // State 4: Synced -> Ready (run SyncOperation)
    let sync_op = forge_core::operations::SyncOperation;
    let sync_plan = sync_op.plan(&ctx).unwrap();
    sync_op.execute(&mut ctx, sync_plan).await.unwrap();

    assert_eq!(get_state(&workspace_root, &cache_dir), forge_core::types::LifecycleState::Ready);

    std::env::set_current_dir(orig_dir).ok();
    fs::remove_dir_all(&temp_dir).ok();
}

#[tokio::test]
async fn test_explain_resolution() {
    let temp_dir = std::env::temp_dir().join("forge_test_explain_resolution");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let workspace_root = temp_dir.join("workspace");
    fs::create_dir_all(&workspace_root).unwrap();

    // 1. Create a config with a runtime
    fs::write(
        workspace_root.join("forge.toml"),
        "[runtimes]\nnode = \"20.10.0\"\n",
    )
    .unwrap();

    let engine = forge_core::Engine::new(workspace_root.clone()).unwrap();
    let res = engine.explain("node").await.unwrap();
    assert_eq!(res.runtime, "node");
    assert_eq!(res.state, "Initialized"); // forge.lock is missing
    
    // Now create a lockfile
    let lockfile = Lockfile {
        runtimes: vec![RuntimeLock {
            name: "node".to_string(),
            version: "20.10.0".to_string(),
            platform: "windows".to_string(),
            arch: "x86_64".to_string(),
            url: "https://example.com/node.zip".to_string(),
            size: 123,
            sha256: "dummy_sha".to_string(),
            emulation: None,
        }],
    };
    fs::write(workspace_root.join("forge.lock"), toml::to_string_pretty(&lockfile).unwrap()).unwrap();
    
    let res2 = engine.explain("node").await.unwrap();
    assert_eq!(res2.state, "Locked"); // cache missing

    fs::remove_dir_all(&temp_dir).ok();
}

#[tokio::test]
async fn test_trace_ascii_formatting() {
    let temp_dir = std::env::temp_dir().join("forge_test_trace_ascii");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();
    
    let journal_file = temp_dir.join(".forge").join("journal.jsonl");
    let event_bus = forge_core::event_bus::EventBus::new_with_journal(10, journal_file);

    let op_id = "op-trace-123";
    
    // Publish mock events
    event_bus.publish(forge_core::types::Event {
        timestamp: "2026-07-01T09:00:00.000-05:00".to_string(),
        operation_id: op_id.to_string(),
        runtime: "all".to_string(),
        phase: "Inspect".to_string(),
        status: forge_core::types::EventStatus::Started,
        message: None,
        ..Default::default()
    }).unwrap();

    event_bus.publish(forge_core::types::Event {
        timestamp: "2026-07-01T09:00:00.010-05:00".to_string(),
        operation_id: op_id.to_string(),
        runtime: "all".to_string(),
        phase: "Inspect".to_string(),
        status: forge_core::types::EventStatus::Finished,
        message: None,
        ..Default::default()
    }).unwrap();

    event_bus.publish(forge_core::types::Event {
        timestamp: "2026-07-01T09:00:00.020-05:00".to_string(),
        operation_id: op_id.to_string(),
        runtime: "node".to_string(),
        phase: "Download".to_string(),
        status: forge_core::types::EventStatus::Started,
        message: None,
        ..Default::default()
    }).unwrap();

    event_bus.publish(forge_core::types::Event {
        timestamp: "2026-07-01T09:00:00.120-05:00".to_string(),
        operation_id: op_id.to_string(),
        runtime: "node".to_string(),
        phase: "Download".to_string(),
        status: forge_core::types::EventStatus::Finished,
        message: None,
        ..Default::default()
    }).unwrap();

    // wait for background writes
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let engine = forge_core::Engine::new(temp_dir.clone()).unwrap();
    let trace = engine.trace(op_id).await.unwrap();
    
    assert!(trace.contains("Operation: op-trace-123"));
    assert!(trace.contains("Inspect (10ms)"));
    assert!(trace.contains("Runtime: node"));
    assert!(trace.contains("Download (100ms)"));

    fs::remove_dir_all(&temp_dir).ok();
}

#[tokio::test]
async fn test_events_live_tailing() {
    let temp_dir = std::env::temp_dir().join("forge_test_live_tailing");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let journal_file = temp_dir.join(".forge").join("journal.jsonl");
    let event_bus = forge_core::event_bus::EventBus::new_with_journal(10, journal_file);
    
    // First, start the live events tailing on the engine
    let engine = forge_core::Engine::new(temp_dir.clone()).unwrap();
    let mut rx = engine.events(true).await.unwrap();

    // Now publish an event
    let event = forge_core::types::Event {
        timestamp: "2026-07-01T09:00:00.000-05:00".to_string(),
        operation_id: "op-live-456".to_string(),
        runtime: "all".to_string(),
        phase: "LivePhase".to_string(),
        status: forge_core::types::EventStatus::Started,
        message: Some("Live message".to_string()),
        ..Default::default()
    };
    event_bus.publish(event.clone()).unwrap();

    // The receiver should get the event in real-time
    let received = tokio::time::timeout(tokio::time::Duration::from_secs(5), rx.recv()).await;
    assert!(received.is_ok());
    let opt_event = received.unwrap();
    assert!(opt_event.is_some());
    let ev = opt_event.unwrap();
    assert_eq!(ev.operation_id, "op-live-456");
    assert_eq!(ev.phase, "LivePhase");

    fs::remove_dir_all(&temp_dir).ok();
}

#[tokio::test]
async fn test_e2e_env_and_secrets() {
    use rand::Rng;
    use std::collections::HashMap;

    let temp_dir = std::env::temp_dir().join(format!("forge_test_e2e_env_secrets_{}", rand::thread_rng().gen::<u32>()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    std::env::set_var("FORGE_MASTER_KEY", "integration-test-master-key");

    let forge_toml_content = r#"
workspace_id = "test-e2e-secrets-workspace"

[config.definitions.PORT]
type = "integer"
required = true
default = 8080
description = "Web server port"

[config.definitions.DB_PASS]
type = "string"
required = true
secret = true
description = "Database secret password"
"#;
    fs::write(temp_dir.join("forge.toml"), forge_toml_content).unwrap();

    let forge_secrets_content = r#"
[secrets]
DB_PASS = { provider = "file" }
"#;
    fs::write(temp_dir.join("forge.secrets"), forge_secrets_content).unwrap();

    let engine = forge_core::Engine::new(temp_dir.clone()).unwrap();

    engine.env_set("MY_VAR", "my-val").await.unwrap();
    let val = engine.env_get("MY_VAR").await.unwrap();
    assert_eq!(val, Some("my-val".to_string()));

    let list = engine.env_list().await.unwrap();
    assert_eq!(list.get("MY_VAR"), Some(&"my-val".to_string()));

    engine.env_unset("MY_VAR").await.unwrap();
    let val_unset = engine.env_get("MY_VAR").await.unwrap();
    assert_eq!(val_unset, None);

    engine.secret_set("DB_PASS", "super-secret-password-xyz").await.unwrap();
    let sec_val = engine.secret_get("DB_PASS").await.unwrap();
    assert_eq!(sec_val, Some("super-secret-password-xyz".to_string()));

    let sec_list = engine.secret_list().await.unwrap();
    assert!(sec_list.contains(&"DB_PASS".to_string()));

    let exported = engine.secret_export().await.unwrap();
    assert_eq!(exported.get("DB_PASS"), Some(&"super-secret-password-xyz".to_string()));

    engine.secret_remove("DB_PASS").await.unwrap();
    let sec_removed = engine.secret_get("DB_PASS").await.unwrap();
    assert_eq!(sec_removed, None);

    let mut import_map = HashMap::new();
    import_map.insert("DB_PASS".to_string(), "imported-password-123".to_string());
    engine.secret_import(&import_map).await.unwrap();
    let imported_val = engine.secret_get("DB_PASS").await.unwrap();
    assert_eq!(imported_val, Some("imported-password-123".to_string()));

    let doc_report = engine.secret_doctor().await.unwrap();
    assert!(!doc_report.is_empty());
    assert!(doc_report.iter().any(|line| line.contains("Fallback store decryption: Successful")));

    let resolved = engine.env_resolve(None).await.unwrap();
    assert_eq!(resolved.vars.get("PORT"), Some(&"8080".to_string()));
    assert_eq!(resolved.vars.get("DB_PASS"), Some(&"imported-password-123".to_string()));

    std::env::remove_var("FORGE_MASTER_KEY");
    fs::remove_dir_all(&temp_dir).ok();
}

