<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useAnimationRegistry } from "./composables/useAnimationRegistry";
import { useAnimationStateMachine } from "./composables/useAnimationStateMachine";
import { usePetSettings } from "./composables/usePetSettings";
import { usePetEvents } from "./composables/usePetEvents";
import type { AnimationEntry, AnimationId, AgentResult } from "./types/pet";
import PetStatusPanel from "./components/PetStatusPanel.vue";
import PetSettingsPanel from "./components/PetSettingsPanel.vue";
import PetChatPanel from "./components/PetChatPanel.vue";
import PetMemoryPanel from "./components/PetMemoryPanel.vue";

// --- Constants ---
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

// Forward reference for pushEvent — resolved after usePetEvents is created.
let pushEventFn: ((event: import("./types/pet").PetEvent) => void) | null = null;

const { state, dispatch, tickerInterval, lastTickerReason, lastInteractionAt, applyAgentResult, interruptWait } =
  useAnimationStateMachine({
    llmEnabled: computed(() => settings.value.llmEnabled),
    petPersonality: computed(() => settings.value.petPersonality),
    petName: computed(() => settings.value.petName),
    llmApiEndpoint: computed(() => settings.value.apiEndpoint),
    llmApiKey: computed(() => settings.value.apiKey),
    llmModel: computed(() => settings.value.model),
    onDecision,
    onSpeak,
    onTickerTick: () => {
      pushEventFn?.({ type: "timer_tick", timestamp: Date.now() });
    },
    onPushEvent: (event) => {
      pushEventFn?.(event);
    },
  });

// --- Pet Events (Agent Loop) ---
function onAgentDecision(result: AgentResult) {
  console.log(`[PetAgent] decision: ${result.decision.action}, stepsUsed: ${result.stepsUsed}`);
  applyAgentResult(result);
}

const { pushEvent } = usePetEvents({
  llmEnabled: computed(() => settings.value.llmEnabled),
  petPersonality: computed(() => settings.value.petPersonality),
  petName: computed(() => settings.value.petName),
  llmApiEndpoint: computed(() => settings.value.apiEndpoint),
  llmApiKey: computed(() => settings.value.apiKey),
  llmModel: computed(() => settings.value.model),
  tickerInterval,
  proactiveIntervalMs: computed(() => settings.value.proactiveIntervalMs),
  minSilenceMs: computed(() => settings.value.minSilenceMs),
  getAnimationState: () => state.value,
  getLastInteractionAt: () => lastInteractionAt.value,
  onDecision: onAgentDecision,
});
pushEventFn = pushEvent;

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

// --- Native context menu (Tauri menu, not in-webview <div>) ---
// Menu ids are defined in src-tauri/src/lib.rs::show_context_menu.
const CTX_MENU_EVENT = "context-menu-click";
let unlistenMenu: UnlistenFn | null = null;

function handleMenuClick(id: string) {
  switch (id) {
    case "ctx.status":
      showOverlay("status");
      break;
    case "ctx.settings":
      showOverlay("settings");
      break;
    case "ctx.chat":
      showOverlay("chat");
      break;
    case "ctx.memory":
      showOverlay("memory");
      break;
    case "ctx.exit":
      getCurrentWindow()
        .close()
        .catch((e) => console.error("[PetExit] close failed:", e));
      break;
    default:
      console.warn(`[PetMenu] unknown menu id: ${id}`);
  }
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
  } else if (dx > CLICK_MAX_PX || dy > CLICK_MAX_PX) {
    // Drag ended
    onPetDragEnd();
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

function onContextMenu(e: MouseEvent) {
  e.preventDefault();
  closeOverlay();
  invoke("show_context_menu").catch((err) => {
    console.error("[PetMenu] show_context_menu failed:", err);
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

// Sprite dimensions (base window size when no overlay is open).
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

const needWindowW = computed(() => {
  let w = spriteW.value;
  if (overlay.value) {
    w = Math.max(w, OVERLAY_EST_W);
  }
  return Math.ceil(w);
});

const needWindowH = computed(() => {
  let h = spriteH.value;
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

  // Window focus/blur events for Agent Loop
  window.addEventListener("focus", onFocusChange);
  window.addEventListener("blur", onFocusChange);

  // Native context menu event listener (Rust → frontend dispatch).
  unlistenMenu = await listen<string>(CTX_MENU_EVENT, (e) => {
    handleMenuClick(e.payload);
  });
});

onUnmounted(() => {
  window.removeEventListener("focus", onFocusChange);
  window.removeEventListener("blur", onFocusChange);
  unlistenMenu?.();
  unlistenMenu = null;
});

function onFocusChange() {
  const focused = document.hasFocus();
  pushEvent({ type: "window_focus_changed", focused, timestamp: Date.now() });
}

function onSpriteAnimationEnd(_e: AnimationEvent) {
  const entry = currentEntry.value;
  if (entry && entry.loopMode === "once") {
    pushEvent({
      type: "animation_completed",
      animationId: entry.id,
      timestamp: Date.now(),
    });
  }
}

function onPetClick() {
  pushEvent({
    type: "user_interaction",
    interaction: "click",
    timestamp: Date.now(),
  });
}

function onPetDragEnd() {
  pushEvent({
    type: "user_interaction",
    interaction: "drag_end",
    timestamp: Date.now(),
  });
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

function onChatMessageSent() {
  interruptWait();
  pushEvent({ type: "user_interaction", interaction: "chat", timestamp: Date.now() });
}
</script>

<template>
  <div v-if="error" class="placeholder">宠物暂时打瞌睡了</div>
  <div
    v-else
    class="pet-area"
    @mousedown="onPetMouseDown"
    @mouseup="onPetMouseUp"
    @click="onPetClick"
    @contextmenu.prevent="onContextMenu"
  >
    <!-- Pet sprite -->
    <div
      v-if="currentEntry"
      :key="currentEntry.id"
      class="pet-sprite"
      :style="spriteStyle"
      @mousedown="startDrag"
      @animationend="onSpriteAnimationEnd"
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
      @chat-sent="onChatMessageSent"
    />
    <PetMemoryPanel
      v-if="overlay === 'memory'"
      :sprite-height="spriteH"
      @close="closeOverlay"
    />
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
