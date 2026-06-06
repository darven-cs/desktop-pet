<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { useAnimationRegistry } from "./composables/useAnimationRegistry";
import { useAnimationStateMachine } from "./composables/useAnimationStateMachine";
import { useContextMenu, type MenuItem } from "./composables/useContextMenu";
import { usePetSettings } from "./composables/usePetSettings";
import type { AnimationEntry, AnimationId } from "./types/pet";
import PetStatusPanel from "./components/PetStatusPanel.vue";
import PetSettingsPanel from "./components/PetSettingsPanel.vue";
import PetChatPanel from "./components/PetChatPanel.vue";
import PetMemoryPanel from "./components/PetMemoryPanel.vue";

// --- Constants ---
const MENU_ITEM_H = 30;
const MENU_PADDING_H = 12;
const MENU_EST_WIDTH = 160;
const OVERLAY_EST_W = 310;
const OVERLAY_EST_H = 390;

const { registry, error } = useAnimationRegistry();
const { settings, lastDecisionReason, updateSettings } = usePetSettings();

const onDecision = (reason: string | null) => {
  lastDecisionReason.value = reason;
};

const petSpeakMessage = ref<{
  message: string;
  animation: string | null;
} | null>(null);

const onSpeak = (message: string, animation: string | null) => {
  petSpeakMessage.value = { message, animation };
  overlay.value = "chat";
};

const { state, dispatch, tickerInterval, lastTickerReason } =
  useAnimationStateMachine({
    llmEnabled: computed(() => settings.value.llmEnabled),
    petPersonality: computed(() => settings.value.petPersonality),
    petName: computed(() => settings.value.petName),
    llmApiEndpoint: computed(() => settings.value.apiEndpoint),
    llmApiKey: computed(() => settings.value.apiKey),
    llmModel: computed(() => settings.value.model),
    onDecision,
    onSpeak,
  });

const {
  isOpen: menuIsOpen,
  items: menuItems,
  open: menuOpen,
  onItemClick: menuOnItemClick,
  activeSubmenu,
  toggleSubmenu,
  menuStyle,
} = useContextMenu();

const overlay = ref<"status" | "settings" | "chat" | "memory" | null>(null);
const chatContextText = ref<string | null>(null);

function showOverlay(panel: "status" | "settings" | "chat" | "memory") {
  overlay.value = overlay.value === panel ? null : panel;
  if (panel !== "chat") {
    chatContextText.value = null;
  }
}

function closeOverlay() {
  overlay.value = null;
  chatContextText.value = null;
  petSpeakMessage.value = null;
}

// Left-click short-press detection for clipboard chat.
let mouseDownPos = { x: 0, y: 0 };
let mouseDownTime = 0;
const CLICK_MAX_MS = 300;
const CLICK_MAX_PX = 5;

function onPetMouseDown(e: MouseEvent) {
  mouseDownPos = { x: e.clientX, y: e.clientY };
  mouseDownTime = Date.now();
}

function onPetMouseUp(e: MouseEvent) {
  const dt = Date.now() - mouseDownTime;
  const dx = Math.abs(e.clientX - mouseDownPos.x);
  const dy = Math.abs(e.clientY - mouseDownPos.y);
  if (dt < CLICK_MAX_MS && dx < CLICK_MAX_PX && dy < CLICK_MAX_PX) {
    // Short click — try reading clipboard for chat context.
    readClipboardAndOpenChat();
  }
}

async function readClipboardAndOpenChat() {
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    const text: string = await invoke("read_clipboard");
    if (text && text.trim()) {
      chatContextText.value = text.trim();
      overlay.value = "chat";
    }
  } catch {
    // Couldn't read clipboard, ignore.
  }
}

function startDrag(e: MouseEvent) {
  if (e.button !== 0) return;
  import("@tauri-apps/api/window")
    .then(({ getCurrentWindow }) => {
      try {
        getCurrentWindow().startDragging();
      } catch (err) {
        console.error("[PetError] startDragging failed:", err);
      }
    })
    .catch((err) => {
      console.error("[PetError] dynamic import failed:", err);
    });
}

const currentEntry = computed<AnimationEntry | undefined>(() =>
  registry.value.find((e) => e.id === state.value.current),
);

const totalDuration = computed(() => {
  const c = currentEntry.value;
  if (!c) return 0;
  return (c.frameCount / c.fps) * 1000;
});

// Sprite dimensions (base window size when no menu/overlay is open).
const spriteW = computed(() => currentEntry.value?.frameWidth ?? 240);
const spriteH = computed(() => currentEntry.value?.frameHeight ?? 240);

