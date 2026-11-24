import { expect, type Page } from "@playwright/test";
import {
  evidencePublicationSchema,
  type EvidencePublication,
  type NativeState,
  verificationJobPublicationSchema,
} from "@meden/agent-protocol/native";

/** Read the actual exact host publication; no legacy status reconstruction. */
export async function projectEvidence(
  page: Page,
  initial?: Pick<NativeState, "project">,
) {
  for (let attempt = 0; attempt < 3; attempt++) {
    const status =
      initial ??
      (await page.evaluate(async () =>
        (
          await fetch("/api/state", {
            headers: {
              Authorization: `Bearer ${sessionStorage.getItem("meden-native-session")}`,
            },
          })
        ).json(),
      ));
    initial = undefined;
    expect(status.project.state).toBe("ready");
    const revision = status.project.revision;
    const response = await page.evaluate(async (revision) => {
      const response = await fetch(
        "/api/evidence?revision=" + encodeURIComponent(revision),
        {
          headers: {
            Authorization: `Bearer ${sessionStorage.getItem("meden-native-session")}`,
          },
        },
      );
      return { status: response.status, body: await response.json() };
    }, revision);
    if (response.status === 409) continue;
    expect(response.status).toBe(200);
    const publication = evidencePublicationSchema.parse(response.body);
    expect(publication.revision).toBe(revision);
    return publication.data;
  }
  throw new Error(
    "Evidence publication did not stabilize after three fresh observations",
  );
}
export function hasCurrentEvidence(data: EvidencePublication["data"]) {
  return data.state === "accepted" && data.evidence.state === "current";
}

/** Actual latest job observation, read independently of commands/model calls. */
export async function verificationJob(page: Page) {
  for (let attempt = 0; attempt < 3; attempt++) {
    const status = await page.evaluate(async () =>
      (
        await fetch("/api/state", {
          headers: {
            Authorization: `Bearer ${sessionStorage.getItem("meden-native-session")}`,
          },
        })
      ).json(),
    );
    if (status.verification_job.state === "idle") return undefined;
    expect(status.verification_job.state).toBe("ready");
    const revision = status.verification_job.revision;
    const response = await page.evaluate(async (revision) => {
      const response = await fetch(
        "/api/verification-job?revision=" + encodeURIComponent(revision),
        {
          headers: {
            Authorization: `Bearer ${sessionStorage.getItem("meden-native-session")}`,
          },
        },
      );
      return { status: response.status, body: await response.json() };
    }, revision);
    if (response.status === 409) continue;
    expect(response.status).toBe(200);
    const publication = verificationJobPublicationSchema.parse(response.body);
    expect(publication.revision).toBe(revision);
    expect(publication.data.context.job).toBe(status.verification_job.job);
    return publication.data;
  }
  throw new Error(
    "Verification observation did not stabilize after three fresh statuses",
  );
}
