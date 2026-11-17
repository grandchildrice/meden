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
  AppBar,
  Button,
  Chip,
  CircularProgress,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Drawer,
  IconButton,
  List,
  ListItemButton,
  ListItemIcon,
  ListItemText,
  Menu,
  MenuItem,
  Snackbar,
  TextField,
  Toolbar,
  Tooltip,
  Typography,
} from "@mui/material";
import ChatBubbleOutlineRounded from "@mui/icons-material/ChatBubbleOutlineRounded";
import AccountTreeOutlined from "@mui/icons-material/AccountTreeOutlined";
import CompareArrowsRounded from "@mui/icons-material/CompareArrowsRounded";
import AssignmentOutlined from "@mui/icons-material/AssignmentOutlined";
import FolderOpenOutlined from "@mui/icons-material/FolderOpenOutlined";
import AddRounded from "@mui/icons-material/AddRounded";
import FileUploadOutlined from "@mui/icons-material/FileUploadOutlined";
import FileDownloadOutlined from "@mui/icons-material/FileDownloadOutlined";
import MoreHorizRounded from "@mui/icons-material/MoreHorizRounded";
import MenuRounded from "@mui/icons-material/MenuRounded";
import ExpandMoreRounded from "@mui/icons-material/ExpandMoreRounded";
import SettingsOutlined from "@mui/icons-material/SettingsOutlined";
import {
  createWorkspace,
  workspaceSchema,
  type Workspace,
} from "@meden/workspace-schema";
import { BrowserWorkspaceRepository } from "../adapters/browser-workspaces";
import { readWorkspaceFile, downloadJson } from "../adapters/files";
import { sampleWorkspace } from "../fixtures/ledger";
import { EmptyState } from "../components/EmptyState";
const ChatView = lazy(() =>
  import("../features/chat/ChatView").then((module) => ({
    default: module.ChatView,
  })),
);
const GraphView = lazy(() =>
  import("../features/graph/GraphView").then((module) => ({
    default: module.GraphView,
  })),
);
const AnalysisView = lazy(() =>
  import("../features/analysis/AnalysisView").then((module) => ({
    default: module.AnalysisView,
  })),
);
const ReviewView = lazy(() =>
  import("../features/review/ReviewView").then((module) => ({
    default: module.ReviewView,
  })),
);

const routes = [
  { id: "chat", label: "会話", icon: ChatBubbleOutlineRounded },
  { id: "graph", label: "依存関係", icon: AccountTreeOutlined },
  { id: "analysis", label: "意味の比較", icon: CompareArrowsRounded },
  { id: "review", label: "レビュー依頼", icon: AssignmentOutlined },
] as const;
type Route = (typeof routes)[number]["id"];
const repository = new BrowserWorkspaceRepository();

