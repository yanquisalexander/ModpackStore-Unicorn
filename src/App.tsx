import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import { Link, Route, Switch, useLocation, useRouter } from "wouter";
import { Button } from "@material-tailwind/react";
import { HomeMainHeader } from "./components/home/MainHeader";
import { toast } from "sonner";
import { ExploreSection } from "./views/ExploreSection";
import { ConfigurationSection } from "./views/ConfigurationSection";
import { PreLaunchInstance } from "./views/PreLaunchInstance";
import { useCheckConnection } from "./utils/checkConnection";
import { LucideLoader } from "lucide-react";
import { MyInstancesSection } from "./views/MyInstancesSection";
import { UpdateStatus } from "./components/UpdateStatus";


function App() {


  const { isConnected, isLoading } = useCheckConnection();


  useEffect(() => {
    if (isLoading) {
      toast.loading("Verificando conexión a internet...", { id: "connection-check" });
    } else {
      if (!isConnected) {
        toast.warning("Sin conexión",
          {
            id: "connection-check",
            duration: 5000,
            richColors: true,
            description: "No se pudo establecer conexión a internet.\n\nAlgunas funciones pueden no estar disponibles.",
          })
      }
    }


  }, [isConnected, isLoading]);


  return (
    <main className="grow  overflow-y-auto">
      {
        isLoading ? (
          <div className="flex items-center justify-center min-h-dvh h-full w-full">
            <LucideLoader className="size-10 -mt-12 animate-spin-clockwise animate-iteration-count-infinite animate-duration-1000 text-white" />
          </div>
        ) : (
          <>
            <UpdateStatus />
            <HomeMainHeader session={null} />
            <Switch>
              <Route path="/" component={ExploreSection} />
              <Route path="/my-instances" component={MyInstancesSection} />
              <Route path="/prelaunch" component={PreLaunchInstance} />


              <Route path="/my-instances">
                Mis Instancias
              </Route>

              <Route path="/settings" component={ConfigurationSection} />

              <Route>404: No such page!</Route>

            </Switch>
          </>
        )
      }


    </main>
  );
}

export default App;
