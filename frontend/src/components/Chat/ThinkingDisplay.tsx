import { useState } from 'react';
import { ChevronDown, ChevronRight, Brain } from 'lucide-react';

interface ThinkingDisplayProps {
  thinking: string;
  tokensUsed?: number;
}

export function ThinkingDisplay({ thinking, tokensUsed }: ThinkingDisplayProps) {
  const [expanded, setExpanded] = useState(false);

  if (!thinking || thinking.trim().length === 0) return null;

  const lines = thinking.split('\n').filter(l => l.trim().length > 0);
  const preview = lines[0]?.slice(0, 80) + (lines[0]?.length > 80 ? '...' : '');

  return (
    <div style={{
      marginBottom: 12,
      border: '1px solid var(--color-rule)',
      borderRadius: 8,
      background: 'var(--color-paper-2)',
      overflow: 'hidden',
    }}>
      {/* Header */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 8,
          padding: '8px 12px',
          cursor: 'pointer',
          userSelect: 'none',
          background: 'var(--color-surface)',
          borderBottom: expanded ? '1px solid var(--color-rule)' : 'none',
        }}
        onClick={() => setExpanded(!expanded)}
      >
        {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
        <Brain size={14} style={{ color: 'var(--color-accent)' }} />
        <span style={{ fontSize: 11, fontWeight: 600, color: 'var(--color-ink-2)' }}>
          Reasoning Process
        </span>
        {tokensUsed && (
          <span style={{ fontSize: 10, color: 'var(--color-muted)', marginLeft: 'auto' }}>
            {tokensUsed} tokens
          </span>
        )}
        {!expanded && (
          <span style={{ fontSize: 10, color: 'var(--color-muted)', fontStyle: 'italic', marginLeft: expanded ? 'auto' : 8, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
            {preview}
          </span>
        )}
      </div>

      {/* Content */}
      {expanded && (
        <div style={{
          padding: '12px 16px',
          fontSize: 11,
          lineHeight: 1.6,
          color: 'var(--color-ink-2)',
          fontFamily: 'var(--font-mono)',
          whiteSpace: 'pre-wrap',
          maxHeight: 400,
          overflowY: 'auto',
        }}>
          {thinking}
        </div>
      )}
    </div>
  );
}
