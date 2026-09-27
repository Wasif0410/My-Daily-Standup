/**
 * What the model is actually told, and what it costs.
 *
 * Mirrors `src-tauri/src/inference/context.rs`. Everything here is assembled
 * in Rust from the task database — the frontend never builds a prompt, for the
 * same reason it never computes a week: §3.6 keeps that judgment deterministic
 * and in one place.
 */
export interface ContextPreview {
  /** Tier 1, rendered. Its meaning is in its indentation. */
  map: string;
  /** The whole prompt, map and ranked context and instructions together. */
  prompt: string;
  /**
   * Estimated, not counted — there is no tokenizer here, and the estimate
   * deliberately runs high. Over-estimating drops a line that might have fit;
   * under-estimating truncates the prompt, and a truncated prompt is a model
   * answering a question it was never fully asked.
   */
  mapTokens: number;
  contextTokens: number;
  totalTokens: number;
}
