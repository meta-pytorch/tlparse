import { useState } from "react";
import type { StackTrieNode as StackTrieNodeType } from "../types/manifest";
import { useOutputStore } from "../stores/outputStore";

interface StackTrieProps {
  nodes: StackTrieNodeType[];
}

export function StackTrie({ nodes }: StackTrieProps) {
  if (nodes.length === 0) {
    return (
      <p style={{ color: "var(--text-secondary)", fontSize: 13 }}>
        No stack trie data available.
      </p>
    );
  }

  return (
    <div className="stack-trie">
      <ul>
        {nodes.map((node, i) => (
          <StackTrieItem
            key={i}
            node={node}
            siblingCount={nodes.length}
          />
        ))}
      </ul>
    </div>
  );
}

/**
 * Renders a single trie node. Mirrors the original Rust fmt_inner logic:
 * - If this node is at a branch point (siblingCount > 1), render with
 *   indentation inside a nested <ul> and a clickable +/- collapse marker.
 * - If this node is on a linear path (siblingCount == 1), render flat
 *   as a simple <li> with no extra nesting or marker.
 */
function StackTrieItem({
  node,
  siblingCount,
}: {
  node: StackTrieNodeType;
  siblingCount: number;
}) {
  const [collapsed, setCollapsed] = useState(false);
  const setSelectedCompileId = useOutputStore((s) => s.setSelectedCompileId);

  const frameLabel = `${node.filename}:${node.line} in ${node.name}`;
  const locSuffix = node.loc ? `    ${node.loc}` : "";

  const terminals = node.terminal.map((compileId, i) => {
    if (!compileId) return null;
    const status = node.terminal_statuses[i] || "missing";
    return (
      <span
        key={i}
        className={`trie-compile-id ${status}`}
        onClick={() => setSelectedCompileId(compileId)}
        style={{ marginRight: 4, cursor: "pointer" }}
      >
        {compileId}
      </span>
    );
  });

  const isBranch = siblingCount > 1;
  const hasChildren = node.children.length > 0;

  if (isBranch) {
    // Branch point: render with nested <ul>, indentation, and collapse marker
    return (
      <li>
        <span>
          {hasChildren && (
            <span
              className={`marker${collapsed ? " collapsed" : ""}`}
              onClick={() => setCollapsed(!collapsed)}
            />
          )}
          {terminals}
          <span className="trie-frame">
            {frameLabel}
            {locSuffix}
          </span>
        </span>
        {hasChildren && !collapsed && (
          <ul>
            {node.children.map((child, i) => (
              <StackTrieItem
                key={i}
                node={child}
                siblingCount={node.children.length}
              />
            ))}
          </ul>
        )}
      </li>
    );
  } else {
    // Linear path: render flat, no extra nesting, no marker
    return (
      <>
        <li>
          {terminals}
          <span className="trie-frame">
            {frameLabel}
            {locSuffix}
          </span>
        </li>
        {node.children.map((child, i) => (
          <StackTrieItem
            key={i}
            node={child}
            siblingCount={node.children.length}
          />
        ))}
      </>
    );
  }
}
