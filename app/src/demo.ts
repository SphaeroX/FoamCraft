// SPDX-License-Identifier: GPL-3.0-or-later
// Demo mode: shows a list of steps one after another, each for a fixed time; pause, back, forward
// and stop. A pure state machine with timers, so it can be tested with fake timers.

export interface DemoState {
  index: number;
  playing: boolean;
  done: boolean;
}

interface Options<T> {
  /** Milliseconds each step is shown. */
  stepMs: number;
  /** Milliseconds of one step when it differs from `stepMs`. */
  durationOf?: (step: T, index: number) => number;
  /** Called once when the demo ends (after the last step or on stop). */
  onEnd?: () => void;
  /** Called after every change of the state. */
  onChange?: (state: DemoState) => void;
}

export class DemoPlayer<T> {
  private index = 0;
  private playing = false;
  private done = false;
  private timer: ReturnType<typeof setTimeout> | null = null;

  constructor(
    private readonly steps: T[],
    private readonly show: (step: T, index: number) => void,
    private readonly options: Options<T>,
  ) {}

  state(): DemoState {
    return { index: this.index, playing: this.playing, done: this.done };
  }

  start(): void {
    this.done = false;
    this.index = 0;
    this.playing = true;
    if (this.steps.length === 0) {
      this.finish();
      return;
    }
    this.enter();
  }

  pause(): void {
    this.playing = false;
    this.clear();
    this.changed();
  }

  resume(): void {
    if (this.done) return;
    this.playing = true;
    this.schedule();
    this.changed();
  }

  next(): void {
    if (this.done) return;
    if (this.index + 1 >= this.steps.length) {
      this.finish();
      return;
    }
    this.index += 1;
    this.enter();
  }

  prev(): void {
    if (this.done) return;
    this.index = Math.max(0, this.index - 1);
    this.enter();
  }

  stop(): void {
    if (this.done) return;
    this.finish();
  }

  private enter(): void {
    this.clear();
    this.show(this.steps[this.index], this.index);
    if (this.playing) this.schedule();
    this.changed();
  }

  private schedule(): void {
    this.clear();
    const ms = this.options.durationOf?.(this.steps[this.index], this.index) ?? this.options.stepMs;
    this.timer = setTimeout(() => this.next(), ms);
  }

  private finish(): void {
    this.clear();
    this.playing = false;
    this.done = true;
    this.changed();
    this.options.onEnd?.();
  }

  private clear(): void {
    if (this.timer !== null) clearTimeout(this.timer);
    this.timer = null;
  }

  private changed(): void {
    this.options.onChange?.(this.state());
  }
}