const spriteStyle = computed(() => {
  const c = currentEntry.value;
  if (!c) return {};
  const totalWidth = c.frameCount * c.frameWidth;
  const iter = c.loopMode === "infinite" ? "infinite" : "1";
  return {
    "--end-x": `-${totalWidth}px`,
    width: `${c.frameWidth}px`,
    height: `${c.frameHeight}px`,
    backgroundImage: `url('${c.sheetPath}')`,
    backgroundSize: `${totalWidth}px ${c.frameHeight}px`,
    animation: `pet-play ${totalDuration.value}ms steps(${c.frameCount}) ${iter}`,
  };
});

function menuPixelVal(styleVal: string | undefined, fallback: number): number {
  if (!styleVal) return fallback;
  const n = parseFloat(styleVal);
  return Number.isFinite(n) ? n : fallback;
}

const needWindowW = computed(() => {
  let w = spriteW.value;
  if (menuIsOpen.value) {
    const left = menuPixelVal(menuStyle.value?.left, 0);
    const menuRight = left + MENU_EST_WIDTH;
    w = Math.max(w, menuRight);
  }
  if (overlay.value) {
    w = Math.max(w, OVERLAY_EST_W);
  }
  return Math.ceil(w);
});

const needWindowH = computed(() => {
  let h = spriteH.value;
  if (menuIsOpen.value) {
    const top = menuPixelVal(menuStyle.value?.top, 0);
    const nItems = menuItems.value.length;
    const menuBottom = top + nItems * MENU_ITEM_H + MENU_PADDING_H;
    h = Math.max(h, menuBottom);
  }
  if (overlay.value) {
    h = Math.max(h, spriteH.value + OVERLAY_EST_H);
  }
  return Math.ceil(h);
});

const preloaded = new Set<string>();
function preload(sheetPath: string): Promise<void> {
  if (preloaded.has(sheetPath)) return Promise.resolve();
  return new Promise((resolve) => {
    const img = new Image();
    img.onload = () => {
      preloaded.add(sheetPath);
      resolve();
    };
    img.onerror = () => {
      console.error(`[PetError] preload failed: ${sheetPath}`);
      resolve();
    };
    img.src = sheetPath;
  });
}

async function setWindowSize(width: number, height: number) {
  try {
    await getCurrentWindow().setSize(new LogicalSize(width, height));
  } catch (e) {
    console.error(`[PetError] setSize ${width}x${height} failed:`, e);
  }
}

watch([needWindowW, needWindowH], async ([w, h]) => {
  await setWindowSize(w, h);
});

watch(
  () => state.value,
  async (s) => {
    if (s.phase === "transitioning" && s.transition) {
      const target = registry.value.find((e) => e.id === s.transition!.to);
      if (target) {
        await preload(target.sheetPath);
      }
      dispatch({ type: "transition_complete" });
    }
  },
);

watch(
  () => settings.value.tickerIntervalMs,
  (next) => {
    if (next > 0 && next !== tickerInterval.value) {
      tickerInterval.value = next;
    }
  },
);

onMounted(async () => {
  for (const entry of registry.value) {
    preload(entry.sheetPath).catch(() => {});
  }
});

function onContextMenu(e: MouseEvent) {
  closeOverlay();

  const animSubmenu: MenuItem[] = registry.value.map((entry) => ({
    label: entry.id,
    onClick: () =>
      dispatch({
        type: "switch_to",
        id: entry.id,
        source: "dispatch",
      }),
    current: entry.id === state.value.current,
  }));

  const items: MenuItem[] = [
    {
      label: "宠物状态",
      onClick: () => showOverlay("status"),
    },
    {
      label: "宠物设定",
      onClick: () => showOverlay("settings"),
    },
    {
      label: "宠物对话",
      onClick: () => showOverlay("chat"),
    },
    {
      label: "宠物记忆",
      onClick: () => showOverlay("memory"),
    },
    { type: "separator" },
    {
      type: "submenu",
      label: "手动切动画",
      children: animSubmenu,
    },
    { type: "separator" },
    {
      label: "退出",
      onClick: () => getCurrentWindow().close(),
    },
  ];

  menuOpen({ x: e.clientX, y: e.clientY }, items);
}

function onSettingsSave(s: typeof settings.value) {
  updateSettings(s);
}

function onChatSwitchAnimation(id: AnimationId) {
  dispatch({
    type: "switch_to",
    id,
    source: "dispatch",
  });
}
</script>

