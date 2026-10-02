import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import { ShortsStudio } from "./components/shorts/ShortsStudio";
import "./index.css";

/** The Shorts Studio window is marked by an init script (see open_shorts_window). */
function isShortsWindow(): boolean {
  const marked = (window as unknown as { __AEROEDITS_WINDOW__?: string }).__AEROEDITS_WINDOW__;
  return marked === "shorts" || new URLSearchParams(window.location.search).get("window") === "shorts";
}

const root = document.getElementById("root");
if (root) {
  ReactDOM.createRoot(root).render(
    <React.StrictMode>
      {isShortsWindow() ? <ShortsStudio /> : <App />}
    </React.StrictMode>,
  );
}
