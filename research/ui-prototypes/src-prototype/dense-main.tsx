import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "../app.css";
import { DensePrototype } from "./Dense";

const root = document.getElementById("root");
if (root !== null) {
  createRoot(root).render(
    <StrictMode>
      <DensePrototype />
    </StrictMode>,
  );
}
