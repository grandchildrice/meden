import { useEffect, useMemo, useState } from "react";
import type { WorkspaceOverview } from "@meden/agent-protocol/native";
import {
  BrowserMapLayouts,
  type PreparedMapLayout,
} from "../../adapters/map-layouts";
import { layoutInput } from "./project-map-layout";

/** One layout request per original inventory. Symbols arriving in lazy chunks
 * do not enter this lifecycle, and old completions cannot replace a new input.
 */
export function useMapLayout(original: WorkspaceOverview | undefined) {
  const store = useMemo(() => new BrowserMapLayouts(), []);
  const input = useMemo(() => {
    if (!original) return undefined;
    try {
      return { request: layoutInput(original) };
    } catch (error) {
      return { error: String(error) };
    }
  }, [original]);
  const [value, setValue] = useState<{
    input: string;
    prepared: PreparedMapLayout;
  }>();
  const [failure, setFailure] = useState<{ input: string; message: string }>();
  useEffect(() => {
    if (!input?.request || value?.input === input.request.text) return;
    let current = true;
    const request = input.request;
    void store
      .prepare(request)
      .then((prepared) => {
        if (current) {
          setValue({ input: request.text, prepared });
          setFailure(undefined);
        }
      })
      .catch((error) => {
        if (current)
          setFailure({ input: request.text, message: String(error) });
      });
    return () => {
      current = false;
    };
  }, [store, input, value?.input]);
  const prepared =
    input?.request && value?.input === input.request.text
      ? value.prepared
      : undefined;
  useEffect(() => {
    if (!prepared || prepared.source !== "computed") return;
    // Persist after a visible commit; optional storage must not hold the map.
    if ("requestIdleCallback" in window) {
      const task = window.requestIdleCallback(() => {
        void store.retain(prepared);
      });
      return () => window.cancelIdleCallback(task);
    }
    const timer = setTimeout(() => {
      void store.retain(prepared);
    }, 0);
    return () => clearTimeout(timer);
  }, [store, prepared]);
  return {
    prepared,
    error:
      input?.error ||
      (input?.request && failure?.input === input.request.text
        ? failure.message
        : undefined),
    loading: Boolean(
      input?.request && !prepared && failure?.input !== input.request.text,
    ),
  };
}
