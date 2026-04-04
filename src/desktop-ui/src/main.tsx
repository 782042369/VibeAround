import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./index.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>
);

// Fade out the static HTML splash after 1s minimum
const splash = document.getElementById("splash");
if (splash) {
  const elapsed = performance.now();
  const remaining = Math.max(0, 1000 - elapsed);
  setTimeout(() => {
    splash.classList.add("hidden");
    setTimeout(() => splash.remove(), 400);
  }, remaining);
}
