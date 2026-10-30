import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "../app.css";
import { Prototype } from "./Prototype";

const root = document.getElementById("root");
if (root !== null) {
  createRoot(root).render(
    <StrictMode>
      <Prototype />
    </StrictMode>,
  );
}
