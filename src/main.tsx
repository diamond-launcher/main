import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "@fontsource/pixelify-sans/400.css";
import "@fontsource/pixelify-sans/500.css";
import "@fontsource/pixelify-sans/700.css";
import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/components.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
