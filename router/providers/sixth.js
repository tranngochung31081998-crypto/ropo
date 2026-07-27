/**
 * Sixth AI Provider
 * - Auto tạo account mới khi pool cạn
 * - Rotate token khi gặp lỗi 401/402/429
 * - Persist account pool vào file JSON
 * - Reasoning content được merge vào content stream
 */

import fetch from 'node-fetch';
import { readFileSync, writeFileSync, existsSync } from 'fs';
import { config } from '../config.js';

const { baseUrl, signupUrl, chatPath, apiVersion, models, defaultModel, poolSize, tokenRotateThreshold, signupHeaders } = config.sixth;

const POOL_FILE = new URL('../data/sixth_pool.json', import.meta.url).pathname.replace(/^\/([A-Z]:)/, '$1');

// ---- helpers ----
function randomStr(len, chars) {
  return Array.from({ length: len }, () => chars[Math.floor(Math.random() * chars.length)]).join('');
}

function genEmail() {
  const first = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'[Math.floor(Math.random() * 26)];
  const rest = randomStr(
    Math.floor(Math.random() * 8) + 10,
    'abcdefghijklmnopqrstuvwxyz0123456789'
  );
  return `${first}${rest}@gmail.com`;
}

function genPassword() {
  const upper = randomStr(2, 'ABCDEFGHIJKLMNOPQRSTUVWXYZ');
  const lower = randomStr(4, 'abcdefghijklmnopqrstuvwxyz');
  const digits = randomStr(3, '0123456789');
  const chars = (upper + lower + digits).split('').sort(() => Math.random() - 0.5).join('');
  return chars + '@';
}

// ---- Account pool persistence ----
function loadPool() {
  try {
    if (existsSync(POOL_FILE)) {
      return JSON.parse(readFileSync(POOL_FILE, 'utf-8'));
    }
  } catch (_) {}
  return [];
}

function savePool(pool) {
  try {
    // Tạo thư mục data nếu chưa có
    const dir = POOL_FILE.replace(/[/\\][^/\\]+$/, '');
    if (!existsSync(dir)) {
      import('fs').then(fs => fs.mkdirSync(dir, { recursive: true })).catch(() => {});
    }
    writeFileSync(POOL_FILE, JSON.stringify(pool, null, 2), 'utf-8');
  } catch (err) {
    console.error('[Sixth] Failed to save pool:', err.message);
  }
}

// ---- Provider class ----
class SixthProvider {
  constructor() {
    this.name = 'sixth';
    this.pool = loadPool(); // [{ email, password, uid, token, usedTokens, dead }]
    this.currentIndex = 0;
    this.totalRequests = 0;
    this.totalErrors = 0;
    this.lastError = null;
    this.healthy = true;
    console.log(`[Sixth] Pool loaded: ${this.pool.length} accounts`);
  }

  // Trả về account đang active (chưa dead)
  _activeAccounts() {
    return this.pool.filter(a => !a.dead);
  }

  _currentAccount() {
    const active = this._activeAccounts();
    if (active.length === 0) return null;
    return active[this.currentIndex % active.length];
  }

  _rotateAccount(reason = '') {
    const active = this._activeAccounts();
    if (active.length === 0) return;
    this.currentIndex = (this.currentIndex + 1) % active.length;
    const next = this._currentAccount();
    console.log(`[Sixth] Rotated account (${reason}) -> ${next?.email}`);
  }

  _markAccountDead(account) {
    account.dead = true;
    savePool(this.pool);
    console.warn(`[Sixth] Account marked dead: ${account.email}`);
  }

  // Tạo account mới qua signup API
  async _createAccount() {
    const email = genEmail();
    const password = genPassword();
    console.log(`[Sixth] Creating new account: ${email}`);

    try {
      const resp = await fetch(signupUrl, {
        method: 'POST',
        headers: { ...signupHeaders, 'Content-Type': 'application/json' },
        body: JSON.stringify({ email, password, auth_provider: 'email' }),
        signal: AbortSignal.timeout(15000),
      });

      if (resp.status !== 201 && resp.status !== 200) {
        const text = await resp.text();
        console.error(`[Sixth] Signup failed ${resp.status}: ${text.slice(0, 100)}`);
        return null;
      }

      const data = await resp.json();
      const token = data?.access_token?.access_token;
      if (!token) {
        console.error('[Sixth] Signup: no token in response');
        return null;
      }

      const account = {
        email,
        password,
        uid: data.uid,
        token,
        usedTokens: 0,
        dead: false,
        createdAt: new Date().toISOString(),
      };

      this.pool.push(account);
      savePool(this.pool);
      console.log(`[Sixth] ✅ New account created: ${email} (uid: ${data.uid})`);
      return account;

    } catch (err) {
      console.error(`[Sixth] Signup error: ${err.message}`);
      return null;
    }
  }

