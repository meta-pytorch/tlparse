import { useState } from "react";
import { Sidebar } from "./Sidebar";
import { ArtifactList } from "./ArtifactList";
import { MetricsPanel } from "./MetricsPanel";
import { FailuresTable } from "./FailuresTable";
import { StackTrie } from "./StackTrie";
import { useOutputStore } from "../stores/outputStore";

type Tab = "artifacts" | "metrics" | "failures";

export function Layout() {
  const { parsed, selectedCompileId, getStatus, getMetrics } =
    useOutputStore();
  const [showOverview, setShowOverview] = useState(true);
  const [activeTab, setActiveTab] = useState<Tab>("artifacts");

  // When selectedCompileId changes via sidebar, exit overview
  const currentCompileId = selectedCompileId;
  if (currentCompileId && showOverview) {
    setShowOverview(false);
  }

  const status = currentCompileId ? getStatus(currentCompileId) : null;
  const metrics = currentCompileId ? getMetrics(currentCompileId) : null;

  return (
    <div className="app">
      <Sidebar
        onSelectOverview={() => setShowOverview(true)}
        showOverview={showOverview && !currentCompileId}
      />
      <div className="main-content">
        {showOverview && !currentCompileId ? (
          <OverviewPanel />
        ) : currentCompileId ? (
          <>
            <div className="content-header">
              <h2>{currentCompileId}</h2>
              {status && (
                <span
                  className={`status-dot ${status}`}
                  style={{ width: 10, height: 10 }}
                />
              )}
            </div>
            <div className="tabs">
              <button
                className={`tab ${activeTab === "artifacts" ? "active" : ""}`}
                onClick={() => setActiveTab("artifacts")}
              >
                Artifacts
              </button>
              <button
                className={`tab ${activeTab === "metrics" ? "active" : ""}`}
                onClick={() => setActiveTab("metrics")}
              >
                Metrics
              </button>
              <button
                className={`tab ${activeTab === "failures" ? "active" : ""}`}
                onClick={() => setActiveTab("failures")}
              >
                Failures
              </button>
            </div>
            <div className="content-body">
              {activeTab === "artifacts" && (
                <ArtifactList compileId={currentCompileId} />
              )}
              {activeTab === "metrics" && metrics && (
                <MetricsPanel
                  metrics={metrics}
                  compileId={currentCompileId}
                />
              )}
              {activeTab === "metrics" && !metrics && (
                <p
                  style={{
                    color: "var(--text-secondary)",
                    fontSize: 13,
                    padding: 16,
                  }}
                >
                  No metrics data available.
                </p>
              )}
              {activeTab === "failures" && (
                <FailuresTable
                  failures={
                    parsed?.failures.filter(
                      (f) => f.compile_id === currentCompileId
                    ) ?? []
                  }
                />
              )}
            </div>
          </>
        ) : (
          <div className="content-body">
            <p style={{ color: "var(--text-secondary)" }}>
              Select a compilation from the sidebar.
            </p>
          </div>
        )}
      </div>
    </div>
  );
}

function OverviewPanel() {
  const { parsed } = useOutputStore();

  return (
    <>
      <div className="content-header">
        <h2>Overview</h2>
      </div>
      <div className="content-body">
        {parsed && (
          <>
            <div className="overview-section">
              <h3>Failures &amp; Restarts ({parsed.failures.length})</h3>
              <FailuresTable failures={parsed.failures} />
            </div>

            {parsed.stack_trie.length > 0 && (
              <div className="overview-section">
                <h3>Stack Trie</h3>
                <StackTrie nodes={parsed.stack_trie} />
              </div>
            )}
          </>
        )}
      </div>
    </>
  );
}
