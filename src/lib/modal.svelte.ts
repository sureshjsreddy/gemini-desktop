export type ModalType = "search" | "workspace" | "templates" | "theme" | "mcp";

class ModalManager {
  active = $state<ModalType | null>(null);

  isOpen(type: ModalType): boolean {
    return this.active === type;
  }

  open(type: ModalType) {
    this.active = type;
  }

  close() {
    this.active = null;
  }

  toggle(type: ModalType) {
    this.active = this.active === type ? null : type;
  }
}

export const modalManager = new ModalManager();
