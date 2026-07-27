/**
 * Qveris Provider
 * Supports 2 capability endpoints:
 *  - Wangsu (wangsu.aigateway.chat.create.v1) → OpenAI chat.completions format
 *  - OpenRouter (openrouter.responses.create.v1) → Responses API format
 *
 * Both return JSON (non-stream). This provider converts the response
 * into a synthetic SSE stream so the router can pipe it uniformly.
 *
 * API: POST https://qveris.ai/api/v1/tools/execute?tool_id={tool_id}
 * Auth: Authorization: Bearer <QVERIS_API_KEY>
 */

import fetch from 'node-fetch';
import { config } from '../config.js';

const QVERIS_BASE = 'https://qveris.ai/api/v1/tools/execute';

const CAPABILITY = {
  wangsu:     'wangsu.aigateway.chat.create.v1.eab6b8e4',
  openrouter: 'openrouter.responses.create.v1.7fb39b2c',
};

// Models and which capability to use
// source: 'wangsu' | 'openrouter'
export const QVERIS_MODELS = [
  // Wangsu models (8)
  { id: 'deepseek-r1',                    name: 'DeepSeek R1',         source: 'wangsu',      provider: 'DeepSeek'   },
  { id: 'deepseek-v3.2',                  name: 'DeepSeek V3.2',       source: 'wangsu',      provider: 'DeepSeek'   },
  { id: 'gpt-5.2',                        name: 'GPT-5.2',             source: 'wangsu',      provider: 'OpenAI'     },
  { id: 'gpt-4.1',                        name: 'GPT-4.1',             source: 'wangsu',      provider: 'OpenAI'     },
  { id: 'claude-3-7',                     name: 'Claude 3.7',          source: 'wangsu',      provider: 'Anthropic'  },
  { id: 'claude-opus-4-5',                name: 'Claude Opus 4.5',     source: 'wangsu',      provider: 'Anthropic'  },
  { id: 'gemini-2.5-flash-image',         name: 'Gemini 2.5 Flash',    source: 'wangsu',      provider: 'Google'     },
  { id: 'gemini-3-pro-image-preview',     name: 'Gemini 3 Pro',        source: 'wangsu',      provider: 'Google'     },
  // OpenRouter models (18)
  { id: 'openai/gpt-5.6-luna',            name: 'GPT-5.6 Luna',        source: 'openrouter',  provider: 'OpenAI'     },
  { id: 'openai/gpt-5.6-terra',           name: 'GPT-5.6 Terra',       source: 'openrouter',  provider: 'OpenAI'     },
  { id: 'openai/gpt-5.6-sol',             name: 'GPT-5.6 Sol',         source: 'openrouter',  provider: 'OpenAI'     },
  { id: 'anthropic/claude-opus-4.8',      name: 'Claude Opus 4.8',     source: 'openrouter',  provider: 'Anthropic'  },
  { id: 'anthropic/claude-sonnet-5',      name: 'Claude Sonnet 5',     source: 'openrouter',  provider: 'Anthropic'  },
  { id: 'anthropic/claude-fable-5',       name: 'Claude Fable 5',      source: 'openrouter',  provider: 'Anthropic'  },
  { id: 'deepseek/deepseek-v4-pro',       name: 'DeepSeek V4 Pro',     source: 'openrouter',  provider: 'DeepSeek'   },
  { id: 'deepseek/deepseek-v4-flash',     name: 'DeepSeek V4 Flash',   source: 'openrouter',  provider: 'DeepSeek'   },
  { id: 'google/gemini-3.1-pro-preview',  name: 'Gemini 3.1 Pro',      source: 'openrouter',  provider: 'Google'     },
  { id: 'google/gemini-3.1-flash-lite',   name: 'Gemini 3.1 Flash Lite', source: 'openrouter', provider: 'Google'   },
  { id: 'x-ai/grok-4.5',                  name: 'Grok 4.5',            source: 'openrouter',  provider: 'xAI'        },
  { id: 'qwen/qwen3.7-plus',              name: 'Qwen 3.7 Plus',       source: 'openrouter',  provider: 'Qwen'       },
  { id: 'moonshotai/kimi-k3',             name: 'Kimi K3',             source: 'openrouter',  provider: 'Moonshot AI'},
  { id: 'moonshotai/kimi-k2.6',           name: 'Kimi K2.6',           source: 'openrouter',  provider: 'Moonshot AI'},
  { id: 'moonshotai/kimi-k2.7-code',      name: 'Kimi K2.7 Code',      source: 'openrouter',  provider: 'Moonshot AI'},
  { id: 'z-ai/glm-5.2',                   name: 'GLM 5.2',             source: 'openrouter',  provider: 'Z.ai'       },
  { id: 'minimax/minimax-m3',             name: 'MiniMax M3',          source: 'openrouter',  provider: 'MiniMax'    },
  { id: 'xiaomi/mimo-v2.5-pro',           name: 'MiMo V2.5 Pro',       source: 'openrouter',  provider: 'Xiaomi'     },
];

