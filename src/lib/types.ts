export interface Workspace {
  id: string;
  name: string;
  path: string;
  model: string;
  system_prompt?: string;
  approval_mode?: ApprovalMode;
  track_file_changes?: boolean;
  reasoning_effort?: string;
  sandbox_enabled?: boolean;
  agent_name?: string;
  additional_dirs?: string;
  created_at: string;
}

export interface Session {
  id: string;
  workspace_id: string;
  title: string;
  acp_session_id?: string;
  acp_engine?: string;
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

export interface AskQuestionItem {
  id?: string;
  question: string;
  header?: string;
  multiSelect?: boolean;
  options?: { label: string; description?: string }[];
}

export interface ToolPermissionPayload {
  request_id: number | string;
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
  mcp_server?: string;
  questions?: AskQuestionItem[];
}

export interface CliModelInfo {
  id: string;
  name?: string;
  description?: string;
  input_token_limit?: number;
  output_token_limit?: number;
  is_default?: boolean;
}

export interface CliModelDiscoveryResult {
  supported: boolean;
  models: CliModelInfo[];
  message: string;
  engine?: string;
  cli_version?: string;
}

export interface ToolExecutionStep {
  id: string;
  title: string;
  tool_name: string;
  status: "in_progress" | "pending" | "completed" | "failed";
  summary?: string;
  error?: string;
  parameters?: any;
  raw_output?: string;
  created_at?: string;
  duration_ms?: number;
}

export interface ToolCallPayload {
  session_id: string;
  tool_call_id: string;
  title: string;
  tool_name: string;
  status: string;
  summary?: string;
  error?: string;
  parameters?: any;
  raw_output?: string;
}

export interface ContextUsageStats {
  usedTokens: number;
  contextWindow: number;
  percentage: number;
}

export interface DynamicAcpCommand {
  name: string;
  description?: string;
  inputHint?: string;
}

export interface QueuedPromptItem {
  id: string;
  prompt: string;
  displayText?: string;
  attachments?: AttachmentItem[];
  createdAt: number;
}

export interface SessionStreamState {
  streamingText: string;
  thoughtText?: string;
  thinkingStartedAt?: number;
  thinkingDurationMs?: number;
  isStreaming: boolean;
  toolPermission: ToolPermissionPayload | null;
  toolPermissionQueue?: ToolPermissionPayload[];
  toolCalls?: ToolExecutionStep[];
  steeringHints?: string[];
  isSteering?: boolean;
  started_at?: number;
  contextUsage?: ContextUsageStats;
  queuedPrompts?: QueuedPromptItem[];
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
  previewUrl?: string;
}

export interface MentionOption {
  id: string;
  label: string;
  insertText: string;
  title: string;
  subtitle: string;
  kind: "file" | "directory" | "git";
}

export interface SlashCommandOption {
  id: string;
  label: string;
  insertText: string;
  title: string;
  description: string;
  category: "action" | "tool" | "mode" | "config";
  engine?: CliEngine | "all";
}

export interface McpServerConfig {
  command?: string;
  args?: string[];
  env?: Record<string, string>;
  cwd?: string;
  url?: string;
  serverUrl?: string;
  headers?: Record<string, string>;
  eager?: boolean;
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

export type ApprovalMode = "default" | "auto_edit" | "auto" | "yolo" | "plan";

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

export interface FileSnippetResult {
  file_path: string;
  file_name: string;
  target_line: number;
  start_line: number;
  end_line: number;
  snippet: string;
  total_lines: number;
  exists: boolean;
  language: string;
}

export interface ParsedDebugError {
  exceptionType: string;
  message: string;
  filePath?: string;
  fileName?: string;
  lineNumber?: number;
  columnNumber?: number;
  rawTrace: string;
  sourceFramework?: "dotnet" | "cpp" | "node" | "python" | "rust" | "generic";
}

export interface RewindResult {
  rewound_messages_count: number;
  restored_files: string[];
  previous_prompt?: string | null;
}

export interface PlanTaskItem {
  id: string;
  text: string;
  completed: boolean;
  category?: string;
}

export interface SessionFileChange {
  file_path: string;
  status: "created" | "modified" | "deleted";
  additions: number;
  deletions: number;
  diff_text: string;
  original_content: string;
  current_content: string;
}

export interface ChatCompressionConfig {
  enabled: boolean;
  contextPercentageThreshold: number;
}

export interface FileFilteringConfig {
  respectGitIgnore: boolean;
  respectGeminiIgnore: boolean;
}

export interface SessionRetentionConfig {
  enabled: boolean;
  maxAge: string;
  maxCount?: number;
}

export interface GitFileItem {
  path: string;
  status: "modified" | "added" | "deleted" | "untracked" | "renamed";
  is_staged: boolean;
}

export interface GitStatusResult {
  is_git_repo: boolean;
  repo_name: string;
  active_branch: string;
  ahead?: number;
  behind?: number;
  staged: GitFileItem[];
  unstaged: GitFileItem[];
}

export interface GlobalCliSettings {
  filePath: string;
  chatCompression: ChatCompressionConfig;
  maxSessionTurns: number;
  fileFiltering: FileFilteringConfig;
  sessionRetention: SessionRetentionConfig;
  rawJson: string;
  agyAgentMode?: string;
  showThinking?: boolean;
  orchestrator?: Record<string, any> | null;
  agyAutoCompactOverride?: string;
  agyDisableEmbeddedRipgrep?: boolean;
}

export interface SaveGlobalCliSettingsPayload {
  chatCompression?: ChatCompressionConfig;
  maxSessionTurns?: number;
  fileFiltering?: FileFilteringConfig;
  sessionRetention?: SessionRetentionConfig;
  rawJson?: string;
  agyAgentMode?: string;
  showThinking?: boolean;
  orchestrator?: Record<string, any> | null;
  agyAutoCompactOverride?: string;
  agyDisableEmbeddedRipgrep?: boolean;
}

export interface AgyPluginValidationResult {
  success: boolean;
  target: string;
  strict: boolean;
  issues: string[];
  rawOutput: string;
}

export interface AgentCapabilitiesState {
  supportsResume: boolean;
  supportsImage: boolean;
  supportsEmbeddedContext: boolean;
  supportsAdditionalDirs: boolean;
  supportsSystemPromptAppend: boolean;
}

export interface ContinuationResult {
  newSession: Session;
  continuationMessage: Message;
}

export type CliEngine = string;

export interface EngineCapabilities {
  transport: string;
  supportsPlugins: boolean;
  supportsLiveModelDiscovery: boolean;
  supportsMcpConfig: boolean;
  supportsMcpImportFromGemini: boolean;
  supportsSkillSync: boolean;
  supportsReasoningEffort: boolean;
  supportsAgentPersona: boolean;
  supportsOrchestratorSettings: boolean;
  supportsNativeCompress: boolean;
  supportsCliInstall: boolean;
  supportsLoginLaunch: boolean;
  supportsSystemPromptAppend: boolean;
  supportsConcurrentSteer: boolean;
}

export interface ModelOptionDto {
  value: string;
  label: string;
}

export interface ModelOptionGroupDto {
  group: string;
  options: ModelOptionDto[];
}

export interface EngineSlashCommandMeta {
  id: string;
  label: string;
  title: string;
  description: string;
  category: string;
}

export interface EngineDescriptor {
  id: string;
  displayName: string;
  shortName: string;
  badgeIcon: string;
  description: string;
  defaultModel: string;
  fallbackTestModel: string;
  globalSettingsHint: string;
  workspaceInstructionsHint: string;
  installHint: string;
  installed: boolean;
  binaryPath?: string | null;
  binaryDir?: string | null;
  inUserPath: boolean;
  cliVersion?: string | null;
  capabilities: EngineCapabilities;
  modelGroups: ModelOptionGroupDto[];
  extraSlashCommands: EngineSlashCommandMeta[];
}

export interface CliEnginesStatus {
  activeEngine: CliEngine;
  geminiInstalled: boolean;
  geminiPath?: string;
  agyInstalled: boolean;
  agyPath?: string;
}

export interface CliPathStatus {
  geminiPath?: string;
  geminiDir?: string;
  geminiInUserPath: boolean;
  agyPath?: string;
  agyDir?: string;
  agyInUserPath: boolean;
}

export interface PathAddResult {
  success: boolean;
  message: string;
  targetDir: string;
  alreadyInPath: boolean;
}

export interface AgyPluginInfo {
  name: string;
  enabled: boolean;
  version?: string;
  displayName?: string;
  description?: string;
  source?: string;
  path?: string;
  skills?: string[];
  rules?: string[];
  mcpServers?: string[];
  logo?: string;
}

export interface AgyPluginOpResult {
  success: boolean;
  message: string;
}

export interface CreateCustomPluginRequest {
  name: string;
  displayName?: string;
  description?: string;
  includeSkill: boolean;
  skillName?: string;
  includeMcp: boolean;
  includeRules: boolean;
  includeAgent: boolean;
  agentName?: string;
}

export interface AkboPoolConfig {
  centralPoolDir: string;
  stageOnly: boolean;
}






