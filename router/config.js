import 'dotenv/config';
import { QVERIS_MODELS } from './providers/qveris.js';

// ── Model catalog ─────────────────────────────────────────────────────────
export const MODELS = [
  // ── Auto (Blackbox first, Sixth fallback) ──
  { id: 'deepseek-v4-flash',  displayName: 'deepseek-v4-flash',  description: 'DeepSeek V4 Flash - fast & free (auto route)', provider: 'auto',     owned_by: 'deepseek'  },
  // ── Blackbox explicit ──
  { id: 'culi-blackbox',      displayName: 'deepseek-v4-flash',  description: 'DeepSeek via Blackbox provider',               provider: 'blackbox', owned_by: 'deepseek'  },
  // ── Sixth models ──
  { id: 'claude-fable-5',     displayName: 'claude-fable-5',     description: 'Claude Fable 5 via Sixth AI',                  provider: 'sixth',    owned_by: 'anthropic' },
  { id: 'gpt-4.1-mini',       displayName: 'gpt-4.1-mini',       description: 'GPT-4.1 Mini via Sixth AI',                    provider: 'sixth',    owned_by: 'openai'    },
  // ── Qveris models (from providers/qveris.js) ──
  ...QVERIS_MODELS.map(m => ({
    id:          m.id,
    displayName: m.id,
    description: `${m.name} via Qveris (${m.provider})`,
    provider:    'qveris',
    owned_by:    m.provider.toLowerCase().replace(/\s+/g, '-'),
  })),
  // ── Aliases ──
  { id: 'gpt-4o',             displayName: 'deepseek-v4-flash',  description: 'Alias → auto',                                 provider: 'auto',     owned_by: 'culi'      },
  { id: 'gpt-4o-mini',        displayName: 'deepseek-v4-flash',  description: 'Alias → auto',                                 provider: 'auto',     owned_by: 'culi'      },
];

// Map: modelId → displayName (dùng trong stream transformer)
export const MODEL_DISPLAY = Object.fromEntries(
  MODELS.map(m => [m.id, m.displayName])
);

// Map: modelId → provider
export const MODEL_PROVIDER = Object.fromEntries(
  MODELS.map(m => [m.id, m.provider])
);

export const config = {
  port: parseInt(process.env.PORT || '4000'),
  routerApiKey: process.env.ROUTER_API_KEY || 'culi-secret-key',
  defaultProvider: process.env.DEFAULT_PROVIDER || 'auto',

  blackbox: {
    baseUrl: 'https://oi-vscode-server-985058387028.europe-west1.run.app',
    userIds: (process.env.BLACKBOX_USERID || '892955990-8351072528-8400000445-9458952030')
      .split(',')
      .map(s => s.trim())
      .filter(Boolean),
    model: 'custom/blackbox-base',
    headers: {
      'Authorization': 'Bearer xxx',
      'version': '1.1',
      'user-agent': 'Cs/JS 4.73.1',
      'x-stainless-arch': 'x64',
      'x-stainless-lang': 'js',
      'x-stainless-os': 'Windows',
      'x-stainless-package-version': '4.73.1',
      'x-stainless-runtime': 'node',
      'x-stainless-runtime-version': 'v24.18.0',
    },
  },

  sixth: {
    baseUrl: 'https://backend.withsix.co',
    signupUrl: 'https://backend.withsix.co/vs-code/auth/signupV2',
    chatPath: '/proxy/azure/openai/deployments/{model}/chat/completions',
    apiVersion: '2024-12-01-preview',
    models: ['claude-fable-5', 'gpt-5.4-mini'],
    defaultModel: 'claude-fable-5', // always use fable-5
    poolSize: parseInt(process.env.SIXTH_POOL_SIZE || '3'),
    tokenRotateThreshold: 50000,
    signupHeaders: {
      'accept': 'application/json, text/plain, */*',
      'accept-encoding': 'identity',
      'origin': 'https://app.trysixth.com',
      'referer': 'https://app.trysixth.com/',
      'user-agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/146.0.0.0 Safari/537.36',
    },
  },
};
