use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use uuid::Uuid;
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub path: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub workspace_id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub tool_calls_json: Option<String>,
    pub token_count: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub id: String,
    pub title: String,
    pub category: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub message_id: String,
    pub session_id: String,
    pub session_title: String,
    pub role: String,
    pub snippet: String,
    pub created_at: String,
}

pub struct DbManager {
    conn: Mutex<Connection>,
}

impl DbManager {
    pub fn new() -> Result<Self, String> {
        let app_dir = dirs_fallback().map_err(|e| e.to_string())?;
        fs::create_dir_all(&app_dir).map_err(|e| format!("Failed to create AppData directory: {}", e))?;

        let db_path = app_dir.join("gemini_desktop.db");
        let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open SQLite database: {}", e))?;

        let manager = Self {
            conn: Mutex::new(conn),
        };

        manager.init_tables().map_err(|e| format!("DB Migration Error: {}", e))?;
        manager.seed_defaults().map_err(|e| format!("DB Seed Error: {}", e))?;

        Ok(manager)
    }

    fn init_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS workspaces (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                path TEXT NOT NULL,
                model TEXT NOT NULL DEFAULT 'gemini-2.5-pro',
                system_prompt TEXT,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                title TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                tool_calls_json TEXT,
                token_count INTEGER DEFAULT 0,
                created_at TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS prompts (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                category TEXT NOT NULL,
                content TEXT NOT NULL
            );

            -- Full Text Search Table
            CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
                message_id UNINDEXED,
                session_id UNINDEXED,
                content
            );

            -- Triggers to maintain FTS index
            CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages BEGIN
                INSERT INTO messages_fts(message_id, session_id, content)
                VALUES (new.id, new.session_id, new.content);
            END;

            CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages BEGIN
                DELETE FROM messages_fts WHERE message_id = old.id;
            END;
            "
        )?;

        Ok(())
    }

    fn seed_defaults(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM workspaces", [], |r| r.get(0))?;
        if count == 0 {
            let now = Utc::now().to_rfc3339();
            let default_workspaces = vec![
                ("ws-personal", "Personal", "C:\\", "gemini-2.5-flash"),
            ];

            for (id, name, path, model) in default_workspaces {
                conn.execute(
                    "INSERT INTO workspaces (id, name, path, model, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![id, name, path, model, now],
                )?;
            }
        } else {
            // Remove legacy default workspaces if present
            let _ = conn.execute(
                "DELETE FROM workspaces WHERE id IN ('ws-kod2', 'ws-hr', 'ws-arch')",
                [],
            );
        }

        let prompt_count: i64 = conn.query_row("SELECT COUNT(*) FROM prompts", [], |r| r.get(0))?;
        if prompt_count == 0 {
            let default_prompts = vec![
                ("Analyze Architecture", "Architecture", "Analyze the architecture of the current workspace. Identify component boundaries, dependencies, potential bottlenecks, and single points of failure."),
                ("Generate Tests", "Testing", "Review the selected file or function and generate exhaustive unit tests covering edge cases, null inputs, and error scenarios."),
                ("Refactor Code", "Development", "Analyze this code for clarity, performance, and adherence to modern clean code standards. Propose an incremental refactor."),
                ("Generate Knowledge Base", "Documentation", "Summarize this module into a comprehensive markdown knowledge base entry including purpose, API contracts, and usage examples."),
                ("Review Design", "Design", "Critique this design or architectural pattern against SOLID and DRY principles. Recommend improvements."),
                ("Find Dead Code", "Maintenance", "Inspect this file/module and detect any unused methods, unreachable branches, dead variables, or obsolete dependencies."),
            ];

            for (title, cat, content) in default_prompts {
                let id = Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT INTO prompts (id, title, category, content) VALUES (?1, ?2, ?3, ?4)",
                    params![id, title, cat, content],
                )?;
            }
        }

        Ok(())
    }

    pub fn list_workspaces(&self) -> Result<Vec<Workspace>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, name, path, model, system_prompt, created_at FROM workspaces ORDER BY name ASC")
            .map_err(|e| e.to_string())?;

        let rows = stmt.query_map([], |row| {
            Ok(Workspace {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                model: row.get(3)?,
                system_prompt: row.get(4)?,
                created_at: row.get(5)?,
            })
        }).map_err(|e| e.to_string())?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(|e| e.to_string())?);
        }
        Ok(list)
    }

    pub fn save_workspace(&self, ws: Workspace) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO workspaces (id, name, path, model, system_prompt, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                path = excluded.path,
                model = excluded.model,
                system_prompt = excluded.system_prompt",
            params![ws.id, ws.name, ws.path, ws.model, ws.system_prompt, ws.created_at],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_workspace(&self, id: &str) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM workspaces WHERE id = ?1", params![id]).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_sessions(&self, workspace_id: &str) -> Result<Vec<Session>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, workspace_id, title, created_at, updated_at FROM sessions WHERE workspace_id = ?1 ORDER BY updated_at DESC")
            .map_err(|e| e.to_string())?;

        let rows = stmt.query_map(params![workspace_id], |row| {
            Ok(Session {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                title: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        }).map_err(|e| e.to_string())?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(|e| e.to_string())?);
        }
        Ok(list)
    }

    pub fn create_session(&self, workspace_id: &str, title: &str) -> Result<Session, String> {
        let conn = self.conn.lock().unwrap();
        let id = format!("session-{}", Uuid::new_v4());
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO sessions (id, workspace_id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, workspace_id, title, now, now],
        ).map_err(|e| e.to_string())?;

        Ok(Session {
            id,
            workspace_id: workspace_id.to_string(),
            title: title.to_string(),
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn rename_session(&self, session_id: &str, title: &str) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE sessions SET title = ?1, updated_at = ?2 WHERE id = ?3",
            params![title, now, session_id],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_session(&self, session_id: &str) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM messages WHERE session_id = ?1", params![session_id]).map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM sessions WHERE id = ?1", params![session_id]).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_messages(&self, session_id: &str) -> Result<Vec<Message>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, session_id, role, content, tool_calls_json, token_count, created_at FROM messages WHERE session_id = ?1 ORDER BY created_at ASC")
            .map_err(|e| e.to_string())?;

        let rows = stmt.query_map(params![session_id], |row| {
            Ok(Message {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                tool_calls_json: row.get(4)?,
                token_count: row.get(5)?,
                created_at: row.get(6)?,
            })
        }).map_err(|e| e.to_string())?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(|e| e.to_string())?);
        }
        Ok(list)
    }

    pub fn save_message(&self, msg: Message) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO messages (id, session_id, role, content, tool_calls_json, token_count, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![msg.id, msg.session_id, msg.role, msg.content, msg.tool_calls_json, msg.token_count, msg.created_at],
        ).map_err(|e| e.to_string())?;

        // Update session's updated_at timestamp
        let now = Utc::now().to_rfc3339();
        let _ = conn.execute("UPDATE sessions SET updated_at = ?1 WHERE id = ?2", params![now, msg.session_id]);

        Ok(())
    }

    pub fn search_fts(&self, query: &str) -> Result<Vec<SearchResult>, String> {
        let conn = self.conn.lock().unwrap();
        let sanitized = query.replace('"', "\"\"").trim().to_string();
        if sanitized.is_empty() {
            return Ok(Vec::new());
        }

        let fts_query = format!("\"{}\"*", sanitized);
        let mut stmt = conn.prepare(
            "SELECT m.id, m.session_id, s.title, m.role, snippet(messages_fts, 2, '<b>', '</b>', '...', 15), m.created_at
             FROM messages_fts f
             JOIN messages m ON f.message_id = m.id
             JOIN sessions s ON m.session_id = s.id
             WHERE messages_fts MATCH ?1
             ORDER BY rank
             LIMIT 50"
        ).map_err(|e| e.to_string())?;

        let rows = stmt.query_map(params![fts_query], |row| {
            Ok(SearchResult {
                message_id: row.get(0)?,
                session_id: row.get(1)?,
                session_title: row.get(2)?,
                role: row.get(3)?,
                snippet: row.get(4)?,
                created_at: row.get(5)?,
            })
        }).map_err(|e| e.to_string())?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(|e| e.to_string())?);
        }
        Ok(list)
    }

    pub fn list_prompts(&self) -> Result<Vec<PromptTemplate>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, title, category, content FROM prompts ORDER BY category, title")
            .map_err(|e| e.to_string())?;

        let rows = stmt.query_map([], |row| {
            Ok(PromptTemplate {
                id: row.get(0)?,
                title: row.get(1)?,
                category: row.get(2)?,
                content: row.get(3)?,
            })
        }).map_err(|e| e.to_string())?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(|e| e.to_string())?);
        }
        Ok(list)
    }

    #[cfg(test)]
    pub fn new_in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
        let manager = Self {
            conn: Mutex::new(conn),
        };
        manager.init_tables().map_err(|e| format!("DB Migration Error: {}", e))?;
        manager.seed_defaults().map_err(|e| format!("DB Seed Error: {}", e))?;
        Ok(manager)
    }
}

