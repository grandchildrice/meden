import { createTheme } from "@mui/material/styles";

export const theme = createTheme({
  palette: {
    primary: { main: "#285d64", light: "#e4f0f0", dark: "#16434a" },
    secondary: { main: "#53636d" },
    background: { default: "#f7f9fa", paper: "#ffffff" },
    text: { primary: "#1c252b", secondary: "#58656d" },
    divider: "#dee5e8",
    success: { main: "#2f6c51" },
    warning: { main: "#93621e" },
    error: { main: "#ad352b" },
  },
  shape: { borderRadius: 12 },
  typography: {
    fontFamily:
      '"Roboto Variable", "Noto Sans CJK JP", "Noto Sans JP", sans-serif',
    h1: { fontSize: 24, fontWeight: 500, lineHeight: 1.4 },
    h2: { fontSize: 20, fontWeight: 500, lineHeight: 1.5 },
    h3: { fontSize: 16, fontWeight: 500 },
    body1: { fontSize: 14, lineHeight: 1.7 },
    body2: { fontSize: 13, lineHeight: 1.6 },
    caption: { fontSize: 12, lineHeight: 1.5 },
    button: { textTransform: "none", fontWeight: 500, fontSize: 14 },
  },
  components: {
    MuiButton: {
      defaultProps: { disableElevation: true },
      styleOverrides: {
        root: { minHeight: 44, borderRadius: 24, paddingInline: 20 },
      },
    },
    MuiIconButton: { styleOverrides: { root: { width: 48, height: 48 } } },
    MuiChip: {
      styleOverrides: {
        root: { borderRadius: 7, height: 28 },
        label: { fontSize: 12, fontWeight: 500 },
      },
    },
    MuiPaper: { defaultProps: { elevation: 0 } },
    MuiDialog: {
      defaultProps: { maxWidth: "sm", fullWidth: true },
      styleOverrides: { paper: { borderRadius: 20 } },
    },
    MuiTooltip: { defaultProps: { arrow: true } },
    MuiTextField: { defaultProps: { variant: "outlined" } },
    MuiListItemButton: {
      styleOverrides: {
        root: {
          borderRadius: 24,
          minHeight: 48,
          "&.Mui-selected": { background: "#deebed" },
          "&.Mui-selected:hover": { background: "#d5e5e7" },
        },
      },
    },
  },
});
