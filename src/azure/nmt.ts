import type { AppConfig } from '../config.js';
import { AzureAuth, Semaphore, postJson } from './http.js';

interface TranslateResult {
  translations: { text: string; to: string }[];
}

/** Azure Translator (neural machine translation) v3, HTML mode so inline tags survive. */
export class NmtClient {
  constructor(
    private readonly cfg: AppConfig,
    private readonly auth: AzureAuth,
    private readonly limiter: Semaphore,
  ) {}

  get available(): boolean {
    return !!this.cfg.translatorEndpoint;
  }

  async translate(texts: string[], to: string, from?: string): Promise<string[]> {
    if (!this.cfg.translatorEndpoint) throw new Error('AZURE_TRANSLATOR_ENDPOINT is not configured');
    const out: string[] = [];
    // Service limits: 1000 elements and 50,000 characters per request.
    let batch: string[] = [];
    let chars = 0;
    const flush = async () => {
      if (!batch.length) return;
      const params = new URLSearchParams({ 'api-version': '3.0', to, textType: 'html' });
      if (from) params.set('from', from);
      const current = batch;
      const res = await this.limiter.run(async () =>
        postJson<TranslateResult[]>(
          `${this.cfg.translatorEndpoint}/translator/text/v3.0/translate?${params}`,
          await this.auth.headers('translator'),
          current.map((Text) => ({ Text })),
          this.cfg.requestTimeoutMs,
        ),
      );
      out.push(...res.map((r) => r.translations[0]?.text ?? ''));
      batch = [];
      chars = 0;
    };
    for (const t of texts) {
      if (batch.length >= 100 || chars + t.length > 40000) await flush();
      batch.push(t);
      chars += t.length;
    }
    await flush();
    return out;
  }
}

/** Masked segment <-> HTML that Azure Translator keeps intact. */
export function maskedToHtml(masked: string): string {
  return masked.replace(/<x(\d+)\/>/g, '<span class="notranslate" id="x$1">x$1</span>').replace(/<(\/?)g(\d+)>/g, (_, slash: string, n: string) =>
    slash ? '</b>' : `<b id="g${n}">`,
  );
}

export function htmlToMasked(html: string): string {
  let out = html.replace(/<span class="notranslate" id="x(\d+)">[^<]*<\/span>/g, '<x$1/>');
  // Rebuild paired tags by matching open/close order.
  const stack: string[] = [];
  out = out.replace(/<b id="g(\d+)">|<\/b>/g, (m, n?: string) => {
    if (n) {
      stack.push(n);
      return `<g${n}>`;
    }
    const top = stack.pop();
    return top ? `</g${top}>` : '';
  });
  return out;
}
