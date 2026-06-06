<script setup lang="ts">
import type { AnimationState, AnimationEntry } from "../types/pet";

defineProps<{
  state: AnimationState;
  entry: AnimationEntry | undefined;
  lastDecisionReason: string | null;
  llmEnabled: boolean;
  tickerIntervalMs: number;
  spriteHeight: number;
}>();

const emit = defineEmits<{
  close: [];
}>();
</script>

<template>
  <div class="overlay-mask" @mousedown="emit('close')">
    <div class="overlay-panel" :style="{ top: spriteHeight + 'px' }" @mousedown.stop>
      <div class="panel-header">
        <span>宠物状态</span>
        <button class="close-btn" @click="emit('close')">✕</button>
      </div>
      <div class="panel-body">
        <div class="stat-row">
          <span class="label">当前动画</span>
          <span class="value">{{ entry?.id ?? state.current }}</span>
        </div>
        <div class="stat-row">
          <span class="label">阶段</span>
          <span class="value">{{ state.phase }}</span>
        </div>
        <div class="stat-row">
          <span class="label">循环次数</span>
          <span class="value">{{ state.iteration }}</span>
        </div>
        <div class="stat-row" v-if="lastDecisionReason">
          <span class="label">上次决策理由</span>
          <span class="value reason">{{ lastDecisionReason }}</span>
        </div>
        <div class="stat-row">
          <span class="label">LLM 状态</span>
          <span class="value" :class="llmEnabled ? 'on' : 'off'">
            {{ llmEnabled ? '已连接' : '已关闭' }}
          </span>
        </div>
        <div class="stat-row">
          <span class="label">Ticker 间隔</span>
          <span class="value">{{ (tickerIntervalMs / 1000).toFixed(1) }}s</span>
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
  width: 220px;
  background: rgba(255, 255, 255, 0.96);
  border: 1px solid #d0d0d0;
  border-radius: 8px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
  font-size: 13px;
  user-select: none;
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
.panel-body {
  padding: 8px 12px;
}
.stat-row {
  display: flex;
  justify-content: space-between;
  padding: 4px 0;
}
.label {
  color: #888;
}
.value {
  color: #333;
  font-weight: 500;
}
.value.on {
  color: #22a65e;
}
.value.off {
  color: #e05d44;
}
.value.reason {
  max-width: 120px;
  text-align: right;
  font-style: italic;
}
</style>
