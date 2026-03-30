//! Local Crush connector wrapper.
//!
//! The actual parsing/detection logic comes from `franken-agent-detection`, but
//! this wrapper keeps Crush participating in cass through the repo-local
//! `Connector` trait and module layout.

use anyhow::Result;

use crate::connectors::{Connector, DetectionResult, NormalizedConversation, ScanContext};

pub struct CrushConnector {
    inner: Box<dyn Connector + Send>,
}

impl Default for CrushConnector {
    fn default() -> Self {
        Self::new()
    }
}

impl CrushConnector {
    pub fn new() -> Self {
        Self {
            inner: crate::connectors::fad_adapter::crush(),
        }
    }
}

impl Connector for CrushConnector {
    fn detect(&self) -> DetectionResult {
        self.inner.detect()
    }

    fn scan(&self, ctx: &ScanContext) -> Result<Vec<NormalizedConversation>> {
        self.inner.scan(ctx)
    }

    fn count_disk_files(&self) -> Option<usize> {
        self.inner.count_disk_files()
    }

    fn reconciliation_notes(&self) -> Option<String> {
        self.inner.reconciliation_notes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{Connection, params};
    use std::path::Path;
    use tempfile::TempDir;

    fn create_crush_fixture(path: &Path) {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(
            "
            CREATE TABLE sessions (
                id TEXT PRIMARY KEY,
                title TEXT,
                prompt_tokens INTEGER,
                completion_tokens INTEGER,
                cost REAL
            );
            CREATE TABLE messages (
                session_id TEXT NOT NULL,
                role TEXT,
                parts TEXT,
                created_at INTEGER,
                model TEXT,
                provider TEXT
            );
            ",
        )
        .unwrap();

        conn.execute(
            "INSERT INTO sessions (id, title, prompt_tokens, completion_tokens, cost)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params!["sess-1", "Crush Session", 10_i64, 20_i64, 0.42_f64],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO messages (session_id, role, parts, created_at, model, provider)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                "sess-1",
                "user",
                r#"[{"type":"text","text":"Hello from user"}]"#,
                1_000_i64,
                Option::<String>::None,
                Option::<String>::None
            ],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO messages (session_id, role, parts, created_at, model, provider)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                "sess-1",
                "assistant",
                r#"[{"type":"text","text":"Hello from crush"}]"#,
                2_000_i64,
                Some("gpt-4.1".to_string()),
                Some("openai".to_string())
            ],
        )
        .unwrap();
    }

    fn message_summary(conversation: &NormalizedConversation) -> Vec<(i64, String, String)> {
        conversation
            .messages
            .iter()
            .map(|message| (message.idx, message.role.clone(), message.content.clone()))
            .collect()
    }

    #[test]
    fn wrapper_scan_matches_fad_adapter_for_explicit_sqlite_db() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("crush.db");
        create_crush_fixture(&db_path);

        let ctx = ScanContext::local_default(db_path.clone(), None);
        let wrapper_conversations = CrushConnector::new().scan(&ctx).unwrap();
        let adapter_conversations = crate::connectors::fad_adapter::crush().scan(&ctx).unwrap();

        assert_eq!(wrapper_conversations.len(), 1);
        assert_eq!(adapter_conversations.len(), 1);

        let wrapper = &wrapper_conversations[0];
        let adapter = &adapter_conversations[0];

        assert_eq!(wrapper.agent_slug, "crush");
        assert_eq!(wrapper.external_id.as_deref(), Some("sess-1"));
        assert_eq!(wrapper.title.as_deref(), Some("Crush Session"));
        assert_eq!(wrapper.source_path, db_path);
        assert_eq!(wrapper.started_at, Some(1_000));
        assert_eq!(wrapper.ended_at, Some(2_000));
        assert_eq!(
            message_summary(wrapper),
            vec![
                (0, "user".to_string(), "Hello from user".to_string()),
                (1, "assistant".to_string(), "Hello from crush".to_string()),
            ]
        );

        assert_eq!(wrapper.external_id, adapter.external_id);
        assert_eq!(wrapper.title, adapter.title);
        assert_eq!(wrapper.source_path, adapter.source_path);
        assert_eq!(wrapper.started_at, adapter.started_at);
        assert_eq!(wrapper.ended_at, adapter.ended_at);
        assert_eq!(message_summary(wrapper), message_summary(adapter));
    }
}
