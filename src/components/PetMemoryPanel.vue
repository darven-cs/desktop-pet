<script setup lang="ts">
import { ref, onMounted } from "vue";

defineProps<{
  spriteHeight: number;
}>();

const emit = defineEmits<{
  close: [];
}>();

interface MemoryEntry {
  id: string;
  timestamp: number;
  kind: string;
  content: string;
  importance: number;
  metadata?: Record<string, unknown>;
}

const memories = ref<MemoryEntry[]>([]);
const filter = ref<string>("");
const loading = ref(false);
const error = ref<string | null>(null);

const kindLabels: Record<string, string> = {
  observation: "观察",
  interaction: "交互",
  decision: "决策",
  conversation: "对话",
  reflection: "反思",
};

const kindOptions = [
  { value: "", label: "全部" },
  { value: "conversation", label: "对话" },
  { value: "decision", label: "决策" },
  { value: "interaction", label: "交互" },
  { value: "observation", label: "观察" },
  { value: "reflection", label: "反思" },
];

function formatTime(ts: number): string {
  return new Date(ts).toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function importanceStars(v: number): string {
  const n = Math.round(v * 5);
  return "★".repeat(n) + "☆".repeat(5 - n);
}

async function loadMemories() {
  loading.value = true;
  error.value = null;
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    const result = await invoke<MemoryEntry[]>("get_memories", {
      kind: filter.value || null,
      limit: 50,
    });
    memories.value = result;
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    error.value = msg;
    console.error("[PetMemory] load failed:", msg);
  } finally {
    loading.value = false;
  }
}

onMounted(loadMemories);

async function onFilterChange(kind: string) {
  filter.value = kind;
  await loadMemories();
}
</script>

<template>
  <div class="overlay-mask" @mousedown="emit('close')">
    <div class="overlay-panel" :style="{ top: spriteHeight + 'px' }" @mousedown.stop>
      <div class="panel-header">
        <span>宠物记忆</span>
        <button class="close-btn" @click="emit('close')">✕</button>
      </div>

      <div class="memory-filter">
        <button
          v-for="opt in kindOptions"
          :key="opt.value"
          class="filter-btn"
          :class="{ active: filter === opt.value }"
          @click="onFilterChange(opt.value)"
        >
          {{ opt.label }}
        </button>
      </div>

      <div class="memory-list">
        <div v-if="loading" class="memory-placeholder">加载中...</div>
        <div v-else-if="error" class="memory-placeholder error">加载失败：{{ error }}</div>
        <div v-else-if="!memories.length" class="memory-placeholder">暂无记忆</div>

        <div v-for="mem in memories" :key="mem.id" class="memory-entry">
          <div class="memory-kind">{{ kindLabels[mem.kind] ?? mem.kind }}</div>
          <div class="memory-content">{{ mem.content }}</div>
          <div class="memory-foot">
            <span class="memory-time">{{ formatTime(mem.timestamp) }}</span>
            <span class="memory-stars">{{ importanceStars(mem.importance) }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay-mask {
  position: fixed;
  inset: 0;
  z-index: 99;
}
.overlay-panel {
  position: absolute;
  left: 0;
  margin-top: 4px;
  width: 280px;
  max-height: 380px;
  background: rgba(255, 255, 255, 0.96);
  border: 1px solid #d0d0d0;
  border-radius: 8px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
  font-size: 13px;
  user-select: none;
  display: flex;
  flex-direction: column;
}
.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  border-bottom: 1px solid #eee;
  font-weight: 600;
}
.close-btn {
  background: none;
  border: none;
  cursor: pointer;
  font-size: 14px;
  color: #999;
  padding: 0 2px;
}
.close-btn:hover {
  color: #333;
}
.memory-filter {
  display: flex;
  gap: 4px;
  padding: 6px 12px;
  border-bottom: 1px solid #eee;
  flex-wrap: wrap;
}
.filter-btn {
  padding: 2px 8px;
  border: 1px solid #d0d0d0;
  border-radius: 10px;
  background: #fff;
  font-size: 11px;
  cursor: pointer;
  color: #666;
}
.filter-btn.active {
  background: #0078d7;
  color: #fff;
  border-color: #0078d7;
}
.memory-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px 12px;
  max-height: 260px;
}
.memory-placeholder {
  text-align: center;
  color: #999;
  padding: 20px 0;
  font-size: 12px;
}
.memory-placeholder.error {
  color: #d32f2f;
}
.memory-entry {
  padding: 6px 0;
  border-bottom: 1px solid #f5f5f5;
}
.memory-entry:last-child {
  border-bottom: none;
}
.memory-kind {
  font-size: 10px;
  color: #0078d7;
  margin-bottom: 2px;
}
.memory-content {
  color: #333;
  line-height: 1.4;
  word-break: break-word;
}
.memory-foot {
  display: flex;
  justify-content: space-between;
  margin-top: 2px;
}
.memory-time {
  font-size: 10px;
  color: #999;
}
.memory-stars {
  font-size: 8px;
  color: #ff9500;
}
</style>
