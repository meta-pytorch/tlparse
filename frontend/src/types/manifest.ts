// TypeScript types mirroring the tlparse output format

export interface OutputFile {
  url: string;
  name: string;
  number: number;
  suffix: string;
  readable_url: string | null;
}

export interface CompileDirectoryEntry {
  artifacts: OutputFile[];
}

// compile_directory.json: Record<compileIdString, CompileDirectoryEntry>
export type CompileDirectory = Record<string, CompileDirectoryEntry>;

// Types extracted from raw.jsonl entries

export interface FrameSummary {
  filename: number; // index into string_table
  line: number;
  name: string;
  loc?: string | null;
}

export interface CompilationMetrics {
  co_name?: string | null;
  co_filename?: string | null;
  co_firstlineno?: number | null;
  cache_size?: number | null;
  accumulated_cache_size?: number | null;
  guard_count?: number | null;
  shape_env_guard_count?: number | null;
  graph_op_count?: number | null;
  graph_node_count?: number | null;
  graph_input_count?: number | null;
  start_time?: number | null;
  entire_frame_compile_time_s?: number | null;
  backend_compile_time_s?: number | null;
  inductor_compile_time_s?: number | null;
  code_gen_time_s?: number | null;
  fail_type?: string | null;
  fail_reason?: string | null;
  fail_user_frame_filename?: string | null;
  fail_user_frame_lineno?: number | null;
  non_compliant_ops?: string[] | null;
  compliant_custom_ops?: string[] | null;
  restart_reasons?: string[] | null;
  dynamo_time_before_restart_s?: number | null;
}

// A raw.jsonl entry (only the fields we care about)
export interface RawJsonlEntry {
  // Compile ID fields
  frame_id?: number;
  frame_compile_id?: number;
  attempt?: number;
  compiled_autograd_id?: number;

  // Entry types (mutually exclusive)
  dynamo_start?: { stack?: FrameSummary[] };
  compilation_metrics?: CompilationMetrics;
  dynamo_output_graph?: unknown;
  dynamo_guards?: unknown;

  // Payload info
  payload_filename?: string;
  has_payload?: string;
}

export type CompileStatus = "ok" | "error" | "empty" | "break" | "missing";

export interface Failure {
  compile_id: string;
  failure_type: string;
  reason: string;
}

export interface StackTrieNode {
  name: string;
  filename: string;
  line: number;
  loc: string | null;
  terminal: string[];
  terminal_statuses: CompileStatus[];
  children: StackTrieNode[];
}

// Parsed state derived from raw.jsonl + compile_directory.json
export interface ParsedOutput {
  string_table: (string | null)[];
  compile_directory: CompileDirectory;
  metrics: Record<string, CompilationMetrics[]>; // compile_id -> metrics list
  failures: Failure[];
  stack_trie: StackTrieNode[];
  compile_ids: string[]; // ordered list
}
