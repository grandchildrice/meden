import { expect, type Page } from "@playwright/test";
import {
  evidencePublicationSchema,
  type EvidencePublication,
  type NativeState,
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
