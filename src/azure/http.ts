import { DefaultAzureCredential, ManagedIdentityCredential, ChainedTokenCredential, type TokenCredential } from '@azure/identity';
import type { AppConfig } from '../config.js';

const SCOPE = 'https://cognitiveservices.azure.com/.default';

export type Service = 'openai' | 'translator';

export class AzureAuth {
  private credential?: TokenCredential;
  private token?: { value: string; expires: number };

  constructor(private readonly cfg: AppConfig) {
    if (!cfg.apiKey) {
      const dev = new DefaultAzureCredential(cfg.tenantId ? { tenantId: cfg.tenantId } : {});
      this.credential = cfg.managedIdentityClientId
        ? new ChainedTokenCredential(new ManagedIdentityCredential({ clientId: cfg.managedIdentityClientId }), dev)
        : dev;
    }
  }

  get mode(): 'api-key' | 'entra' {
    return this.cfg.apiKey ? 'api-key' : 'entra';
  }

  async headers(service: Service): Promise<Record<string, string>> {
    if (this.cfg.apiKey) {
      switch (service) {
        case 'openai':
          return { 'api-key': this.cfg.apiKey };
        case 'translator':
          return {
            'Ocp-Apim-Subscription-Key': this.cfg.apiKey,
            ...(this.cfg.translatorRegion ? { 'Ocp-Apim-Subscription-Region': this.cfg.translatorRegion } : {}),
          };
      }
    }
    return { Authorization: `Bearer ${await this.bearer()}` };
  }

  private async bearer(): Promise<string> {
    if (this.cfg.staticAccessToken) return this.cfg.staticAccessToken;
    if (this.token && this.token.expires - Date.now() > 5 * 60_000) return this.token.value;
    const t = await this.credential!.getToken(SCOPE);
    if (!t) throw new Error('could not acquire a Microsoft Entra token for Azure AI services');
    this.token = { value: t.token, expires: t.expiresOnTimestamp };
    return t.token;
  }
}

export class HttpError extends Error {
  constructor(
    readonly status: number,
    readonly body: string,
  ) {
    super(`HTTP ${status}: ${body.slice(0, 500)}`);
  }
}

const RETRYABLE = new Set([408, 409, 429, 500, 502, 503, 504]);

export async function postJson<T>(url: string, headers: Record<string, string>, body: unknown, timeoutMs: number, attempts = 6): Promise<T> {
  let lastErr: unknown;
  for (let attempt = 1; attempt <= attempts; attempt++) {
    try {
      const res = await fetch(url, {
        method: 'POST',
        headers: { 'content-type': 'application/json', ...headers },
        body: JSON.stringify(body),
        signal: AbortSignal.timeout(timeoutMs),
      });
      const text = await res.text();
      if (res.ok) return JSON.parse(text) as T;
      const err = new HttpError(res.status, text);
      if (!RETRYABLE.has(res.status) || attempt === attempts) throw err;
      lastErr = err;
      const retryAfter = Number(res.headers.get('retry-after-ms') ?? NaN) || Number(res.headers.get('retry-after') ?? NaN) * 1000;
      await sleep(Number.isFinite(retryAfter) && retryAfter > 0 ? Math.min(retryAfter, 60_000) : backoff(attempt));
    } catch (e) {
      if (e instanceof HttpError && !RETRYABLE.has(e.status)) throw e;
      if (attempt === attempts) throw e;
      lastErr = e;
      await sleep(backoff(attempt));
    }
  }
  throw lastErr;
}

const backoff = (attempt: number) => Math.min(30_000, 1000 * 2 ** (attempt - 1)) * (0.75 + Math.random() * 0.5);
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

export class Semaphore {
  private active = 0;
  private queue: (() => void)[] = [];
  constructor(private readonly max: number) {}

  async run<T>(fn: () => Promise<T>): Promise<T> {
    if (this.active >= this.max) await new Promise<void>((r) => this.queue.push(r));
    this.active++;
    try {
      return await fn();
    } finally {
      this.active--;
      this.queue.shift()?.();
    }
  }
}
