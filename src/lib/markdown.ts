import { marked } from "marked";
import hljs from "highlight.js";
import DOMPurify from "dompurify";

// Custom renderer to format code blocks with language pills and copy buttons
const renderer = new marked.Renderer();

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
  const sanitized = content.replace(/\[MODE_UPDATE\]\s*[a-zA-Z0-9_]*/g, "").trim();
  try {
    const rawHtml = marked.parse(sanitized) as string;
    if (typeof window !== "undefined" && DOMPurify && typeof DOMPurify.sanitize === "function") {
      return DOMPurify.sanitize(rawHtml, {
        ADD_TAGS: ["button"],
        ADD_ATTR: ["data-code", "target"],
      });
    }
    return rawHtml;
  } catch (e) {
    return sanitized;
  }
}
