<script lang="ts">
  import type { WorkspaceFileEntry } from "$lib/types";
  import {
    Search,
    X,
    Folder,
    FileCode,
    CornerDownLeft,
    ChevronRight,
    ArrowUp,
    Paperclip,
    Eye,
    EyeOff,
  } from "lucide-svelte";
  import Tooltip from "$lib/components/Tooltip.svelte";
  import { isHiddenOrIgnored } from "$lib/utils/fileFilters";
  import { getFileIconColor } from "$lib/utils/fileIcons";

  let {
    isOpen = false,
    mode = "file",
    workspaceFiles = [],
    onClose,
    onSelect,
    onSelectCustomPath,
  }: {
    isOpen: boolean;
    mode: "file" | "directory" | "all";
    workspaceFiles: WorkspaceFileEntry[];
    onClose: () => void;
    onSelect: (entry: WorkspaceFileEntry) => void;
    onSelectCustomPath: (path: string) => void;
  } = $props();

  let search = $state("");
  let currentDir = $state("");
  let selectedIndex = $state(0);
  let showHidden = $state(false);
  let searchInputEl: HTMLInputElement | null = $state(null);

  // Reset navigation when modal opens
  $effect(() => {
    if (isOpen) {
      search = "";
      currentDir = "";
      selectedIndex = 0;
      setTimeout(() => searchInputEl?.focus(), 60);
    }
  });

  // Breadcrumbs for folder navigation
  let breadcrumbs = $derived.by(() => {
    const crumbs: { label: string; path: string }[] = [{ label: "Workspace Root", path: "" }];
    if (!currentDir) return crumbs;

    const parts = currentDir.split("/");
    let accum = "";
    for (const part of parts) {
      accum = accum ? `${accum}/${part}` : part;
      crumbs.push({ label: part, path: accum });
    }
    return crumbs;
  });

  // Items in active view: either search results across workspace OR direct children of currentDir
  let activeItems = $derived.by(() => {
    const q = search.trim().toLowerCase();
    const querySearchesHidden = q.startsWith(".");

    // Case 1: Search mode across entire workspace
    if (q) {
      return workspaceFiles
        .filter((entry) => {
          if (mode === "file" && entry.is_dir) return false;
          if (mode === "directory" && !entry.is_dir) return false;

          // Hidden & ignored filtering
          if (!showHidden && !querySearchesHidden) {
            if (isHiddenOrIgnored(entry.relative_path)) return false;
          }

          return (
            entry.name.toLowerCase().includes(q) ||
            entry.relative_path.toLowerCase().includes(q)
          );
        })
        .slice(0, 60);
    }

    // Case 2: Hierarchical navigation under currentDir
    const prefix = currentDir ? `${currentDir}/` : "";
    const directDirsMap = new Map<string, { entry: WorkspaceFileEntry; count: number }>();
    const directFiles: WorkspaceFileEntry[] = [];

    for (const entry of workspaceFiles) {
      const rel = entry.relative_path.replace(/\\/g, "/");

      // Skip entries outside current directory
      if (currentDir) {
        if (!rel.startsWith(prefix)) continue;
      }

      // Check hidden/ignored
      if (!showHidden && !querySearchesHidden) {
        if (isHiddenOrIgnored(rel)) continue;
      }

      const remainder = currentDir ? rel.slice(prefix.length) : rel;
      if (!remainder) continue;

      const parts = remainder.split("/");
      const immediateName = parts[0];
      const immediatePath = currentDir ? `${currentDir}/${immediateName}` : immediateName;

      if (parts.length === 1) {
        if (entry.is_dir) {
          if (!directDirsMap.has(immediatePath)) {
            directDirsMap.set(immediatePath, {
              entry: {
                name: immediateName,
                relative_path: immediatePath,
                is_dir: true,
                extension: undefined,
              },
              count: 0,
            });
          }
        } else {
          if (mode !== "directory") {
            directFiles.push(entry);
          }
        }
      } else {
        // Deeper child means immediateName is a folder
        let existing = directDirsMap.get(immediatePath);
        if (!existing) {
          existing = {
            entry: {
              name: immediateName,
              relative_path: immediatePath,
              is_dir: true,
              extension: undefined,
            },
            count: 1,
          };
          directDirsMap.set(immediatePath, existing);
        } else {
          existing.count++;
        }
      }
    }

    // Sort folders alphabetically
    const dirs = Array.from(directDirsMap.values())
      .map((d) => d.entry)
      .sort((a, b) => a.name.localeCompare(b.name, undefined, { sensitivity: "base" }));

    // Sort files alphabetically
    directFiles.sort((a, b) => a.name.localeCompare(b.name, undefined, { sensitivity: "base" }));

    return [...dirs, ...directFiles];
  });

  // Keep selected index valid
  $effect(() => {
    if (activeItems.length === 0) {
      selectedIndex = 0;
    } else if (selectedIndex >= activeItems.length) {
      selectedIndex = activeItems.length - 1;
    }
  });

  function navigateInto(dirPath: string) {
    currentDir = dirPath;
    selectedIndex = 0;
    search = "";
  }

  function navigateUp() {
    if (!currentDir) return;
    const parts = currentDir.split("/");
    parts.pop();
    currentDir = parts.join("/");
    selectedIndex = 0;
    search = "";
  }

  function navigateToCrumb(path: string) {
    currentDir = path;
    selectedIndex = 0;
    search = "";
  }

  function chooseEntry(entry: WorkspaceFileEntry) {
    onSelect(entry);
    onClose();
  }

  function attachCurrentFolder() {
    if (!currentDir) return;
    const name = currentDir.split("/").pop() || currentDir;
    onSelect({
      name,
      relative_path: currentDir,
      is_dir: true,
      extension: undefined,
    });
    onClose();
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      onClose();
    } else if (e.key === "Backspace" && !search && currentDir) {
      e.preventDefault();
      navigateUp();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      if (activeItems.length > 0) {
        selectedIndex = (selectedIndex + 1) % activeItems.length;
      }
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      if (activeItems.length > 0) {
        selectedIndex = (selectedIndex - 1 + activeItems.length) % activeItems.length;
      }
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (activeItems.length > 0 && selectedIndex < activeItems.length) {
        const item = activeItems[selectedIndex];
        if (item.is_dir) {
          if (mode === "directory" && e.ctrlKey) {
            chooseEntry(item);
          } else {
            navigateInto(item.relative_path);
          }
        } else {
          chooseEntry(item);
        }
      } else if (search.trim()) {
        onSelectCustomPath(search.trim());
        onClose();
      }
    }
  }

  function handleCustomSubmit() {
    if (search.trim()) {
      onSelectCustomPath(search.trim());
      onClose();
    }
  }
