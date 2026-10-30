import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "../app.css";
import { UxSkeleton } from "./UxSkeleton";

const root = document.getElementById("root");
if (root !== null) {
  createRoot(root).render(
    <StrictMode>
      <UxSkeleton />
    </StrictMode>,
  );
}
