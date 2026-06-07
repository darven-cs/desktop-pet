// Mirrored from src-tauri/src/types.rs (R7: Rust is the source of truth).
// Keep these in sync when Rust structs change. 02 spec will switch to
// ts-rs for auto-generation.

export type AnimationId =
  | "touch_nose"
  | "think"
  | "poop"
  | "shush"
  | "thumbs_up"
  | "nervous"
  | "sleep"
  | "peek"
  | "knead"
  | "heartbeat"
  | "cloud";

export interface AnimationEntry {
  id: AnimationId;
  sheetPath: string;
  frameCount: number;
  frameWidth: number;
  frameHeight: number;
  fps: number;
  loopMode: "infinite" | "once";
}

export type Phase = "playing" | "idle" | "transitioning";

export interface AnimationState {
  phase: Phase;
  current: AnimationId;
  iteration: number;
  transition?: { from: AnimationId; to: AnimationId; progress: number };
}

export type Decision =
  | { action: "stay" }
  | { action: "switch"; to: AnimationId; reason?: string }
  | { action: "speak"; message: string; animation?: string }
  | { action: "enter_idle" }
  | { action: "exit_idle" }
  | { action: "wait"; durationMs: number; reason?: string }
  | { action: "set_reminder"; message: string; delaySeconds: number };

export type PetEvent =
  | { type: "timer_tick"; timestamp: number }
  | { type: "user_interaction"; interaction: "click" | "drag_end" | "double_click" | "chat"; timestamp: number }
  | { type: "animation_completed"; animationId: string; timestamp: number }
  | { type: "window_focus_changed"; focused: boolean; timestamp: number }
  | { type: "reminder_triggered"; message: string; timestamp: number };

export interface AgentResult {
  decision: Decision;
  stepsUsed: number;
  toolCallsMade: string[];
}

export interface DecisionContext {
  currentState: AnimationState;
  lastInteractionAt: number;
  tickerIntervalMs: number;
  // 02: optional fields for LLM context
  timeOfDay?: string;
  recentHistory?: string[];
  // 02: runtime-overridable pet settings
  llmEnabled?: boolean;
  petPersonality?: string;
  petName?: string;
  memoryContext?: string;
  // 02: runtime-overridable API config (overrides .env)
  llmApiEndpoint?: string;
  llmApiKey?: string;
  llmModel?: string;
  // 04: frontend-provided compacted events summary
  eventsSummary?: string;
}

export type AppErrorCode =
  | "E_ANIM_NOT_FOUND"
  | "E_FRAMES_MISSING"
  | "E_INVALID_CONTEXT"
  | "E_INTERNAL";

export interface AppError {
  code: AppErrorCode;
  message: string;
}

// --- Chat & Memory types (sync with src-tauri/src/memory/types.rs) ---

export type MemoryKind =
  | "observation"
  | "interaction"
  | "decision"
  | "conversation"
  | "reflection";

export interface MemoryEntry {
  id: string;
  timestamp: number;
  kind: MemoryKind;
  content: string;
  importance: number;
  metadata?: Record<string, unknown>;
}

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
