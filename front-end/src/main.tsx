import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import { ShortsStudio } from "./components/shorts/ShortsStudio";
import "./index.css";

const root = document.getElementById("root");
if (root) {
  ReactDOM.createRoot(root).render(
    <React.StrictMode>
      {new URLSearchParams(window.location.search).get("window") === "shorts" ? <ShortsStudio /> : <App />}
    </React.StrictMode>,
  );
}
