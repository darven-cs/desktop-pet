import { ref, watch, type Ref } from "vue";

export interface PetSettings {
  llmEnabled: boolean;
  petPersonality: string;
  petName: string;
  tickerIntervalMs: number;
  proactiveIntervalMs: number;
  minSilenceMs: number;
  apiEndpoint: string;
  apiKey: string;
  model: string;
}

const LS_KEY = "pet-settings";

const defaults: PetSettings = {
  llmEnabled: true,
  petPersonality: "",
  petName: "",
  tickerIntervalMs: 30000,
  proactiveIntervalMs: 300000,
  minSilenceMs: 120000,
  apiEndpoint: "",
  apiKey: "",
  model: "gpt-4o-mini",
};

function load(): PetSettings {
  try {
    const raw = localStorage.getItem(LS_KEY);
    if (raw) {
      return { ...defaults, ...JSON.parse(raw) };
    }
  } catch {
    // corrupted data → use defaults
  }
  return { ...defaults };
}

function save(s: PetSettings) {
  localStorage.setItem(LS_KEY, JSON.stringify(s));
}

const settings: Ref<PetSettings> = ref(load());
const lastDecisionReason: Ref<string | null> = ref(null);

watch(
  settings,
  (next) => save(next),
  { deep: true },
);

export function usePetSettings() {
  function updateSettings(partial: Partial<PetSettings>) {
    settings.value = { ...settings.value, ...partial };
  }

  return {
    settings,
    lastDecisionReason,
    updateSettings,
  };
}
