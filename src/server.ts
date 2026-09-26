import Fastify, { type FastifyReply, type FastifyRequest } from 'fastify';
import { timingSafeEqual, createHash } from 'node:crypto';
import { loadConfig } from './config.js';
import { MarkdownTranslator, UnknownLanguageError, type TranslateOptions } from './pipeline.js';

const MAX_FILES = 500;
const MAX_NAME = 512;

class BadRequest extends Error {}

function parseFiles(body: unknown): Record<string, string> {
  if (!body || typeof body !== 'object' || Array.isArray(body)) throw new BadRequest('body must be a JSON object: { "file.md": "markdown text" }');
  const entries = Object.entries(body as Record<string, unknown>);
  if (!entries.length) throw new BadRequest('body contains no files');
  if (entries.length > MAX_FILES) throw new BadRequest(`at most ${MAX_FILES} files per request`);
  const files: Record<string, string> = {};
  for (const [name, value] of entries) {
    if (!name || name.length > MAX_NAME) throw new BadRequest(`invalid file name "${name.slice(0, 40)}"`);
    if (typeof value !== 'string') throw new BadRequest(`value of "${name}" must be a string`);
    files[name] = value;
  }
  return files;
}

function options(q: Record<string, string | undefined>): TranslateOptions {
  const o: TranslateOptions = {};
  if (q.formality) {
    if (q.formality !== 'formal' && q.formality !== 'informal') throw new BadRequest('formality must be "formal" or "informal"');
    o.formality = q.formality;
  }
  if (q.sourceLanguage) {
    if (!/^[A-Za-z]{2,3}(?:-[A-Za-z0-9]{2,8})*$/.test(q.sourceLanguage)) throw new BadRequest('invalid sourceLanguage');
    o.sourceLanguage = q.sourceLanguage;
  }
  if (q.review) o.review = /^(1|true|yes)$/i.test(q.review);
  if (q.engine) {
    if (q.engine !== 'gpt' && q.engine !== 'nmt') throw new BadRequest('engine must be "gpt" or "nmt"');
    o.engine = q.engine;
  }
  if (q.doNotTranslate) o.doNotTranslate = q.doNotTranslate.split(',').map((s) => s.trim()).filter(Boolean).slice(0, 200);
  if (q.structuralContext) o.structuralContext = /^(1|true|yes)$/i.test(q.structuralContext);
  if (q.docstrings) o.docstrings = /^(1|true|yes|translate)$/i.test(q.docstrings);
  return o;
}

const digest = (s: string) => createHash('sha256').update(s).digest();

export async function buildServer() {
  const cfg = loadConfig();
  const translator = await MarkdownTranslator.create(cfg);
  const app = Fastify({
    logger: { level: process.env.LOG_LEVEL ?? 'info' },
    bodyLimit: cfg.bodyLimitBytes,
    requestTimeout: 30 * 60_000,
  });

  if (!cfg.apiAuthKey) app.log.warn('MDT_API_KEY is not set: the API accepts unauthenticated requests (use only for local testing)');
  app.log.info({ auth: translator.auth.mode, openai: !!cfg.openaiEndpoint, translator: !!cfg.translatorEndpoint, review: cfg.review }, 'mdtranslator configuration');

  const expected = cfg.apiAuthKey ? digest(cfg.apiAuthKey) : undefined;
  app.addHook('onRequest', async (req: FastifyRequest, reply: FastifyReply) => {
    if (!expected || req.url === '/healthz') return;
    const given = req.headers['x-api-key'];
    if (typeof given !== 'string' || !timingSafeEqual(digest(given), expected)) {
      await reply.code(401).send({ error: 'missing or invalid x-api-key header' });
    }
  });

  app.setErrorHandler((err: Error, _req, reply) => {
    if (err instanceof BadRequest) return reply.code(400).send({ error: err.message });
    if (err instanceof UnknownLanguageError) return reply.code(422).send({ error: err.message, supported: [...translator.catalog.languages.values()].map((l) => l.code) });
    const status = (err as { statusCode?: number }).statusCode;
    if (status && status < 500) return reply.code(status).send({ error: err.message });
    app.log.error(err);
    return reply.code(500).send({ error: 'translation failed' });
  });

  app.get('/healthz', async () => ({ status: 'ok' }));

  app.get('/languages', async () => ({
    defaultTargets: translator.catalog.defaultTargets,
    languages: [...translator.catalog.languages.values()].map((l) => ({ code: l.code, name: l.name })),
  }));

  // POST /translate/fr  body { "main.md": "..." }  ->  { "main.md": "..." }
  app.post<{ Params: { lang: string }; Querystring: Record<string, string | undefined> }>('/translate/:lang', async (req) => {
    const files = parseFiles(req.body);
    const result = await translator.translateFiles(files, [req.params.lang], options(req.query));
    const code = translator.resolveLanguages([req.params.lang])[0].code;
    const out = result.translations[code];
    return /^(1|true|yes)$/i.test(req.query.includeReport ?? '') ? { files: out, report: result.reports, usage: result.usage } : out;
  });

  // POST /translate?to=fr,sv  body { "main.md": "..." }  ->  { "fr": { "main.md": "..." }, "sv": { ... } }
  app.post<{ Querystring: Record<string, string | undefined> }>('/translate', async (req) => {
    const files = parseFiles(req.body);
    const to = req.query.to?.split(',').map((s) => s.trim()).filter(Boolean);
    const result = await translator.translateFiles(files, to, options(req.query));
    return /^(1|true|yes)$/i.test(req.query.includeReport ?? '') ? { translations: result.translations, report: result.reports, usage: result.usage } : result.translations;
  });

  return { app, cfg };
}
