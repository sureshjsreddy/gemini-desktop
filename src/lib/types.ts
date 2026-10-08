export interface Workspace {
  id: string;
  name: string;
  path: string;
  model: string;
  system_prompt?: string;
  approval_mode?: ApprovalMode;
  created_at: string;
}

export interface Session {
  id: string;
  workspace_id: string;
  title: string;
  created_at: string;
  updated_at: string;
}

export interface Message {
  id: string;
  session_id: string;
  role: "user" | "assistant" | "system" | "tool";
  content: string;
  tool_calls_json?: string;
  token_count: number;
  created_at: string;
}

export interface PromptTemplate {
  id: string;
  title: string;
  category: string;
  content: string;
}

export interface SearchResult {
  message_id: string;
  session_id: string;
  session_title: string;
  role: string;
  snippet: string;
  created_at: string;
}

export interface PermissionOption {
  option_id: string;
  name: string;
  kind: string;
}

export interface ToolPermissionPayload {
  request_id: number;
  session_id: string;
  tool_call_id?: string;
  tool_name: string;
  title?: string;
  kind?: string;
  parameters: any;
  locations?: any;
  content?: any;
  reason?: string;
  options: PermissionOption[];
}

export interface SessionStreamState {
  streamingText: string;
  isStreaming: boolean;
  toolPermission: ToolPermissionPayload | null;
}

export interface GeminiEnvStatus {
  installed: boolean;
  path?: string;
  version?: string;
  details: string;
}

export interface WorkspaceFileEntry {
  name: string;
  relative_path: string;
  is_dir: boolean;
  extension?: string;
}

export interface TreeNode {
  name: string;
  path: string;
  isDir: boolean;
  extension?: string;
  children: TreeNode[];
  isLoaded?: boolean;
  isLoading?: boolean;
}

export interface AttachmentItem {
  id: string;
  name: string;
  path: string;
  kind: "file" | "directory" | "git" | "external";
}

export interface MentionOption {
  id: string;
  label: string;
  insertText: string;
  title: string;
  subtitle: string;
  kind: "file" | "directory" | "git";
}

export interface McpServerConfig {
  command: string;
  args?: string[];
  env?: Record<string, string>;
  cwd?: string;
  disabled?: boolean;
}

export interface McpConfigResponse {
  file_path: string;
  mcp_servers: Record<string, McpServerConfig>;
}

export interface TerminalCommandResult {
  stdout: string;
  stderr: string;
  exit_code: number;
  duration_ms: number;
}

export interface UpdateInfo {
  update_available: boolean;
  current_version: string;
  latest_version: string;
  package_id: string;
  release_url?: string;
}

export type ApprovalMode = "default" | "auto_edit" | "yolo" | "plan";

export type GeminiAuthMode = "api_key" | "vertex_ai";

export interface GeminiAuthConfig {
  auth_mode: GeminiAuthMode;
  api_key?: string;
  google_cloud_project?: string;
  google_cloud_location?: string;
  google_app_credentials?: string;
}

export interface CliInstallResult {
  success: boolean;
  message: string;
  installed_path?: string;
  version?: string;
}

export interface CliTestResult {
  success: boolean;
  latency_ms: number;
  message: string;
  details?: string;
}

export interface GCloudStatus {
  installed: boolean;
  active_project?: string;
  active_account?: string;
  has_adc: boolean;
  has_gemini_oauth?: boolean;
  path?: string;
}

export interface WorkspacePathValidation {
  is_valid: boolean;
  exists: boolean;
  error_message?: string;
}

