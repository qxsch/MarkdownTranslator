import { existsSync } from 'node:fs';

if (existsSync('.env')) process.loadEnvFile('.env');
const { buildServer } = await import('./server.js');

const { app, cfg } = await buildServer();
await app.listen({ host: '0.0.0.0', port: cfg.port });

for (const signal of ['SIGINT', 'SIGTERM'] as const) {
  process.on(signal, () => {
    app.close().then(() => process.exit(0), () => process.exit(1));
  });
}
