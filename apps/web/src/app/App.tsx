import {
  lazy,
  Suspense,
  useEffect,
  useRef,
  useState,
  type ChangeEvent,
} from "react";
import {
  Alert,
  Button,
  CircularProgress,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Drawer,
  IconButton,
  Snackbar,
  TextField,
  Tooltip,
  Typography,
  useMediaQuery,
} from "@mui/material";
import AddCommentOutlined from "@mui/icons-material/AddCommentOutlined";
import SearchRounded from "@mui/icons-material/SearchRounded";
import SettingsOutlined from "@mui/icons-material/SettingsOutlined";
import ViewSidebarOutlined from "@mui/icons-material/ViewSidebarOutlined";
import InsertDriveFileOutlined from "@mui/icons-material/InsertDriveFileOutlined";
import CloseRounded from "@mui/icons-material/CloseRounded";
import {
  workspaceSchema,
  type Workspace,
  type Conversation,
} from "@meden/workspace-schema";
import { BrowserWorkspaceRepository } from "../adapters/browser-workspaces";
import { readWorkspaceFile, downloadJson } from "../adapters/files";
import {
  refreshVendingGraph,
  createVendingWorkspace,
  DEMO_WORKSPACE_ID,
  DEMO_HISTORY_ID,
  demoResponse,
  titleFromInstruction,
} from "../fixtures/meden-demo";
import type { ArtifactTab } from "../features/artifacts/ArtifactPanel";
import { ArtifactCard } from "../features/chat/ArtifactCard";
import { ChatView } from "../features/chat/ChatView";
const ArtifactPanel = lazy(() =>
  import("../features/artifacts/ArtifactPanel").then((module) => ({
    default: module.ArtifactPanel,
  })),
);
const repository = new BrowserWorkspaceRepository();
const SELECTION_KEY = "meden-chat-selection";

