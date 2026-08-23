import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "@/app/App";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import { BoardRoot } from "@/features/boards/BoardRoot";
import { QuickAddWindow } from "@/features/quick-add/QuickAddWindow";
import { parseBoardKind } from "@/types/board";
import "@/styles/tokens.css";
import "@/styles/theme.css";
import "@/styles/global.css";

const rootElement = document.getElementById("root");

if (!rootElement) {
  throw new Error("Root element #root not found in index.html");
}

// One bundle serves every window, routed by query parameter rather than by a
// Vite entry point each. A `?board=` means a sticky note; `?window=quick-add`
// means the tray's capture box; neither means the main planning view.
const params = new URLSearchParams(window.location.search);
const board = parseBoardKind(params.get("board"));
const quickAdd = params.get("window") === "quick-add";

if (board || quickAdd) {
  // Both are transparent, so the desktop shows through their rounded corners.
  document.body.classList.add("board-window");
}

function currentWindow() {
  if (board) return <BoardRoot kind={board} />;
  if (quickAdd) return <QuickAddWindow />;
  return <App />;
}

// Every window gets a boundary at its root. Without one, a single throw
// unmounts the tree and a frameless transparent window renders as an opaque
// white rectangle — no title, no controls, nothing to say what happened.
ReactDOM.createRoot(rootElement).render(
  <React.StrictMode>
    <ErrorBoundary>{currentWindow()}</ErrorBoundary>
  </React.StrictMode>,
);
