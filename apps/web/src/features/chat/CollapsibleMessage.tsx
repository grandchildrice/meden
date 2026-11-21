import {
  useId,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { useAuiState } from "@assistant-ui/react";

/** Fold rendered height, including code blocks and narrow sidebar wrapping. */
export function CollapsibleMessage({ children }: { children: ReactNode }) {
  const id = useId();
  const content = useRef<HTMLDivElement>(null);
  const [overflow, setOverflow] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const text = useAuiState((state) =>
    state.message.content
      .filter((part) => part.type === "text")
      .map((part) => part.text)
      .join("\n"),
  );
  useLayoutEffect(() => {
    const element = content.current;
    if (!element) return;
    const measure = () =>
      setOverflow(element.getBoundingClientRect().height > 280);
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  const collapsed = overflow && !expanded;
  return (
    <div className="collapsible-message">
      <div
        id={id}
        className={"message-body" + (collapsed ? " is-collapsed" : "")}
      >
        <div ref={content} inert={collapsed ? true : undefined}>
          {children}
        </div>
      </div>
      {collapsed && <span className="sr-only">{text.slice(0, 400)}…</span>}
      {overflow && (
        <button
          className="message-expand"
          aria-expanded={!collapsed}
          aria-controls={id}
          onClick={() => setExpanded(!expanded)}
        >
          {collapsed ? "Show more" : "Show less"}
        </button>
      )}
    </div>
  );
}
