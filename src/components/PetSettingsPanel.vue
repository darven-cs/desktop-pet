<script setup lang="ts">
import { ref } from "vue";
import type { PetSettings } from "../composables/usePetSettings";

const props = defineProps<{
  settings: PetSettings;
  spriteHeight: number;
}>();

const emit = defineEmits<{
  close: [];
  save: [settings: PetSettings];
}>();

const llmEnabled = ref(props.settings.llmEnabled);
const apiEndpoint = ref(props.settings.apiEndpoint);
const apiKey = ref(props.settings.apiKey);
const model = ref(props.settings.model);
const petPersonality = ref(props.settings.petPersonality);
const petName = ref(props.settings.petName);
const tickerIntervalMs = ref(props.settings.tickerIntervalMs);

function onSubmit() {
  emit("save", {
    llmEnabled: llmEnabled.value,
    apiEndpoint: apiEndpoint.value,
    apiKey: apiKey.value,
    model: model.value,
    petPersonality: petPersonality.value,
    petName: petName.value,
    tickerIntervalMs: tickerIntervalMs.value,
  });
  emit("close");
}
</script>

<template>
  <div class="overlay-mask" @mousedown="emit('close')">
    <div class="overlay-panel" :style="{ top: spriteHeight + 'px' }" @mousedown.stop>
      <div class="panel-header">
        <span>宠物设定</span>
        <button class="close-btn" @click="emit('close')">✕</button>
      </div>
      <div class="panel-body">
        <label class="setting-row">
          <span>LLM 开关</span>
          <input type="checkbox" v-model="llmEnabled" />
        </label>

        <label class="setting-col">
          <span>API 地址</span>
          <input
            type="text"
            v-model="apiEndpoint"
            class="text-input wide"
            placeholder="https://api.openai.com/v1/chat/completions"
          />
        </label>

        <label class="setting-col">
          <span>API Key</span>
          <input
            type="password"
            v-model="apiKey"
            class="text-input wide"
            placeholder="sk-..."
          />
        </label>

        <label class="setting-row">
          <span>模型</span>
          <input
            type="text"
            v-model="model"
            class="text-input"
            placeholder="gpt-4o-mini"
          />
        </label>

        <label class="setting-row">
          <span>Ticker 间隔 (ms)</span>
          <input
            type="number"
            v-model.number="tickerIntervalMs"
            class="num-input"
            min="1000"
            step="1000"
          />
        </label>

        <label class="setting-row">
          <span>宠物名字</span>
          <input
            type="text"
            v-model="petName"
            class="text-input"
            placeholder="给宠物起个名字"
          />
        </label>

        <label class="setting-col">
          <span>宠物人格</span>
          <textarea
            v-model="petPersonality"
            class="text-area"
            rows="3"
            placeholder="好奇心旺盛、偶尔偷懒、喜欢吸引用户注意"
          ></textarea>
        </label>

        <button class="save-btn" @click="onSubmit">保存</button>
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
  padding: 10px 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.setting-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.setting-col {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.text-input {
  width: 140px;
  padding: 2px 6px;
  border: 1px solid #ccc;
  border-radius: 4px;
  font-size: 12px;
}
.text-input.wide {
  width: 100%;
  box-sizing: border-box;
}
.num-input {
  width: 80px;
  padding: 2px 6px;
  border: 1px solid #ccc;
  border-radius: 4px;
  font-size: 12px;
}
.text-area {
  padding: 4px 6px;
  border: 1px solid #ccc;
  border-radius: 4px;
  font-size: 12px;
  resize: vertical;
  font-family: inherit;
}
.save-btn {
  align-self: flex-end;
  padding: 4px 16px;
  background: #0078d7;
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
}
.save-btn:hover {
  background: #106ebe;
}
</style>
