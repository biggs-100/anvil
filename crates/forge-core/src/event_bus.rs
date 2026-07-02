use tokio::sync::broadcast;
use crate::types::Event;
use std::io::Write;

static JOURNAL_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<Event>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        Self::new_internal(capacity, None)
    }

    pub fn new_with_journal(capacity: usize, path: std::path::PathBuf) -> Self {
        Self::new_internal(capacity, Some(path))
    }

    fn new_internal(capacity: usize, custom_path: Option<std::path::PathBuf>) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        let mut rx = sender.subscribe();
        
        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                let journal_path = if let Some(ref path) = custom_path {
                    path.clone()
                } else if let Ok(override_path) = std::env::var("FORGE_JOURNAL_PATH") {
                    std::path::PathBuf::from(override_path)
                } else {
                    let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
                    let mut dir = current_dir.clone();
                    let mut workspace_root = current_dir.clone();
                    loop {
                        if dir.join("forge.toml").exists() {
                            workspace_root = dir;
                            break;
                        }
                        if let Some(parent) = dir.parent() {
                            dir = parent.to_path_buf();
                        } else {
                            break;
                        }
                    }
                    workspace_root.join(".forge").join("journal.jsonl")
                };

                if let Some(parent) = journal_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }

                if let Ok(serialized) = serde_json::to_string(&event) {
                    let _lock = JOURNAL_MUTEX.lock().unwrap();
                    if let Ok(mut file) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&journal_path)
                    {
                        let _ = writeln!(file, "{}", serialized);
                    }
                }
            }
        });

        Self { sender }
    }

    pub fn publish(&self, event: Event) -> Result<usize, String> {
        match self.sender.send(event) {
            Ok(subscribers) => Ok(subscribers),
            Err(_) => Ok(0), // No receivers is a successful no-op
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_ndjson_serialization() {
        let temp_dir = std::env::temp_dir().join("forge_test_event_bus");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();
        let temp_journal = temp_dir.join("journal_serialize.jsonl");

        let event_bus = EventBus::new_with_journal(10, temp_journal.clone());
        let event = Event {
            timestamp: "2026-07-01T09:24:30-05:00".to_string(),
            operation_id: "op-test-serialize".to_string(),
            runtime: "node".to_string(),
            phase: "TestPhase".to_string(),
            status: crate::types::EventStatus::Started,
            message: Some("hello".to_string()),
            ..Default::default()
        };

        event_bus.publish(event.clone()).unwrap();
        
        // Wait a bit for background task to process
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let content = std::fs::read_to_string(&temp_journal).unwrap();
        let parsed: Event = serde_json::from_str(&content.trim()).unwrap();
        assert_eq!(parsed.operation_id, "op-test-serialize");
        assert_eq!(parsed.runtime, "node");

        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[tokio::test]
    async fn test_concurrent_appends() {
        let temp_dir = std::env::temp_dir().join("forge_test_event_bus_concurrent");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();
        let temp_journal = temp_dir.join("journal_concurrent.jsonl");

        let event_bus = EventBus::new_with_journal(100, temp_journal.clone());
        let mut handles = Vec::new();
        
        for i in 0..20 {
            let eb = event_bus.clone();
            let handle = tokio::spawn(async move {
                let event = Event {
                    timestamp: "2026-07-01T09:24:30-05:00".to_string(),
                    operation_id: format!("op-test-{}", i),
                    runtime: "all".to_string(),
                    phase: "TestPhase".to_string(),
                    status: crate::types::EventStatus::Finished,
                    message: None,
                    ..Default::default()
                };
                eb.publish(event).unwrap();
            });
            handles.push(handle);
        }

        for h in handles {
            h.await.unwrap();
        }

        // Wait for background writes to finish
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

        let content = std::fs::read_to_string(&temp_journal).unwrap();
        let lines: Vec<&str> = content.trim().split('\n').filter(|s| !s.is_empty()).collect();
        assert_eq!(lines.len(), 20);

        for line in lines {
            let parsed: Event = serde_json::from_str(line).unwrap();
            assert!(parsed.operation_id.starts_with("op-test-"));
        }

        std::fs::remove_dir_all(&temp_dir).ok();
    }
}
