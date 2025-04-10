import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { AppTitleBar } from "./components/AppTitleBar";
import { ThemeProvider } from "@material-tailwind/react";
import { LucideShoppingBag } from "lucide-react";
import { Toaster } from "sonner";
import { GlobalContextProvider } from "./stores/GlobalContext";

// @ts-ignore
const isTauri = window.__TAURI__ !== undefined;

if (!isTauri) {
  alert("This application is designed to run in Tauri. Please run it in the Tauri environment.");
  throw new Error("This application is designed to run in Tauri. Please run it in the Tauri environment.");
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <GlobalContextProvider>
      <AppTitleBar />
      <App />
      <Toaster theme="dark" />
    </GlobalContextProvider>
  </React.StrictMode>,
);
