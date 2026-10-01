import type { ApprovalMode } from "$lib/types";
import { Zap, Rocket, Shield, BookOpen } from "lucide-svelte";

export interface ApprovalModeConfig {
  id: ApprovalMode;
  label: string;
  shortLabel: string;
  description: string;
  icon: any;
  badgeClass: string;
  iconClass: string;
  tagClass: string;
}

export const APPROVAL_MODES: ApprovalModeConfig[] = [
  {
    id: "auto_edit",
    label: "Auto-Edit",
    shortLabel: "Auto-Edit",
    description: "Auto-approves file changes; asks before running terminal commands",
    icon: Zap,
    badgeClass: "bg-amber-500/15 hover:bg-amber-500/25 text-amber-600 dark:text-amber-400 border-amber-500/30",
    iconClass: "text-amber-500",
    tagClass: "bg-amber-500/15 text-amber-600 dark:text-amber-400 border border-amber-500/30",
  },
  {
    id: "yolo",
    label: "YOLO (Autonomous)",
    shortLabel: "YOLO",
    description: "Auto-approves all file changes and terminal commands without prompting",
    icon: Rocket,
    badgeClass: "bg-emerald-500/15 hover:bg-emerald-500/25 text-emerald-600 dark:text-emerald-400 border-emerald-500/30",
    iconClass: "text-emerald-500",
    tagClass: "bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 border border-emerald-500/30",
  },
  {
    id: "default",
    label: "Ask Permission",
    shortLabel: "Ask",
    description: "Prompts for confirmation before modifying files or executing commands",
    icon: Shield,
    badgeClass: "bg-sky-500/15 hover:bg-sky-500/25 text-sky-600 dark:text-sky-400 border-sky-500/30",
    iconClass: "text-sky-500",
    tagClass: "bg-sky-500/15 text-sky-600 dark:text-sky-400 border border-sky-500/30",
  },
  {
    id: "plan",
    label: "Plan Mode",
    shortLabel: "Plan",
    description: "Read-only research and design phase; file and terminal writes are disabled",
    icon: BookOpen,
    badgeClass: "bg-purple-500/15 hover:bg-purple-500/25 text-purple-600 dark:text-purple-400 border-purple-500/30",
    iconClass: "text-purple-500",
    tagClass: "bg-purple-500/15 text-purple-600 dark:text-purple-400 border border-purple-500/30",
  },
];

export function getModeConfig(mode?: string): ApprovalModeConfig {
  return APPROVAL_MODES.find((m) => m.id === mode) || APPROVAL_MODES[0];
}
