import { CheckCircle2, XCircle, Clock, Loader2 } from 'lucide-react';
import { ToolCallCard } from './ToolCallCard';
import './ToolExecutionPanel.css';

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

interface ToolExecutionPanelProps {
  toolCalls?: ToolCall[];
  toolResults?: ToolResult[];
}

export function ToolExecutionPanel({ toolCalls = [], toolResults = [] }: ToolExecutionPanelProps) {
  if (!toolCalls.length && !toolResults.length) {
    return null;
  }

  // Create a map of results by tool call ID
  const resultsMap = new Map(toolResults.map(r => [r.id, r]));

  // Calculate statistics
  const totalCalls = toolCalls.length;
  const completedCalls = toolResults.length;
  const successfulCalls = toolResults.filter(r => r.success).length;
  const failedCalls = toolResults.filter(r => !r.success).length;
  const inProgressCalls = totalCalls - completedCalls;

  return (
    <div className="tool-execution-panel">
      <div className="tool-panel-header">
        <div className="tool-panel-title">
          <span className="tool-icon">🔧</span>
          <span>Tool Execution</span>
        </div>
        <div className="tool-panel-stats">
          {inProgressCalls > 0 && (
            <span className="stat-badge stat-progress">
              <Loader2 size={12} className="spin-icon" />
              {inProgressCalls} running
            </span>
          )}
          {successfulCalls > 0 && (
            <span className="stat-badge stat-success">
              <CheckCircle2 size={12} />
              {successfulCalls} success
            </span>
          )}
          {failedCalls > 0 && (
            <span className="stat-badge stat-error">
              <XCircle size={12} />
              {failedCalls} failed
            </span>
          )}
        </div>
      </div>

      <div className="tool-panel-body">
        {toolCalls.map((call) => {
          const result = resultsMap.get(call.id);
          return (
            <ToolCallCard
              key={call.id}
              toolCall={call}
              toolResult={result}
            />
          );
        })}
      </div>
    </div>
  );
}
