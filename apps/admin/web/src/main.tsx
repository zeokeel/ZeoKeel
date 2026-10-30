import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { AdminApp } from "./app";
import "./style.css";

const root = document.getElementById("root");
if (root !== null) createRoot(root).render(<StrictMode><AdminApp /></StrictMode>);
