import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import { Toaster } from "@/components/ui/sonner";

import { AppErrorBoundary } from "@/components/AppErrorBoundary";
import Settings from "@/routes/Settings";
import Hud from "@/routes/Hud";
import "@/index.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <AppErrorBoundary>
      <BrowserRouter>
        <Routes>
          <Route path="/" element={<Settings />} />
          <Route path="/hud" element={<Hud />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
        <Toaster position="bottom-right" />
      </BrowserRouter>
    </AppErrorBoundary>
  </React.StrictMode>,
);
