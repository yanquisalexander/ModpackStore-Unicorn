import { CreateInstanceDialog } from "@/components/CreateInstanceDialog";
import { InstanceCard } from "@/components/InstanceCard";
import { trackSectionView } from "@/lib/analytics";
import { useGlobalContext } from "@/stores/GlobalContext";
import { useInstances } from "@/stores/InstancesContext";
import { useTasksContext } from "@/stores/TasksContext";
import { TauriCommandReturns } from "@/types/TauriCommandReturns";
import { useCheckConnection } from "@/utils/checkConnection";
import { invoke } from "@tauri-apps/api/core";
import { LucidePackageOpen } from "lucide-react";
import { useEffect, useState } from "react"


export const MyInstancesSection = ({ offlineMode }: { offlineMode?: boolean }) => {
    const { titleBarState, setTitleBarState } = useGlobalContext()
    const { hasInternetAccess } = useCheckConnection()
    const { instances: instancesOnContext } = useInstances()
    const { instancesBootstraping } = useTasksContext()
    console.log({ instancesBootstraping })

    const [instances, setInstances] = useState<any[]>([])
    const fetchInstances = async () => {
        const instances = await invoke('get_all_instances') as any
        console.log("Instances fetched from Tauri:", instances)
        setInstances(instances)
    }

    useEffect(() => {

        fetchInstances()
    }, [])

    useEffect(() => {
        if (offlineMode) return // Prevents setting title bar state if in offline mode
        setTitleBarState({
            ...titleBarState,
            title: "Mis instancias",
            icon: LucidePackageOpen,
            canGoBack: true,
            customIconClassName: "bg-yellow-500/10",
            opaque: true,
        });

        trackSectionView("my-instances")
    }, [])

    return (
        <div className="mx-auto max-w-7xl px-8 py-10 overflow-y-auto h-full">
            <header className="flex flex-col mb-16">
                <h1 className="tracking-tight inline font-semibold text-2xl bg-gradient-to-b from-teal-200 to-teal-500 bg-clip-text text-transparent">
                    Mis instancias
                </h1>
                <p className="text-gray-400 text-base max-w-2xl">
                    Aquí puedes ver y gestionar todas tus instancias de Modpack Store.
                </p>
            </header>

            <div className="grid grid-cols-1 sm:grid-cols-3 lg:grid-cols-4 gap-4">
                {instances.map((instance) => (
                    <InstanceCard
                        key={instance.instanceId}
                        instance={instance}
                        isBootstrapping={instancesBootstraping.some((id) => id === instance.instanceId)}
                        onInstanceRemoved={fetchInstances}
                        running={instancesOnContext.some((i) => i.id === instance.instanceId && i.status === "running")}
                    />
                ))}
                {
                    (!offlineMode || hasInternetAccess) && (
                        <CreateInstanceDialog onInstanceCreated={fetchInstances} />
                    )
                }
            </div>


        </div >
    )
}