  // Đảm bảo pool có ít nhất 1 account sẵn sàng
  async _ensurePool() {
    const active = this._activeAccounts();
    if (active.length < poolSize) {
      const needed = poolSize - active.length;
      console.log(`[Sixth] Pool low (${active.length}/${poolSize}), creating ${needed} accounts...`);
      const creates = Array.from({ length: needed }, () => this._createAccount());
      await Promise.allSettled(creates);
    }
    return this._activeAccounts().length > 0;
  }

  _buildHeaders(token) {
    return {
      'Content-Type': 'application/json',
      'accept': '*/*',
      'accept-encoding': 'identity',
      'Authorization': `Bearer ${token}`,
      'user-agent': 'node',
      'x-sixth-total-cached': '0',
      'x-sixth-total-input': '0',
      'x-sixth-total-output': '0',
    };
  }

  _buildUrl(modelName) {
    const m = models.includes(modelName) ? modelName : defaultModel;
    return `${baseUrl}${chatPath.replace('{model}', m)}?api-version=${apiVersion}`;
  }

  /**
   * Gọi Sixth API và trả về raw fetch Response (stream)
   * Tự động rotate/create account khi cần
   */
  async chat(payload) {
    // Đảm bảo pool có account
    const ready = await this._ensurePool();
    if (!ready) {
      console.error('[Sixth] Pool empty and failed to create accounts');
      return null;
    }

    // Thử tối đa pool.length lần (mỗi lần với account khác nhau)
    const maxAttempts = Math.max(this._activeAccounts().length, 1) + 1;

    for (let attempt = 0; attempt < maxAttempts; attempt++) {
      const account = this._currentAccount();
      if (!account) break;

      // Sixth luôn dùng claude-fable-5 bất kể client request model gì
      const modelName = defaultModel;

      const body = {
        model: modelName,
        messages: payload.messages,
        max_tokens: payload.max_tokens || 2048,  // minimum 2048 để qua reasoning phase
        stream: true,
        ...(payload.temperature !== undefined && { temperature: payload.temperature }),
        ...(payload.top_p !== undefined && { top_p: payload.top_p }),
      };

      this.totalRequests++;
      const url = this._buildUrl(modelName);

      try {
        const resp = await fetch(url, {
          method: 'POST',
          headers: this._buildHeaders(account.token),
          body: JSON.stringify(body),
          signal: AbortSignal.timeout(60000),
        });

        // Token hết hạn / unauthorized -> rotate
        if (resp.status === 401 || resp.status === 403) {
          console.warn(`[Sixth] Account ${account.email} got ${resp.status}, rotating...`);
          this._markAccountDead(account);
          await this._ensurePool();
          this._rotateAccount(`${resp.status}`);
          continue;
        }

        // Rate limit -> rotate
        if (resp.status === 429) {
          console.warn(`[Sixth] Account ${account.email} rate limited, rotating...`);
          this._rotateAccount('429');
          continue;
        }

        if (!resp.ok) {
          const text = await resp.text();
          this.lastError = `HTTP ${resp.status}: ${text.slice(0, 100)}`;
          this.totalErrors++;
          console.error(`[Sixth] Error ${resp.status}: ${this.lastError}`);
          return null;
        }

        // Cập nhật token usage ước tính
        account.usedTokens = (account.usedTokens || 0) + (payload.max_tokens || 1000);
        if (tokenRotateThreshold > 0 && account.usedTokens >= tokenRotateThreshold) {
          console.log(`[Sixth] Account ${account.email} reached token threshold, scheduling rotate`);
          // Rotate sau request này (không block stream)
          setImmediate(() => this._rotateAccount('threshold'));
        }
        savePool(this.pool);

        this.healthy = true;
        return resp; // raw fetch Response để pipe stream

      } catch (err) {
        this.lastError = err.message;
        this.totalErrors++;
        console.error(`[Sixth] Request error (${account.email}): ${err.message}`);
        this._rotateAccount('error');
      }
    }

    this.healthy = false;
    return null;
  }

  status() {
    const active = this._activeAccounts();
    return {
      name: this.name,
      healthy: this.healthy,
      poolSize: this.pool.length,
      activeAccounts: active.length,
      currentAccount: active[this.currentIndex % Math.max(active.length, 1)]?.email || null,
      totalRequests: this.totalRequests,
      totalErrors: this.totalErrors,
      lastError: this.lastError,
    };
  }
}

export const sixthProvider = new SixthProvider();
