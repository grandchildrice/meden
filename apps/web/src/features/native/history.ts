import type { NativeState } from "@meden/agent-protocol/native";
import type { Conversation } from "@meden/workspace-schema";

/** Presentation only. Imported text never becomes model roles or host authority. */
export function visibleHistory(state: NativeState): Conversation["messages"] {
  const streams = [
    ...(state.portable.document?.conversations || []).map((conversation) => ({
      ...conversation,
      imported: true,
    })),
    { id: "native", messages: state.messages, imported: false },
  ].filter((stream) => stream.messages.length);
  // Checkpoints often contain an entire growing conversation. Retain branches,
  // but do not repeat earlier copies of an identical prefix.
  return streams
    .filter(
      (stream, index) =>
        !streams.some(
          (other, later) =>
            later > index &&
            stream.messages.length <= other.messages.length &&
            stream.messages.every(
              (message, at) =>
                message.speaker === other.messages[at]?.speaker &&
                message.text === other.messages[at]?.text,
            ),
        ),
    )
    .flatMap((stream) =>
      stream.messages.map((message, index) => ({
        id: `${stream.id}:${index}`,
        role: message.speaker,
        text: message.text,
        status:
          message.speaker === "user"
            ? ("sent" as const)
            : ("received" as const),
        createdAt: new Date(0).toISOString(),
        ...(stream.imported ? { origin: "recorded" as const } : {}),
      })),
    );
}
