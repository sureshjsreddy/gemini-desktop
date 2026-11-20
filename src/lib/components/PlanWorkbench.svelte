<script lang="ts">
  import type { Workspace, ApprovalMode, PlanTaskItem } from "$lib/types";
  import {
    ListTodo,
    CheckSquare,
    Square,
    ChevronDown,
    ChevronUp,
    Zap,
    Shield,
    Plus,
    Trash2,
    X,
  } from "lucide-svelte";

  let {
    workspace = null,
    sessionId = null,
    approvalMode = "default",
    tasks = [],
    isOpen = false,
    onTasksChange,
    onExecutePlan,
    onRefinePlan,
    onClose,
  }: {
    workspace: Workspace | null;
    sessionId?: string | null;
    approvalMode: ApprovalMode;
    tasks: PlanTaskItem[];
    isOpen?: boolean;
    onTasksChange?: (updated: PlanTaskItem[]) => void;
    onExecutePlan: (targetMode: "auto_edit" | "default", tasks: PlanTaskItem[]) => void;
    onRefinePlan: () => void;
    onClose?: () => void;
  } = $props();

  let isCollapsed = $state(false);
  let newTaskInput = $state("");

  function toggleTask(id: string) {
    const updated = tasks.map((t) => (t.id === id ? { ...t, completed: !t.completed } : t));
    onTasksChange?.(updated);
  }

  function removeTask(id: string) {
    const updated = tasks.filter((t) => t.id !== id);
    onTasksChange?.(updated);
  }

  function handleAddTask() {
    const trimmed = newTaskInput.trim();
    if (!trimmed) return;
    const newTask: PlanTaskItem = {
      id: `task-manual-${Date.now()}-${Math.random().toString(36).slice(2)}`,
      text: trimmed,
      completed: false,
      category: "Custom Tasks",
    };
    onTasksChange?.([...tasks, newTask]);
    newTaskInput = "";
  }

  let completedCount = $derived(tasks.filter((t) => t.completed).length);
  let totalCount = $derived(tasks.length);
  let percent = $derived(totalCount > 0 ? Math.round((completedCount / totalCount) * 100) : 0);
</script>

