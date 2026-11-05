import { describe, expect, it } from 'vitest';
import { fromBase64Url, parseSceneHash, sceneHash, toBase64Url } from './share';

function bytes(n: number, seed = 1): Uint8Array {
  const out = new Uint8Array(n);
  let x = seed;
  for (let i = 0; i < n; i++) {
    x = (x * 1103515245 + 12345) >>> 0;
    out[i] = x >>> 24;
  }
  return out;
}

describe('base64url', () => {
  it('round-trips every length from 0 to 64', () => {
    for (let n = 0; n <= 64; n++) {
      const b = bytes(n, n);
      expect(fromBase64Url(toBase64Url(b))).toEqual(b);
    }
  });

  it('round-trips a large snapshot', () => {
    const b = bytes(200_000);
    expect(fromBase64Url(toBase64Url(b))).toEqual(b);
  });

  it('uses only URL-safe characters and no padding', () => {
    const text = toBase64Url(new Uint8Array([0xfb, 0xff, 0xfe, 0x3e, 0x3f]));
    expect(text).toMatch(/^[A-Za-z0-9_-]+$/);
    expect(text).toBe('-__-Pj8');
  });

  it('rejects text that is not base64url', () => {
    expect(fromBase64Url('abc+')).toBeNull();
    expect(fromBase64Url('ab/c')).toBeNull();
    expect(fromBase64Url('a')).toBeNull();
    expect(fromBase64Url('héllo')).toBeNull();
  });
});

describe('scene hash', () => {
  it('round-trips through the hash', () => {
    const snap = new Uint8Array([68, 72, 76, 1, 64, 1, 200, 0, 0, 255, 255, 3]);
    const hash = sceneHash(snap);
    expect(hash.startsWith('#s=')).toBe(true);
    expect(parseSceneHash(hash)).toEqual(snap);
  });

  it('ignores empty or unrelated hashes', () => {
    expect(parseSceneHash('')).toBeNull();
    expect(parseSceneHash('#')).toBeNull();
    expect(parseSceneHash('#about')).toBeNull();
    expect(parseSceneHash('#s=')).toBeNull();
    expect(parseSceneHash('#s=!!!')).toBeNull();
  });

  it('finds the scene among other parameters', () => {
    const snap = new Uint8Array([1, 2, 3]);
    expect(parseSceneHash(`#v=2&${sceneHash(snap).slice(1)}`)).toEqual(snap);
  });
});
