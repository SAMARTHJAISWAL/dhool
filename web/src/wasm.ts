// Loads the Rust core and wraps its raw `extern "C"` exports in a small,
// typed class. All bulk data is read straight out of WebAssembly memory.

import type { MatId } from './materials';
import type { Painter } from './presets';

/** The exports of sim/src/ffi.rs, exactly as the module provides them. */
interface Exports {
  readonly memory: WebAssembly.Memory;
  init(width: number, height: number, seed: number): void;
  width(): number;
  height(): number;
  tick(): void;
  tick_n(n: number): void;
  ticks(): number;
  render(): void;
  pixels_ptr(): number;
  paint(x: number, y: number, radius: number, mat: number): void;
  stamp(x: number, y: number, radius: number, mat: number): void;
  fill_rect(x: number, y: number, w: number, h: number, mat: number): void;
  clear(): void;
  get(x: number, y: number): number;
  count(mat: number): number;
  material_count(): number;
  save(): number;
  io_ptr(): number;
  io_reserve(len: number): number;
  load(len: number): number;
}

async function instantiate(url: string): Promise<WebAssembly.Instance> {
  const response = fetch(url);
  try {
    return (await WebAssembly.instantiateStreaming(response, {})).instance;
  } catch {
    // Some static hosts serve .wasm without application/wasm, which makes
    // streaming compilation refuse it. Fall back to a plain buffer.
    const bytes = await (await fetch(url)).arrayBuffer();
    return (await WebAssembly.instantiate(bytes, {})).instance;
  }
}

export class Sim implements Painter {
  private image: ImageData | null = null;

  private constructor(private readonly ex: Exports) {}

  static async load(url: string): Promise<Sim> {
    const instance = await instantiate(url);
    return new Sim(instance.exports as unknown as Exports);
  }

  get width(): number {
    return this.ex.width();
  }

  get height(): number {
    return this.ex.height();
  }

  get ticks(): number {
    return this.ex.ticks();
  }

  get materialCount(): number {
    return this.ex.material_count();
  }

  init(width: number, height: number, seed: number): void {
    this.ex.init(width, height, seed >>> 0);
    this.image = null;
  }

  tick(n = 1): void {
    if (n === 1) this.ex.tick();
    else if (n > 1) this.ex.tick_n(n);
  }

  /** Renders and returns an ImageData that aliases WASM memory (no copy). */
  frame(): ImageData {
    this.ex.render();
    const w = this.width;
    const h = this.height;
    const ptr = this.ex.pixels_ptr();
    const img = this.image;
    // Memory growth detaches the old ArrayBuffer, and init() reallocates the
    // pixel buffer; in either case build a fresh view.
    if (!img || img.data.buffer !== this.ex.memory.buffer || img.data.byteOffset !== ptr || img.width !== w || img.height !== h) {
      const view = new Uint8ClampedArray(this.ex.memory.buffer, ptr, w * h * 4);
      this.image = new ImageData(view, w, h);
    }
    return this.image as ImageData;
  }

  paint(x: number, y: number, radius: number, mat: MatId): void {
    this.ex.paint(x, y, radius, mat);
  }

  stamp(x: number, y: number, r: number, mat: MatId): void {
    this.ex.stamp(x, y, r, mat);
  }

  fillRect(x: number, y: number, w: number, h: number, mat: MatId): void {
    this.ex.fill_rect(x, y, w, h, mat);
  }

  clear(): void {
    this.ex.clear();
  }

  get(x: number, y: number): number {
    return this.ex.get(x, y);
  }

  /** Cell count for a material as of the last frame(). */
  count(mat: MatId): number {
    return this.ex.count(mat);
  }

  save(): Uint8Array {
    const len = this.ex.save();
    // slice() copies, so the result survives the next call into WASM.
    return new Uint8Array(this.ex.memory.buffer, this.ex.io_ptr(), len).slice();
  }

  load(bytes: Uint8Array): boolean {
    const ptr = this.ex.io_reserve(bytes.length);
    new Uint8Array(this.ex.memory.buffer, ptr, bytes.length).set(bytes);
    const ok = this.ex.load(bytes.length) === 1;
    if (ok) this.image = null;
    return ok;
  }
}
