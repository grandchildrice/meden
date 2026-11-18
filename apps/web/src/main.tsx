import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { CssBaseline, ThemeProvider } from "@mui/material";
import "@fontsource-variable/roboto";
import "@xyflow/react/dist/style.css";
import { theme } from "./app/theme";
import { App } from "./app/App";
import { NativeApp } from "./app/NativeApp";
import "./app/styles.css";

function readNativeToken() {
  const fragment = new URLSearchParams(location.hash.slice(1)).get("session");
  if (fragment) {
    sessionStorage.setItem("meden-native-session", fragment);
    history.replaceState(null, "", location.pathname);
  }
  return fragment || sessionStorage.getItem("meden-native-session");
}
function Bootstrap() {
  const [token, setToken] = useState(readNativeToken);
  useEffect(() => {
    const changed = () => {
      const next = readNativeToken();
      // A restarted host has fresh response policies as well as a fresh token.
      // Opening its fragment URL can otherwise be a same-document navigation.
      if (next !== token) location.reload();
      else setToken(next);
    };
    addEventListener("hashchange", changed);
    return () => removeEventListener("hashchange", changed);
  }, [token]);
  return token ? <NativeApp key={token} token={token} /> : <App />;
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ThemeProvider theme={theme}>
      <CssBaseline />
      <Bootstrap />
    </ThemeProvider>
  </StrictMode>,
);
