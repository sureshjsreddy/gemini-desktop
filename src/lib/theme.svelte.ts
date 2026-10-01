export type ThemeId =
  | "soft-dark"
  | "warm-dusk"
  | "platinum-dark"
  | "platinum-white"
  | "gemini-aurora"
  | "cyber-emerald";

export type FontSize = "normal" | "comfortable";

export interface ThemeInfo {
  id: ThemeId;
  name: string;
  tagline: string;
  accent: string;
  bgPreview: string;
  surfacePreview: string;
  borderPreview: string;
  badge?: string;
}

export const THEMES: ThemeInfo[] = [
  {
    id: "soft-dark",
    name: "Soft Dark (Eye Comfort)",
    tagline: "Low-glare charcoal slate & soft sky — gentle on eyes",
    accent: "#38bdf8",
    bgPreview: "#1e222a",
    surfacePreview: "#242933",
    borderPreview: "#394150",
    badge: "Recommended",
  },
  {
    id: "warm-dusk",
    name: "Warm Dusk (Sepia Dark)",
    tagline: "Warm roasted charcoal & amber paper tone — zero blue light",
    accent: "#f59e0b",
    bgPreview: "#1f1d1a",
    surfacePreview: "#272420",
    borderPreview: "#433e36",
    badge: "Night Reading",
  },
  {
    id: "platinum-dark",
    name: "Platinum Dark",
    tagline: "Deep midnight slate & royal blue contrast",
    accent: "#3b82f6",
    bgPreview: "#0b0f19",
    surfacePreview: "#172033",
    borderPreview: "#2d3b55",
  },
  {
    id: "platinum-white",
    name: "Platinum Light",
    tagline: "Crisp white & clean slate grey",
    accent: "#2563eb",
    bgPreview: "#f8fafc",
    surfacePreview: "#ffffff",
    borderPreview: "#cbd5e1",
  },
  {
    id: "gemini-aurora",
    name: "Gemini Aurora",
    tagline: "Cosmic indigo & violet glow",
    accent: "#8b5cf6",
    bgPreview: "#090b14",
    surfacePreview: "#111424",
    borderPreview: "#222744",
  },
  {
    id: "cyber-emerald",
    name: "Cyber Emerald",
    tagline: "Abyssal pine & vivid mint",
    accent: "#10b981",
    bgPreview: "#081210",
    surfacePreview: "#0e1e1b",
    borderPreview: "#193832",
  },
];

class ThemeManager {
  current = $state<ThemeId>("soft-dark");
  fontSize = $state<FontSize>("normal");

  init() {
    if (typeof window === "undefined") return;
    const saved = localStorage.getItem("gemini_desktop_theme") as ThemeId | null;
    if (saved && THEMES.some((t) => t.id === saved)) {
      this.current = saved;
    } else {
      this.current = "soft-dark";
    }

    const savedFontSize = localStorage.getItem("gemini_desktop_font_size") as FontSize | null;
    if (savedFontSize === "comfortable" || savedFontSize === "normal") {
      this.fontSize = savedFontSize;
    }

    this.apply();
  }

  setTheme(id: ThemeId) {
    this.current = id;
    if (typeof window !== "undefined") {
      localStorage.setItem("gemini_desktop_theme", id);
      this.apply();
    }
  }

  setFontSize(size: FontSize) {
    this.fontSize = size;
    if (typeof window !== "undefined") {
      localStorage.setItem("gemini_desktop_font_size", size);
      this.apply();
    }
  }

  private apply() {
    if (typeof document !== "undefined") {
      document.documentElement.setAttribute("data-theme", this.current);
      document.documentElement.setAttribute("data-font-size", this.fontSize);
    }
  }
}

export const themeManager = new ThemeManager();