{#if isOpen}
  <div class="px-4 sm:px-6 mb-3">
    <div
      class="w-full max-w-4xl lg:max-w-5xl xl:max-w-6xl 2xl:max-w-7xl mx-auto rounded-xl border border-sky-500/30 bg-surface-elevated/95 backdrop-blur-xs shadow-xl overflow-hidden transition-all text-xs"
    >
    <!-- Header -->
    <div
      class="px-4 py-2 bg-sky-500/10 border-b border-sky-500/20 flex items-center justify-between"
    >
      <div class="flex items-center gap-2">
        <div
          class="w-6 h-6 rounded-md bg-sky-500/20 text-sky-400 flex items-center justify-center font-bold"
        >
          <ListTodo size={14} />
        </div>
        <div>
          <span class="font-semibold text-primary-theme flex items-center gap-2">
            Plan Mode Workbench
            {#if totalCount > 0}
              <span
                class="text-[10px] font-mono px-1.5 py-0.5 rounded {completedCount === totalCount ? 'bg-emerald-500/20 text-emerald-300 border-emerald-500/30' : 'bg-sky-500/20 text-sky-300 border-sky-500/30'}"
              >
                {completedCount}/{totalCount} Completed ({percent}%)
              </span>
            {/if}
          </span>
        </div>
      </div>

      <div class="flex items-center gap-1.5">
        {#if totalCount > 0}
          <div class="w-20 h-1.5 bg-surface rounded-full overflow-hidden mr-1 border border-subtle">
            <div class="h-full {completedCount === totalCount ? 'bg-emerald-400' : 'bg-sky-400'} transition-all duration-300" style="width: {percent}%"></div>
          </div>
        {/if}

        <button
          type="button"
          onclick={() => (isCollapsed = !isCollapsed)}
          class="p-1 rounded text-secondary-theme hover:text-primary-theme hover:bg-surface transition-colors cursor-pointer"
          aria-label={isCollapsed ? "Expand plan workbench" : "Collapse plan workbench"}
        >
          {#if isCollapsed}
            <ChevronDown size={14} />
          {:else}
            <ChevronUp size={14} />
          {/if}
        </button>

        {#if onClose}
          <button
            type="button"
            onclick={onClose}
            class="p-1 rounded text-muted-theme hover:text-primary-theme hover:bg-surface transition-colors cursor-pointer ml-1"
            aria-label="Close plan workbench"
          >
            <X size={14} />
          </button>
        {/if}
      </div>
    </div>

    <!-- Workbench Body -->
    {#if !isCollapsed}
      <div class="p-3 max-h-56 overflow-y-auto space-y-1.5 divide-y divide-subtle/30">
        {#if tasks.length === 0}
          <div class="text-center py-5 text-muted-theme text-xs space-y-1">
            <p>No tasks in the current plan.</p>
            <p class="text-[11px] text-secondary-theme">
              Add tasks manually below, or ask the agent to create an implementation plan.
            </p>
          </div>
        {:else}
          {#each tasks as task (task.id)}
            <div class="pt-1.5 first:pt-0 flex items-start gap-2.5 group">
              <button
                type="button"
                onclick={() => toggleTask(task.id)}
                class="mt-0.5 text-muted-theme hover:text-sky-400 transition-colors cursor-pointer shrink-0"
                aria-label={task.completed ? "Mark incomplete" : "Mark complete"}
              >
                {#if task.completed}
                  <CheckSquare size={14} class="text-sky-400" />
                {:else}
                  <Square size={14} />
                {/if}
              </button>
              <span
                class="text-xs leading-relaxed select-text flex-1 {task.completed
                  ? 'line-through text-muted-theme'
                  : 'text-primary-theme'}"
              >
                {task.text}
              </span>
              {#if task.category}
                <span
                  class="text-[10px] text-muted-theme/70 font-mono shrink-0 hidden group-hover:inline"
                >
                  {task.category}
                </span>
              {/if}
              <button
                type="button"
                onclick={() => removeTask(task.id)}
                class="opacity-0 group-hover:opacity-100 text-muted-theme hover:text-rose-400 p-0.5 rounded transition-all cursor-pointer shrink-0"
                aria-label="Delete task"
              >
                <Trash2 size={12} />
              </button>
            </div>
          {/each}
        {/if}
      </div>

      <!-- Add Manual Task Input -->
      <div class="px-3 py-2 bg-surface/50 border-t border-subtle/40 flex items-center gap-2">
        <input
          type="text"
          bind:value={newTaskInput}
          onkeydown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              handleAddTask();
            }
          }}
          placeholder="Add a step to the plan (press Enter)..."
          class="flex-1 bg-surface border border-subtle/60 rounded-md px-2.5 py-1 text-xs text-primary-theme placeholder:text-muted-theme focus:outline-none focus:border-sky-500"
        />
        <button
          type="button"
          onclick={handleAddTask}
          disabled={!newTaskInput.trim()}
          class="flex items-center gap-1 px-2.5 py-1 rounded-md text-xs font-medium bg-sky-500/20 text-sky-300 hover:bg-sky-500/30 transition-colors cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed shrink-0"
        >
          <Plus size={12} />
          <span>Add</span>
        </button>
      </div>

      <!-- Action Footer -->
      <div
        class="px-3 py-2 bg-surface/40 border-t border-subtle/40 flex flex-wrap items-center justify-between gap-2"
      >
        <span class="text-[11px] {completedCount === totalCount && totalCount > 0 ? 'text-emerald-400 font-medium' : 'text-secondary-theme'}">
          {#if completedCount === totalCount && totalCount > 0}
            ✨ All plan tasks completed! Uncheck to rerun or add new tasks above.
          {:else}
            Ready to execute? Choose how the agent should proceed with these tasks:
          {/if}
        </span>

        <div class="flex items-center gap-2">
          <button
            type="button"
            onclick={() => onExecutePlan("auto_edit", tasks)}
            disabled={tasks.length === 0 || completedCount === totalCount}
            class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium bg-emerald-500/20 text-emerald-300 hover:bg-emerald-500/30 border border-emerald-500/30 transition-colors shadow-xs cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
          >
            <Zap size={13} class="text-emerald-400" />
            <span>{completedCount === totalCount && totalCount > 0 ? "All Tasks Completed" : "Approve & Execute (Auto-Edit)"}</span>
          </button>

          <button
            type="button"
            onclick={() => onExecutePlan("default", tasks)}
            disabled={tasks.length === 0 || completedCount === totalCount}
            class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium bg-surface hover:bg-surface-hover text-secondary-theme hover:text-primary-theme border border-theme-default transition-colors cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
          >
            <Shield size={13} class="text-sky-400" />
            <span>Approve & Execute (Ask)</span>
          </button>
        </div>
      </div>
    {/if}
    </div>
  </div>
{/if}
