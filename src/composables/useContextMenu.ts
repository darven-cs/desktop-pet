import { computed, onMounted, onUnmounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";

export interface MenuItem {
  type?: "item" | "separator" | "submenu";
  label?: string;
  onClick?: () => void;
  current?: boolean;
  children?: MenuItem[]; // submenu items
}

const SCREEN_MARGIN = 8;
const ESTIMATED_MENU_WIDTH = 140;
const ESTIMATED_ITEM_HEIGHT = 30;
const ESTIMATED_MENU_PADDING = 8;

// Module-level guard: only one context menu open at a time (spec §3.3).
let activeCloser: (() => void) | null = null;

function countItems(items: MenuItem[]): number {
  let n = 0;
  for (const item of items) {
    if (item.type === "separator") {
      n += 1;
    } else {
      n += 1;
    }
  }
  return n;
}

export function useContextMenu() {
  const isOpen = ref(false);
  const items = ref<MenuItem[]>([]);
  const position = ref({ x: 0, y: 0 });
  const activeSubmenu = ref<string | null>(null);

  function close() {
    if (!isOpen.value) return;
    isOpen.value = false;
    activeSubmenu.value = null;
    if (activeCloser === close) {
      activeCloser = null;
    }
  }

  async function open(anchor: { x: number; y: number }, menuItems: MenuItem[]) {
    if (activeCloser && activeCloser !== close) {
      activeCloser();
    }

    items.value = menuItems;
    activeSubmenu.value = null;

    const win = getCurrentWindow();
    const outerPos = await win.outerPosition();
    const winX = outerPos.x;
    const winY = outerPos.y;
    const screenW = window.screen.width;
    const screenH = window.screen.height;

    const n = countItems(menuItems);
    const menuH = n * ESTIMATED_ITEM_HEIGHT + ESTIMATED_MENU_PADDING * 2;
    const menuW = ESTIMATED_MENU_WIDTH;

    let menuX = anchor.x;
    let menuY = anchor.y;

    if (winX + anchor.x + menuW > screenW - SCREEN_MARGIN) {
      menuX = anchor.x - menuW;
    }
    if (winY + anchor.y + menuH > screenH - SCREEN_MARGIN) {
      menuY = anchor.y - menuH;
    }

    if (winX + menuX + menuW > screenW - SCREEN_MARGIN) {
      menuX = screenW - SCREEN_MARGIN - menuW - winX;
    }
    if (winY + menuY + menuH > screenH - SCREEN_MARGIN) {
      menuY = screenH - SCREEN_MARGIN - menuH - winY;
    }
    if (winX + menuX < SCREEN_MARGIN) {
      menuX = SCREEN_MARGIN - winX;
    }
    if (winY + menuY < SCREEN_MARGIN) {
      menuY = SCREEN_MARGIN - winY;
    }

    position.value = { x: menuX, y: menuY };
    isOpen.value = true;
    activeCloser = close;
  }

  function onItemClick(item: MenuItem) {
    if (item.type === "submenu") return; // handled by hover
    if (item.onClick) {
      const cb = item.onClick;
      close();
      cb();
    } else {
      close();
    }
  }

  function toggleSubmenu(label: string) {
    activeSubmenu.value = activeSubmenu.value === label ? null : label;
  }

  function onSubmenuItemClick(item: MenuItem) {
    if (item.onClick) {
      const cb = item.onClick;
      close();
      cb();
    }
  }

  function onOutsideMouseDown(e: MouseEvent) {
    if (!isOpen.value) return;
    const target = e.target as HTMLElement | null;
    if (target && target.closest(".context-menu")) return;
    close();
  }

  function onKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape" && isOpen.value) {
      close();
    }
  }

  onMounted(() => {
    document.addEventListener("mousedown", onOutsideMouseDown);
    document.addEventListener("keydown", onKeyDown);
  });

  onUnmounted(() => {
    document.removeEventListener("mousedown", onOutsideMouseDown);
    document.removeEventListener("keydown", onKeyDown);
    if (isOpen.value) close();
  });

  const menuStyle = computed(() => ({
    left: `${position.value.x}px`,
    top: `${position.value.y}px`,
  }));

  return {
    isOpen,
    items,
    open,
    close,
    onItemClick,
    activeSubmenu,
    toggleSubmenu,
    onSubmenuItemClick,
    menuStyle,
  };
}
