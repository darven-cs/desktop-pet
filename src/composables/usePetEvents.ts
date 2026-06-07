import { ref, type Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type {
  PetEvent,
  DecisionContext,
  AnimationState,
  AgentResult,
} from "../types/pet";

interface CompactedEvent {
  type: PetEvent["type"];
  interaction?: string;
  focused?: boolean;
  timestamp: number;
  count: number;
}

const MAX_COMPACTED_EVENTS = 10;

function eventGroupKey(ev: PetEvent): { type: PetEvent["type"]; interaction?: string } {
  if (ev.type === "user_interaction") {
    return { type: "user_interaction", interaction: ev.interaction };
  }
  return { type: ev.type };
}

/** Merge consecutive same-type events into compacted summaries. */
function compactEvents(events: PetEvent[]): CompactedEvent[] {
  const groups: CompactedEvent[] = [];

  for (const ev of events) {
    const key = eventGroupKey(ev);
    const last = groups[groups.length - 1];
    if (last && last.type === key.type && last.interaction === key.interaction) {
      last.count++;
    } else {
      groups.push({
        type: key.type,
        interaction: key.interaction,
        focused: ev.type === "window_focus_changed" ? ev.focused : undefined,
        timestamp: ev.type === "timer_tick" ? ev.timestamp
          : ev.type === "user_interaction" ? ev.timestamp
          : ev.type === "window_focus_changed" ? ev.timestamp
          : 0,
        count: 1,
      });
    }
  }

  if (groups.length > MAX_COMPACTED_EVENTS) {
    const half = Math.floor(MAX_COMPACTED_EVENTS / 2);
    return [...groups.slice(0, half), ...groups.slice(groups.length - half)];
  }

  return groups;
}

function formatCompactedSummary(events: CompactedEvent[]): string {
  if (events.length === 0) return "";
  const lines = events.map((ev) => {
    const ts = formatTime(ev.timestamp);
    const label = compactedLabel(ev);
    const suffix = ev.count > 1 ? ` x${ev.count}` : "";
    return `- [${ts}] ${label}${suffix}`;
  });
  return `最近事件：\n${lines.join("\n")}`;
}

function compactedLabel(ev: CompactedEvent): string {
  switch (ev.type) {
    case "timer_tick":
      return "timer_tick";
    case "user_interaction":
      return `user_interaction(${ev.interaction ?? "unknown"})`;
    case "animation_completed":
      return "animation_completed";
    case "window_focus_changed":
      return `window_focus_changed(${ev.focused ? "focused" : "unfocused"})`;
  }
}

function formatTime(millis: number): string {
  const d = new Date(millis);
  const h = String(d.getHours()).padStart(2, "0");
  const m = String(d.getMinutes()).padStart(2, "0");
  const s = String(d.getSeconds()).padStart(2, "0");
  return `${h}:${m}:${s}`;
}

function isDecidableEvent(ev: PetEvent): boolean {
  return ev.type === "user_interaction" || ev.type === "window_focus_changed";
}

export function usePetEvents(petSettings: {
  llmEnabled: Ref<boolean>;
  petPersonality: Ref<string>;
  petName: Ref<string>;
  llmApiEndpoint: Ref<string>;
  llmApiKey: Ref<string>;
  llmModel: Ref<string>;
  tickerInterval: Ref<number>;
  getAnimationState: () => AnimationState;
  getLastInteractionAt: () => number;
  onDecision: (result: AgentResult) => void;
}) {
  const queue: PetEvent[] = [];
  let inFlight = false;
  let debounceTimer: number | null = null;
  const queueLength = ref(0);

  function pushEvent(event: PetEvent) {
    queue.push(event);
    queueLength.value = queue.length;

    if (event.type === "timer_tick") {
      // TimerTick: only flush if there are decidable events in the queue.
      if (queue.some(isDecidableEvent)) {
        flush();
      } else {
        // No decidable events — drain timer_tick to prevent queue growth.
        for (let i = queue.length - 1; i >= 0; i--) {
          if (queue[i].type === "timer_tick") {
            queue.splice(i, 1);
          }
        }
        queueLength.value = queue.length;
      }
    } else {
      // Non-timer events: debounce 2s.
      if (debounceTimer !== null) {
        clearTimeout(debounceTimer);
      }
      debounceTimer = window.setTimeout(() => {
        debounceTimer = null;
        flush();
      }, 2000);
    }
  }

  async function flush() {
    if (inFlight) return;
    if (queue.length === 0) return;

    inFlight = true;
    const batch = queue.splice(0); // drain all
    queueLength.value = 0;

    try {
      // Step 1: filter out animation_completed.
      const filtered = batch.filter((e) => e.type !== "animation_completed");

      // Step 2: if no decidable events remain, skip LLM call.
      if (!filtered.some(isDecidableEvent)) {
        return;
      }

      // Step 3: compact events and build summary with count info.
      const compacted = compactEvents(filtered);
      const summary = formatCompactedSummary(compacted);

      // Step 4: pass compacted events (fewer items) + frontend-built summary.
      const eventsForAgent: PetEvent[] = compacted.map((ce) => {
        if (ce.type === "user_interaction") {
          return {
            type: "user_interaction" as const,
            interaction: (ce.interaction ?? "click") as "click" | "drag_end" | "double_click",
            timestamp: ce.timestamp,
          };
        }
        if (ce.type === "window_focus_changed") {
          return {
            type: "window_focus_changed" as const,
            focused: ce.focused ?? true,
            timestamp: ce.timestamp,
          };
        }
        return { type: "timer_tick" as const, timestamp: ce.timestamp };
      });

      const now = new Date();
      const ctx: DecisionContext = {
        currentState: petSettings.getAnimationState(),
        lastInteractionAt: petSettings.getLastInteractionAt(),
        tickerIntervalMs: petSettings.tickerInterval.value,
        timeOfDay: now.toLocaleTimeString("zh-CN", {
          hour: "2-digit",
          minute: "2-digit",
        }),
        llmEnabled: petSettings.llmEnabled.value,
        petPersonality: petSettings.petPersonality.value || undefined,
        petName: petSettings.petName.value || undefined,
        llmApiEndpoint: petSettings.llmApiEndpoint.value || undefined,
        llmApiKey: petSettings.llmApiKey.value || undefined,
        llmModel: petSettings.llmModel.value || undefined,
        eventsSummary: summary,
      };

      const result = await invoke<AgentResult>("agent_decide", {
        events: eventsForAgent,
        context: ctx,
      });
      petSettings.onDecision(result);
    } catch (e) {
      console.error("[PetAgent] agent_decide failed:", e);
    } finally {
      inFlight = false;
    }
  }

  return {
    pushEvent,
    flush,
    queueLength,
  };
}
