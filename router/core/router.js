/**
 * CuliRouter - Load balancer + failover logic
 *
 * Strategy "auto":
 *   1. Thử Blackbox trước (nhanh, không cần account)
 *   2. Nếu Blackbox fail → failover sang Sixth
 *   3. Nếu cả 2 fail → trả lỗi 503
 *
 * Strategy "blackbox": chỉ dùng Blackbox
 * Strategy "sixth"   : chỉ dùng Sixth
 *
 * Stream pipeline:
 *   Provider trả về raw fetch Response → router pipe SSE chunks
 *   → transform reasoning_content thành content (Sixth quirk)
 *   → forward tới Express Response
 */

import { blackboxProvider } from '../providers/blackbox.js';
import { sixthProvider } from '../providers/sixth.js';
import { qverisProvider } from '../providers/qveris.js';
import { config, MODEL_DISPLAY, MODEL_PROVIDER } from '../config.js';

// SSE transformer:
// - Bỏ qua tất cả chunks thuần reasoning_content (thinking phase)
// - Chỉ emit chunks khi content thật bắt đầu xuất hiện
// - Rewrite field "model" thành displayName client-facing
async function* transformStream(nodeStream, displayModel) {
  let buffer = '';
  let contentStarted = false;

  for await (const chunk of nodeStream) {
    buffer += chunk.toString('utf-8');
    const lines = buffer.split('\n');
    buffer = lines.pop();

    for (const line of lines) {
      const trimmed = line.trim();
      if (!trimmed) continue;

      if (trimmed === 'data: [DONE]') {
        yield 'data: [DONE]\n\n';
        continue;
      }

      if (!trimmed.startsWith('data: ')) {
        yield line + '\n';
        continue;
      }

      try {
        const parsed = JSON.parse(trimmed.slice(6));
        const choices = parsed.choices || [];

        const mapped = choices.map(ch => {
          const delta = ch.delta || {};
          const content = delta.content;
          const role    = delta.role;

          // Role chunk (assistant intro) → luôn giữ
          if (role && (content === '' || content === null || content === undefined)) {
            return { ...ch, delta: { role } };
          }

          if (content !== null && content !== undefined) {
            if (content !== '') contentStarted = true;
            return { ...ch, delta: { role: delta.role, content } };
          }
          // Chunk chỉ có reasoning_content → bỏ qua
          return { ...ch, delta: { content: null } };
        });

        const hasUseful = mapped.some(ch =>
          ch.delta.role !== undefined || ch.delta.content !== null
        );
        if (!hasUseful && !contentStarted) continue;

        // Rewrite model name → displayName
        const out = JSON.stringify({
          ...parsed,
          model: displayModel,
          choices: mapped,
        });
        yield `data: ${out}\n\n`;

      } catch (_) {
        yield line + '\n';
      }
    }
  }

  if (buffer.trim()) yield buffer + '\n';
}

// Pipe raw fetch Response body vào Express res
async function pipeStream(fetchResp, expressRes, provider, displayModel) {
  return new Promise((resolve, reject) => {
    expressRes.setHeader('Content-Type', 'text/event-stream; charset=utf-8');
    expressRes.setHeader('Cache-Control', 'no-cache');
    expressRes.setHeader('Connection', 'keep-alive');
    expressRes.setHeader('X-Culi-Provider', provider);
    expressRes.setHeader('X-Culi-Model', displayModel);
    expressRes.flushHeaders();

    const body = fetchResp.body;
    const gen = transformStream(body, displayModel);

    (async () => {
      try {
        for await (const chunk of gen) {
          if (!expressRes.writableEnded) expressRes.write(chunk);
        }
        if (!expressRes.writableEnded) {
          expressRes.write('data: [DONE]\n\n');
          expressRes.end();
        }
        resolve();
      } catch (err) {
        if (!expressRes.writableEnded) expressRes.end();
        reject(err);
      }
    })();

    // Client disconnect
    expressRes.on('close', () => {
      body.destroy?.();
      resolve();
    });
  });
}

