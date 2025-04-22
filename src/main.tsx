import React, { useEffect } from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { AppTitleBar } from "./components/AppTitleBar";
import { Toaster } from "sonner";
import { GlobalContextProvider } from "./stores/GlobalContext";
import { TasksProvider } from "./stores/TasksContext";
import { InstancesProvider } from "./stores/InstancesContext";
import { UpdateStatus } from "./components/UpdateStatus";
import { start as startDiscordRpc } from "tauri-plugin-drpc";
import { AuthProvider } from "./stores/AuthContext";
import { ConfigDialogProvider } from "./stores/ConfigDialogContext";
import { useConfigDialog } from "./stores/ConfigDialogContext";
import { ConfigurationDialog } from "./components/ConfigurationDialog";

const ConfigDialogLoader = () => {
  const { isConfigOpen, closeConfigDialog, openConfigDialog } = useConfigDialog();

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key === ',') {
        openConfigDialog();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [openConfigDialog]);

  return <ConfigurationDialog isOpen={isConfigOpen} onClose={closeConfigDialog} />;
};


startDiscordRpc("943184136976334879").catch((err) => {
  console.error("Failed to start Discord RPC:", err);
  // This is only a warning, we can still run the app without it
})


ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <GlobalContextProvider>
    <AuthProvider>
      <TasksProvider>
        <InstancesProvider>
          <ConfigDialogProvider>
            <AppTitleBar />
            <ConfigDialogLoader />
            <App />
            <Toaster theme="dark" />
            <UpdateStatus />
          </ConfigDialogProvider>
        </InstancesProvider>
      </TasksProvider>
    </AuthProvider>
  </GlobalContextProvider>
);
