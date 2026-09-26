/**
 * Local stand-in for Azure OpenAI (chat completions with JSON schema) and Azure Translator, for testing the
 * service layer of the Rust binary without Azure. Translations are pseudo translations (test/helpers.ts).
 *
 *   npx tsx rust/tools/mock-azure.ts [port]
 *   MOCK_THROTTLE=1   first request of each kind answers 429 with retry-after-ms
 *   MOCK_BREAK=1      first translation of each segment drops its tags (fails validation, forces a retry)
 *   MOCK_FAIL=s3      segment ids that always come back broken (forces the NMT fallback)
 *   MOCK_REVIEW=1     the reviewer returns one edit per batch
 *   MOCK_TOKEN=abc    Microsoft Entra token that /{tenant}/oauth2/v2.0/token issues (point AZURE_AUTHORITY_HOST at
 *                     this server); chat and translator requests must then carry it as their bearer token
 */
import { createServer } from 'node:http';
import { pseudo, pseudoWords } from '../../test/helpers.ts';

const port = Number(process.argv[2] ?? 8787);
const throttled = new Set<string>();
const attempts = new Map<string, number>();
const failing = new Set((process.env.MOCK_FAIL ?? '').split(',').filter(Boolean));
const log: string[] = [];

function reply(res: import('node:http').ServerResponse, status: number, body: unknown, headers: Record<string, string> = {}) {
  res.writeHead(status, { 'content-type': 'application/json', ...headers });
  res.end(JSON.stringify(body));
}

createServer((req, res) => {
  let raw = '';
  req.on('data', (c) => (raw += c));
  req.on('end', () => {
    const url = new URL(req.url ?? '/', `http://${req.headers.host}`);
    const kind = url.pathname.includes('/translator/') ? 'nmt' : 'chat';
    if (url.pathname === '/_log') return reply(res, 200, log);
    if (url.pathname.endsWith('/oauth2/v2.0/token')) {
      const form = new URLSearchParams(raw);
      log.push(`token: tenant=${url.pathname.split('/')[1]} client=${form.get('client_id')} grant=${form.get('grant_type')} scope=${form.get('scope')}`);
      return reply(res, 200, { token_type: 'Bearer', access_token: process.env.MOCK_TOKEN ?? 'mock-token', expires_in: 3600 });
    }
    if (!req.headers['api-key'] && !req.headers['ocp-apim-subscription-key'] && !req.headers.authorization) return reply(res, 401, { error: 'no credentials' });
    if (process.env.MOCK_TOKEN && req.headers.authorization !== `Bearer ${process.env.MOCK_TOKEN}`) {
      log.push(`${kind}: 401 unexpected credentials`);
      return reply(res, 401, { error: 'unexpected credentials' });
    }
    if (process.env.MOCK_THROTTLE && !throttled.has(kind)) {
      throttled.add(kind);
      log.push(`${kind}: 429`);
      return reply(res, 429, { error: 'throttled' }, { 'retry-after-ms': '50' });
    }
    const body = JSON.parse(raw);
    if (kind === 'nmt') {
      log.push(`nmt: ${body.length} texts to ${url.searchParams.get('to')} from ${url.searchParams.get('from')} textType=${url.searchParams.get('textType')}`);
      const translated = body.map((b: { Text: string }) => ({
        translations: [{ to: url.searchParams.get('to'), text: b.Text.replace(/(^|>)([^<]*)/g, (_m, a: string, t: string) => a + pseudoWords(t)) }],
      }));
      return reply(res, 200, translated);
    }
    const schema = body.response_format?.json_schema?.name;
    const user = body.messages[body.messages.length - 1].content as string;
    const payload = schema === 'analysis' ? {} : JSON.parse(user.slice(user.indexOf('\n') + 1));
    const usage = { prompt_tokens: Math.ceil(JSON.stringify(body).length / 4), completion_tokens: 50 };
    const answer = (content: unknown) => reply(res, 200, { choices: [{ message: { content: JSON.stringify(content) }, finish_reason: 'stop' }], usage });
    log.push(`chat ${schema}: model=${body.model} reasoning=${body.reasoning_effort} segments=${payload.segments?.length ?? 0}${payload.segments?.some((s: { problems?: unknown }) => s.problems) ? ' (retry)' : ''}`);
    if (schema === 'analysis') {
      return answer({ sourceLanguage: 'en', register: 'neutral', registerEvidence: 'Plain documentation.', domain: 'software', audience: 'developers', summary: 'A test document.', doNotTranslate: ['Contoso'], terminology: [{ term: 'cache', note: 'segment cache' }] });
    }
    if (schema === 'translations') {
      const translations = payload.segments.map((s: { id: string; text: string }) => {
        const n = (attempts.get(s.id) ?? 0) + 1;
        attempts.set(s.id, n);
        const broken = failing.has(s.id) || (process.env.MOCK_BREAK && n === 1);
        return { id: s.id, text: broken ? `<x99/>${pseudo(s.text)}` : pseudo(s.text) };
      });
      return answer({ translations });
    }
    if (schema === 'review') {
      const first = payload.segments[0];
      const edits = process.env.MOCK_REVIEW && first ? [{ id: first.id, category: 'fluency', severity: 'minor', explanation: 'Mock edit.', text: String(first.translation).replace('Ü', 'Ö') }] : [];
      return answer({ edits });
    }
    if (schema === 'mqm') {
      // Evaluation judges: every item is a tie without errors.
      const items: { id: string }[] = payload.items ?? payload.segments ?? [];
      return answer({ results: items.map((it) => ({ id: it.id, errorsA: [], errorsB: [], better: 'tie' })) });
    }
    reply(res, 400, { error: `unknown schema ${schema}` });
  });
}).listen(port, '127.0.0.1', () => console.log(`mock Azure listening on http://127.0.0.1:${port}`));
