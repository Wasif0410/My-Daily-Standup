import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "@/app/App";
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

ReactDOM.createRoot(rootElement).render(
  <React.StrictMode>{currentWindow()}</React.StrictMode>,
);