export function App() {
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [route, setRoute] = useState<Route>("chat");
  const [claimId, setClaimId] = useState<string | undefined>();
  const [declarationId, setDeclarationId] = useState<string | undefined>();
  const [conversationId, setConversationId] = useState<string | undefined>();
  const [loading, setLoading] = useState(true);
  const [storageError, setStorageError] = useState("");
  const [notification, setNotification] = useState("");
  const [drawer, setDrawer] = useState(false);
  const [dialog, setDialog] = useState<"create" | "settings" | "remove" | null>(
    null,
  );
  const [name, setName] = useState("");
  const [menu, setMenu] = useState<HTMLElement | null>(null);
  const [workspaceMenu, setWorkspaceMenu] = useState<HTMLElement | null>(null);
  const [saving, setSaving] = useState(false);
  const fileInput = useRef<HTMLInputElement>(null);
  const queue = useRef(Promise.resolve());
  const operation = useRef(0);
  const unsaved = useRef(new Map<string, Workspace>());
  const workspace = workspaces.find((item) => item.id === activeId);
  const routeLabel = routes.find((item) => item.id === route)!.label;

  const load = () => {
    setLoading(true);
    setStorageError("");
    repository
      .list()
      .then((values) => {
        setWorkspaces(values);
        const retained = localStorage.getItem("meden-active-workspace");
        setActiveId(
          values.find((value) => value.id === retained)?.id ||
            values[0]?.id ||
            null,
        );
      })
      .catch(() =>
        setStorageError(
          "ワークスペースを読み込めませんでした。ブラウザの保存領域を確認して、再読み込みしてください。",
        ),
      )
      .finally(() => setLoading(false));
  };
  useEffect(load, []);
  useEffect(() => {
    if (activeId) localStorage.setItem("meden-active-workspace", activeId);
  }, [activeId]);

  const persist = (next: Workspace): Promise<boolean> => {
    const parsed = workspaceSchema.parse({
      ...next,
      updatedAt: new Date().toISOString(),
    });
    setWorkspaces((previous) => [
      parsed,
      ...previous.filter((item) => item.id !== parsed.id),
    ]);
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
          "変更を保存できませんでした。保存を再試行するか、JSONを書き出して変更を保管してください。",
        );
        return false;
      }
    });
    queue.current = result.then(() => {});
    return result;
  };
  const retryStorage = () => {
    if (unsaved.current.size)
      for (const next of unsaved.current.values()) void persist(next);
    else load();
  };
  const open = (next: Workspace) => {
    persist(next);
    setActiveId(next.id);
    setRoute("chat");
    setClaimId(undefined);
    setDeclarationId(undefined);
    setConversationId(undefined);
    setDrawer(false);
  };
  const onImport = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;
    try {
      const next = await readWorkspaceFile(file);
      next.id = crypto.randomUUID();
      open(next);
      setNotification("ワークスペースを読み込みました");
    } catch (error) {
      setNotification(
        error instanceof Error ? error.message : "読み込めませんでした",
      );
    }
  };
  const newConversation = () => {
    if (!workspace) {
      setDialog("create");
      return;
    }
    const id = crypto.randomUUID();
    persist({
      ...workspace,
      conversations: [
        ...workspace.conversations,
        { id, title: "新しい会話", messages: [] },
      ],
    });
    setConversationId(id);
    setRoute("chat");
    setDrawer(false);
  };
  const navigate = (next: Route) => {
    setRoute(next);
    setDrawer(false);
  };
  const navigateToClaim = (id: string) => {
    setClaimId(id);
    navigate("analysis");
  };
  const navigateToDeclaration = (id: string) => {
    setDeclarationId(id);
    navigate("graph");
  };
  const sidebar = (
    <div className="sidebar-content">
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
      <Button
        className="workspace-picker"
        variant="outlined"
        startIcon={<FolderOpenOutlined />}
        endIcon={<ExpandMoreRounded />}
        onClick={(event) => setWorkspaceMenu(event.currentTarget)}
      >
        <span className="workspace-name">
          {workspace?.name || "ワークスペース"}
        </span>
      </Button>
      <Button
        className="new-conversation"
        variant="contained"
        startIcon={<AddRounded />}
        onClick={newConversation}
      >
        新しい会話
      </Button>
      <nav aria-label="メイン">
        <List component="div" disablePadding>
          {routes.map((item) => (
            <ListItemButton
              key={item.id}
              selected={route === item.id}
              onClick={() => navigate(item.id)}
              aria-current={route === item.id ? "page" : undefined}
            >
              <ListItemIcon>
                <item.icon fontSize="small" />
              </ListItemIcon>
              <ListItemText primary={item.label} />
            </ListItemButton>
          ))}
        </List>
      </nav>
      {workspace && (
        <div className="conversation-history">
          <Typography variant="caption" color="text.secondary">
            最近の会話
          </Typography>
          {workspace.conversations
            .slice(-6)
            .reverse()
            .map((conversation) => (
              <button
                key={conversation.id}
                className={
                  conversationId === conversation.id
                    ? "history-item active"
                    : "history-item"
                }
                onClick={() => {
                  setConversationId(conversation.id);
                  navigate("chat");
                }}
              >
                {conversation.title}
              </button>
            ))}
        </div>
      )}
      <div className="sidebar-bottom">
        <Button
          startIcon={<SettingsOutlined />}
          color="secondary"
          onClick={() => setDialog("settings")}
        >
          設定
        </Button>
        <Typography variant="caption" color="text.secondary">
          {saving
            ? "保存中…"
            : storageError
              ? "保存できません"
              : "ブラウザに保存"}
        </Typography>
      </div>
    </div>
  );

  return (
    <div className="app-shell">
      <a href="#main-content" className="skip-link">
        本文へ移動
      </a>
      <aside className="desktop-sidebar">{sidebar}</aside>
      <Drawer
        open={drawer}
        onClose={() => setDrawer(false)}
        slotProps={{ paper: { sx: { width: 264 } } }}
      >
        {sidebar}
      </Drawer>
      <div className="main-column">
        <AppBar
          position="static"
          color="transparent"
          elevation={0}
          className="app-bar"
        >
          <Toolbar className="app-toolbar">
            <IconButton
              className="mobile-menu"
              aria-label="ナビゲーションを開く"
              onClick={() => setDrawer(true)}
            >
              <MenuRounded />
            </IconButton>
            <div className="breadcrumb">
              <span>{workspace?.name || "Meden"}</span>
              <span aria-hidden="true">/</span>
              <Typography component="span">
                {workspace ? routeLabel : "ワークスペース"}
              </Typography>
            </div>
            <div className="toolbar-actions">
              {workspace?.origin === "sample" && (
                <Chip label="サンプル" size="small" variant="outlined" />
              )}
              {workspace?.origin === "imported" && (
                <Chip label="取り込みデータ" size="small" variant="outlined" />
              )}
              <Tooltip title="エージェントの接続状態" describeChild>
                <Button
                  color="secondary"
                  className="connection-status"
                  onClick={() => setDialog("settings")}
                >
                  <span className="connection-dot" />
                  未接続
                </Button>
              </Tooltip>
              <Tooltip title="ワークスペースの操作">
                <IconButton
                  aria-label="ワークスペースの操作"
                  onClick={(event) => setMenu(event.currentTarget)}
                >
                  <MoreHorizRounded />
                </IconButton>
              </Tooltip>
            </div>
          </Toolbar>
        </AppBar>
        {storageError && (
          <Alert
            severity="error"
            action={
              <Button color="inherit" onClick={retryStorage}>
                {unsaved.current.size ? "保存を再試行" : "再読み込み"}
              </Button>
            }
          >
            {storageError}
          </Alert>
        )}
        <main id="main-content" tabIndex={-1} className="main-content">
          {loading ? (
            <div className="loading">
              <CircularProgress
                size={28}
                aria-label="ワークスペースを読み込み中"
              />
            </div>
          ) : !workspace ? (
            <EmptyState
              icon={<FolderOpenOutlined fontSize="large" />}
              title="ワークスペースを開く"
              detail="保存したJSONを読み込むか、新しく作成します。"
            >
              <Button
                variant="contained"
                startIcon={<FileUploadOutlined />}
                onClick={() => fileInput.current?.click()}
              >
                JSONを読み込む
              </Button>
              <Button
                variant="outlined"
                onClick={() => {
                  setName("");
                  setDialog("create");
                }}
              >
                新規作成
              </Button>
              <Button onClick={() => open(sampleWorkspace())}>
                サンプルを開く
              </Button>
            </EmptyState>
          ) : (
            <Suspense
              fallback={
                <div className="loading">
                  <CircularProgress size={28} aria-label="画面を読み込み中" />
                </div>
              }
            >
              {workspace.origin === "sample" && (
                <div className="workspace-notice" role="note">
                  サンプルデータです。解析・証明は実行していません。
                </div>
              )}
              {workspace.origin === "imported" && (
                <div className="workspace-notice" role="note">
                  取り込んだ結果を表示しています。この端末では再検証していません。
                </div>
              )}
              {route === "chat" && (
                <ChatView
                  key={workspace.id + (conversationId || "")}
                  workspace={workspace}
                  conversationId={conversationId}
                  onChange={persist}
                  onSelectConversation={setConversationId}
                  onClaim={navigateToClaim}
                  onGraph={navigateToDeclaration}
                />
              )}
              {route === "graph" && (
                <GraphView
                  key={workspace.id}
                  workspace={workspace}
                  initialId={declarationId}
                  onClaim={navigateToClaim}
                />
              )}
              {route === "analysis" && (
                <AnalysisView
                  key={workspace.id}
                  workspace={workspace}
                  claimId={claimId}
                  onSelectClaim={setClaimId}
                  onChange={persist}
                  onGraph={navigateToDeclaration}
                  onReview={(id) => {
                    setClaimId(id);
                    navigate("review");
                  }}
                />
              )}
              {route === "review" && (
                <ReviewView
                  key={workspace.id}
                  workspace={workspace}
                  claimId={claimId}
                  onChange={persist}
                  onNotify={setNotification}
                />
              )}
            </Suspense>
          )}
        </main>
      </div>
      <input
        ref={fileInput}
        type="file"
        accept=".json,application/json"
        aria-label="ワークスペースのJSON"
        hidden
        onChange={onImport}
      />
      <Menu
        anchorEl={workspaceMenu}
        open={Boolean(workspaceMenu)}
        onClose={() => setWorkspaceMenu(null)}
      >
        {workspaces.map((item) => (
          <MenuItem
            key={item.id}
            selected={activeId === item.id}
            onClick={() => {
              setActiveId(item.id);
              setWorkspaceMenu(null);
              setConversationId(undefined);
              setClaimId(undefined);
              setDeclarationId(undefined);
            }}
          >
            {item.name}
          </MenuItem>
        ))}
        <MenuItem
          onClick={() => {
            setWorkspaceMenu(null);
            setName("");
            setDialog("create");
          }}
        >
          <AddRounded fontSize="small" /> 新規作成
        </MenuItem>
        <MenuItem
          onClick={() => {
            setWorkspaceMenu(null);
            fileInput.current?.click();
          }}
        >
          <FileUploadOutlined fontSize="small" /> JSONを読み込む
        </MenuItem>
        <MenuItem
          onClick={() => {
            setWorkspaceMenu(null);
            open(sampleWorkspace());
          }}
        >
          サンプルを開く
        </MenuItem>
      </Menu>
      <Menu anchorEl={menu} open={Boolean(menu)} onClose={() => setMenu(null)}>
        <MenuItem
          disabled={!workspace}
          onClick={() => {
            if (workspace) downloadJson("meden-workspace.json", workspace);
            setMenu(null);
          }}
        >
          <FileDownloadOutlined fontSize="small" /> JSONを書き出す
        </MenuItem>
        <MenuItem
          onClick={() => {
            setMenu(null);
            fileInput.current?.click();
          }}
        >
          <FileUploadOutlined fontSize="small" /> JSONを読み込む
        </MenuItem>
        <MenuItem
          disabled={!workspace}
          onClick={() => {
            setMenu(null);
            setDialog("remove");
          }}
        >
          ワークスペースを削除
        </MenuItem>
      </Menu>
      <Dialog open={dialog === "create"} onClose={() => setDialog(null)}>
        <DialogTitle>ワークスペースを作成</DialogTitle>
        <form
          onSubmit={(event) => {
            event.preventDefault();
            if (name.trim()) {
              open(createWorkspace(name.trim()));
              setDialog(null);
            }
          }}
        >
          <DialogContent>
            <TextField
              autoFocus
              fullWidth
              label="名前"
              value={name}
              onChange={(event) => setName(event.target.value)}
              slotProps={{ htmlInput: { maxLength: 300 } }}
            />
          </DialogContent>
          <DialogActions>
            <Button onClick={() => setDialog(null)}>キャンセル</Button>
            <Button type="submit" variant="contained" disabled={!name.trim()}>
              作成
            </Button>
          </DialogActions>
        </form>
      </Dialog>
      <Dialog open={dialog === "settings"} onClose={() => setDialog(null)}>
        <DialogTitle>設定</DialogTitle>
        <DialogContent>
          <div className="settings-row">
            <Typography variant="h3">エージェント</Typography>
            <Chip size="small" label="未接続" />
          </div>
          <Typography color="text.secondary">
            実行系への接続は未実装です。会話は下書きとして保存できます。
          </Typography>
          <div className="settings-section">
            <Typography variant="h3">保存先</Typography>
            <Typography color="text.secondary">
              このブラウザの保存領域。別の端末で開くには、ワークスペースのJSONを書き出してください。
            </Typography>
          </div>
          <div className="settings-section">
            <Typography variant="h3">同期</Typography>
            <Typography color="text.secondary">未設定</Typography>
          </div>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setDialog(null)}>閉じる</Button>
        </DialogActions>
      </Dialog>
      <Dialog open={dialog === "remove"} onClose={() => setDialog(null)}>
        <DialogTitle>このワークスペースを削除しますか？</DialogTitle>
        <DialogContent>
          <Typography>
            「{workspace?.name}
            」をこのブラウザから削除します。必要な場合は、先にJSONを書き出してください。
          </Typography>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => setDialog(null)}>キャンセル</Button>
          <Button
            color="error"
            onClick={async () => {
              if (workspace) {
                await queue.current;
                await repository.remove(workspace.id);
                unsaved.current.delete(workspace.id);
                if (!unsaved.current.size) setStorageError("");
                setWorkspaces((previous) =>
                  previous.filter((item) => item.id !== workspace.id),
                );
                setActiveId(null);
                setDialog(null);
              }
            }}
          >
            削除
          </Button>
        </DialogActions>
      </Dialog>
      <Snackbar
        open={Boolean(notification)}
        message={notification}
        autoHideDuration={7000}
        onClose={() => setNotification("")}
      />
    </div>
  );
}
