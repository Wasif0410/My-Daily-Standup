import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "@/app/App";
import { BoardRoot } from "@/features/boards/BoardRoot";
import { parseBoardKind } from "@/types/board";
import "@/styles/tokens.css";
import "@/styles/theme.css";
import "@/styles/global.css";

const rootElement = document.getElementById("root");

if (!rootElement) {
  throw new Error("Root element #root not found in index.html");
}

// One bundle serves every window. A `?board=` parameter means this window is a
// sticky note rather than the main planning view, which avoids a separate build
// per board.
const board = parseBoardKind(new URLSearchParams(window.location.search).get("board"));

if (board) {
  // Boards are transparent so the desktop shows through their rounded corners.
  document.body.classList.add("board-window");
}

ReactDOM.createRoot(rootElement).render(
  <React.StrictMode>{board ? <BoardRoot kind={board} /> : <App />}</React.StrictMode>,
);
