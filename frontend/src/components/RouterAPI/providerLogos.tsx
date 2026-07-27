import React, { useState } from 'react';

const logoModules = import.meta.glob('/assets/providers/*', {
  eager: true,
  query: '?url',
  import: 'default',
}) as Record<string, string>;

const providerLogoMap: Record<string, string> = {};
for (const fullPath in logoModules) {
  const basename = fullPath.split('/').pop() || '';
  const dot = basename.lastIndexOf('.');
  const stem = dot > 0 ? basename.slice(0, dot).toLowerCase() : basename.toLowerCase();
  providerLogoMap[stem] = logoModules[fullPath];
  providerLogoMap[basename.toLowerCase()] = logoModules[fullPath];
}

const SLUG_ALIASES: Record<string, string[]> = {
  openai: ['oai', 'oai-r', 'oai-cc', 'open-router-openai'],
  anthropic: ['claude', 'anthropic-m'],
  google: ['gemini', 'gemini-cli', 'vertex', 'vertex-partner', 'google-pse', 'google-tts', 'gcp'],
  deepseek: ['deepseek-tui'],
  'x-ai': ['xai', 'grok', 'grok-cli', 'grok-web'],
  qwen: ['alibaba'],
  mistral: [],
  huggingface: ['hf'],
  together: [],
  replicate: [],
  fireworks: [],
  groq: [],
  openrouter: [],
  minimax: ['minimax-cn'],
  moonshot: ['kimi', 'kimi-coding', 'kimchi'],
  zhipu: ['glm', 'glm-cn'],
  alibaba: [],
  tencent: [],
  volcengine: ['byteplus', 'volcengine-ark'],
  xiaomi: ['mimo', 'mimo-free', 'xiaomi-mimo', 'xiaomi-tokenplan'],
  qveris: ['wangsu'],
  sixth: ['sixth-sense'],
  blackbox: ['black-box'],
  ollama: ['ollama-local', 'local-device'],
  azure: ['microsoft'],
  cloudflare: ['cloudflare-ai'],
  nvidia: [],
  meta: ['metaai', 'llama'],
  stability: ['stability-ai'],
  perplexity: ['perplexity-web'],
  cohere: [],
  cerebras: [],
  siliconflow: [],
  doubao: [],
  baidu: [],
  jina: ['jina-ai', 'jina-reader'],
  tavily: [],
  exa: [],
  serper: [],
  searxng: [],
  searchapi: [],
  brave: ['brave-search'],
  assemblyai: [],
  cartesia: [],
  deepgram: [],
  elevenlabs: ['edge-tts', 'aws-polly', 'playht', 'tortoise'],
  fal: ['fal-ai'],
  firecrawl: [],
  recraft: [],
  runwayml: [],
  sdwebui: ['comfyui', 'sd-webui'],
  antimatter: ['antigravity'],
  cline: ['clinepass', 'continue', 'copilot', 'codebuddy-cn', 'cursor', 'roo', 'opencode', 'opencode-go', 'openclaw'],
  codex: ['commandcode', 'iflow', 'jcode', 'kilocode', 'qoder', 'topaz'],
  hyperbolic: ['nebius'],
  voyage: ['voyage-ai'],
  inworld: [],
  hermes: ['droid'],
  nanobanana: [],
  sambanova: [],
  linkup: ['kiro'],
  codebuddy: [],
  cloudfare: [],
  youcom: ['you'],
  chutes: [],
  cerebrasm: [],
  minicpm: [],
  amp: ['ampere'],
};

function normalizeToken(s: string): string {
  return s
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
}

function findLogoByToken(token: string): string | undefined {
  if (!token) return undefined;
  const n = normalizeToken(token);
  if (providerLogoMap[n]) return providerLogoMap[n];
  const compact = n.replace(/-/g, '');
  if (providerLogoMap[compact]) return providerLogoMap[compact];
  const withExt = ['.svg', '.png', '.ico'];
  for (const ext of withExt) {
    if (providerLogoMap[n + ext]) return providerLogoMap[n + ext];
  }
  for (const stem in SLUG_ALIASES) {
    const aliases = SLUG_ALIASES[stem];
    if (stem === n || aliases.some(a => normalizeToken(a) === n)) {
      if (providerLogoMap[stem]) return providerLogoMap[stem];
      if (providerLogoMap[stem + '.svg']) return providerLogoMap[stem + '.svg'];
      if (providerLogoMap[stem + '.png']) return providerLogoMap[stem + '.png'];
      if (providerLogoMap[stem + '.ico']) return providerLogoMap[stem + '.ico'];
    }
  }
  const parts = n.split('-');
  for (let len = parts.length; len >= 1; len -= 1) {
    const sub = parts.slice(0, len).join('-');
    if (providerLogoMap[sub]) return providerLogoMap[sub];
    if (providerLogoMap[sub + '.svg']) return providerLogoMap[sub + '.svg'];
    if (providerLogoMap[sub + '.png']) return providerLogoMap[sub + '.png'];
  }
  return undefined;
}

export function resolveProviderLogo(providerId: string | null | undefined, fallbackId?: string): string | undefined {
  if (providerId) {
    const direct = findLogoByToken(providerId);
    if (direct) return direct;
  }
  if (fallbackId) {
    return findLogoByToken(fallbackId);
  }
  return undefined;
}

export interface ProviderLogoProps {
  provider?: string | null;
  displayName?: string | null;
  size?: number;
  className?: string;
  style?: React.CSSProperties;
  wrapperClass?: string;
  title?: string;
  modelName?: string;
}

export function ProviderLogo({
  provider,
  displayName,
  size = 20,
  className,
  style,
  wrapperClass,
  title,
  modelName,
}: ProviderLogoProps) {
  const [failed, setFailed] = useState(false);
  let src: string | undefined;
  if (!failed) {
    src = resolveProviderLogo(provider || null, modelName);
  }
  const letter = ((displayName || provider || modelName || '?').trim().charAt(0) || '?').toUpperCase();
  const wrapperStyles: React.CSSProperties = {
    width: size,
    height: size,
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    overflow: 'hidden',
    flexShrink: 0,
    borderRadius: 'max(20%, 4px)',
    background: 'var(--color-surface)',
    border: '1px solid var(--color-rule)',
    color: 'var(--color-ink-2)',
    ...style,
  };
  const imgSize = Math.max(6, Math.round(size * 0.72));
  return (
    <span
      className={`prov-logo ${wrapperClass || ''} ${className || ''}`}
      style={wrapperStyles}
      title={title || displayName || provider || undefined}
      aria-hidden="true"
    >
      {src ? (
        <img
          src={src}
          alt={displayName || provider || 'logo'}
          className="prov-logo-img"
          style={{ width: imgSize, height: imgSize, objectFit: 'contain' }}
          onError={() => setFailed(true)}
          draggable={false}
        />
      ) : (
        <span
          className="prov-logo-fallback"
          style={{
            fontFamily: 'var(--font-display)',
            fontWeight: 700,
            fontSize: Math.max(10, Math.round(size * 0.55)),
            lineHeight: 1,
            letterSpacing: '-0.02em',
          }}
        >
          {letter}
        </span>
      )}
    </span>
  );
}

export default ProviderLogo;
