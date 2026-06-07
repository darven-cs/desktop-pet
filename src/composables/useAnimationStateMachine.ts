import { ref, watch, readonly, type Ref } from "vue";
import type {
  AgentResult,
  AnimationEntry,
  AnimationId,
  AnimationState,
  AppError,
  Decision,
  DecisionContext,
} from "../types/pet";
import { getDefaultDecider, type Decider } from "../decider";

// Animation history ring buffer (R9: max 5 entries, FIFO).
const MAX_HISTORY = 5;

function pushHistory(buf: string[], id: string): string[] {
  const next = [...buf, id];
  if (next.length > MAX_HISTORY) return next.slice(next.length - MAX_HISTORY);
  return next;
}

// Internal events; external code only sees `dispatch`.
export type StateEvent =
  | { type: "init"; entry: AnimationEntry }
  | {
      type: "switch_to";
      id: string;
      reason?: string;
      source: "dispatch" | "ticker" | "init";
    }
  | { type: "transition_complete" }
  | { type: "enter_idle" }
  | { type: "exit_idle" };

// Pure reducer (AC-F4.7): no side effects, same input → same output.
function reducer(state: AnimationState, event: StateEvent): AnimationState {
  switch (event.type) {
    case "init":
      return {
        phase: "playing",
        current: event.entry.id as AnimationId,
        iteration: 0,
      };
    case "switch_to": {
      // Same id and idle/playing: no-op (AC-F1.3 — sprite does not restart).
      if (state.current === event.id) return state;
      // Mid-transition: ignore concurrent requests.
      if (state.phase === "transitioning") return state;
      return {
        phase: "transitioning",
        current: state.current,
        iteration: state.iteration,
        transition: {
          from: state.current,
          to: event.id as AnimationId,
          progress: 0,
        },
      };
    }
    case "transition_complete": {
      if (state.phase !== "transitioning" || !state.transition) return state;
      return {
        phase: "playing",
        current: state.transition.to,
        iteration: state.iteration + 1,
      };
    }
    case "enter_idle":
      if (state.phase === "idle") return state;
      return { ...state, phase: "idle" };
    case "exit_idle":
      if (state.phase !== "idle") return state;
      return { ...state, phase: "playing" };
  }
}

function eventSource(event: StateEvent): "dispatch" | "ticker" | "init" {
  switch (event.type) {
    case "init":
      return "init";
    case "switch_to":
      return event.source;
    default:
      return "dispatch";
  }
}

function formatStateChange(
  from: AnimationState,
  to: AnimationState,
  event: StateEvent,
): string {
  return `[PetState] ${from.phase} → ${to.phase}, current: ${to.current}, by: ${eventSource(event)}`;
}

