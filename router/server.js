/**
 * CulirouterAPI Server
 * OpenAI-compatible endpoint: POST /v1/chat/completions
 *
 * Headers:
 *   Authorization: Bearer <ROUTER_API_KEY>   (optional nếu không set)
 *   X-Provider: blackbox | sixth             (optional, override provider)
 *
 * Response: SSE stream (text/event-stream)
 *   X-Culi-Provider: blackbox | sixth        (header cho biết provider đã dùng)
 */

import 'dotenv/config';
import express from 'express';
import { router } from './core/router.js';
import { config, MODELS, MODEL_PROVIDER } from './config.js';

const app = express();
app.use(express.json({ limit: '10mb' }));

// ── CORS ──────────────────────────────────────────────────────────────────
app.use((req, res, next) => {
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type, Authorization, X-Provider');
  if (req.method === 'OPTIONS') return res.sendStatus(204);
  next();
});

// ── Auth middleware (optional) ─────────────────────────────────────────────
function authMiddleware(req, res, next) {
  // Nếu không set ROUTER_API_KEY thì bỏ qua auth
  if (!config.routerApiKey || config.routerApiKey === 'culi-secret-key') {
    return next();
  }
  const auth = req.headers['authorization'] || '';
  const token = auth.startsWith('Bearer ') ? auth.slice(7) : auth;
  if (token !== config.routerApiKey) {
    return res.status(401).json({ error: { message: 'Invalid API key', code: 401 } });
  }
  next();
}

// ── Routes ────────────────────────────────────────────────────────────────

// Health check
app.get('/health', (req, res) => {
  res.json({
    status: 'ok',
    service: 'culi-router-api',
    uptime: process.uptime(),
    timestamp: new Date().toISOString(),
  });
});

// Stats / dashboard
app.get('/stats', (req, res) => {
  res.json(router.stats());
});

// Models list (OpenAI-compatible) - từ catalog trong config.js
app.get('/v1/models', authMiddleware, (req, res) => {
  const now = Math.floor(Date.now() / 1000);
  const data = MODELS.map(m => ({
    id: m.id,
    object: 'model',
    created: now,
    owned_by: m.owned_by,
    display_name: m.displayName,
    description: m.description,
    provider: m.provider,
  }));
  res.json({ object: 'list', data });
});

// ── Main chat endpoint ─────────────────────────────────────────────────────
app.post('/v1/chat/completions', authMiddleware, async (req, res) => {
  const payload = req.body;

  // Validate
  if (!payload?.messages || !Array.isArray(payload.messages) || payload.messages.length === 0) {
    return res.status(400).json({
      error: { message: 'messages array is required', code: 400 },
    });
  }

  // Xác định provider từ:
  // 1. Header X-Provider
  // 2. Model catalog (MODEL_PROVIDER map)
  // 3. Legacy prefixes: culi-blackbox, culi-sixth, culi-auto
  let forceProvider = req.headers['x-provider']?.toLowerCase();

  if (!forceProvider) {
    const modelId = payload.model || '';
    const catalogProvider = MODEL_PROVIDER[modelId];
    if (catalogProvider && catalogProvider !== 'auto') {
      forceProvider = catalogProvider;
    } else if (modelId === 'culi-blackbox') {
      forceProvider = 'blackbox';
    } else if (modelId === 'culi-sixth') {
      forceProvider = 'sixth';
    } else if (modelId === 'culi-qveris') {
      forceProvider = 'qveris';
    }
  }

  // Log request
  const preview = payload.messages.at(-1)?.content?.toString().slice(0, 60) || '';
  console.log(`[API] POST /v1/chat/completions | provider=${forceProvider || 'auto'} | "${preview}..."`);

  try {
    await router.route(payload, res, forceProvider || undefined);
  } catch (err) {
    console.error('[API] Unhandled error:', err);
    if (!res.headersSent) {
      res.status(500).json({ error: { message: err.message, code: 500 } });
    }
  }
});

// Legacy endpoint alias
app.post('/chat/completions', authMiddleware, async (req, res) => {
  req.url = '/v1/chat/completions';
  app.handle(req, res);
});

// ── Start ─────────────────────────────────────────────────────────────────
app.listen(config.port, () => {
  console.log('');
  console.log('╔══════════════════════════════════════════════════╗');
  console.log('║         CulirouterAPI Server Started             ║');
  console.log(`║   Port    : http://localhost:${config.port}                ║`);
  console.log(`║   Provider: ${config.defaultProvider.padEnd(37)}║`);
  console.log('║   Endpoints:                                     ║');
  console.log('║     GET  /health                                 ║');
  console.log('║     GET  /stats                                  ║');
  console.log('║     GET  /v1/models                              ║');
  console.log('║     POST /v1/chat/completions                    ║');
  console.log('╚══════════════════════════════════════════════════╝');
  console.log('');
});

export default app;
