/**
 * Blackbox Provider
 * - Free, không cần auth thật (Bearer xxx)
 * - Rotate userid khi gặp lỗi 401/403
 * - Pipe SSE stream trực tiếp về client
 */

import fetch from 'node-fetch';
import { config } from '../config.js';

const { baseUrl, userIds, model, headers: baseHeaders } = config.blackbox;

class BlackboxProvider {
  constructor() {
    this.name = 'blackbox';
    this.userIdIndex = 0;
    this.failCount = 0;
    this.totalRequests = 0;
    this.totalErrors = 0;
    this.lastError = null;
    this.healthy = true;
  }

  getCurrentUserId() {
    return userIds[this.userIdIndex % userIds.length];
  }

  rotateUserId() {
    this.userIdIndex = (this.userIdIndex + 1) % userIds.length;
    console.log(`[Blackbox] Rotated to userid index ${this.userIdIndex}: ${this.getCurrentUserId()}`);
  }

  _buildHeaders() {
    return {
      'Content-Type': 'application/json',
      'accept': 'application/json',
      'accept-encoding': 'identity',
      ...baseHeaders,
      'userid': this.getCurrentUserId(),
    };
  }

  _normalizeMessages(messages) {
    // Blackbox chỉ cần messages array chuẩn OpenAI
    return messages.map(m => ({
      role: m.role,
      content: typeof m.content === 'string' ? m.content : JSON.stringify(m.content),
    }));
  }

  /**
   * Gọi Blackbox và trả về Response object (stream)
   * @param {object} payload - OpenAI-compatible request body
   * @returns {{ stream: ReadableStream, headers: object } | null}
   */
  async chat(payload) {
    const body = {
      model: model,
      messages: this._normalizeMessages(payload.messages),
      max_tokens: payload.max_tokens || 4096,
      stream: true,
      ...(payload.temperature !== undefined && { temperature: payload.temperature }),
      ...(payload.top_p !== undefined && { top_p: payload.top_p }),
    };

    this.totalRequests++;
    const url = `${baseUrl}/chat/completions`;

    try {
      const resp = await fetch(url, {
        method: 'POST',
        headers: this._buildHeaders(),
        body: JSON.stringify(body),
        signal: AbortSignal.timeout(30000),
      });

      if (resp.status === 401 || resp.status === 403) {
        this.rotateUserId();
        this.failCount++;
        this.lastError = `HTTP ${resp.status}`;
        return null;
      }

      if (!resp.ok) {
        const text = await resp.text();
        this.failCount++;
        this.lastError = `HTTP ${resp.status}: ${text.slice(0, 100)}`;
        console.error(`[Blackbox] Error ${resp.status}: ${this.lastError}`);
        return null;
      }

      this.failCount = 0;
      this.healthy = true;
      return resp; // trả về raw fetch Response để pipe stream

    } catch (err) {
      this.failCount++;
      this.lastError = err.message;
      this.totalErrors++;
      console.error(`[Blackbox] Request failed: ${err.message}`);
      return null;
    }
  }

  status() {
    return {
      name: this.name,
      healthy: this.healthy,
      currentUserId: this.getCurrentUserId(),
      userIdIndex: this.userIdIndex,
      totalUserIds: userIds.length,
      totalRequests: this.totalRequests,
      totalErrors: this.totalErrors,
      failCount: this.failCount,
      lastError: this.lastError,
    };
  }
}

export const blackboxProvider = new BlackboxProvider();
