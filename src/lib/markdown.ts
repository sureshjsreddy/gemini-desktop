import { marked } from "marked";
import hljs from "highlight.js";
import DOMPurify from "dompurify";

// Custom renderer to format code blocks with language pills and copy buttons, plus rich file/web links
const renderer = new marked.Renderer();

const FILE_EXT_PATTERN = /\.(md|markdown|txt|rs|ts|tsx|js|jsx|svelte|json|yaml|yml|toml|py|go|html|css|scss|less|sh|bat|cmd|log|diff|patch|c|cpp|h|hpp|java|kt|cs|rb|php|sql|vue|xml|env|csv|ini|conf|proto)(?::\d+(?::\d+)?|#L\d+(?:-L?\d+)?)?$/i;

renderer.link = function ({ href, title, text }) {
  const cleanHref = (href || "").trim();
  const isWeb = cleanHref.startsWith("http://") || cleanHref.startsWith("https://") || cleanHref.startsWith("mailto:");
  const isFile = !isWeb && (
    cleanHref.startsWith("file://") ||
    FILE_EXT_PATTERN.test(cleanHref) ||
    cleanHref.includes("/brain/") ||
    cleanHref.includes("\\brain\\")
  );

  const displayTitle = title ? `title="${title}"` : (isFile ? `title="Open ${cleanHref} in default app"` : `title="${cleanHref}"`);

  if (isWeb) {
    return `<a href="${cleanHref}" data-path="${cleanHref}" target="_blank" rel="noopener noreferrer" class="chat-external-link inline-flex items-center gap-1 text-accent-theme hover:underline font-medium cursor-pointer" ${displayTitle}>${text}<svg class="inline-block w-3 h-3 ml-0.5 opacity-70" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 3h6v6"/><path d="M10 14L21 3"/><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/></svg></a>`;
  }

  if (isFile) {
    return `<a href="${cleanHref}" data-path="${cleanHref}" class="chat-file-link inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md bg-surface-elevated/80 hover:bg-surface-hover border border-subtle hover:border-accent-theme/40 text-primary-theme font-mono text-[11px] transition-all cursor-pointer shadow-2xs group my-0.5" ${displayTitle}><span class="text-accent-theme text-xs">📄</span><span class="underline decoration-subtle group-hover:decoration-accent-theme">${text}</span><span class="text-[10px] text-muted-theme group-hover:text-accent-theme font-sans">↗</span></a>`;
  }

  return `<a href="${cleanHref}" data-path="${cleanHref}" class="text-accent-theme hover:underline cursor-pointer" ${displayTitle}>${text}</a>`;
};

renderer.code = function ({ text, lang }) {
  const validLang = lang && hljs.getLanguage(lang) ? lang : "plaintext";
  let highlighted = "";
  try {
    highlighted = hljs.highlight(text, { language: validLang }).value;
  } catch {
    highlighted = text;
  }

  const encodedRaw = encodeURIComponent(text);

  return `
    <div class="code-block my-3 rounded-xl overflow-hidden border border-subtle bg-code shadow-xs font-mono text-xs">
      <div class="flex items-center justify-between px-3.5 py-1.5 bg-code-header border-b border-subtle text-secondary-theme select-none">
        <span class="text-[11px] uppercase font-mono font-bold tracking-wider text-accent-theme">${validLang}</span>
        <button 
          type="button"
          class="copy-code-btn px-2.5 py-0.5 rounded text-[11px] bg-surface hover:bg-surface-hover text-secondary-theme hover:text-primary-theme border border-theme-default transition-colors cursor-pointer"
          data-code="${encodedRaw}"
          aria-label="Copy code block"
        >
          Copy
        </button>
      </div>
      <pre class="p-3.5 overflow-x-auto text-code font-mono text-xs leading-relaxed"><code class="hljs">${highlighted}</code></pre>
    </div>
  `;
};

marked.setOptions({
  renderer,
  gfm: true,
  breaks: true,
});

export function renderMarkdown(content: string): string {
  if (!content) return "";
  // Strip any internal CLI control tags like [MODE_UPDATE] autoEdit
  let sanitized = content.replace(/\[MODE_UPDATE\]\s*[a-zA-Z0-9_]*/g, "").trim();
  // Autolink standalone file references like "📄 filename.ext:line" if not already wrapped in markdown link
  sanitized = sanitized.replace(/(^|[\s\n])📄\s+([a-zA-Z0-9_\-\.\/\\]+\.[a-zA-Z0-9_]+(?::\d+(?::\d+)?)?)(?!\))/g, "$1[$2]($2)");

  try {
    const rawHtml = marked.parse(sanitized) as string;
    if (typeof window !== "undefined" && DOMPurify && typeof DOMPurify.sanitize === "function") {
      return DOMPurify.sanitize(rawHtml, {
        ADD_TAGS: ["button", "svg", "path", "span"],
        ADD_ATTR: [
          "data-code",
          "data-path",
          "target",
          "rel",
          "title",
          "viewBox",
          "fill",
          "stroke",
          "stroke-width",
          "stroke-linecap",
          "stroke-linejoin",
        ],
        ALLOWED_URI_REGEXP: /^(?:(?:(?:f|ht)tps?|mailto|tel|callto|sms|cid|xmpp|file):|[^a-z]|[a-z+.\-]+(?:[^a-z+.\-:]|$)|[a-zA-Z0-9_\-\.\/\\:]+)/i,
      });
    }
    return rawHtml;
  } catch (e) {
    return sanitized;
  }
}