export function useAnimationStateMachine(petSettings?: {
  llmEnabled: Ref<boolean>;
  petPersonality: Ref<string>;
  petName: Ref<string>;
  llmApiEndpoint: Ref<string>;
  llmApiKey: Ref<string>;
  llmModel: Ref<string>;
  onDecision?: (reason: string | null) => void;
  onSpeak?: (message: string, animation: string | null) => void;
  /** If provided, ticker ticks call this instead of the old decider. */
  onTickerTick?: () => void;
}) {
  // Spec §3.3: phase 初始 'playing', current 初始 'touch_nose'.
  const state: Ref<AnimationState> = ref({
    phase: "playing",
    current: "touch_nose",
    iteration: 0,
  });

  // Spec R6: ticker interval from env, default 30000, must be > 0.
  const envVal = import.meta.env.VITE_PET_TICKER_INTERVAL_MS;
  const parsed =
    typeof envVal === "string" ? parseInt(envVal, 10) : Number(envVal);
  const tickerInterval: Ref<number> = ref(
    Number.isFinite(parsed) && parsed > 0 ? parsed : 30000,
  );

  if (tickerInterval.value <= 0) {
    const err: AppError = {
      code: "E_INVALID_CONTEXT",
      message: "ticker_interval_ms must be > 0",
    };
    console.error(`[PetError] init → ${err.code}: ${err.message}`);
    throw err;
  }

  const lastInteractionAt: Ref<number> = ref(Date.now());
  const recentHistory: Ref<string[]> = ref([]);
  const lastTickerReason: Ref<string | null> = ref(null);
  let intervalId: number | null = null;
  let inFlight = false;

  const decider: Decider = getDefaultDecider();
  let waitTimeoutId: number | null = null;

  function dispatch(event: StateEvent) {
    if (eventSource(event) === "dispatch") {
      lastInteractionAt.value = Date.now();
    }
    // Track animation history for context (R9).
    if (event.type === "switch_to") {
      recentHistory.value = pushHistory(recentHistory.value, event.id);
    }
    const before = state.value;
    const after = reducer(before, event);
    if (before !== after) {
      console.log(formatStateChange(before, after, event));
      state.value = after;
    }
  }

  async function tickerTick() {
    if (inFlight) return;
    if (state.value.phase === "transitioning") {
      console.log("[PetTicker] skipped, phase=transitioning");
      return;
    }

    // If agent loop is wired up, delegate to the event system.
    if (petSettings?.onTickerTick) {
      petSettings.onTickerTick();
      return;
    }

    // Fallback: old decider path (backward compatibility).
    inFlight = true;
    try {
      const now = new Date();
      const ctx: DecisionContext = {
        currentState: state.value,
        lastInteractionAt: lastInteractionAt.value,
        tickerIntervalMs: tickerInterval.value,
        timeOfDay: now.toLocaleTimeString("zh-CN", {
          hour: "2-digit",
          minute: "2-digit",
        }),
        recentHistory:
          recentHistory.value.length > 0
            ? [...recentHistory.value]
            : undefined,
        llmEnabled: petSettings?.llmEnabled.value,
        petPersonality:
          petSettings?.petPersonality.value || undefined,
        petName:
          petSettings?.petName.value || undefined,
        llmApiEndpoint:
          petSettings?.llmApiEndpoint.value || undefined,
        llmApiKey:
          petSettings?.llmApiKey.value || undefined,
        llmModel:
          petSettings?.llmModel.value || undefined,
      };
      let decision: Decision;
      try {
        decision = await decider(ctx);
      } catch (e) {
        const err = e as AppError;
        console.error(
          `[PetError] decide_next_state → ${err.code}: ${err.message}`,
        );
        decision = { action: "stay" };
      }
      applyDecision(decision);
    } finally {
      inFlight = false;
    }
  }

  function applyDecision(decision: Decision) {
    switch (decision.action) {
      case "stay":
        lastTickerReason.value = null;
        petSettings?.onDecision?.(null);
        break;
      case "switch":
        recentHistory.value = pushHistory(recentHistory.value, decision.to);
        lastTickerReason.value = decision.reason ?? null;
        petSettings?.onDecision?.(decision.reason ?? null);
        dispatch({
          type: "switch_to",
          id: decision.to,
          reason: decision.reason,
          source: "ticker",
        });
        break;
      case "enter_idle":
        dispatch({ type: "enter_idle" });
        break;
      case "exit_idle":
        dispatch({ type: "exit_idle" });
        break;
      case "speak":
        lastTickerReason.value = decision.message;
        petSettings?.onSpeak?.(decision.message, decision.animation ?? null);
        break;
      case "wait": {
        const durationMs = decision.durationMs;
        lastTickerReason.value =
          decision.reason ?? `等待 ${Math.round(durationMs / 1000)}s`;
        petSettings?.onDecision?.(decision.reason ?? null);
        stopTicker();
        waitTimeoutId = window.setTimeout(() => {
          waitTimeoutId = null;
          startTicker();
          // 可以在这里触发一个 timer_tick 事件立即决策
          console.log(
            `[PetAgent] wait ended after ${durationMs}ms, resuming ticker`,
          );
        }, durationMs);
        break;
      }
    }
  }

  function applyAgentResult(result: AgentResult) {
    applyDecision(result.decision);
  }

  function startTicker() {
    if (intervalId !== null) return;
    intervalId = window.setInterval(tickerTick, tickerInterval.value);
  }

  function stopTicker() {
    if (intervalId !== null) {
      window.clearInterval(intervalId);
      intervalId = null;
    }
    if (waitTimeoutId !== null) {
      window.clearTimeout(waitTimeoutId);
      waitTimeoutId = null;
    }
  }

  watch(tickerInterval, (newVal, oldVal) => {
    if (newVal <= 0) {
      const err: AppError = {
        code: "E_INVALID_CONTEXT",
        message: "ticker_interval_ms must be > 0",
      };
      console.error(`[PetError] init → ${err.code}: ${err.message}`);
      throw err;
    }
    console.log(`[PetTicker] interval: ${oldVal}ms → ${newVal}ms`);
    stopTicker();
    startTicker();
  });

  startTicker();

  return {
    state: readonly(state),
    dispatch,
    tickerInterval,
    lastInteractionAt,
    lastTickerReason,
    startTicker,
    stopTicker,
    applyAgentResult,
  };
}
