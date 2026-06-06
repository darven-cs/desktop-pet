<script setup lang="ts">
import { onMounted } from "vue";

const FRAME_COUNT = 28;
const FRAME_WIDTH = 240;
const FRAME_HEIGHT = 240;
const FRAME_DURATION = 1000 / 25; // 25fps = 40ms per frame

onMounted(() => {
  const pet = document.querySelector(".pet-sprite") as HTMLElement;
  if (!pet) return;

  pet.style.backgroundImage = "url('/sprites/touch_nose_sheet.png')";
  pet.style.backgroundSize = `${FRAME_COUNT * FRAME_WIDTH}px ${FRAME_HEIGHT}px`;
});
</script>

<template>
  <div
    class="pet-area"
    @mousedown="startDrag"
  >
    <div class="pet-sprite"></div>
  </div>
</template>

<script lang="ts">
export default {
  methods: {
    startDrag(e: MouseEvent) {
      if (e.button === 0) {
        import("@tauri-apps/api/window").then(({ getCurrentWindow }) => {
          getCurrentWindow().startDragging();
        });
      }
    },
  },
};
</script>

<style>
html, body {
  margin: 0;
  padding: 0;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: transparent;
}

#app {
  width: 100%;
  height: 100%;
  background: transparent;
}
</style>

<style scoped>
.pet-area {
  width: 240px;
  height: 240px;
  cursor: grab;
  user-select: none;
  display: flex;
  align-items: center;
  justify-content: center;
}

.pet-area:active {
  cursor: grabbing;
}

.pet-sprite {
  width: 240px;
  height: 240px;
  background-repeat: no-repeat;
  animation: play 1120ms steps(28) infinite;
}

@keyframes play {
  from {
    background-position-x: 0;
  }
  to {
    background-position-x: -6720px;
  }
}
</style>