</script>

{#if isOpen}
  <div
    class="fixed inset-0 bg-black/65 backdrop-blur-xs z-50 flex items-start justify-center pt-16 p-4"
    onclick={onClose}
    onkeydown={handleKeyDown}
    role="presentation"
  >
    <div
      class="w-full max-w-2xl bg-surface border border-theme-default rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[80vh]"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      tabindex="-1"
    >
      <!-- Modal Header -->
      <div class="px-4 py-3 border-b border-subtle flex items-center justify-between bg-surface-elevated/50 select-none">
        <div class="flex items-center gap-2">
          {#if mode === "directory"}
            <div class="w-5 h-5 rounded-md bg-sky-500/15 text-sky-400 flex items-center justify-center">
              <Folder size={14} />
            </div>
            <div>
              <h2 class="text-xs font-semibold text-primary-theme">Attach Workspace Folder</h2>
              <p class="text-[10px] text-muted-theme">Navigate into subfolders or click Attach to add to chat context</p>
            </div>
          {:else}
            <div class="w-5 h-5 rounded-md bg-accent-theme/15 text-accent-theme flex items-center justify-center">
              <FileCode size={14} />
            </div>
            <div>
              <h2 class="text-xs font-semibold text-primary-theme">Attach Workspace File</h2>
              <p class="text-[10px] text-muted-theme">Browse folders or search to select a file for chat context</p>
            </div>
          {/if}
        </div>

        <div class="flex items-center gap-1">
          <!-- Toggle Hidden Files & Folders -->
          <Tooltip
            text={showHidden ? "Hide Hidden Files & Folders (.*)" : "Show Hidden Files & Folders"}
            position="bottom"
          >
            <button
              type="button"
              onclick={() => (showHidden = !showHidden)}
              class="p-1.5 rounded-lg hover:bg-surface transition-colors cursor-pointer {showHidden ? 'bg-accent-subtle text-accent-theme' : 'text-muted-theme hover:text-primary-theme'}"
              aria-label={showHidden ? "Hide hidden files" : "Show hidden files"}
            >
              {#if showHidden}
                <Eye size={14} />
              {:else}
                <EyeOff size={14} />
              {/if}
            </button>
          </Tooltip>

          <!-- Close Modal -->
          <button
            type="button"
            onclick={onClose}
            class="p-1.5 text-muted-theme hover:text-primary-theme rounded-lg hover:bg-surface transition-colors cursor-pointer"
            aria-label="Close dialog"
          >
            <X size={15} />
          </button>
        </div>
      </div>

      <!-- Search Bar -->
      <div class="p-3 border-b border-subtle flex items-center gap-2.5 bg-surface">
        <Search size={14} class="text-muted-theme shrink-0" />
        <input
          type="text"
          bind:this={searchInputEl}
          bind:value={search}
          placeholder={mode === "directory" ? "Search folders or type path... (e.g. src/components/)" : "Search files across workspace... (e.g. ChatView.svelte)"}
          class="flex-1 bg-transparent text-primary-theme text-xs focus:outline-none placeholder:text-muted-theme"
        />
        {#if search.trim()}
          <button
            type="button"
            onclick={() => {
              search = "";
              searchInputEl?.focus();
            }}
            class="p-0.5 text-muted-theme hover:text-primary-theme cursor-pointer mr-1"
            aria-label="Clear search"
          >
            <X size={13} />
          </button>
          <button
            type="button"
            onclick={handleCustomSubmit}
            class="flex items-center gap-1 px-2 py-1 rounded-md bg-accent-subtle text-accent-theme hover:bg-accent-theme hover:text-white text-[11px] font-medium transition-colors cursor-pointer"
            title="Use typed path directly"
          >
            <span>Attach Path</span>
            <CornerDownLeft size={11} />
          </button>
        {/if}
      </div>

      <!-- Breadcrumbs Bar (When not actively searching) -->
      {#if !search.trim()}
        <div class="px-3 py-2 bg-surface-elevated/40 border-b border-subtle flex items-center justify-between gap-2 overflow-x-auto text-xs select-none">
          <div class="flex items-center gap-1 min-w-0 flex-1 overflow-hidden">
            {#each breadcrumbs as crumb, idx (crumb.path)}
              {#if idx > 0}
                <ChevronRight size={12} class="text-muted-theme/60 shrink-0" />
              {/if}
              <button
                type="button"
                onclick={() => navigateToCrumb(crumb.path)}
                class="px-1.5 py-0.5 rounded text-xs transition-colors cursor-pointer truncate max-w-[140px] {idx === breadcrumbs.length - 1 ? 'font-semibold text-accent-theme bg-accent-subtle/40' : 'text-muted-theme hover:text-primary-theme hover:bg-surface'}"
                title={crumb.path || "Workspace Root"}
              >
                {crumb.label}
              </button>
            {/each}
          </div>

          <!-- Attach Current Directory Button (Directory Mode) -->
          {#if mode === "directory" && currentDir}
            <button
              type="button"
              onclick={attachCurrentFolder}
              class="flex items-center gap-1 px-2.5 py-1 rounded bg-accent-theme hover:bg-accent-hover text-white text-[11px] font-medium transition-colors cursor-pointer shrink-0 shadow-xs"
            >
              <Paperclip size={11} />
              <span>Attach This Folder (@{currentDir}/)</span>
            </button>
          {/if}
        </div>
      {/if}

      <!-- Items List -->
      <div class="flex-1 overflow-y-auto divide-y divide-subtle/30 p-1 min-h-[220px]">
        <!-- Parent Directory Navigation Row (When inside a subfolder and not searching) -->
        {#if !search.trim() && currentDir}
          <button
            type="button"
            onclick={navigateUp}
            class="w-full px-3 py-2 text-left flex items-center gap-2.5 rounded-lg hover:bg-surface-hover text-secondary-theme hover:text-primary-theme transition-colors cursor-pointer group"
          >
            <div class="w-5 h-5 rounded flex items-center justify-center bg-surface-elevated text-muted-theme group-hover:text-primary-theme shrink-0">
              <ArrowUp size={13} />
            </div>
            <div class="flex-1 min-w-0">
              <span class="text-xs font-mono font-medium">.. (Parent Directory)</span>
            </div>
            <span class="text-[10px] text-muted-theme font-mono opacity-60">Backspace</span>
          </button>
        {/if}

        {#if activeItems.length === 0}
          <div class="py-12 text-center text-xs text-muted-theme flex flex-col items-center gap-2">
            {#if search.trim()}
              <p>No workspace {mode === "directory" ? "folders" : "files"} matching "{search}".</p>
              <button
                type="button"
                onclick={handleCustomSubmit}
                class="mt-2 inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-surface-elevated hover:bg-surface text-accent-theme border border-subtle text-xs cursor-pointer transition-colors"
              >
                <span>Attach as custom path: <code class="font-mono">{search.trim()}</code></span>
              </button>
            {:else}
              <Folder size={24} class="opacity-30 text-muted-theme" />
              <p>This folder is empty or contains only hidden files.</p>
              {#if !showHidden}
                <button
                  type="button"
                  onclick={() => (showHidden = true)}
                  class="text-accent-theme hover:underline cursor-pointer text-xs"
                >
                  Show hidden files
                </button>
              {/if}
            {/if}
          </div>
        {:else}
          {#each activeItems as entry, idx (entry.relative_path)}
            {@const isSelected = idx === selectedIndex}
            {@const isHiddenItem = entry.name.startsWith(".")}

            <div
              role="button"
              tabindex="0"
              class="w-full px-3 py-2 text-left flex items-center justify-between rounded-lg transition-colors cursor-pointer select-none {isSelected ? 'bg-accent-theme/15 text-accent-theme border border-accent-subtle' : 'hover:bg-surface-hover text-secondary-theme hover:text-primary-theme'} {isHiddenItem ? 'opacity-70 italic' : ''}"
              onclick={() => {
                selectedIndex = idx;
                if (entry.is_dir) {
                  // In directory mode or file mode, clicking folder row navigates into it
                  navigateInto(entry.relative_path);
                } else {
                  chooseEntry(entry);
                }
              }}
              onmouseenter={() => (selectedIndex = idx)}
              onkeydown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  if (entry.is_dir) {
                    navigateInto(entry.relative_path);
                  } else {
                    chooseEntry(entry);
                  }
                }
              }}
            >
              <div class="flex items-center gap-2.5 min-w-0 flex-1">
                {#if entry.is_dir}
                  <Folder size={16} class="text-[#f59e0b] shrink-0" />
                {:else}
                  <FileCode size={16} style="color: {getFileIconColor(entry.extension)};" class="shrink-0" />
                {/if}

                <div class="min-w-0 flex-1">
                  <div class="text-xs font-mono font-medium text-primary-theme truncate flex items-center gap-1.5">
                    <span>{entry.name}{entry.is_dir ? "/" : ""}</span>
                  </div>
                  <div class="text-[10px] text-muted-theme truncate font-mono">
                    {entry.relative_path.replace(/\\/g, "/")}{entry.is_dir ? "/" : ""}
                  </div>
                </div>
              </div>

              <!-- Item Actions -->
              <div class="flex items-center gap-1.5 ml-2 shrink-0">
                {#if entry.is_dir}
                  <!-- In Directory Mode, offer direct "Attach" button for this folder -->
                  {#if mode === "directory"}
                    <button
                      type="button"
                      onclick={(e) => {
                        e.stopPropagation();
                        chooseEntry(entry);
                      }}
                      class="flex items-center gap-1 px-2 py-0.5 rounded bg-accent-subtle hover:bg-accent-theme hover:text-white text-accent-theme text-[11px] font-medium transition-colors cursor-pointer"
                      title="Attach folder @{entry.relative_path}/"
                    >
                      <Paperclip size={10} />
                      <span>Attach</span>
                    </button>
                  {/if}

                  <!-- Open folder indicator -->
                  <div class="flex items-center text-muted-theme hover:text-primary-theme px-1 py-0.5 text-[11px]">
                    <span class="text-[10px] mr-0.5">Open</span>
                    <ChevronRight size={12} />
                  </div>
                {:else}
                  <span class="text-[10px] text-muted-theme font-mono">
                    @{entry.relative_path.replace(/\\/g, "/")}
                  </span>
                {/if}
              </div>
            </div>
          {/each}
        {/if}
      </div>

      <!-- Footer Info Strip -->
      <div class="px-4 py-2 bg-surface-elevated/50 border-t border-subtle flex items-center justify-between text-[11px] text-muted-theme select-none">
        <span class="font-mono text-[10px]">
          {#if search.trim()}
            Found {activeItems.length} matching items
          {:else}
            {activeItems.length} items in {currentDir ? `/${currentDir}` : "workspace root"}
          {/if}
        </span>
        <div class="flex items-center gap-2 text-[10px]">
          <span><kbd class="px-1 py-0.5 rounded bg-surface border border-subtle font-mono text-[9px]">↑↓</kbd> Select</span>
          <span>&bull;</span>
          <span><kbd class="px-1 py-0.5 rounded bg-surface border border-subtle font-mono text-[9px]">Enter</kbd> Open / Attach</span>
          {#if currentDir}
            <span>&bull;</span>
            <span><kbd class="px-1 py-0.5 rounded bg-surface border border-subtle font-mono text-[9px]">Backspace</kbd> Up</span>
          {/if}
          <span>&bull;</span>
          <span><kbd class="px-1 py-0.5 rounded bg-surface border border-subtle font-mono text-[9px]">Esc</kbd> Close</span>
        </div>
      </div>
    </div>
  </div>
{/if}
