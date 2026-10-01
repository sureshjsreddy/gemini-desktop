use crate::commands::AppState;
use crate::database::{Message, PromptTemplate, SearchResult, Session, Workspace};
use tauri::State;

#[tauri::command]
pub fn get_workspaces(state: State<AppState>) -> Result<Vec<Workspace>, String> {
    state.db.list_workspaces()
}

#[tauri::command]
pub fn save_workspace(state: State<AppState>, workspace: Workspace) -> Result<(), String> {
    state.db.save_workspace(workspace)
}

#[tauri::command]
pub fn delete_workspace(state: State<AppState>, id: String) -> Result<(), String> {
    state.db.delete_workspace(&id)
}

#[tauri::command]
pub fn get_sessions(state: State<AppState>, workspace_id: String) -> Result<Vec<Session>, String> {
    state.db.list_sessions(&workspace_id)
}

#[tauri::command]
pub fn create_session(state: State<AppState>, workspace_id: String, title: String) -> Result<Session, String> {
    state.db.create_session(&workspace_id, &title)
}

#[tauri::command]
pub fn rename_session(state: State<AppState>, session_id: String, title: String) -> Result<(), String> {
    state.db.rename_session(&session_id, &title)
}

#[tauri::command]
pub fn delete_session(state: State<AppState>, session_id: String) -> Result<(), String> {
    state.acp_session.remove_session(&session_id);
    state.db.delete_session(&session_id)
}

#[tauri::command]
pub fn get_session_messages(state: State<AppState>, session_id: String) -> Result<Vec<Message>, String> {
    state.db.list_messages(&session_id)
}

#[tauri::command]
pub fn save_message(state: State<AppState>, msg: Message) -> Result<(), String> {
    state.db.save_message(msg)
}

#[tauri::command]
pub fn get_prompts(state: State<AppState>) -> Result<Vec<PromptTemplate>, String> {
    state.db.list_prompts()
}

#[tauri::command]
pub fn search_history(state: State<AppState>, query: String) -> Result<Vec<SearchResult>, String> {
    state.db.search_fts(&query)
}

pub fn format_session_export(session_id: &str, messages: &[Message], format: &str) -> Result<String, String> {
    match format {
        "json" => serde_json::to_string_pretty(messages).map_err(|e| e.to_string()),
        "txt" => {
            let mut txt = String::new();
            for m in messages {
                txt.push_str(&format!("{}: {}\n\n", m.role.to_uppercase(), m.content));
            }
            Ok(txt)
        }
        _ => {
            // Markdown export
            let mut md = String::new();
            md.push_str(&format!("# Chat Export - Session {}\n\n", session_id));
            for m in messages {
                if m.role == "user" {
                    md.push_str(&format!("### 👤 User\n\n{}\n\n---\n\n", m.content));
                } else {
                    md.push_str(&format!("### ✨ Gemini\n\n{}\n\n---\n\n", m.content));
                }
            }
            Ok(md)
        }
    }
}

#[tauri::command]
pub fn export_session(state: State<AppState>, session_id: String, format: String) -> Result<String, String> {
    let messages = state.db.list_messages(&session_id)?;
    format_session_export(&session_id, &messages, &format)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_session_export() {
        let messages = vec![
            Message {
                id: "msg-1".to_string(),
                session_id: "sess-test".to_string(),
                role: "user".to_string(),
                content: "Explain Rust borrowing".to_string(),
                tool_calls_json: None,
                token_count: 5,
                created_at: "2026-09-23T12:00:00Z".to_string(),
            },
            Message {
                id: "msg-2".to_string(),
                session_id: "sess-test".to_string(),
                role: "assistant".to_string(),
                content: "Borrowing allows references without transfer of ownership.".to_string(),
                tool_calls_json: None,
                token_count: 10,
                created_at: "2026-09-23T12:00:05Z".to_string(),
            },
        ];

        // 1. Markdown
        let md = format_session_export("sess-test", &messages, "md").unwrap();
        assert!(md.contains("# Chat Export - Session sess-test"));
        assert!(md.contains("### 👤 User"));
        assert!(md.contains("Explain Rust borrowing"));
        assert!(md.contains("### ✨ Gemini"));

        // 2. Text
        let txt = format_session_export("sess-test", &messages, "txt").unwrap();
        assert!(txt.contains("USER: Explain Rust borrowing"));
        assert!(txt.contains("ASSISTANT: Borrowing allows references"));

        // 3. JSON
        let json_str = format_session_export("sess-test", &messages, "json").unwrap();
        let parsed: Vec<Message> = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].content, "Explain Rust borrowing");
    }
}
