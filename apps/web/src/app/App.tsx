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
  Chip,
  CircularProgress,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Drawer,
  IconButton,
  Menu,
  MenuItem,
  Snackbar,
  TextField,
  Tooltip,
  Typography,
  useMediaQuery,
} from "@mui/material";
import AddCommentOutlined from "@mui/icons-material/AddCommentOutlined";
import SearchRounded from "@mui/icons-material/SearchRounded";
import SettingsOutlined from "@mui/icons-material/SettingsOutlined";
import MenuRounded from "@mui/icons-material/MenuRounded";
import MoreHorizRounded from "@mui/icons-material/MoreHorizRounded";
import AccountTreeOutlined from "@mui/icons-material/AccountTreeOutlined";
import VerifiedOutlined from "@mui/icons-material/VerifiedOutlined";
import VerticalSplitOutlined from "@mui/icons-material/VerticalSplitOutlined";
import CloseRounded from "@mui/icons-material/CloseRounded";
import {
  workspaceSchema,
  type Workspace,
  type Conversation,
} from "@meden/workspace-schema";
import { BrowserWorkspaceRepository } from "../adapters/browser-workspaces";
import { readWorkspaceFile, downloadJson } from "../adapters/files";
import {
  createVendingWorkspace,
  DEMO_WORKSPACE_ID,
  demoResponse,
  titleFromInstruction,
} from "../fixtures/meden-demo";
import type { ArtifactTab } from "../features/artifacts/ArtifactPanel";
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
  const [saving, setSaving] = useState(false);
  const [storageError, setStorageError] = useState("");
  const [notification, setNotification] = useState("");
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [searchOpen, setSearchOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [panelOpen, setPanelOpen] = useState(false);
  const [panelTab, setPanelTab] = useState<ArtifactTab>("preview");
  const [settings, setSettings] = useState(false);
  const [menu, setMenu] = useState<HTMLElement | null>(null);
  const [queueDialog, setQueueDialog] = useState(false);
  const fileInput = useRef<HTMLInputElement>(null);
  const queue = useRef(Promise.resolve());
  const unsaved = useRef(new Map<string, Workspace>());
  const operation = useRef(0);
  const narrow = useMediaQuery("(max-width:1140px)");
  const mobile = useMediaQuery("(max-width:839px)");
  const workspace =
    workspaces.find((item) => item.id === activeId) || workspaces[0];
  const conversation = workspace?.conversations.find(
    (item) => item.id === conversationId,
  );
  const running = conversationId ? pending.has(conversationId) : false;
  const hasMessages = Boolean(conversation?.messages.length);

  const select = (workspaceId: string, id: string | null) => {
    setActiveId(workspaceId);
    setConversationId(id);
    setSidebarOpen(false);
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
            setActiveId(selected.workspaceId);
            setConversationId(selected.conversationId);
          }
        } catch {
          /* An unreadable preference does not discard stored conversations. */
        }
        setStorageError("");
      })
      .catch(() =>
        setStorageError("会話を読み込めませんでした。再読み込みしてください。"),
      )
      .finally(() => setLoading(false));
  };
  useEffect(load, []);
  const persist = (next: Workspace): Promise<boolean> => {
    const parsed = workspaceSchema.parse({
      ...next,
      updatedAt: new Date().toISOString(),
    });
    library.current = [
      parsed,
      ...library.current.filter((item) => item.id !== parsed.id),
    ];
    setWorkspaces(library.current);
    unsaved.current.set(parsed.id, parsed);
    setSaving(true);
    const current = ++operation.current;
    const result = queue.current.then(async () => {
      try {
        await repository.save(parsed);
        if (unsaved.current.get(parsed.id) === parsed)
          unsaved.current.delete(parsed.id);
        if (current === operation.current) {
          setSaving(false);
          if (!unsaved.current.size) setStorageError("");
        }
        return true;
      } catch {
        if (current === operation.current) setSaving(false);
        setStorageError(
          "会話を保存できませんでした。保存を再試行するか、JSONを書き出してください。",
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
    const id = conversationId || crypto.randomUUID();
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
      /依存|graph/i.test(text)
        ? "graph"
        : /証明|検証|verify/i.test(text)
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
      openPanel("graph");
      setNotification("読み込みました");
    } catch (error) {
      setNotification(
        error instanceof Error ? error.message : "読み込めませんでした",
      );
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
            aria-label="会話一覧を閉じる"
            onClick={() => setSidebarOpen(false)}
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
        新しい会話
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
        会話を検索
      </Button>
      {searchOpen && (
        <TextField
          autoFocus
          size="small"
          fullWidth
          label="会話を検索"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
      )}
      <div className="conversation-history" aria-label="会話履歴">
        <Typography variant="caption" color="text.secondary">
          会話
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
            一致する会話はありません
          </Typography>
        )}
      </div>
      <div className="sidebar-bottom">
        <Button
          color="inherit"
          startIcon={<SettingsOutlined />}
          onClick={() => setSettings(true)}
        >
          設定
        </Button>
        <Typography variant="caption" color="text.secondary">
          {saving
            ? "保存中…"
            : storageError
              ? "保存できません"
              : "このブラウザに保存"}
        </Typography>
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
        onClose={() => setPanelOpen(false)}
        onChange={persist}
        onNotify={setNotification}
        running={running}
      />
    </Suspense>
  );
  return (
    <div className="app-shell">
      <a className="skip-link" href="#main-content">
        本文へ移動
      </a>
      <aside className="desktop-sidebar" aria-label="会話サイドバー">
        {sidebar}
      </aside>
      <Drawer
        open={sidebarOpen}
        onClose={() => setSidebarOpen(false)}
        slotProps={{ paper: { sx: { width: 268 } } }}
      >
        {sidebar}
      </Drawer>
      <div className="main-column">
        <header className="chat-topbar">
          <IconButton
            className="mobile-menu"
            aria-label="会話一覧を開く"
            onClick={() => setSidebarOpen(true)}
          >
            <MenuRounded />
          </IconButton>
          {hasMessages ? (
            <Typography component="h1" className="conversation-title">
              {conversation?.title}
            </Typography>
          ) : (
            <Typography className="model-label">Meden</Typography>
          )}
          <div className="toolbar-actions">
            <Tooltip title="meden-demoの実装・会話・検証記録を使うGUIデモ">
              <Chip size="small" label="デモ" variant="outlined" />
            </Tooltip>
            <Tooltip title="証明の状態">
              <span>
                <IconButton
                  aria-label="証明の状態を表示"
                  disabled={!hasMessages}
                  aria-pressed={panelOpen && panelTab === "proof"}
                  onClick={() => openPanel("proof")}
                >
                  <VerifiedOutlined fontSize="small" />
                </IconButton>
              </span>
            </Tooltip>
            <Tooltip title="依存関係">
              <span>
                <IconButton
                  aria-label="依存関係を表示"
                  disabled={!hasMessages}
                  aria-pressed={panelOpen && panelTab === "graph"}
                  onClick={() => openPanel("graph")}
                >
                  <AccountTreeOutlined fontSize="small" />
                </IconButton>
              </span>
            </Tooltip>
            <Tooltip title={panelOpen ? "パネルを閉じる" : "実装を表示"}>
              <span>
                <IconButton
                  aria-label={panelOpen ? "パネルを閉じる" : "実装を表示"}
                  disabled={!hasMessages}
                  onClick={() =>
                    panelOpen ? setPanelOpen(false) : openPanel("preview")
                  }
                >
                  <VerticalSplitOutlined fontSize="small" />
                </IconButton>
              </span>
            </Tooltip>
            <IconButton
              aria-label="その他の操作"
              onClick={(event) => setMenu(event.currentTarget)}
            >
              <MoreHorizRounded />
            </IconButton>
          </div>
        </header>
        {storageError && (
          <Alert
            severity="error"
            action={
              <Button color="inherit" onClick={retry}>
                {unsaved.current.size ? "保存を再試行" : "再読み込み"}
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
              <CircularProgress size={26} aria-label="会話を読み込み中" />
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
                onOpenArtifact={() => openPanel("preview")}
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
        aria-label="ワークスペースのJSON"
        hidden
        onChange={onImport}
      />
      <Menu anchorEl={menu} open={Boolean(menu)} onClose={() => setMenu(null)}>
        <MenuItem
          disabled={!workspace}
          onClick={() => {
            if (workspace) downloadJson("meden-workspace.json", workspace);
            setMenu(null);
          }}
        >
          ワークスペースを書き出す
        </MenuItem>
        <MenuItem
          onClick={() => {
            fileInput.current?.click();
            setMenu(null);
          }}
        >
          JSONを読み込む
        </MenuItem>
        <MenuItem
          disabled={!conversationId}
          onClick={() => {
            setMenu(null);
            setQueueDialog(true);
          }}
        >
          会話を削除
        </MenuItem>
      </Menu>
      <Dialog open={settings} onClose={() => setSettings(false)}>
        <DialogTitle>設定</DialogTitle>
        <DialogContent>
          <Typography variant="h3">デモモード</Typography>
          <Typography color="text.secondary" sx={{ mt: 1 }}>
            meden-demoの実装・会話・検証記録を使っています。送信後の応答やパネル表示はGUIのデモです。
          </Typography>
          <Typography variant="h3" sx={{ mt: 3 }}>
            保存先
          </Typography>
          <Typography color="text.secondary" sx={{ mt: 1 }}>
            このブラウザの保存領域。実際のエージェントやLeanは実行していません。
          </Typography>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setSettings(false)}>閉じる</Button>
        </DialogActions>
      </Dialog>
      <Dialog open={queueDialog} onClose={() => setQueueDialog(false)}>
        <DialogTitle>この会話を削除しますか？</DialogTitle>
        <DialogContent>
          <Typography>{conversation?.title}</Typography>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setQueueDialog(false)}>キャンセル</Button>
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
            削除
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
