<script setup lang="ts">
import { ref, nextTick, watch, onMounted } from "vue";
import { usePetChat, type ChatMessage } from "../composables/usePetChat";
import type { AnimationId } from "../types/pet";

const props = defineProps<{
  spriteHeight: number;
  contextText?: string | null;
  petSpeakMessage?: { message: string; animation: string | null } | null;
}>();

const emit = defineEmits<{
  close: [];
  switchAnimation: [id: AnimationId];
  chatSent: [];
}>();

const { messages, isLoading, error, sendMessage } = usePetChat();

const inputText = ref("");
const inputRef = ref<HTMLInputElement | null>(null);
const messagesEnd = ref<HTMLElement | null>(null);

function appendProactiveMessage(msg: { message: string; animation: string | null }) {
  const petMsg: ChatMessage = {
    role: "assistant",
    content: msg.message,
    timestamp: Date.now(),
    animationTriggered: msg.animation ?? null,
  };
  messages.value = [...messages.value, petMsg];
  if (msg.animation) {
    emit("switchAnimation", msg.animation as AnimationId);
  }
}

// If contextText was provided, auto-send or prefill.
// If petSpeakMessage was provided (LLM主动对话), show it directly.
onMounted(async () => {
  if (props.petSpeakMessage) {
    appendProactiveMessage(props.petSpeakMessage);
    return;
  }

  if (props.contextText) {
    inputText.value = props.contextText;
    const resp = await sendMessage(props.contextText, props.contextText);
    if (resp?.animation) {
      emit("switchAnimation", resp.animation as AnimationId);
    }
  }
});

// Watch for subsequent proactive messages while the panel is already open.
watch(
  () => props.petSpeakMessage,
  (newMsg) => {
    if (newMsg) {
      appendProactiveMessage(newMsg);
    }
  },
);

watch(messages, async () => {
  await nextTick();
  messagesEnd.value?.scrollIntoView({ behavior: "smooth" });
});

async function onSend() {
  const text = inputText.value.trim();
  if (!text || isLoading.value) return;
  inputText.value = "";
  emit("chatSent");

  const resp = await sendMessage(text);
  if (resp?.animation) {
    emit("switchAnimation", resp.animation as AnimationId);
  }
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    onSend();
  }
}

function formatTime(ts: number): string {
  return new Date(ts).toLocaleTimeString("zh-CN", {
    hour: "2-digit",
    minute: "2-digit",
  });
}
</script>

<template>
  <div class="chat-overlay" @mousedown.stop>
    <div class="chat-header">
      <span>宠物对话</span>
      <button class="chat-close-btn" @click="emit('close')">✕</button>
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
          <div class="msg-meta">
            <span class="msg-time">{{ formatTime(msg.timestamp) }}</span>
            <span v-if="msg.animationTriggered" class="msg-anim-tag">
              🎬 {{ msg.animationTriggered }}
            </span>
          </div>
        </div>
      </div>

      <div v-if="isLoading" class="chat-typing">
        <span class="typing-dot">●</span>
        <span class="typing-dot">●</span>
        <span class="typing-dot">●</span>
      </div>

      <div v-if="error" class="chat-error">发送失败：{{ error }}</div>

      <div ref="messagesEnd"></div>
    </div>

    <div class="chat-input-row">
      <input
        ref="inputRef"
        v-model="inputText"
        type="text"
        class="chat-input"
        placeholder="对宠物说点什么..."
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
</template>

<style scoped>
.chat-overlay {
  position: fixed;
  top: v-bind("props.spriteHeight + 'px'");
  left: 0;
  width: 300px;
  max-height: 360px;
  background: rgba(255, 255, 255, 0.97);
  border: 1px solid #d0d0d0;
  border-radius: 8px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.18);
  display: flex;
  flex-direction: column;
  z-index: 50;
  font-size: 13px;
}

.chat-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  border-bottom: 1px solid #e0e0e0;
  font-weight: 600;
  color: #333;
}

.chat-close-btn {
  background: none;
  border: none;
  font-size: 16px;
  cursor: pointer;
  color: #999;
  padding: 2px 6px;
  border-radius: 4px;
}

.chat-close-btn:hover {
  background: #f0f0f0;
  color: #333;
}

.chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 10px 12px;
  min-height: 100px;
  max-height: 240px;
}

.chat-msg {
  margin-bottom: 8px;
}

.msg-bubble {
  max-width: 85%;
  padding: 6px 10px;
  border-radius: 10px;
  line-height: 1.5;
}

.chat-msg.user .msg-bubble {
  background: #0078d7;
  color: #fff;
  margin-left: auto;
  border-bottom-right-radius: 4px;
}

.chat-msg.assistant .msg-bubble {
  background: #f0f0f0;
  color: #222;
  margin-right: auto;
  border-bottom-left-radius: 4px;
}

.msg-text {
  word-break: break-word;
}

.msg-meta {
  display: flex;
  gap: 6px;
  font-size: 10px;
  margin-top: 3px;
  opacity: 0.7;
}

.msg-anim-tag {
  color: #ff9500;
}

.chat-typing {
  padding: 8px 12px;
  color: #999;
}

.typing-dot {
  animation: blink 1.4s infinite;
  margin-right: 2px;
}

.typing-dot:nth-child(2) {
  animation-delay: 0.2s;
}

.typing-dot:nth-child(3) {
  animation-delay: 0.4s;
}

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
  padding: 8px 12px;
  border-top: 1px solid #e0e0e0;
}

.chat-input {
  flex: 1;
  padding: 6px 10px;
  border: 1px solid #d0d0d0;
  border-radius: 16px;
  font-size: 13px;
  outline: none;
}

.chat-input:focus {
  border-color: #0078d7;
}

.chat-send-btn {
  background: #0078d7;
  color: #fff;
  border: none;
  border-radius: 16px;
  padding: 6px 14px;
  font-size: 13px;
  cursor: pointer;
}

.chat-send-btn:disabled {
  background: #ccc;
  cursor: not-allowed;
}
</style>
