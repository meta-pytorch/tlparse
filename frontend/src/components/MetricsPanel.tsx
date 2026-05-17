import type { CompilationMetrics } from "../types/manifest";

interface MetricsPanelProps {
  metrics: CompilationMetrics;
  compileId: string;
}

export function MetricsPanel({ metrics, compileId }: MetricsPanelProps) {
  return (
    <div>
      <h3 style={{ marginBottom: 12, fontSize: 14 }}>
        Compilation Metrics for {compileId}
      </h3>
      <table className="metrics-table">
        <tbody>
          <MetricRow label="Function" value={metrics.co_name} />
          <MetricRow label="File" value={metrics.co_filename} />
          <MetricRow label="First Line" value={metrics.co_firstlineno} />
          <MetricRow
            label="Total Compile Time"
            value={formatTime(metrics.entire_frame_compile_time_s)}
          />
          <MetricRow
            label="Backend Compile Time"
            value={formatTime(metrics.backend_compile_time_s)}
          />
          <MetricRow
            label="Inductor Compile Time"
            value={formatTime(metrics.inductor_compile_time_s)}
          />
          <MetricRow
            label="Code Gen Time"
            value={formatTime(metrics.code_gen_time_s)}
          />
          <MetricRow label="Guard Count" value={metrics.guard_count} />
          <MetricRow
            label="Shape Env Guard Count"
            value={metrics.shape_env_guard_count}
          />
          <MetricRow label="Graph Op Count" value={metrics.graph_op_count} />
          <MetricRow label="Graph Node Count" value={metrics.graph_node_count} />
          <MetricRow label="Graph Input Count" value={metrics.graph_input_count} />
          <MetricRow label="Cache Size" value={metrics.cache_size} />
          <MetricRow
            label="Accumulated Cache Size"
            value={metrics.accumulated_cache_size}
          />
          {metrics.fail_type && (
            <>
              <MetricRow label="Failure Type" value={metrics.fail_type} isError />
              <MetricRow label="Failure Reason" value={metrics.fail_reason} isError />
              <MetricRow
                label="User Frame"
                value={
                  metrics.fail_user_frame_filename
                    ? `${metrics.fail_user_frame_filename}:${metrics.fail_user_frame_lineno}`
                    : null
                }
                isError
              />
            </>
          )}
          {metrics.restart_reasons && metrics.restart_reasons.length > 0 && (
            <MetricRow
              label="Restart Reasons"
              value={metrics.restart_reasons.join(", ")}
            />
          )}
          {metrics.dynamo_time_before_restart_s != null && (
            <MetricRow
              label="Dynamo Time Before Restart"
              value={formatTime(metrics.dynamo_time_before_restart_s)}
            />
          )}
          {metrics.non_compliant_ops && metrics.non_compliant_ops.length > 0 && (
            <MetricRow
              label="Non-Compliant Ops"
              value={metrics.non_compliant_ops.join(", ")}
              isError
            />
          )}
          {metrics.compliant_custom_ops &&
            metrics.compliant_custom_ops.length > 0 && (
              <MetricRow
                label="Compliant Custom Ops"
                value={metrics.compliant_custom_ops.join(", ")}
              />
            )}
        </tbody>
      </table>
    </div>
  );
}

function MetricRow({
  label,
  value,
  isError,
}: {
  label: string;
  value: string | number | null | undefined;
  isError?: boolean;
}) {
  if (value == null) return null;
  return (
    <tr>
      <th>{label}</th>
      <td style={isError ? { color: "var(--status-error)" } : undefined}>
        {String(value)}
      </td>
    </tr>
  );
}

function formatTime(seconds: number | null | undefined): string | null {
  if (seconds == null) return null;
  if (seconds < 0.001) return `${(seconds * 1_000_000).toFixed(0)} \u00b5s`;
  if (seconds < 1) return `${(seconds * 1000).toFixed(1)} ms`;
  return `${seconds.toFixed(3)} s`;
}
