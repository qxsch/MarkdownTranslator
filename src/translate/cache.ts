import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

/** Segment-level translation memory: in-memory LRU with optional file persistence. */
export class TranslationCache {
  private mem = new Map<string, string>();

  constructor(
    private readonly dir?: string,
    private readonly maxEntries = 200_000,
  ) {}

  static key(parts: unknown[]): string {
    return createHash('sha256').update(JSON.stringify(parts)).digest('hex');
  }

  async get(key: string): Promise<string | undefined> {
    const hit = this.mem.get(key);
    if (hit !== undefined) {
      this.mem.delete(key);
      this.mem.set(key, hit);
      return hit;
    }
    if (!this.dir) return undefined;
    try {
      const v = JSON.parse(await readFile(this.path(key), 'utf8')) as { t: string };
      this.remember(key, v.t);
      return v.t;
    } catch {
      return undefined;
    }
  }

  async set(key: string, value: string): Promise<void> {
    this.remember(key, value);
    if (!this.dir) return;
    const p = this.path(key);
    await mkdir(join(this.dir, key.slice(0, 2)), { recursive: true });
    await writeFile(p, JSON.stringify({ t: value }), 'utf8');
  }

  private remember(key: string, value: string) {
    this.mem.set(key, value);
    if (this.mem.size > this.maxEntries) this.mem.delete(this.mem.keys().next().value!);
  }

  private path(key: string): string {
    return join(this.dir!, key.slice(0, 2), `${key}.json`);
  }
}
