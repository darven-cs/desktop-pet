import { ref } from "vue";

export interface ChatMessage {
  role: "user" | "assistant";
  content: string;
  timestamp: number;
  animationTriggered?: string | null;
}

export interface ChatResponse {
  message: string;
  animation?: string | null;
}

export function usePetChat() {
  const messages = ref<ChatMessage[]>([]);
  const isLoading = ref(false);
  const error = ref<string | null>(null);

  async function sendMessage(
    text: string,
    contextText?: string | null,
  ): Promise<ChatResponse | null> {
    isLoading.value = true;
    error.value = null;
    try {
      // 1. Add user message locally.
      const userMsg: ChatMessage = {
        role: "user",
        content: text,
        timestamp: Date.now(),
      };
      messages.value = [...messages.value, userMsg];

      // 2. Call backend.
      const { invoke } = await import("@tauri-apps/api/core");
      const response: ChatResponse = await invoke("send_message", {
        context: buildContext(),
        text,
        contextText: contextText ?? null,
      });

      // 3. Add pet response.
      const petMsg: ChatMessage = {
        role: "assistant",
        content: response.message,
        timestamp: Date.now(),
        animationTriggered: response.animation ?? null,
      };
      messages.value = [...messages.value, petMsg];

      return response;
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      error.value = msg;
      console.error("[PetChat] sendMessage failed:", msg);
      return null;
    } finally {
      isLoading.value = false;
    }
  }

  function clearMessages() {
    messages.value = [];
    error.value = null;
  }

  return {
    messages,
    isLoading,
    error,
    sendMessage,
    clearMessages,
  };
}

/** Build a minimal DecisionContext for chat commands. */
function buildContext(): Record<string, unknown> {
  return {
    currentState: { phase: "playing", current: "touch_nose", iteration: 0 },
    lastInteractionAt: Date.now(),
    tickerIntervalMs: 30000,
    timeOfDay: new Date().toLocaleTimeString("zh-CN", {
      hour: "2-digit",
      minute: "2-digit",
    }),
    llmEnabled: getSettings().llmEnabled,
    petPersonality: getSettings().petPersonality || undefined,
    petName: getSettings().petName || undefined,
    llmApiEndpoint: getSettings().apiEndpoint || undefined,
    llmApiKey: getSettings().apiKey || undefined,
    llmModel: getSettings().model || undefined,
  };
}

function getSettings(): {
  llmEnabled: boolean;
  petPersonality: string;
  petName: string;
  apiEndpoint: string;
  apiKey: string;
  model: string;
} {
  try {
    const raw = localStorage.getItem("pet-settings");
    if (raw) return JSON.parse(raw);
  } catch {
    // ignore
  }
  return {
    llmEnabled: true,
    petPersonality: "",
    petName: "",
    apiEndpoint: "",
    apiKey: "",
    model: "gpt-4o-mini",
  };
}
