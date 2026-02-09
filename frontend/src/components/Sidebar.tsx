import { useOutputStore } from "../stores/outputStore";
import type { CompileStatus } from "../types/manifest";

interface SidebarProps {
  onSelectOverview: () => void;
  showOverview: boolean;
}

export function Sidebar({ onSelectOverview, showOverview }: SidebarProps) {
  const { selectedCompileId, setSelectedCompileId, getCompileIds, getStatus } =
    useOutputStore();

  const compileIds = getCompileIds();

  return (
    <div className="sidebar">
      <div className="sidebar-header">tlparse viewer</div>
      <div className="sidebar-nav">
        <button
          className={`sidebar-item ${showOverview ? "selected" : ""}`}
          onClick={() => {
            setSelectedCompileId(null);
            onSelectOverview();
          }}
        >
          Overview
        </button>
        <div className="sidebar-section">
          Compilations ({compileIds.length})
        </div>
        {compileIds.map((id) => {
          const status = getStatus(id);
          return (
            <button
              key={id}
              className={`sidebar-item ${selectedCompileId === id ? "selected" : ""}`}
              onClick={() => setSelectedCompileId(id)}
            >
              <StatusDot status={status} />
              {id}
            </button>
          );
        })}
      </div>
    </div>
  );
}

function StatusDot({ status }: { status: CompileStatus }) {
  return <span className={`status-dot ${status}`} title={status} />;
}
