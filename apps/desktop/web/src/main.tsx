import "@fontsource/public-sans/400.css";
import "@fontsource/public-sans/500.css";
import "@fontsource/public-sans/600.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/500.css";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./app.css";
import { createTransport } from "./transport";

const container = document.getElementById("root");
if (container === null) {
  throw new Error("missing #root element");
}

createRoot(container).render(
  <StrictMode>
    <App transport={createTransport()} />
  </StrictMode>,
);
