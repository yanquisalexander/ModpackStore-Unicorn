import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import { Link, Route, Router, Switch, useLocation, useRouter } from "wouter";
import { HomeMainHeader } from "./components/home/MainHeader";
import { toast } from "sonner";
import { ExploreSection } from "./views/ExploreSection";
import { PreLaunchInstance } from "./views/PreLaunchInstance";
import { useCheckConnection } from "./utils/checkConnection";
import { LucideLoader } from "lucide-react";
import { MyInstancesSection } from "./views/MyInstancesSection";
import { useAuthentication } from "./stores/AuthContext";
import { Login } from "./views/Login";
import { NotFound } from "./views/NotFound";
import { KonamiCode } from "./components/KonamiCode";
import { AccountsSection } from "./views/AccountsSection";
import { initAnalytics } from "./lib/analytics";
import { trackEvent } from "@aptabase/web";
import { ModpackOverview } from "./views/ModpackOverview";
import { preloadSounds } from "./utils/sounds";
import { OfflineMode } from "./views/OfflineMode";
import NoticeTestBuild from "./components/NoticeTestBuild";
import CommandPalette from "./components/CommandPalette";


// Componente de carga para unificar la presentación
const LoadingScreen = () => (
  <div className="absolute inset-0 flex items-center justify-center min-h-dvh h-full w-full">
    <LucideLoader className="size-10 -mt-12 animate-spin-clockwise animate-iteration-count-infinite animate-duration-1000 text-white" />
  </div>
);

function App() {
  const { loading: authLoading, isAuthenticated, session } = useAuthentication();
  const { isConnected, isLoading: connectionLoading } = useCheckConnection();

  useEffect(() => {
    if (connectionLoading) {
      toast.loading("Verificando conexión a internet...", { id: "connection-check" });
    } else {
      if (!isConnected) {
        toast.warning("Sin conexión", {
          id: "connection-check",
          duration: 5000,
          richColors: true,
          description: "No se pudo establecer conexión a internet.\n\nAlgunas funciones pueden no estar disponibles.",
        });
      }
    }
  }, [isConnected, connectionLoading]);

  useEffect(() => {
    initAnalytics();
    preloadSounds();

    trackEvent("app_launch", {
      name: "App Launch",
      timestamp: new Date().toISOString(),
    });
  }, []);

  // Mostrar loader en cualquier estado de carga para evitar flashes
  if (authLoading || connectionLoading) {
    return <LoadingScreen />;
  }

  // Si no hay conexión, mostrar el modo sin conexión
  if (!isConnected) {
    /* 
      Minimal router (Offline mode at /) and prelaunch instance
    */

    return (
      <Switch>
        <Route path="/" component={OfflineMode} />
        <Route path="/prelaunch/:instanceId">
          {(params) => <PreLaunchInstance instanceId={params.instanceId} />}
        </Route>
      </Switch>
    );
  }

  // Si no hay autenticación, mostrar el login
  if (!isAuthenticated) {
    return <Login />;
  }


  // Si hay conexión, mostrar la aplicación normal
  return (
    <main className="overflow-y-auto h-full">
      <HomeMainHeader />
      <div className="h-[calc(100vh-6rem)]">
        <Switch>
          <Route path="/" component={ExploreSection} />
          <Route path="/my-instances">
            {() => <MyInstancesSection offlineMode={false} />}
          </Route>
          <Route path="/prelaunch/:instanceId">
            {(params) => <PreLaunchInstance instanceId={params.instanceId} />}
          </Route>
          <Route path="/modpack/:modpackId">
            {(params) => <ModpackOverview modpackId={params.modpackId} />}
          </Route>
          <Route path="/mc-accounts" component={AccountsSection} />

          {
            session?.publisher?.id && (
              <Route path="/creators">
                <div>
                  Contenido exclusivo para creadores
                </div>
              </Route>
            )
          }
          <Route>
            <NotFound />
          </Route>
        </Switch>
        <NoticeTestBuild />
        <CommandPalette />
        <KonamiCode />
      </div>
    </main>
  );
}

export default App;