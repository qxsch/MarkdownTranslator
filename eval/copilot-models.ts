/** Lists the GitHub Copilot models available to the signed-in account (for EVAL_JUDGES=copilot:<model>). */
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { CopilotClient } from '@github/copilot-sdk';

const work = mkdtempSync(join(tmpdir(), 'mdt-judge-'));
const client = new CopilotClient({ mode: 'empty', logLevel: 'error', workingDirectory: work, baseDirectory: join(work, 'home') });
try {
  await client.start();
  for (const m of await client.listModels()) {
    const efforts = (m as { supportedReasoningEfforts?: string[] }).supportedReasoningEfforts ?? [];
    console.log(`copilot:${m.id}${efforts.length ? `  (reasoning: ${efforts.join(', ')})` : ''}`);
  }
} finally {
  await client.stop();
  rmSync(work, { recursive: true, force: true });
}
