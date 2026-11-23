import { useEffect, useReducer, useRef } from "react";

export type PublicationRead<T> =
  { state: "ready"; publication: T } | { state: "obsolete" };
type Reader<T> = (
  revision: string,
  signal: AbortSignal,
) => Promise<PublicationRead<T>>;
interface Entry<T> {
  abort: AbortController;
  publication?: T;
  observation: object | undefined;
  availability: string;
  conflicts: number;
  obsolete?: boolean;
  error?: string;
}

/** Own one request per wanted immutable artifact; status ticks do not refetch.
 * Entries not referenced by their current views are cancelled/released.
 */
export function usePublications<T extends { revision: string }>(
  revisions: string[],
  read: Reader<T>,
  observation: object | undefined,
  epoch: number,
) {
  const entries = useRef(new Map<string, Entry<T>>());
  const [version, redraw] = useReducer((n: number) => n + 1, 0);
  const key = JSON.stringify([...new Set(revisions)].sort());
  const availability = JSON.stringify(observation) || "";
  useEffect(() => {
    return () => {
      for (const entry of entries.current.values()) entry.abort.abort();
      entries.current.clear();
    };
  }, [read, epoch]);
  useEffect(() => {
    const wanted = new Set<string>(JSON.parse(key));
    for (const [id, entry] of entries.current) {
      if (!wanted.has(id)) {
        entry.abort.abort();
        entries.current.delete(id);
      }
    }
    for (const id of wanted) {
      const previous = entries.current.get(id);
      if (
        previous &&
        (!previous.obsolete || previous.observation === observation) &&
        !(previous.conflicts >= 3 && previous.availability !== availability)
      )
        continue;
      const entry: Entry<T> = {
        abort: new AbortController(),
        observation,
        availability,
        conflicts:
          previous?.availability === availability ? previous.conflicts : 0,
      };
      entries.current.set(id, entry);
      void read(id, entry.abort.signal)
        .then((result) => {
          if (entry.abort.signal.aborted || entries.current.get(id) !== entry)
            return;
          if (result.state === "ready") entry.publication = result.publication;
          else {
            entry.conflicts++;
            // A fresh status may advertise the same content ID after clear /
            // republish. Retry only after a new status observation, at most
            // three attempts, then expose an explicit local retry.
            entry.obsolete = entry.conflicts < 3;
            if (!entry.obsolete)
              entry.error =
                "The view changed while loading. Retry to read its current version.";
          }
          redraw();
        })
        .catch((error) => {
          if (entry.abort.signal.aborted || entries.current.get(id) !== entry)
            return;
          entry.error = String(error);
          redraw();
        });
    }
  }, [key, read, epoch, version, observation, availability]);
  return {
    get: (revision?: string) =>
      revision ? entries.current.get(revision) : undefined,
    retry: (revision: string) => {
      entries.current.get(revision)?.abort.abort();
      entries.current.delete(revision);
      // Trigger the effect even though the set of wanted IDs has not changed.
      redraw();
    },
  };
}
