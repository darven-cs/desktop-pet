import { invoke } from "@tauri-apps/api/core";
import type { Decision, DecisionContext } from "../types/pet";

// Independent module so 02 spec can swap in an LLM-backed implementation
// without touching the state machine (spec 01 §3.3 decider/).
export type Decider = (ctx: DecisionContext) => Promise<Decision>;

// 01 default: thin pass-through to the Rust `decide_next_state` command,
// which always returns `Stay`. 02 will replace this with an LLM client.
export function getDefaultDecider(): Decider {
  return async (ctx) => {
    return await invoke<Decision>("decide_next_state", { context: ctx });
  };
}
