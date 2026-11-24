import { useCallback } from "react";
import type {
  NativeState,
  VerificationJobPublication,
} from "@meden/agent-protocol/native";
import { NativeClient, NativeHttpError } from "../../adapters/native-client";
import { usePublications, useRetainedPublication } from "./usePublications";

/** The same immutable query/lifecycle serves conversation and explicit host work.
 * A host observation never replaces or starts conversation activity.
 */
export function useCheckProgress(
  client: NativeClient,
  status: NativeState["verification_job"] | undefined,
  epoch: number,
) {
  const revision = status?.state === "ready" ? status.revision : undefined;
  const job = status?.state === "ready" ? status.job : undefined;
  const role = status?.state === "ready" ? status.role : undefined;
  const read = useCallback(
    async (revision: string, signal: AbortSignal) => {
      if (!job || !role) throw new Error("Check context is unavailable");
      try {
        return {
          state: "ready" as const,
          publication: await client.verificationJob(
            revision,
            job,
            role,
            signal,
          ),
        };
      } catch (error) {
        if (error instanceof NativeHttpError && error.status === 409)
          return { state: "obsolete" as const };
        throw error;
      }
    },
    [client, job, role],
  );
  const reads = usePublications<VerificationJobPublication>(
    revision ? [revision] : [],
    read,
    status,
    epoch,
  );
  const data = useRetainedPublication(
    reads.get(revision)?.publication?.data,
    job,
  );
  return {
    data,
    error:
      status?.state === "unavailable"
        ? status.message
        : reads.get(revision)?.error,
    loading: Boolean(revision),
    retry: revision ? () => reads.retry(revision) : undefined,
  };
}
