import type { AppConfig } from '../config.js';
import { AzureAuth, Semaphore, postJson } from './http.js';

export interface ChatMessage {
  role: 'system' | 'user' | 'assistant';
  content: string;
}

export interface Usage {
  promptTokens: number;
  completionTokens: number;
  calls: number;
}

export class UsageMeter {
  readonly byDeployment = new Map<string, Usage>();
  add(deployment: string, prompt: number, completion: number) {
    const u = this.byDeployment.get(deployment) ?? { promptTokens: 0, completionTokens: 0, calls: 0 };
    u.promptTokens += prompt;
    u.completionTokens += completion;
    u.calls++;
    this.byDeployment.set(deployment, u);
  }
  toJSON() {
    return Object.fromEntries(this.byDeployment);
  }
}

export class TruncatedOutputError extends Error {}

interface ChatResponse {
  choices: { message: { content: string | null; refusal?: string | null }; finish_reason: string }[];
  usage?: { prompt_tokens: number; completion_tokens: number };
}

/** Azure OpenAI v1 chat completions with strict JSON-schema structured output. */
export class ChatClient {
  constructor(
    private readonly cfg: AppConfig,
    private readonly auth: AzureAuth,
    private readonly limiter: Semaphore,
    readonly usage = new UsageMeter(),
  ) {}

  get available(): boolean {
    return !!this.cfg.openaiEndpoint;
  }

  async json<T>(deployment: string, messages: ChatMessage[], schemaName: string, schema: object, reasoningEffort?: string): Promise<T> {
    if (!this.cfg.openaiEndpoint) throw new Error('AZURE_OPENAI_ENDPOINT is not configured');
    const body: Record<string, unknown> = {
      model: deployment,
      messages,
      response_format: { type: 'json_schema', json_schema: { name: schemaName, strict: true, schema } },
      max_completion_tokens: 64000,
    };
    if (reasoningEffort) body.reasoning_effort = reasoningEffort;
    const res = await this.limiter.run(async () =>
      postJson<ChatResponse>(`${this.cfg.openaiEndpoint}/openai/v1/chat/completions`, await this.auth.headers('openai'), body, this.cfg.requestTimeoutMs),
    );
    this.usage.add(deployment, res.usage?.prompt_tokens ?? 0, res.usage?.completion_tokens ?? 0);
    const choice = res.choices[0];
    if (choice.finish_reason === 'length') throw new TruncatedOutputError('model output truncated');
    if (!choice.message.content) throw new Error(`empty model response${choice.message.refusal ? `: ${choice.message.refusal}` : ''}`);
    return JSON.parse(choice.message.content) as T;
  }
}
