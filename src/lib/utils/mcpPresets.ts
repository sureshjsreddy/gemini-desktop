export interface McpPreset {
  id: string;
  label: string;
  description: string;
  icon: string;
  config: {
    key: string;
    command: string;
    args: string[];
    env: Record<string, string>;
    cwd?: string;
  };
}

/**
 * Builds the 1-click MCP server presets list for `McpModal.svelte`.
 */
export function buildMcpPresets(workspacePath?: string): McpPreset[] {
  return [
    {
      id: "playwright",
      label: "Playwright Browser",
      description: "Headless/headed browser automation, screenshots, and DOM inspection",
      icon: "🎭",
      config: {
        key: "playwright",
        command: "npx",
        args: ["-y", "@playwright/mcp@latest"],
        env: {},
      },
    },
    {
      id: "github-docker",
      label: "GitHub (Docker)",
      description: "Official GitHub MCP server running inside Docker",
      icon: "🐙",
      config: {
        key: "github",
        command: "docker",
        args: [
          "run",
          "-i",
          "--rm",
          "-e",
          "GITHUB_PERSONAL_ACCESS_TOKEN",
          "ghcr.io/github/github-mcp-server:latest",
        ],
        env: {
          GITHUB_PERSONAL_ACCESS_TOKEN: "${GITHUB_PERSONAL_ACCESS_TOKEN}",
        },
      },
    },
    {
      id: "filesystem",
      label: "Local Filesystem",
      description: "Secure local directory access for Gemini CLI",
      icon: "📁",
      config: {
        key: "filesystem",
        command: "npx",
        args: ["-y", "@modelcontextprotocol/server-filesystem", workspacePath || "C:\\projects"],
        env: {},
      },
    },
    {
      id: "memory",
      label: "Knowledge Graph Memory",
      description: "Persistent memory & entities graph across sessions",
      icon: "🧠",
      config: {
        key: "memory",
        command: "npx",
        args: ["-y", "@modelcontextprotocol/server-memory"],
        env: {},
      },
    },
    {
      id: "akbo",
      label: "AKBO AI Knowledge & Dreaming",
      description: "Codebase Knowledge Graph, runtime observations, and out-of-band memory consolidation",
      icon: "🌙",
      config: {
        key: "akbo",
        command: "npx",
        args: ["-y", "akbo-cli@3.2.0", "mcp"],
        env: {},
      },
    },
    {
      id: "postgres",
      label: "PostgreSQL Database",
      description: "Inspect schema, run read-only queries on Postgres",
      icon: "🐘",
      config: {
        key: "postgres",
        command: "npx",
        args: ["-y", "@modelcontextprotocol/server-postgres", "postgresql://localhost/mydb"],
        env: {},
      },
    },
    {
      id: "azure-devops",
      label: "Azure DevOps (MCP)",
      description: "Work items, PRs, pipelines, and git repos in Azure DevOps",
      icon: "☁️",
      config: {
        key: "azure-devops",
        command: "npx",
        args: ["-y", "@azure-devops/mcp", "--organization", "https://dev.azure.com/YOUR_ORG"],
        env: {
          AZURE_DEVOPS_EXT_PAT: "${AZURE_DEVOPS_EXT_PAT}",
        },
      },
    },
    {
      id: "brave-search",
      label: "Brave Web Search",
      description: "Real-time web search capabilities via Brave Search API",
      icon: "🔍",
      config: {
        key: "brave-search",
        command: "npx",
        args: ["-y", "@modelcontextprotocol/server-brave-search"],
        env: {
          BRAVE_API_KEY: "${BRAVE_API_KEY}",
        },
      },
    },
  ];
}
