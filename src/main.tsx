import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { AppTitleBar } from "./components/AppTitleBar";
import { Toaster } from "sonner";
import { GlobalContextProvider } from "./stores/GlobalContext";
import { isTauri } from "@tauri-apps/api/core";
import { TasksProvider } from "./stores/TasksContext";
import { InstancesProvider } from "./stores/InstancesContext";

if (!isTauri()) {
  const msg = "This app requires Tauri to run. Please run it in a Tauri environment.";
  console.error(msg);
  alert(msg);
  throw new Error(msg);
}


ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <GlobalContextProvider>
      <TasksProvider>
        <InstancesProvider>
          <AppTitleBar />
          <App />
          <Toaster theme="dark" />
        </InstancesProvider>
      </TasksProvider>
    </GlobalContextProvider>
  </React.StrictMode>,
);
