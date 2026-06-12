<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { usePetChat } from "../composables/usePetChat";
import type { AnimationId } from "../types/pet";

const props = defineProps<{
  spriteHeight: number;
  selectedText: string;
}>();

const emit = defineEmits<{
  close: [];
  switchAnimation: [id: AnimationId];
  chatSent: [];
}>();

type Tab = "translate" | "chat";
const activeTab = ref<Tab>("translate");

const { messages, isLoading, error, sendMessage } = usePetChat();
const translatedText = ref<string | null>(null);
const translateError = ref<string | null>(null);
const isTranslating = ref(false);

function getSettings(): { apiEndpoint: string; apiKey: string; model: string } {
  try {
    const raw = localStorage.getItem("pet-settings");
    if (raw) return JSON.parse(raw);
  } catch {
    // ignore
  }
  return { apiEndpoint: "", apiKey: "", model: "" };
}

async function doTranslate() {
  if (!props.selectedText || isTranslating.value) return;
  isTranslating.value = true;
  translateError.value = null;

  try {
    const settings = getSettings();
    const result: string = await invoke("translate_text", {
      text: props.selectedText,
      fromLang: "en",
      toLang: "zh",
      apiKey: settings.apiKey || null,
      endpoint: settings.apiEndpoint || null,
      model: settings.model || null,
    });
    translatedText.value = result;
  } catch (e: any) {
    // Try to extract error message from Tauri error object
    let errorMsg = String(e);
    if (e && typeof e === 'object' && e.message) {
      errorMsg = e.message;
    } else if (e && typeof e === 'object' && e.error) {
      const err = e.error;
      if (typeof err === 'object' && err.message) {
        errorMsg = err.message;
      } else {
        errorMsg = JSON.stringify(err);
      }
    }
    translateError.value = errorMsg;
  } finally {
    isTranslating.value = false;
  }
}

async function onSendChat(text: string) {
  emit("chatSent");
  const resp = await sendMessage(text, props.selectedText);
  if (resp?.animation) {
    emit("switchAnimation", resp.animation as AnimationId);
  }
}

const inputText = ref("");
const messagesEnd = ref<HTMLElement | null>(null);

async function onSend() {
  const text = inputText.value.trim();
  if (!text || isLoading.value) return;
  inputText.value = "";
  await onSendChat(text);
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    onSend();
  }
}

onMounted(() => {
  doTranslate();
});
</script>

<template>
  <div class="translate-overlay" @mousedown.stop>
    <div class="panel-header">
      <div class="tab-bar">
        <button
          class="tab-btn"
          :class="{ active: activeTab === 'translate' }"
          @click="activeTab = 'translate'"
        >
          翻译
        </button>
        <button
          class="tab-btn"
          :class="{ active: activeTab === 'chat' }"
          @click="activeTab = 'chat'"
        >
          对话
        </button>
      </div>
      <button class="close-btn" @click="emit('close')">✕</button>
    </div>

    <!-- Translate Tab -->
    <div v-if="activeTab === 'translate'" class="tab-content">
      <div class="source-text">
        <div class="label">原文</div>
        <div class="text-box">{{ selectedText }}</div>
      </div>

      <div class="result-area">
        <div class="label">翻译</div>
        <div v-if="isTranslating" class="loading">翻译中...</div>
        <div v-else-if="translateError" class="error">翻译失败：{{ translateError }}</div>
        <div v-else-if="translatedText" class="text-box result">{{ translatedText }}</div>
        <div v-else class="loading">等待翻译...</div>
      </div>
    </div>

    <!-- Chat Tab -->
    <div v-if="activeTab === 'chat'" class="tab-content chat-tab">
      <div class="context-hint">
        <span class="label">选中文本：</span>{{ selectedText }}
      </div>

      <div class="chat-messages">
        <div
          v-for="(msg, i) in messages"
          :key="i"
          class="chat-msg"
          :class="msg.role"
        >
          <div class="msg-bubble">
            <div class="msg-text">{{ msg.content }}</div>
          </div>
        </div>
        <div v-if="isLoading" class="chat-typing">
          <span class="typing-dot">●</span>
          <span class="typing-dot">●</span>
          <span class="typing-dot">●</span>
        </div>
        <div v-if="error" class="chat-error">{{ error }}</div>
        <div ref="messagesEnd"></div>
      </div>

      <div class="chat-input-row">
        <input
          v-model="inputText"
          type="text"
          class="chat-input"
          placeholder="继续对话..."
          :disabled="isLoading"
          @keydown="onKeydown"
        />
        <button
          class="chat-send-btn"
          :disabled="isLoading || !inputText.trim()"
          @click="onSend"
        >
          发送
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.translate-overlay {
  position: fixed;
  top: v-bind("props.spriteHeight + 'px'");
  left: 0;
  width: 300px;
  background: rgba(255, 255, 255, 0.97);
  border: 1px solid #d0d0d0;
  border-radius: 8px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.18);
  z-index: 50;
  font-size: 13px;
  overflow: hidden;
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 10px;
  border-bottom: 1px solid #e0e0e0;
}

