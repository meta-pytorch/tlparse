import type {
  RawJsonlEntry,
  CompileDirectory,
  CompilationMetrics,
  Failure,
  StackTrieNode,
  FrameSummary,
  CompileStatus,
  ParsedOutput,
} from "../types/manifest";

function formatCompileId(entry: RawJsonlEntry): string {
  const parts: string[] = [];
  if (entry.compiled_autograd_id != null) {
    parts.push(`!${entry.compiled_autograd_id}/`);
  }
  const fid = entry.frame_id ?? "-";
  const fcid = entry.frame_compile_id ?? "-";
  let s = `[${parts.join("")}${fid}/${fcid}`;
  if (entry.attempt != null && entry.attempt !== 0) {
    s += `_${entry.attempt}`;
  }
  s += "]";
  return s;
}

function computeStatus(metricsList: CompilationMetrics[]): CompileStatus {
  if (metricsList.length === 0) return "missing";
  if (metricsList.some((m) => m.fail_type != null)) return "error";
  if (metricsList.some((m) => (m.graph_op_count ?? 0) === 0)) return "empty";
  if (
    metricsList.some(
      (m) =>
        m.restart_reasons != null &&
        m.restart_reasons.length > 0
    )
  )
    return "break";
  return "ok";
}

function simplifyFilename(filename: string): string {
  const parts = filename.split("#link-tree/");
  if (parts.length > 1) return parts[1];
  const re = /[^/]+-seed-nspid[^/]+\//;
  const match = re.exec(filename);
  if (match) return filename.slice(match.index + match[0].length);
  return filename;
}

function insertIntoTrie(
  root: StackTrieNode[],
  stack: FrameSummary[],
  compileId: string,
  status: CompileStatus,
  stringTable: (string | null)[]
): void {
  let children = root;

  for (const frame of stack) {
    const filename = stringTable[frame.filename] ?? "(unknown)";
    const simplified = simplifyFilename(filename);

    let existing = children.find(
      (n) => n.filename === simplified && n.line === frame.line && n.name === frame.name
    );
    if (!existing) {
      existing = {
        name: frame.name,
        filename: simplified,
        line: frame.line,
        loc: frame.loc ?? null,
        terminal: [],
        terminal_statuses: [],
        children: [],
      };
      children.push(existing);
    }
    children = existing.children;
  }

  // The last node in the path gets the terminal compile ID
  // Walk back up to find the leaf we just inserted into
  let leaf = root;
  for (const frame of stack) {
    const filename = stringTable[frame.filename] ?? "(unknown)";
    const simplified = simplifyFilename(filename);
    const node = leaf.find(
      (n) => n.filename === simplified && n.line === frame.line && n.name === frame.name
    );
    if (node) leaf = node.children;
  }

  // Find the last node (parent of current leaf children)
  let current = root;
  let lastNode: StackTrieNode | null = null;
  for (const frame of stack) {
    const filename = stringTable[frame.filename] ?? "(unknown)";
    const simplified = simplifyFilename(filename);
    const node = current.find(
      (n) => n.filename === simplified && n.line === frame.line && n.name === frame.name
    );
    if (node) {
      lastNode = node;
      current = node.children;
    }
  }
  if (lastNode) {
    lastNode.terminal.push(compileId);
    lastNode.terminal_statuses.push(status);
  }
}

/**
 * Parse raw.jsonl content and compile_directory.json into a structured ParsedOutput.
 */
export function parseRawJsonl(
  rawJsonl: string,
  compileDirectory: CompileDirectory
): ParsedOutput {
  const lines = rawJsonl.split("\n").filter((l) => l.trim().length > 0);

  let stringTable: (string | null)[] = [];
  const metrics: Record<string, CompilationMetrics[]> = {};
  const failures: Failure[] = [];
  const stacks: { compileId: string; stack: FrameSummary[] }[] = [];

  for (const line of lines) {
    let entry: RawJsonlEntry & { string_table?: (string | null)[] };
    try {
      entry = JSON.parse(line);
    } catch {
      continue;
    }

    // First line is the string table
    if (entry.string_table) {
      stringTable = entry.string_table;
      continue;
    }

    const compileId = formatCompileId(entry);

    // Extract dynamo_start stacks for the trie
    if (entry.dynamo_start?.stack) {
      stacks.push({ compileId, stack: entry.dynamo_start.stack });
    }

    // Extract compilation metrics
    if (entry.compilation_metrics) {
      const m = entry.compilation_metrics;
      if (!metrics[compileId]) metrics[compileId] = [];
      metrics[compileId].push(m);

      // Extract failures
      if (m.restart_reasons) {
        for (const reason of m.restart_reasons) {
          failures.push({
            compile_id: compileId,
            failure_type: "RestartAnalysis",
            reason,
          });
        }
      }
      if (m.fail_type) {
        failures.push({
          compile_id: compileId,
          failure_type: m.fail_type,
          reason: m.fail_reason ?? "",
        });
      }
    }
  }

  // Build stack trie
  const stackTrie: StackTrieNode[] = [];
  for (const { compileId, stack } of stacks) {
    const status = computeStatus(metrics[compileId] ?? []);
    // Reverse stack so root (outermost frame) comes first
    const reversed = [...stack].reverse();
    insertIntoTrie(stackTrie, reversed, compileId, status, stringTable);
  }

  // Ordered compile IDs (from compile_directory.json preserves insertion order)
  const compileIds = Object.keys(compileDirectory);

  return {
    string_table: stringTable,
    compile_directory: compileDirectory,
    metrics,
    failures,
    stack_trie: stackTrie,
    compile_ids: compileIds,
  };
}

export { computeStatus };
