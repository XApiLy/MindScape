import type {
  CanvasViewportState,
  SaveCanvasViewportInput,
} from "../domain/commands";
import type { CanvasViewport } from "./graphProjection";

type TimerHandle = ReturnType<typeof setTimeout>;

type ViewportWriteState = {
  pending: SaveCanvasViewportInput | null;
  timer: TimerHandle | null;
  lastDispatchedAt: number;
  queue: Promise<void>;
};

type CanvasViewportPersistenceOptions = {
  intervalMs?: number;
  now?: () => number;
  setTimer?: (callback: () => void, delayMs: number) => TimerHandle;
  clearTimer?: (timer: TimerHandle) => void;
  onError?: (error: unknown) => void;
};

export async function loadCanvasViewport(
  read: (conversationId: string) => Promise<CanvasViewportState | null>,
  conversationId: string,
): Promise<CanvasViewport | null> {
  const state = await read(conversationId);
  return state ? { x: state.x, y: state.y, zoom: state.zoom } : null;
}

export class CanvasViewportPersistence {
  private readonly states = new Map<string, ViewportWriteState>();

  private readonly write: (input: SaveCanvasViewportInput) => Promise<unknown>;

  private readonly intervalMs: number;

  private readonly now: () => number;

  private readonly setTimer: (callback: () => void, delayMs: number) => TimerHandle;

  private readonly clearTimer: (timer: TimerHandle) => void;

  private readonly onError: (error: unknown) => void;

  constructor(
    write: (input: SaveCanvasViewportInput) => Promise<unknown>,
    options: CanvasViewportPersistenceOptions = {},
  ) {
    this.write = write;
    this.intervalMs = options.intervalMs ?? 250;
    this.now = options.now ?? Date.now;
    this.setTimer = options.setTimer ?? setTimeout;
    this.clearTimer = options.clearTimer ?? clearTimeout;
    this.onError = options.onError ?? (() => undefined);
  }

  schedule(conversationId: string, viewport: CanvasViewport) {
    const state = this.getState(conversationId);
    state.pending = { conversationId, ...viewport };
    if (state.timer !== null) return;

    const elapsed = this.now() - state.lastDispatchedAt;
    const delayMs = Number.isFinite(elapsed)
      ? Math.max(0, this.intervalMs - elapsed)
      : 0;
    if (delayMs === 0) {
      this.dispatch(state);
      return;
    }

    state.timer = this.setTimer(() => {
      state.timer = null;
      this.dispatch(state);
    }, delayMs);
  }

  async flush(conversationId: string) {
    const state = this.states.get(conversationId);
    if (!state) return;
    if (state.timer !== null) {
      this.clearTimer(state.timer);
      state.timer = null;
    }
    this.dispatch(state);
    await state.queue;
  }

  async flushAll() {
    await Promise.all([...this.states.keys()].map((conversationId) => this.flush(conversationId)));
  }

  private getState(conversationId: string) {
    const existing = this.states.get(conversationId);
    if (existing) return existing;
    const state: ViewportWriteState = {
      pending: null,
      timer: null,
      lastDispatchedAt: Number.NEGATIVE_INFINITY,
      queue: Promise.resolve(),
    };
    this.states.set(conversationId, state);
    return state;
  }

  private dispatch(state: ViewportWriteState) {
    const input = state.pending;
    if (!input) return;
    state.pending = null;
    state.lastDispatchedAt = this.now();
    state.queue = state.queue
      .catch(() => undefined)
      .then(async () => {
        await this.write(input);
      })
      .catch((error: unknown) => {
        this.onError(error);
      });
  }
}