fn dirs_fallback() -> std::io::Result<PathBuf> {
    if let Ok(app_data) = std::env::var("LOCALAPPDATA") {
        Ok(PathBuf::from(app_data).join("GeminiDesktop"))
    } else if let Ok(home) = std::env::var("USERPROFILE") {
        Ok(PathBuf::from(home).join(".geminidesktop"))
    } else {
        Ok(PathBuf::from(".").join("data"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_init_and_seed() {
        let db = DbManager::new_in_memory().expect("in-memory db initialization failed");
        let workspaces = db.list_workspaces().expect("failed to list workspaces");
        assert_eq!(workspaces.len(), 1);
        assert_eq!(workspaces[0].name, "Personal");

        let prompts = db.list_prompts().expect("failed to list prompts");
        assert!(prompts.len() >= 6);
    }

    #[test]
    fn test_session_message_and_fts5_search() {
        let db = DbManager::new_in_memory().expect("in-memory db initialization failed");
        let session = db.create_session("ws-personal", "Architecture Review").expect("failed to create session");

        let msg = Message {
            id: "msg-test-1".to_string(),
            session_id: session.id.clone(),
            role: "assistant".to_string(),
            content: "We discovered forty-two validator functions in the codebase.".to_string(),
            tool_calls_json: None,
            token_count: 12,
            created_at: Utc::now().to_rfc3339(),
        };
        db.save_message(msg).expect("failed to save message");

        let results = db.search_fts("validator").expect("FTS search failed");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id, session.id);
        assert!(results[0].snippet.contains("validator"));
    }

    #[test]
    fn test_workspace_crud_and_cascade_delete() {
        let db = DbManager::new_in_memory().expect("in-memory db initialization failed");

        // 1. Create custom workspace
        let now = Utc::now().to_rfc3339();
        let ws = Workspace {
            id: "ws-custom".to_string(),
            name: "Alpha Project".to_string(),
            path: "C:\\Alpha".to_string(),
            model: "gemini-2.5-pro".to_string(),
            system_prompt: Some("Custom system instruction".to_string()),
            created_at: now.clone(),
        };
        db.save_workspace(ws).expect("failed to save workspace");

        let workspaces = db.list_workspaces().unwrap();
        assert_eq!(workspaces.len(), 2); // Personal + Alpha Project
        let found = workspaces.into_iter().find(|w| w.id == "ws-custom").unwrap();
        assert_eq!(found.name, "Alpha Project");
        assert_eq!(found.model, "gemini-2.5-pro");

        // 2. Update workspace (ON CONFLICT)
        let updated_ws = Workspace {
            id: "ws-custom".to_string(),
            name: "Alpha Project Updated".to_string(),
            path: "C:\\AlphaV2".to_string(),
            model: "gemini-3.5-flash".to_string(),
            system_prompt: None,
            created_at: now,
        };
        db.save_workspace(updated_ws).expect("failed to update workspace");
        let workspaces_v2 = db.list_workspaces().unwrap();
        let updated = workspaces_v2.into_iter().find(|w| w.id == "ws-custom").unwrap();
        assert_eq!(updated.name, "Alpha Project Updated");
        assert_eq!(updated.path, "C:\\AlphaV2");
        assert_eq!(updated.model, "gemini-3.5-flash");

        // 3. Create child session and message
        let session = db.create_session("ws-custom", "Sprint 1 Discussion").expect("failed to create session");
        let msg = Message {
            id: "msg-alpha-1".to_string(),
            session_id: session.id.clone(),
            role: "user".to_string(),
            content: "What is the sprint velocity?".to_string(),
            tool_calls_json: None,
            token_count: 6,
            created_at: Utc::now().to_rfc3339(),
        };
        db.save_message(msg).expect("failed to save message");

        assert_eq!(db.list_sessions("ws-custom").unwrap().len(), 1);
        assert_eq!(db.list_messages(&session.id).unwrap().len(), 1);

        // 4. Delete workspace and verify relational cascade delete
        db.delete_workspace("ws-custom").expect("failed to delete workspace");
        let remaining_ws = db.list_workspaces().unwrap();
        assert_eq!(remaining_ws.len(), 1); // Only Personal remains
        assert!(!remaining_ws.iter().any(|w| w.id == "ws-custom"));

        // Sessions and messages must be cascaded
        assert_eq!(db.list_sessions("ws-custom").unwrap().len(), 0);
        assert_eq!(db.list_messages(&session.id).unwrap().len(), 0);
    }

    #[test]
    fn test_fts5_cleanup_on_delete() {
        let db = DbManager::new_in_memory().expect("in-memory db initialization failed");
        let session = db.create_session("ws-personal", "FTS Cleanup Test").expect("failed to create session");

        let msg = Message {
            id: "msg-fts-unique".to_string(),
            session_id: session.id.clone(),
            role: "assistant".to_string(),
            content: "The quantum xylophone resonance frequency is 432 Hz.".to_string(),
            tool_calls_json: None,
            token_count: 10,
            created_at: Utc::now().to_rfc3339(),
        };
        db.save_message(msg).expect("failed to save message");

        // Verify FTS finds it
        let before_del = db.search_fts("xylophone").expect("FTS search failed");
        assert_eq!(before_del.len(), 1);

        // Delete session (which deletes messages)
        db.delete_session(&session.id).expect("failed to delete session");

        // Verify FTS no longer returns deleted message
        let after_del = db.search_fts("xylophone").expect("FTS search failed");
        assert_eq!(after_del.len(), 0);
    }

    #[test]
    fn test_session_rename() {
        let db = DbManager::new_in_memory().expect("in-memory db initialization failed");
        let session = db.create_session("ws-personal", "Draft Title").expect("failed to create session");
        assert_eq!(session.title, "Draft Title");

        db.rename_session(&session.id, "Finalized Architecture Plan").expect("failed to rename session");
        let sessions = db.list_sessions("ws-personal").unwrap();
        let target = sessions.into_iter().find(|s| s.id == session.id).expect("session not found");
        assert_eq!(target.title, "Finalized Architecture Plan");
    }
}


