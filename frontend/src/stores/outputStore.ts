import { create } from "zustand";
import type {
  ParsedOutput,
  OutputFile,
  CompileStatus,
  CompilationMetrics,
} from "../types/manifest";
import { computeStatus } from "../utils/parseJsonl";

interface OutputState {
  // Data
  parsed: ParsedOutput | null;

  // UI state
  selectedCompileId: string | null;
  loading: boolean;
  error: string | null;

  // Actions
  setParsed: (parsed: ParsedOutput) => void;
  setSelectedCompileId: (id: string | null) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;

  // Derived data helpers
  getCompileIds: () => string[];
  getArtifacts: (compileId: string) => OutputFile[];
  getStatus: (compileId: string) => CompileStatus;
  getMetrics: (compileId: string) => CompilationMetrics | null;
}

export const useOutputStore = create<OutputState>((set, get) => ({
  parsed: null,
  selectedCompileId: null,
  loading: false,
  error: null,

  setParsed: (parsed) => set({ parsed }),
  setSelectedCompileId: (id) => set({ selectedCompileId: id }),
  setLoading: (loading) => set({ loading }),
  setError: (error) => set({ error }),

  getCompileIds: () => {
    const { parsed } = get();
    return parsed?.compile_ids ?? [];
  },

  getArtifacts: (compileId: string) => {
    const { parsed } = get();
    return parsed?.compile_directory[compileId]?.artifacts ?? [];
  },

  getStatus: (compileId: string) => {
    const { parsed } = get();
    if (!parsed) return "missing";
    return computeStatus(parsed.metrics[compileId] ?? []);
  },

  getMetrics: (compileId: string) => {
    const { parsed } = get();
    if (!parsed) return null;
    const list = parsed.metrics[compileId];
    if (!list || list.length === 0) return null;
    return list[list.length - 1]; // most recent
  },
}));
