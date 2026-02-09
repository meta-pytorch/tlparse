import { useEffect } from "react";
import { useOutputStore } from "../stores/outputStore";
import { parseRawJsonl } from "../utils/parseJsonl";
import type { CompileDirectory } from "../types/manifest";

/**
 * Hook that loads tlparse output data.
 * Priority:
 *   1. Inline data (injected by make_viewer.sh, works with file://)
 *   2. fetch raw.jsonl + compile_directory.json (requires HTTP server)
 */
export function useManifest() {
  const { parsed, loading, error, setParsed, setLoading, setError } =
    useOutputStore();

  useEffect(() => {
    let cancelled = false;

    async function loadData() {
      setLoading(true);
      setError(null);

      // 1. Try inline data (works with file:// protocol)
      const rawEl = document.getElementById("tlparse-raw-jsonl");
      const dirEl = document.getElementById("tlparse-compile-directory");
      if (rawEl && dirEl) {
        const rawText = rawEl.textContent?.trim();
        const dirText = dirEl.textContent?.trim();
        if (rawText && rawText.length > 2 && dirText && dirText.length > 2) {
          try {
            const compileDir: CompileDirectory = JSON.parse(dirText);
            const result = parseRawJsonl(rawText, compileDir);
            if (!cancelled) {
              setParsed(result);
              setLoading(false);
              return;
            }
          } catch {
            // Invalid inline data, continue to fetch
          }
        }
      }

      // 2. Try fetching both files (requires server)
      try {
        const [rawRes, dirRes] = await Promise.all([
          fetch("raw.jsonl"),
          fetch("compile_directory.json"),
        ]);
        if (rawRes.ok && dirRes.ok) {
          const rawText = await rawRes.text();
          const compileDir: CompileDirectory = await dirRes.json();
          const result = parseRawJsonl(rawText, compileDir);
          if (!cancelled) {
            setParsed(result);
            setLoading(false);
            return;
          }
        }
      } catch {
        // Not available
      }

      if (!cancelled) {
        setError(
          "Could not load tlparse output. " +
            "Run scripts/make_viewer.sh <output_dir> to create a standalone viewer, " +
            "or serve the output directory with an HTTP server."
        );
        setLoading(false);
      }
    }

    loadData();
    return () => {
      cancelled = true;
    };
  }, [setParsed, setLoading, setError]);

  return { parsed, loading, error };
}
