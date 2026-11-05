// Fixed-timestep accumulator. The simulation always advances in whole ticks
// of `stepMs`, independent of the display's refresh rate; the speed setting
// scales how much simulated time each real millisecond is worth.

export class FixedStep {
  private acc = 0;

  constructor(
    readonly stepMs = 1000 / 60,
    /** Upper bound on ticks per frame, so a stalled tab cannot spiral. */
    readonly maxTicks = 16,
  ) {}

  /** Returns how many ticks to run for a frame that took `dtMs`. */
  advance(dtMs: number, speed = 1): number {
    if (!(dtMs > 0) || !(speed > 0)) return 0;
    // Clamp long gaps (tab switch, breakpoint) to a few frames.
    this.acc += Math.min(dtMs, 250) * speed;
    const ticks = Math.min(Math.floor(this.acc / this.stepMs), this.maxTicks);
    this.acc -= ticks * this.stepMs;
    if (ticks === this.maxTicks) this.acc = 0;
    return ticks;
  }

  reset(): void {
    this.acc = 0;
  }
}

/** Rolling rate counter, e.g. frames or ticks per second. */
export class RateMeter {
  private count = 0;
  private since = 0;
  rate = 0;

  /** Adds `n` events at time `now`; refreshes `rate` every `windowMs`. */
  add(n: number, now: number, windowMs = 500): void {
    if (this.since === 0) this.since = now;
    this.count += n;
    const elapsed = now - this.since;
    if (elapsed >= windowMs) {
      this.rate = (this.count * 1000) / elapsed;
      this.count = 0;
      this.since = now;
    }
  }
}