.tab-bar {
  display: flex;
  gap: 4px;
}

.tab-btn {
  background: none;
  border: none;
  padding: 4px 12px;
  font-size: 13px;
  cursor: pointer;
  color: #666;
  border-radius: 4px;
}

.tab-btn.active {
  background: #0078d7;
  color: #fff;
}

.close-btn {
  background: none;
  border: none;
  font-size: 14px;
  cursor: pointer;
  color: #999;
  padding: 2px 6px;
  border-radius: 4px;
}

.close-btn:hover {
  background: #f0f0f0;
  color: #333;
}

.tab-content {
  padding: 10px 12px;
}

.label {
  font-size: 11px;
  color: #888;
  margin-bottom: 4px;
}

.text-box {
  background: #f5f5f5;
  border-radius: 6px;
  padding: 8px 10px;
  line-height: 1.5;
  word-break: break-word;
  max-height: 120px;
  overflow-y: auto;
}

.text-box.result {
  background: #e8f4fd;
  color: #1a5f9e;
}

.source-text {
  margin-bottom: 10px;
}

.result-area {
  margin-top: 8px;
}

.loading {
  color: #888;
  font-style: italic;
  padding: 4px 0;
}

.error {
  color: #d32f2f;
  font-size: 12px;
  padding: 4px 0;
}

/* Chat tab */
.chat-tab {
  display: flex;
  flex-direction: column;
  max-height: 300px;
}

.context-hint {
  font-size: 11px;
  color: #666;
  background: #f0f0f0;
  padding: 4px 8px;
  border-radius: 4px;
  margin-bottom: 8px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.chat-messages {
  flex: 1;
  overflow-y: auto;
  min-height: 60px;
  max-height: 180px;
}

.chat-msg {
  margin-bottom: 6px;
}

.msg-bubble {
  max-width: 85%;
  padding: 5px 8px;
  border-radius: 8px;
  line-height: 1.4;
}

.chat-msg.user .msg-bubble {
  background: #0078d7;
  color: #fff;
  margin-left: auto;
  border-bottom-right-radius: 3px;
}

.chat-msg.assistant .msg-bubble {
  background: #f0f0f0;
  color: #222;
  margin-right: auto;
  border-bottom-left-radius: 3px;
}

.msg-text {
  word-break: break-word;
}

.chat-typing {
  padding: 4px 8px;
  color: #999;
}

.typing-dot {
  animation: blink 1.4s infinite;
  margin-right: 2px;
}

.typing-dot:nth-child(2) { animation-delay: 0.2s; }
.typing-dot:nth-child(3) { animation-delay: 0.4s; }

@keyframes blink {
  0%, 60%, 100% { opacity: 0.2; }
  30% { opacity: 1; }
}

.chat-error {
  color: #d32f2f;
  font-size: 12px;
  padding: 4px 0;
}

.chat-input-row {
  display: flex;
  gap: 6px;
  padding-top: 8px;
  border-top: 1px solid #e0e0e0;
}

.chat-input {
  flex: 1;
  padding: 5px 8px;
  border: 1px solid #d0d0d0;
  border-radius: 12px;
  font-size: 12px;
  outline: none;
}

.chat-input:focus {
  border-color: #0078d7;
}

.chat-send-btn {
  background: #0078d7;
  color: #fff;
  border: none;
  border-radius: 12px;
  padding: 5px 12px;
  font-size: 12px;
  cursor: pointer;
}

.chat-send-btn:disabled {
  background: #ccc;
  cursor: not-allowed;
}
</style>