<template>
  <div v-if="error" class="placeholder">宠物暂时打瞌睡了</div>
  <div
    v-else
    class="pet-area"
    @mousedown="onPetMouseDown"
    @mouseup="onPetMouseUp"
    @contextmenu.prevent="onContextMenu"
  >
    <!-- Pet sprite -->
    <div
      v-if="currentEntry"
      :key="currentEntry.id"
      class="pet-sprite"
      :style="spriteStyle"
      @mousedown="startDrag"
    ></div>

    <!-- Overlay panels -->
    <PetStatusPanel
      v-if="overlay === 'status'"
      :state="state"
      :entry="currentEntry"
      :last-decision-reason="lastTickerReason"
      :llm-enabled="settings.llmEnabled"
      :ticker-interval-ms="tickerInterval"
      :sprite-height="spriteH"
      @close="closeOverlay"
    />
    <PetSettingsPanel
      v-if="overlay === 'settings'"
      :settings="settings"
      :sprite-height="spriteH"
      @close="closeOverlay"
      @save="onSettingsSave"
    />
    <PetChatPanel
      v-if="overlay === 'chat'"
      :sprite-height="spriteH"
      :context-text="chatContextText"
      :pet-speak-message="petSpeakMessage"
      @close="closeOverlay"
      @switch-animation="onChatSwitchAnimation"
    />
    <PetMemoryPanel
      v-if="overlay === 'memory'"
      :sprite-height="spriteH"
      @close="closeOverlay"
    />
  </div>

  <!-- Context menu -->
  <div
    v-if="menuIsOpen"
    class="context-menu"
    :style="menuStyle"
    @contextmenu.prevent
  >
    <template v-for="(item, i) in menuItems" :key="i">
      <div v-if="item.type === 'separator'" class="menu-separator"></div>

      <div
        v-else-if="item.type === 'submenu'"
        class="menu-item submenu-parent"
        :class="{ 'submenu-open': activeSubmenu === item.label }"
        @mousedown.stop
        @click.stop="toggleSubmenu(item.label!)"
      >
        <span>{{ item.label }}</span>
        <span class="submenu-arrow">▶</span>
        <div v-if="activeSubmenu === item.label" class="submenu-dropdown">
          <div
            v-for="(child, j) in item.children"
            :key="j"
            class="menu-item"
            :class="{ current: child.current }"
            @mousedown.stop
            @click="menuOnItemClick(child)"
          >
            {{ child.label }}
            <span v-if="child.current" class="current-dot">●</span>
          </div>
        </div>
      </div>

      <div
        v-else
        class="menu-item"
        @mousedown.stop
        @click="menuOnItemClick(item)"
      >
        {{ item.label }}
        <span v-if="item.current" class="current-dot">●</span>
      </div>
    </template>
  </div>
</template>

<style>
html,
body,
#app {
  margin: 0;
  padding: 0;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: transparent;
}
</style>

<style scoped>
.pet-area {
  cursor: grab;
  user-select: none;
  display: flex;
  align-items: center;
  justify-content: center;
  position: fixed;
  top: 0;
  left: 0;
}
.pet-area:active {
  cursor: grabbing;
}
.pet-sprite {
  background-repeat: no-repeat;
}
.placeholder {
  width: 100vw;
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #666;
  font-size: 14px;
  user-select: none;
}
.context-menu {
  position: fixed;
  background: rgba(255, 255, 255, 0.96);
  border: 1px solid #d0d0d0;
  border-radius: 6px;
  padding: 4px 0;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.18);
  z-index: 100;
  min-width: 130px;
  font-size: 13px;
  user-select: none;
}
.menu-item {
  padding: 6px 12px;
  cursor: pointer;
  white-space: nowrap;
  color: #222;
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.menu-item:hover {
  background: rgba(0, 120, 215, 0.12);
}
.menu-separator {
  height: 1px;
  background: #e0e0e0;
  margin: 4px 0;
}
.submenu-parent {
  position: relative;
}
.submenu-arrow {
  font-size: 10px;
  margin-left: 16px;
  color: #999;
}
.submenu-dropdown {
  position: absolute;
  left: 100%;
  top: -4px;
  background: rgba(255, 255, 255, 0.96);
  border: 1px solid #d0d0d0;
  border-radius: 6px;
  padding: 4px 0;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.18);
  min-width: 100px;
  z-index: 101;
}
.current-dot {
  color: #0078d7;
  font-size: 8px;
  margin-left: 8px;
}
</style>

<style>
@keyframes pet-play {
  from {
    background-position-x: 0;
  }
  to {
    background-position-x: var(--end-x);
  }
}
</style>
