/**
 * Judge providers for the evaluation framework (host only; the translator itself always uses Foundry).
 *
 *   translate-alt, foundry:translate      Foundry deployment, called with strict JSON-schema output
 *   copilot:claude-opus-5.5               GitHub Copilot SDK model, run as an agent with read-only document tools
 */
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

type FoundryChat = { json<T>(deployment: string, messages: { role: 'system' | 'user'; content: string }[], name: string, schema: object, effort?: string): Promise<T> };

export interface JudgeRequest {
  system: string;
  user: string;
  schemaName: string;
  schema: object;
  /** Whole documents the agent may read and search (Copilot judges only). */
  documents?: Record<string, string>;
  /** Returns an error message when the parsed answer is unusable. */
  validate?: (value: unknown) => string | undefined;
}

export interface Judge {
  id: string;
  provider: 'foundry' | 'copilot';
  model: string;
  family: string;
  json<T>(req: JudgeRequest): Promise<T>;
}

export const familyOf = (model: string) =>
  /^claude/i.test(model) ? 'anthropic' : /^gemini/i.test(model) ? 'google' : /^grok/i.test(model) ? 'xai' : /^mai/i.test(model) ? 'microsoft' : 'openai';

class Limiter {
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

/** Rejects when `p` does not settle in time (SDK waits can otherwise hang on a stuck session). */
function withTimeout<T>(p: Promise<T>, ms: number, what: string): Promise<T> {
  let timer: NodeJS.Timeout;
  return Promise.race([p, new Promise<never>((_, reject) => (timer = setTimeout(() => reject(new Error(`${what} timed out after ${ms} ms`)), ms)))]).finally(() => clearTimeout(timer));
}

/** First JSON object in a model answer (tolerates code fences and surrounding prose). */
export function parseJsonAnswer(text: string): unknown {
  const fenced = /```(?:json)?\s*([\s\S]*?)```/.exec(text);
  const body = (fenced ? fenced[1] : text).trim();
  const start = body.indexOf('{');
  const end = body.lastIndexOf('}');
  if (start === -1 || end < start) throw new Error('no JSON object in the answer');
  return JSON.parse(body.slice(start, end + 1));
}

function numbered(text: string, from = 1, to = Number.MAX_SAFE_INTEGER): string {
  const lines = text.split('\n');
  const a = Math.max(1, from);
  const b = Math.min(lines.length, to, a + 399);
  return lines.slice(a - 1, b).map((l, i) => `${a + i}: ${l}`).join('\n') + (b < lines.length ? `\n... (${lines.length - b} more lines; call again with start_line=${b + 1})` : '');
}

export async function createJudges(ids: string[], foundry: FoundryChat): Promise<{ judges: Judge[]; close(): Promise<void> }> {
  const judges: Judge[] = [];
  const copilotIds = ids.filter((id) => id.startsWith('copilot:'));
  let closeCopilot = async () => {};

  for (const id of ids.filter((x) => !x.startsWith('copilot:'))) {
    const deployment = id.replace(/^foundry:/, '');
    judges.push({
      id, provider: 'foundry', model: deployment, family: process.env.EVAL_FOUNDRY_FAMILY ?? 'openai',
      json: <T>(req: JudgeRequest) =>
        foundry.json<T>(deployment, [{ role: 'system', content: req.system }, { role: 'user', content: req.user }], req.schemaName, req.schema, 'high'),
    });
  }

  if (copilotIds.length) {
    let sdk: typeof import('@github/copilot-sdk');
    try {
      sdk = await import('@github/copilot-sdk');
    } catch {
      throw new Error('Copilot judges need the evaluator dependencies: run "npm install --prefix eval" first');
    }
    const work = mkdtempSync(join(tmpdir(), 'mdt-judge-'));
    // Empty mode: no user instructions, memory or session store leak into the judge; state lives in a temp dir.
    const client = new sdk.CopilotClient({ mode: 'empty', logLevel: 'error', workingDirectory: work, baseDirectory: join(work, 'home') });
    try {
      await client.start();
    } catch (e) {
      throw new Error(`GitHub Copilot SDK could not start (${(e as Error).message}). Sign in with the Copilot CLI or set GH_TOKEN / COPILOT_GITHUB_TOKEN.`);
    }
    const models = new Map((await client.listModels()).map((m) => [m.id, m]));
    const limiter = new Limiter(Number(process.env.EVAL_COPILOT_CONCURRENCY ?? 4));
    const timeoutMs = Number(process.env.EVAL_COPILOT_TIMEOUT_MS ?? 600000);
    closeCopilot = async () => {
      await client.stop().catch(() => undefined);
      rmSync(work, { recursive: true, force: true });
    };

    for (const id of copilotIds) {
      const model = id.slice('copilot:'.length);
      const info = models.get(model) as { supportedReasoningEfforts?: string[] } | undefined;
      if (!info) {
        await closeCopilot();
        throw new Error(`Copilot model "${model}" is not available to this account. Available: ${[...models.keys()].join(', ')}`);
      }
      const efforts = info.supportedReasoningEfforts ?? [];
      const reasoningEffort = (['high', 'medium'] as const).find((e) => efforts.includes(e));

      const json = <T>(req: JudgeRequest): Promise<T> =>
        limiter.run(async () => {
          const docs = req.documents ?? {};
          const names = Object.keys(docs);
          const docError = (name: unknown) => `unknown document "${String(name)}"; available: ${names.join(', ')}`;
          const tools = names.length
            ? [
                sdk.defineTool('list_documents', {
                  description: 'List the documents available for this judgement with their line counts.',
                  parameters: { type: 'object', properties: {}, additionalProperties: false },
                  skipPermission: true,
                  handler: async () => names.map((n) => `${n} (${docs[n].split('\n').length} lines)`).join('\n'),
                }),
                sdk.defineTool('read_document', {
                  description: 'Read a document with line numbers (at most 400 lines per call).',
                  parameters: {
                    type: 'object',
                    properties: { name: { type: 'string', enum: names }, start_line: { type: 'integer', minimum: 1 }, end_line: { type: 'integer', minimum: 1 } },
                    required: ['name'],
                    additionalProperties: false,
                  },
                  skipPermission: true,
                  handler: async (a: { name: string; start_line?: number; end_line?: number }) => (a.name in docs ? numbered(docs[a.name], a.start_line, a.end_line) : docError(a.name)),
                }),
                sdk.defineTool('search_documents', {
                  description: 'Find lines containing a text (case-insensitive) in one or all documents. Returns "document:line: text".',
                  parameters: { type: 'object', properties: { query: { type: 'string', minLength: 1, maxLength: 200 }, name: { type: 'string', enum: names } }, required: ['query'], additionalProperties: false },
                  skipPermission: true,
                  handler: async (a: { query: string; name?: string }) => {
                    if (a.name && !(a.name in docs)) return docError(a.name);
                    const q = a.query.toLowerCase();
                    const hits: string[] = [];
                    for (const n of a.name ? [a.name] : names) docs[n].split('\n').forEach((l, i) => l.toLowerCase().includes(q) && hits.push(`${n}:${i + 1}: ${l}`));
                    return hits.length ? hits.slice(0, 60).join('\n') + (hits.length > 60 ? `\n... ${hits.length - 60} more` : '') : 'no matches';
                  },
                }),
              ]
            : [];
          const system = `${req.system}

${names.length ? 'Tools: list_documents, read_document and search_documents give read-only access to the complete source document, both complete candidate documents and the project guidelines. Use them when the surrounding document matters (terminology consistency, register, what a heading or table refers to). The items to judge are in the user message.\n' : ''}Answer with a single JSON object that conforms to this JSON Schema, and nothing else (no prose, no code fences):
${JSON.stringify(req.schema)}`;
          const session = await withTimeout(client.createSession({
            model,
            reasoningEffort,
            systemMessage: { mode: 'replace', content: system },
            tools,
            availableTools: new sdk.ToolSet().addCustom('*'),
            onPermissionRequest: () => ({ kind: 'reject', feedback: 'Only the document tools are available.' }),
            infiniteSessions: { enabled: false },
          } as never), 120000, 'createSession');
          try {
            let prompt = req.user;
            let lastError = '';
            for (let attempt = 0; attempt < 3; attempt++) {
              const msg = await withTimeout(session.sendAndWait({ prompt }, timeoutMs), timeoutMs + 30000, 'sendAndWait');
              const text = (msg as { data?: { content?: string } } | undefined)?.data?.content ?? '';
              try {
                const value = parseJsonAnswer(text);
                const problem = req.validate?.(value);
                if (!problem) return value as T;
                lastError = problem;
              } catch (e) {
                lastError = (e as Error).message;
              }
              prompt = `Your answer could not be used: ${lastError}. Reply again with only the JSON object that conforms to the schema, covering every item.`;
            }
            throw new Error(`copilot:${model} returned no usable answer: ${lastError}`);
          } finally {
            await session.abort().catch(() => undefined);
            await withTimeout(session.disconnect(), 30000, 'disconnect').catch(() => undefined);
            await withTimeout(client.deleteSession(session.sessionId), 30000, 'deleteSession').catch(() => undefined);
          }
        });
      judges.push({ id, provider: 'copilot', model, family: familyOf(model), json });
    }
  }
  return { judges, close: () => closeCopilot() };
}
