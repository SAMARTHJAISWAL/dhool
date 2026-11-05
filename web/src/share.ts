// Scene sharing: the Rust core writes a run-length snapshot, which we put in
// the URL hash as unpadded base64url (RFC 4648 section 5).

const HASH_KEY = 's';

export function toBase64Url(bytes: Uint8Array): string {
  let bin = '';
  for (let i = 0; i < bytes.length; i += 0x8000) {
    bin += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  }
  return btoa(bin).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

export function fromBase64Url(text: string): Uint8Array | null {
  if (!/^[A-Za-z0-9_-]*$/.test(text) || text.length % 4 === 1) return null;
  const b64 = text.replace(/-/g, '+').replace(/_/g, '/');
  const padded = b64 + '='.repeat((4 - (b64.length % 4)) % 4);
  try {
    const bin = atob(padded);
    const out = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
    return out;
  } catch {
    return null;
  }
}

/** `#s=<base64url>` for a snapshot. */
export function sceneHash(bytes: Uint8Array): string {
  return `#${HASH_KEY}=${toBase64Url(bytes)}`;
}

/** Snapshot bytes from a location hash, or null if there is none. */
export function parseSceneHash(hash: string): Uint8Array | null {
  const params = new URLSearchParams(hash.replace(/^#/, ''));
  const value = params.get(HASH_KEY);
  if (!value) return null;
  const bytes = fromBase64Url(value);
  return bytes && bytes.length > 0 ? bytes : null;
}
