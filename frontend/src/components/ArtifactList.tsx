import { useOutputStore } from "../stores/outputStore";

interface ArtifactListProps {
  compileId: string;
}

export function ArtifactList({ compileId }: ArtifactListProps) {
  const getArtifacts = useOutputStore((s) => s.getArtifacts);
  const artifacts = getArtifacts(compileId);

  if (artifacts.length === 0) {
    return (
      <p style={{ color: "var(--text-secondary)", fontSize: 13 }}>
        No artifacts
      </p>
    );
  }

  return (
    <div className="artifact-list">
      {artifacts.map((a) => (
        <div key={a.url} className="artifact-item">
          <a href={a.url} target="_blank" rel="noreferrer">
            {a.name}
          </a>
          {a.suffix && <span className="artifact-suffix">{a.suffix}</span>}
          {a.readable_url && (
            <a
              href={a.readable_url}
              target="_blank"
              rel="noreferrer"
              style={{ fontSize: 12, marginLeft: 4 }}
            >
              (readable)
            </a>
          )}
        </div>
      ))}
    </div>
  );
}