// Lookup map: modelId → source
const MODEL_SOURCE = Object.fromEntries(QVERIS_MODELS.map(m => [m.id, m.source]));

// Convert Qveris JSON response → SSE string (synthetic stream)
function toSSE(content, modelId, usage = {}) {
  const id = `qveris-${Date.now()}`;
  const created = Math.floor(Date.now() / 1000);

  // Chunk 1: role
  const c1 = JSON.stringify({
    id, object: 'chat.completion.chunk', created, model: modelId,
    choices: [{ index: 0, delta: { role: 'assistant', content: '' }, finish_reason: null }],
  });

  // Chunk 2+: content tokens (split by words to simulate streaming)
  const words = content.split(/(?<=\s)/);
  const contentChunks = words.map(w => JSON.stringify({
    id, object: 'chat.completion.chunk', created, model: modelId,
    choices: [{ index: 0, delta: { content: w }, finish_reason: null }],
  }));

  // Final chunk with usage
  const cFinal = JSON.stringify({
    id, object: 'chat.completion.chunk', created, model: modelId,
    choices: [{ index: 0, delta: {}, finish_reason: 'stop' }],
    usage: {
      prompt_tokens: usage.prompt_tokens || usage.input_tokens || 0,
      completion_tokens: usage.completion_tokens || usage.output_tokens || 0,
      total_tokens: usage.total_tokens || 0,
    },
  });

  const lines = [
    `data: ${c1}`,
    ...contentChunks.map(c => `data: ${c}`),
    `data: ${cFinal}`,
    'data: [DONE]',
    '',
  ];
  return lines.join('\n\n');
}

class QverisProvider {
  constructor() {
    this.name = 'qveris';
    this.apiKey = process.env.QVERIS_API_KEY || '';
    this.totalRequests = 0;
    this.totalErrors = 0;
    this.lastError = null;
    this.healthy = true;
    this.remainingCredits = null;

    if (!this.apiKey) {
      console.warn('[Qveris] ⚠️  No QVERIS_API_KEY set in .env');
    } else {
      console.log('[Qveris] Provider initialized ✅');
    }
  }

  _getSource(modelId) {
    // Check explicit map first
    if (MODEL_SOURCE[modelId]) return MODEL_SOURCE[modelId];
    // Heuristic: contains '/' likely openrouter
    if (modelId.includes('/')) return 'openrouter';
    return 'wangsu';
  }

