import { ref, type Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { AnimationEntry, AppError } from "../types/pet";

// Spec 01 §3.3 useAnimationRegistry: fetch list once on init, expose
// reactive registry + loading + error. Frontend never hardcodes the list.
export function useAnimationRegistry() {
  const registry: Ref<AnimationEntry[]> = ref([]);
  const isLoading: Ref<boolean> = ref(true);
  const error: Ref<AppError | null> = ref(null);

  async function load() {
    isLoading.value = true;
    error.value = null;
    try {
      registry.value = await invoke<AnimationEntry[]>("list_animations");
    } catch (e) {
      error.value = e as AppError;
    } finally {
      isLoading.value = false;
    }
  }

  load();

  function has(id: string): boolean {
    return registry.value.some((e) => e.id === id);
  }

  function get(id: string): AnimationEntry | undefined {
    return registry.value.find((e) => e.id === id);
  }

  return { registry, isLoading, error, load, has, get };
}