export function App() {
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const library = useRef<Workspace[]>([]);
  const [activeId, setActiveId] = useState(DEMO_WORKSPACE_ID);
  const [conversationId, setConversationId] = useState<string | null>(null);
  const [pending, setPending] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(true);
  const [storageError, setStorageError] = useState("");
  const [notification, setNotification] = useState("");
  const [desktopSidebarOpen, setDesktopSidebarOpen] = useState(true);
  const [mobileSidebarOpen, setMobileSidebarOpen] = useState(false);
  const [searchOpen, setSearchOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [panelOpen, setPanelOpen] = useState(false);
  const [panelTab, setPanelTab] = useState<ArtifactTab>("preview");
  const [settings, setSettings] = useState(false);
  const [queueDialog, setQueueDialog] = useState(false);
  const artifactToggle = useRef<HTMLButtonElement>(null);
  const fileInput = useRef<HTMLInputElement>(null);
  const queue = useRef(Promise.resolve());
  const unsaved = useRef(new Map<string, Workspace>());
  const narrow = useMediaQuery("(max-width:1040px)");
  const mobile = useMediaQuery("(max-width:839px)");
  const workspace =
    workspaces.find((item) => item.id === activeId) || workspaces[0];
  const conversation = workspace?.conversations.find(
    (item) => item.id === conversationId,
  );
  const running = conversationId ? pending.has(conversationId) : false;
  const sidebarOpen = mobile ? mobileSidebarOpen : desktopSidebarOpen;
  const sidebarLabel = sidebarOpen
    ? "Close conversations"
    : "Open conversations";
  const artifactLabel = panelOpen ? "Close artifact" : "Open artifact";
  const hasMessages = Boolean(conversation?.messages.length);

  const select = (workspaceId: string, id: string | null) => {
    setActiveId(workspaceId);
    setConversationId(id);
    setMobileSidebarOpen(false);
    localStorage.setItem(
      SELECTION_KEY,
      JSON.stringify({ workspaceId, conversationId: id }),
    );
  };
  const load = () => {
    setLoading(true);
    repository
      .list()
      .then((values) => {
        values = values.map(refreshVendingGraph);
        const next = values.some((item) => item.id === DEMO_WORKSPACE_ID)
          ? values
          : [createVendingWorkspace(), ...values];
        library.current = next;
        setWorkspaces(next);
        try {
          const selected = JSON.parse(
            localStorage.getItem(SELECTION_KEY) || "null",
          );
          if (
            selected &&
            next.some((item) => item.id === selected.workspaceId)
          ) {
            const selectedWorkspace = next.find(
              (item) => item.id === selected.workspaceId,
            )!;
            const selectedConversation =
              selected.conversationId &&
              !selectedWorkspace.conversations.some(
                (item) => item.id === selected.conversationId,
              )
                ? selectedWorkspace.conversations[0]?.id || null
                : selected.conversationId;
            setActiveId(selected.workspaceId);
            setConversationId(selectedConversation);
            if (selectedConversation !== selected.conversationId) {
              const oldDraft = localStorage.getItem(
                "meden-input:" +
                  selected.workspaceId +
                  ":" +
                  selected.conversationId,
              );
              const key =
                "meden-input:" +
                selected.workspaceId +
                ":" +
                selectedConversation;
              if (oldDraft) {
                const current = localStorage.getItem(key);
                localStorage.setItem(
                  key,
                  [current, oldDraft].filter(Boolean).join("\n\n"),
                );
              }
              localStorage.setItem(
                SELECTION_KEY,
                JSON.stringify({
                  workspaceId: selected.workspaceId,
                  conversationId: selectedConversation,
                }),
              );
            }
          }
        } catch {
          /* An unreadable preference does not discard stored conversations. */
        }
        setStorageError("");
      })
      .catch(() =>
        setStorageError("Could not load conversations. Reload to retry."),
      )
      .finally(() => setLoading(false));
  };
  useEffect(load, []);
  const persist = (next: Workspace): Promise<boolean> => {
    const parsed = workspaceSchema.parse({
      ...refreshVendingGraph(next),
      updatedAt: new Date().toISOString(),
    });
    library.current = [
      parsed,
      ...library.current.filter((item) => item.id !== parsed.id),
    ];
    setWorkspaces(library.current);
    unsaved.current.set(parsed.id, parsed);
    const result = queue.current.then(async () => {
      try {
        await repository.save(parsed);
        if (unsaved.current.get(parsed.id) === parsed)
          unsaved.current.delete(parsed.id);
        if (!unsaved.current.size) setStorageError("");
        return true;
      } catch {
        setStorageError(
          "Could not save the conversation. Retry saving or export JSON.",
        );
        return false;
      }
    });
    queue.current = result.then(() => {});
    return result;
  };
  const retry = () => {
    if (unsaved.current.size)
      for (const next of unsaved.current.values()) void persist(next);
    else load();
  };
  const newChat = () => {
    select(DEMO_WORKSPACE_ID, null);
    setPanelOpen(false);
    setQuery("");
    setSearchOpen(false);
  };
  const openPanel = (tab: ArtifactTab) => {
    setPanelTab(tab);
    setPanelOpen(true);
  };
  const send = async (text: string) => {
    if (!workspace || !text.trim()) return;
    const target = workspace.id;
    const id =
      workspace.id === DEMO_WORKSPACE_ID
        ? DEMO_HISTORY_ID
        : conversationId || crypto.randomUUID();
    if (pending.has(id)) return;
    const original = library.current.find((item) => item.id === target)!;
    const thread = original.conversations.find((item) => item.id === id);
    const next: Conversation = {
      id,
      title: thread?.messages.length
        ? thread.title
        : titleFromInstruction(text),
      messages: [
        ...(thread?.messages || []),
        {
          id: crypto.randomUUID(),
          role: "user",
          text,
          status: "sent",
          origin: "demo",
          createdAt: new Date().toISOString(),
        },
      ],
    };
    const first = {
      ...original,
      conversations: [
        ...original.conversations.filter((item) => item.id !== id),
        next,
      ],
    };
    setPending((previous) => new Set([...previous, id]));
    select(target, id);
    openPanel(
      /Dependencies|graph|Proof|検証|verify|意味/i.test(text)
        ? "proof"
        : "preview",
    );
    await persist(first);
    await new Promise((resolve) => setTimeout(resolve, 350));
    const latest = library.current.find((item) => item.id === target);
    const current = latest?.conversations.find((item) => item.id === id);
    if (latest && current) {
      const reply = {
        id: crypto.randomUUID(),
        role: "assistant" as const,
        text: demoResponse(text, latest),
        status: "received" as const,
        origin: "demo" as const,
        createdAt: new Date().toISOString(),
      };
      await persist({
        ...latest,
        conversations: latest.conversations.map((item) =>
          item.id === id
            ? { ...item, messages: [...item.messages, reply] }
            : item,
        ),
      });
    }
    setPending((previous) => {
      const next = new Set(previous);
      next.delete(id);
      return next;
    });
  };
  const onImport = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;
    try {
      const next = await readWorkspaceFile(file);
      next.id = crypto.randomUUID();
      await persist(next);
      select(next.id, next.conversations[0]?.id || null);
      openPanel("proof");
      setNotification("Imported");
    } catch (error) {
      setNotification(error instanceof Error ? error.message : "Import failed");
    }
  };
  const threads = workspaces
    .flatMap((item) =>
      item.conversations
        .filter((thread) => thread.messages.length)
        .map((thread) => ({ workspaceId: item.id, conversation: thread })),
    )
    .sort((a, b) =>
      (b.conversation.messages.at(-1)?.createdAt || "").localeCompare(
        a.conversation.messages.at(-1)?.createdAt || "",
      ),
    );
  const filtered = threads.filter(
    (item) =>
      !query ||
      item.conversation.title.toLowerCase().includes(query.toLowerCase()) ||
      item.conversation.messages.some((message) =>
        message.text.toLowerCase().includes(query.toLowerCase()),
      ),
  );
  const sidebar = (
    <div className="sidebar-content">
      <div className="sidebar-brand-row">
        <div className="brand">
          <svg width="24" height="24" viewBox="0 0 24 24" aria-hidden="true">
            <path
              d="M3 19 9 5l6 14 6-14"
              fill="none"
              stroke="currentColor"
              strokeWidth="2.5"
              strokeLinejoin="round"
            />
          </svg>
          <span>Meden</span>
        </div>
        {mobile && (
          <IconButton
            aria-label="Close conversations"
            onClick={() => setMobileSidebarOpen(false)}
          >
            <CloseRounded fontSize="small" />
          </IconButton>
        )}
      </div>
      <Button
        className="sidebar-action"
        color="inherit"
        startIcon={<AddCommentOutlined />}
        onClick={newChat}
      >
        New message
      </Button>
      <Button
        className="sidebar-action"
        color="inherit"
        startIcon={<SearchRounded />}
        onClick={() => {
          setSearchOpen((value) => !value);
          setQuery("");
        }}
      >
        Search conversations
      </Button>
      {searchOpen && (
        <TextField
          autoFocus
          size="small"
          fullWidth
          label="Search conversations"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
      )}
      <div className="conversation-history" aria-label="Conversation history">
        <Typography variant="caption" color="text.secondary">
          Conversation
        </Typography>
        {filtered.map((item) => (
          <button
            key={item.workspaceId + item.conversation.id}
            className={
              "history-item " +
              (activeId === item.workspaceId &&
              conversationId === item.conversation.id
                ? "active"
                : "")
            }
            aria-current={
              activeId === item.workspaceId &&
              conversationId === item.conversation.id
                ? "page"
                : undefined
            }
            onClick={() => {
              select(item.workspaceId, item.conversation.id);
              if (item.workspaceId === DEMO_WORKSPACE_ID)
                void persist(
                  library.current.find(
                    (value) => value.id === item.workspaceId,
                  )!,
                );
            }}
          >
            {item.conversation.title}
          </button>
        ))}
        {query && !filtered.length && (
          <Typography
            variant="body2"
            color="text.secondary"
            className="empty-history"
          >
            No matching conversations
          </Typography>
        )}
      </div>
      <div className="sidebar-bottom">
        <Button
          color="inherit"
          startIcon={<SettingsOutlined />}
          onClick={() => setSettings(true)}
        >
          Settings
        </Button>
      </div>
    </div>
  );
  const artifacts = workspace && (
    <Suspense
      fallback={
        <div className="loading">
          <CircularProgress size={24} />
        </div>
      }
    >
      <ArtifactPanel
        key={workspace.id}
        workspace={workspace}
        tab={panelTab}
        onTab={setPanelTab}
        onClose={() => {
          setPanelOpen(false);
          if (!narrow) artifactToggle.current?.focus();
        }}
        onChange={persist}
        onNotify={setNotification}
        running={running}
      />
    </Suspense>
  );
  return (
    <div className="app-shell">
      <a className="skip-link" href="#main-content">
        Skip to content
      </a>
      {mobile ? (
        <Drawer
          open={mobileSidebarOpen}
          onClose={() => setMobileSidebarOpen(false)}
          slotProps={{ paper: { sx: { width: 268 } } }}
        >
          <aside id="conversation-sidebar" aria-label="Conversation sidebar">
            {sidebar}
          </aside>
        </Drawer>
      ) : (
        <aside
          id="conversation-sidebar"
          className="desktop-sidebar"
          aria-label="Conversation sidebar"
          hidden={!desktopSidebarOpen}
        >
          {sidebar}
        </aside>
      )}
      <div className="main-column">
        <header className="chat-topbar">
          <Tooltip title={sidebarLabel} disableInteractive>
            <IconButton
              className="sidebar-toggle"
              aria-label={sidebarLabel}
              aria-expanded={sidebarOpen}
              aria-controls={sidebarOpen ? "conversation-sidebar" : undefined}
              onClick={() =>
                mobile
                  ? setMobileSidebarOpen((open) => !open)
                  : setDesktopSidebarOpen((open) => !open)
              }
            >
              <ViewSidebarOutlined fontSize="small" />
            </IconButton>
          </Tooltip>
          {hasMessages ? (
            <Typography component="h1" className="conversation-title">
              {conversation?.title}
            </Typography>
          ) : (
            <Typography className="model-label">Meden</Typography>
          )}
          {workspace && (
            <Tooltip title={artifactLabel} disableInteractive>
              <IconButton
                ref={artifactToggle}
                className="artifact-toggle"
                aria-label={artifactLabel}
                aria-expanded={panelOpen}
                onClick={() => setPanelOpen((open) => !open)}
              >
                <InsertDriveFileOutlined fontSize="small" />
              </IconButton>
            </Tooltip>
          )}
        </header>
        {storageError && (
          <Alert
            severity="error"
            action={
              <Button color="inherit" onClick={retry}>
                {unsaved.current.size ? "Retry save" : "Reload"}
              </Button>
            }
          >
            {storageError}
          </Alert>
        )}
        <main
          id="main-content"
          tabIndex={-1}
          className={
            "conversation-workbench " +
            (panelOpen && !narrow ? "with-artifact" : "")
          }
        >
          {loading ? (
            <div className="loading">
              <CircularProgress size={26} aria-label="Loading conversation" />
            </div>
          ) : (
            workspace && (
              <ChatView
                projectName={workspace.name}
                conversation={conversation}
                draftKey={
                  "meden-input:" + activeId + ":" + (conversationId || "new")
                }
                running={running}
                onSend={send}
                artifact={
                  <ArtifactCard
                    workspace={workspace}
                    onOpen={() => openPanel("preview")}
                  />
                }
              />
            )
          )}{" "}
          {!narrow && panelOpen && artifacts}
        </main>
      </div>
      <Drawer
        anchor="right"
        open={narrow && panelOpen}
        onClose={() => setPanelOpen(false)}
        slotProps={{ paper: { sx: { width: "min(720px, 100vw)" } } }}
      >
        {artifacts}
      </Drawer>
      <input
        type="file"
        ref={fileInput}
        accept=".json,application/json"
        aria-label="Workspace JSON"
        hidden
        onChange={onImport}
      />
      <Dialog open={settings} onClose={() => setSettings(false)}>
        <DialogTitle>Settings</DialogTitle>
        <DialogContent className="workspace-actions">
          <Button
            disabled={!workspace}
            onClick={() => {
              if (workspace) downloadJson("meden-workspace.json", workspace);
            }}
          >
            Export workspace
          </Button>
          <Button
            onClick={() => {
              fileInput.current?.click();
              setSettings(false);
            }}
          >
            Import JSON
          </Button>
          <Button
            color="error"
            disabled={!conversationId}
            onClick={() => {
              setSettings(false);
              setQueueDialog(true);
            }}
          >
            Delete conversation
          </Button>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setSettings(false)}>Close</Button>
        </DialogActions>
      </Dialog>
      <Dialog open={queueDialog} onClose={() => setQueueDialog(false)}>
        <DialogTitle>Delete this conversation?</DialogTitle>
        <DialogContent>
          <Typography>{conversation?.title}</Typography>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setQueueDialog(false)}>Cancel</Button>
          <Button
            color="error"
            onClick={async () => {
              if (workspace) {
                await persist({
                  ...workspace,
                  conversations: workspace.conversations.filter(
                    (item) => item.id !== conversationId,
                  ),
                });
                select(activeId, null);
                setPanelOpen(false);
              }
              setQueueDialog(false);
            }}
          >
            Delete
          </Button>
        </DialogActions>
      </Dialog>
      <Snackbar
        open={Boolean(notification)}
        message={notification}
        onClose={() => setNotification("")}
        autoHideDuration={6000}
      />
    </div>
  );
}