  async _callWangsu(modelId, messages, temperature) {
    const url = `${QVERIS_BASE}?tool_id=${CAPABILITY.wangsu}`;
    const body = {
      search_id: 'culi-router',
      parameters: {
        model: modelId,
        messages,
        ...(temperature !== undefined && { temperature }),
      },
      max_response_size: 65536, // lớn hơn để nhận đủ stream
    };

    const resp = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${this.apiKey}`,
      },
      body: JSON.stringify(body),
      signal: AbortSignal.timeout(60000),
    });

    if (!resp.ok) {
      const text = await resp.text();
      throw new Error(`Wangsu HTTP ${resp.status}: ${text.slice(0, 100)}`);
    }

    const json = await resp.json();

    if (!json.success) {
      throw new Error(`Wangsu call failed: ${JSON.stringify(json).slice(0, 100)}`);
    }

    if (json.remaining_credits !== undefined) {
      this.remainingCredits = json.remaining_credits;
    }

    // Wangsu trả về SSE stream dạng string trong result.data
    // hoặc truncated + download URL
    let rawSse = '';

    if (json.result?.full_content_file_url) {
      // Truncated - cần download từ URL
      const dlResp = await fetch(json.result.full_content_file_url, {
        signal: AbortSignal.timeout(30000),
      });
      if (!dlResp.ok) throw new Error(`Wangsu download failed: ${dlResp.status}`);
      rawSse = await dlResp.text();
    } else {
      rawSse = typeof json.result?.data === 'string'
        ? json.result.data
        : JSON.stringify(json.result?.data || '');
    }

    // Parse SSE lines → extract content + reasoning
    let content = '';
    let usage   = {};

    for (const line of rawSse.split('\n')) {
      const t = line.trim();
      if (!t.startsWith('data: ') || t === 'data: [DONE]') continue;
      try {
        const chunk  = JSON.parse(t.slice(6));
        const choice = chunk.choices?.[0];
        if (!choice) continue;
        const delta = choice.delta || {};
        if (delta.content) content += delta.content;
        if (chunk.usage && chunk.usage.total_tokens) usage = chunk.usage;
      } catch (_) {}
    }

    return { content, modelId, usage };
  }

  async _callOpenRouter(modelId, messages, temperature) {
    const url = `${QVERIS_BASE}?tool_id=${CAPABILITY.openrouter}`;

    // Convert messages to OpenRouter Responses API format
    const input = messages.map(m => ({ role: m.role, content: m.content }));
    // Extract system message as instructions
    const sysMsg = messages.find(m => m.role === 'system');
    const userMsgs = messages.filter(m => m.role !== 'system');

    const body = {
      search_id: 'culi-router',
      parameters: {
        model: modelId,
        input: userMsgs.map(m => ({ role: m.role, content: m.content })),
        ...(sysMsg && { instructions: sysMsg.content }),
        ...(temperature !== undefined && { temperature }),
      },
      max_response_size: 20480,
    };

    const resp = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${this.apiKey}`,
      },
      body: JSON.stringify(body),
      signal: AbortSignal.timeout(60000),
    });

    if (!resp.ok) {
      const text = await resp.text();
      throw new Error(`OpenRouter HTTP ${resp.status}: ${text.slice(0, 100)}`);
    }

    const json = await resp.json();

    if (!json.success) {
      throw new Error(`OpenRouter call failed: ${JSON.stringify(json).slice(0, 100)}`);
    }

    if (json.remaining_credits !== undefined) {
      this.remainingCredits = json.remaining_credits;
    }

    const data = json.result?.data;
    if (!data) throw new Error('OpenRouter: no result.data');

    // Parse Responses API output
    let content = '';
    for (const item of data.output || []) {
      for (const ci of item.content || []) {
        if (ci.type === 'output_text' && ci.text) content += ci.text;
      }
    }

    const usage = data.usage || {};
    return { content, modelId: data.model || modelId, usage };
  }

  /**
   * Chat với Qveris - trả về ReadableStream SSE synthetic
   */
  async chat(payload) {
    if (!this.apiKey) {
      this.lastError = 'No API key configured';
      return null;
    }

    const modelId = payload.model || 'deepseek/deepseek-v4-flash';
    const messages = payload.messages || [];
    const temperature = payload.temperature;
    const source = this._getSource(modelId);

    this.totalRequests++;
    console.log(`[Qveris] Calling ${source} model: ${modelId}`);

    try {
      let result;
      if (source === 'wangsu') {
        result = await this._callWangsu(modelId, messages, temperature);
      } else {
        result = await this._callOpenRouter(modelId, messages, temperature);
      }

      if (!result.content) {
        this.lastError = 'Empty response from Qveris';
        this.totalErrors++;
        return null;
      }

      console.log(`[Qveris] ✅ ${source} → ${result.content.length} chars (credits: ${this.remainingCredits})`);
      this.healthy = true;

      // Bọc content thành synthetic SSE stream
      const sseBody = toSSE(result.content, result.modelId, result.usage);
      const readable = Buffer.from(sseBody, 'utf-8');

      // Trả về object có interface giống fetch Response
      return {
        ok: true,
        status: 200,
        body: {
          [Symbol.asyncIterator]: async function* () {
            yield readable;
          },
          on: (event, handler) => {
            if (event === 'data') handler(readable);
            if (event === 'end') setTimeout(handler, 0);
          },
          destroy: () => {},
        },
        _isSynthetic: true,
        _provider: `qveris-${source}`,
      };

    } catch (err) {
      this.lastError = err.message;
      this.totalErrors++;
      this.healthy = false;
      console.error(`[Qveris] Error: ${err.message}`);
      return null;
    }
  }

  status() {
    return {
      name: this.name,
      healthy: this.healthy,
      hasApiKey: !!this.apiKey,
      remainingCredits: this.remainingCredits,
      modelCount: QVERIS_MODELS.length,
      totalRequests: this.totalRequests,
      totalErrors: this.totalErrors,
      lastError: this.lastError,
    };
  }
}

export const qverisProvider = new QverisProvider();
