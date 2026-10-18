import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import "./index.css";
import { useProjectStore } from "./stores/projectStore";

// Development only: lets the browser preview load a stand-in project without the desktop app.
if (import.meta.env.DEV) {
  (window as unknown as { __aeroProjectStore?: typeof useProjectStore }).__aeroProjectStore = useProjectStore;
}

const root = document.getElementById("root");
if (root) {
  ReactDOM.createRoot(root).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
}
