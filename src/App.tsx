import { useEffect, useState, useCallback } from "react";
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
  const { isConnected, isLoading: connectionLoading, hasInternetAccess } = useCheckConnection();
  const [hasShownConnectionToast, setHasShownConnectionToast] = useState(false);


  // Optimizado: Control de notificaciones de conexión con estado para evitar notificaciones duplicadas
  useEffect(() => {
    const connectionToastId = "connection-check";

    if (connectionLoading) {
      toast.loading("Verificando conexión...", { id: connectionToastId });
      return;
    }

    // Solo mostrar toast si no se ha mostrado antes o si el estado cambió
    if (!hasShownConnectionToast) {
      if (!isConnected) {
        if (hasInternetAccess) {
          toast.warning("Servidor no disponible", {
            id: connectionToastId,
            duration: 5000,
            richColors: true,
            description: "No hemos podido conectarnos al servidor.\n\nAlgunas funciones pueden no estar disponibles.",
          });
        } else {
          toast.warning("Sin conexión a internet", {
            id: connectionToastId,
            duration: 5000,
            richColors: true,
            description: "No se detectó una conexión a internet activa.\n\nEstás en modo sin conexión.",
          });
        }
      }
      setHasShownConnectionToast(true);
    }
  }, [isConnected, connectionLoading, hasInternetAccess, hasShownConnectionToast]);

  // Optimizado: Inicialización única con array de dependencias vacío
  useEffect(() => {
    initAnalytics();
    preloadSounds();

    try {
      trackEvent("app_launch", {
        name: "App Launch",
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      console.error("Error tracking app launch event:", error);
    }

    // Función de limpieza (cleanup) para evitar efectos secundarios
    return () => {
      // Aquí podrías añadir lógica de limpieza si fuera necesaria
    };
  }, []); // Array vacío para ejecutar solo una vez al montar

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
        <Route component={NotFound} />
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

          {session?.publisher?.id && (
            <Route path="/creators">
              <div>Contenido exclusivo para creadores</div>
            </Route>
          )}
          <Route component={NotFound} />
        </Switch>
        <NoticeTestBuild />
        <CommandPalette />
        <KonamiCode />
      </div>
    </main>
  );
}

export default App;