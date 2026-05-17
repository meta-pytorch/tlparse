import type { Failure } from "../types/manifest";

interface FailuresTableProps {
  failures: Failure[];
}

export function FailuresTable({ failures }: FailuresTableProps) {
  if (failures.length === 0) {
    return (
      <p style={{ color: "var(--text-secondary)", fontSize: 13, padding: 16 }}>
        No failures or restarts.
      </p>
    );
  }

  return (
    <table className="failures-table">
      <thead>
        <tr>
          <th>Compile ID</th>
          <th>Type</th>
          <th>Reason</th>
        </tr>
      </thead>
      <tbody>
        {failures.map((f, i) => (
          <tr key={i}>
            <td style={{ fontFamily: "var(--font-mono)", fontSize: 12 }}>
              {f.compile_id}
            </td>
            <td>{f.failure_type}</td>
            <td>
              <pre>{f.reason}</pre>
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
