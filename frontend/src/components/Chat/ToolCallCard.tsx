import { useState } from 'react';
import { ChevronDown, ChevronRight, CheckCircle2, XCircle, Loader2, Clock } from 'lucide-react';
import './ToolCallCard.css';

type ToolCall = {
  id: string;
  name: string;
  arguments: any;
  timestamp?: number;
};

type ToolResult = {
  id: string;
  name: string;
  success: boolean;
  data: any;
  duration_ms: number;
  timestamp?: number;
};

interface ToolCallCardProps {
  toolCall: ToolCall;
  toolResult?: ToolResult;
}

// Tool icon mapping
const TOOL_ICONS: Record<string, string> = {
  filesystem: '📁',
  write_file: '✍️',
  read_file: '📖',
  list_files: '📋',
  terminal: '💻',
  web_search: '🔍',
  web_fetch: '🌐',
  graphify: '📊',
  chunk_reader: '📄',
  search_replace: '🔤',
  default: '🔧',
};

function getToolIcon(toolName: string): string {
  // Check exact match first
  if (TOOL_ICONS[toolName]) return TOOL_ICONS[toolName];
  
  // Check partial match
  for (const [key, icon] of Object.entries(TOOL_ICONS)) {
    if (toolName.includes(key)) return icon;
  }
  
  return TOOL_ICONS.default;
}

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(2)}s`;
}

function truncateString(str: string, maxLength: number = 100): string {
  if (str.length <= maxLength) return str;
  return str.substring(0, maxLength) + '...';
}

export function ToolCallCard({ toolCall, toolResult }: ToolCallCardProps) {
  const [expanded, setExpanded] = useState(false);
  
  const isRunning = !toolResult;
  const isSuccess = toolResult?.success ?? false;
  const isFailed = toolResult && !toolResult.success;

  // Get status color
  let statusClass = 'status-running';
  if (isSuccess) statusClass = 'status-success';
  if (isFailed) statusClass = 'status-error';

  return (
    <div className={`tool-call-card ${statusClass}`}>
      <div className="tool-card-header" onClick={() => setExpanded(!expanded)}>
        <div className="tool-card-left">
          <button className="expand-btn" aria-label={expanded ? 'Collapse' : 'Expand'}>
            {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
          </button>
          <span className="tool-icon-badge">{getToolIcon(toolCall.name)}</span>
          <div className="tool-info">
            <div className="tool-name">{toolCall.name}</div>
            {toolResult && (
              <div className="tool-duration">
                <Clock size={10} />
                {formatDuration(toolResult.duration_ms)}
              </div>
            )}
          </div>
        </div>
        <div className="tool-card-right">
          {isRunning && (
            <div className="status-badge status-running">
              <Loader2 size={12} className="spin-icon" />
              Running
            </div>
          )}
          {isSuccess && (
            <div className="status-badge status-success">
              <CheckCircle2 size={12} />
              Success
            </div>
          )}
          {isFailed && (
            <div className="status-badge status-error">
              <XCircle size={12} />
              Failed
            </div>
          )}
        </div>
      </div>

      {expanded && (
        <div className="tool-card-body">
          {/* Arguments */}
          <div className="tool-section">
            <div className="section-label">Arguments</div>
            <pre className="code-block">
              {JSON.stringify(toolCall.arguments, null, 2)}
            </pre>
          </div>

          {/* Result */}
          {toolResult && (
            <div className="tool-section">
              <div className="section-label">
                {toolResult.success ? 'Result' : 'Error'}
              </div>
              <pre className={`code-block ${toolResult.success ? '' : 'error-block'}`}>
                {typeof toolResult.data === 'string'
                  ? toolResult.data
                  : JSON.stringify(toolResult.data, null, 2)}
              </pre>
            </div>
          )}

          {/* Metadata */}
          <div className="tool-metadata">
            <span className="meta-item">ID: {truncateString(toolCall.id, 12)}</span>
            {toolResult && (
              <span className="meta-item">Duration: {formatDuration(toolResult.duration_ms)}</span>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
