import { marked } from "marked";
import hljs from "highlight.js";

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
          class="copy-code-btn px-2.5 py-0.5 rounded text-[11px] bg-surface hover:bg-surface-hover text-secondary-theme hover:text-primary-theme border border-theme-default transition-colors cursor-pointer"
          data-code="${encodedRaw}"
          onclick="
            const code = decodeURIComponent(this.getAttribute('data-code'));
            navigator.clipboard.writeText(code);
            const orig = this.innerText;
            this.innerText = 'Copied!';
            setTimeout(() => { this.innerText = orig; }, 1500);
          "
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
    return marked.parse(sanitized) as string;
  } catch (e) {
    return sanitized;
  }
}