// Collect SSE stream từ provider thành 1 JSON response duy nhất (non-streaming mode)
// Kích hoạt khi client gửi "stream": false (chuẩn OpenAI)
async function collectStream(fetchResp, expressRes, provider, displayModel) {
  const body = fetchResp.body;
  const gen = transformStream(body, displayModel);

  let id = `chatcmpl-${Date.now()}`;
  let created = Math.floor(Date.now() / 1000);
  let role = 'assistant';
  let content = '';
  let finishReason = 'stop';
  let usage = null;

  try {
    for await (const sseLine of gen) {
      const trimmed = sseLine.trim();
      if (!trimmed.startsWith('data: ') || trimmed === 'data: [DONE]') continue;
      try {
        const parsed = JSON.parse(trimmed.slice(6));
        if (parsed.id) id = parsed.id;
        if (parsed.created) created = parsed.created;
        if (parsed.usage) usage = parsed.usage;
        const ch = parsed.choices?.[0];
        if (ch) {
          if (ch.delta?.role) role = ch.delta.role;
          if (ch.delta?.content) content += ch.delta.content;
          if (ch.finish_reason) finishReason = ch.finish_reason;
        }
      } catch (_) { /* bỏ qua chunk parse lỗi */ }
    }
  } finally {
    body.destroy?.();
  }

  expressRes.setHeader('X-Culi-Provider', provider);
  expressRes.setHeader('X-Culi-Model', displayModel);
  expressRes.json({
    id,
    object: 'chat.completion',
    created,
    model: displayModel,
    choices: [{
      index: 0,
      message: { role, content },
      finish_reason: finishReason,
    }],
    usage: usage || { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 },
  });
}

class CuliRouter {
  constructor() {
    this.providers = {
      blackbox: blackboxProvider,
      sixth:    sixthProvider,
      qveris:   qverisProvider,
    };
    this.requestCount  = 0;
    this.successCount  = 0;
    this.failoverCount = 0;
  }

  /**
   * Route một chat request tới provider phù hợp và pipe stream về expressRes
   * @param {object} payload   - OpenAI-compatible request body
   * @param {object} expressRes - Express Response object
   * @param {string} [forceProvider] - 'blackbox' | 'sixth' | undefined
   */
  async route(payload, expressRes, forceProvider) {
    this.requestCount++;
    const strategy = forceProvider || config.defaultProvider;
    const requestedModel = payload.model || 'culi-auto';

    // Resolve display name: dùng catalog nếu có, fallback theo provider
    const resolveDisplay = (provider) => {
      if (MODEL_DISPLAY[requestedModel]) return MODEL_DISPLAY[requestedModel];
      if (provider === 'sixth') return 'claude-fable-5';
      return 'deepseek-v4-flash';
    };

    // Client gửi "stream": false → gom stream trả về 1 JSON duy nhất (chuẩn OpenAI)
    // Mặc định (stream=true hoặc không gửi) → giữ nguyên SSE stream như cũ
    const respond = async (fetchResp, providerName) => {
      const display = resolveDisplay(providerName);
      if (payload.stream === false) {
        await collectStream(fetchResp, expressRes, providerName, display);
      } else {
        await pipeStream(fetchResp, expressRes, providerName, display);
      }
    };

    // ── Single provider mode ──────────────────────────────────────────────
    if (strategy === 'blackbox' || strategy === 'sixth' || strategy === 'qveris') {
      const provider = this.providers[strategy];
      const resp = await provider.chat(payload);
      if (resp) {
        this.successCount++;
        await respond(resp, strategy);
        return;
      }
      return this._sendError(expressRes, 503, `Provider ${strategy} unavailable: ${provider.lastError}`);
    }

    // ── Auto mode: Blackbox first, Sixth fallback ─────────────────────────
    console.log(`[Router] #${this.requestCount} → trying blackbox...`);
    const bbResp = await blackboxProvider.chat(payload);

    if (bbResp) {
      this.successCount++;
      console.log(`[Router] #${this.requestCount} ✅ blackbox`);
      await respond(bbResp, 'blackbox');
      return;
    }

    // Blackbox failed → failover
    this.failoverCount++;
    console.warn(`[Router] #${this.requestCount} ⚠️ blackbox failed (${blackboxProvider.lastError}), failover → sixth`);

    const sixthResp = await sixthProvider.chat(payload);
    if (sixthResp) {
      this.successCount++;
      console.log(`[Router] #${this.requestCount} ✅ sixth (failover)`);
      await respond(sixthResp, 'sixth');
      return;
    }

    // Cả 2 đều fail
    console.error(`[Router] #${this.requestCount} ❌ both providers failed`);
    return this._sendError(expressRes, 503, 'All providers unavailable', {
      blackbox: blackboxProvider.lastError,
      sixth: sixthProvider.lastError,
    });
  }

  _sendError(res, status, message, details = {}) {
    if (res.headersSent) return;
    res.status(status).json({
      error: {
        message,
        type: 'router_error',
        code: status,
        details,
      },
    });
  }

  stats() {
    return {
      router: {
        requestCount: this.requestCount,
        successCount: this.successCount,
        failoverCount: this.failoverCount,
        successRate: this.requestCount
          ? `${((this.successCount / this.requestCount) * 100).toFixed(1)}%`
          : '0%',
      },
      providers: {
        blackbox: blackboxProvider.status(),
        sixth:   sixthProvider.status(),
        qveris:  qverisProvider.status(),
      },
    };
  }
}

export const router = new CuliRouter();
