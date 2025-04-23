import { Link } from "wouter"
import { LucidePlay, LucideHardDrive, LucideMoreVertical, LucideSettings, LucideTrash2, LucideDownload, LucideRefreshCw, LucideGamepad2 } from "lucide-react"
import { useState } from "react"
import {
    ContextMenu,
    ContextMenuContent,
    ContextMenuItem,
    ContextMenuSeparator,
    ContextMenuTrigger,
} from "@/components/ui/context-menu"
import { toast } from "sonner"
import { playSound } from "@/utils/sounds"
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
} from "@/components/ui/alert-dialog"
import { invoke } from "@tauri-apps/api/core"

//                            onDelete={() => openDeleteDialog(instance)}


export const InstanceCard = ({ instance, className = "", running, onInstanceRemoved }: { instance: any, className?: string, running?: boolean, onInstanceRemoved: () => void }) => {
    const [isOpen, setIsOpen] = useState(false)
    const [showDeleteAlert, setShowDeleteAlert] = useState(false)

    const handleContextAction = (action: string) => {
        playSound("ERROR_NOTIFICATION")
        toast("Acción no implementada", {
            description: `La acción "${action}" no está implementada en este momento.`,
            duration: 3000,
            action: {
                label: "Cerrar",
                onClick: () => toast.dismiss(),
            },
        })
    }

    const handleDeleteInstance = async () => {
        try {
            await invoke('remove_instance', { instanceId: instance.instanceId })
            //playSound("SUCCESS_NOTIFICATION")
            toast.success('Instancia eliminada correctamente')
            onInstanceRemoved()
        } catch (error) {
            playSound("ERROR_NOTIFICATION")
            console.error('Error al eliminar instancia:', error)
            toast.error(`Error al eliminar instancia: ${(error as any)?.message || 'Error desconocido'}`)
        } finally {
            setShowDeleteAlert(false)
        }
    }

    return (
        <>
            <ContextMenu onOpenChange={setIsOpen} modal={false}>
                <ContextMenuTrigger asChild>
                    <article className={`z-10 group relative overflow-hidden rounded-xl border border-white/20 h-full
                      transition 
                      before:left-1/2 before:bottom-0 before:-translate-x-1/2 before:w-full before:h-1/2 
                      before:rounded-full before:bg-black before:absolute before:translate-y-full 
                      hover:before:translate-y-1/2 before:blur-3xl before:-z-10 before:transition before:duration-200 
                      after:left-0 after:bottom-0 after:-translate-x-full after:after:translate-y-full 
                      hover:after:-translate-x-1/2 hover:after:translate-y-1/2 after:w-2/2 after:aspect-square 
                      after:rounded-2xl after:bg-black after:absolute after:blur-3xl hover:after:opacity-40 
                      after:-z-10 after:opacity-0 after:transition after:duration-200 ${className} ${isOpen ? 'ring-2 ring-sky-500 ring-offset-2 ring-offset-black' : ''}`}>

                        <Link href={`/prelaunch/${instance.instanceId}`} className="flex aspect-video flex-col h-full p-4">
                            {/* Background image */}
                            <img
                                src={instance.bannerUrl || "/images/modpack-fallback.webp"}
                                onError={(e) => { e.currentTarget.src = "/images/modpack-fallback.webp" }}
                                className="absolute inset-0 -z-20 transform-gpu animate-fade-in object-cover w-full h-full rounded-xl transition duration-500 group-hover:scale-105 group-hover:opacity-80"
                                alt={instance.instanceName}
                            />

                            {/* 
                                Running badge, pero a la inversa (Se debe mostrar cuando no es hover)
                            */}

                            {
                                running && (
                                    <div className="opacity-100 flex transition flex-col gap-2 flex-1 absolute top-4 left-3">
                                        <div className="flex justify-end items-center flex-wrap gap-2 transition opacity-100 -translate-y-1 group-hover:translate-y-0 group-hover:opacity-0 duration-300">
                                            <span className="backdrop-blur-2xl text-xs border rounded-full inline-flex items-center gap-1 py-1 px-2 font-medium bg-green-600 text-white border-white/10">
                                                <LucideGamepad2 className="h-4 w-auto" />
                                                Corriendo
                                            </span>
                                        </div>
                                    </div>
                                )
                            }



                            {/* Badge for installed status */}
                            <div className="opacity-100 flex transition flex-col gap-2 flex-1">
                                <div className="flex justify-end items-center flex-wrap gap-2 transition group-hover:opacity-100 -translate-y-1 group-hover:translate-y-0 opacity-0 duration-300">
                                    <span className="backdrop-blur-2xl text-xs border rounded-full inline-flex items-center gap-1 py-1 px-2 font-medium bg-emerald-700 text-white border-white/10">
                                        <LucideHardDrive className="h-4 w-auto" />
                                        Instalada
                                    </span>
                                </div>
                            </div>

                            {/* Title and actions section */}
                            <div className="flex flex-wrap gap-y-6 items-end justify-between mt-8 transition group-hover:opacity-100 translate-y-1 group-hover:translate-y-0 opacity-0 duration-300">
                                <div>
                                    <h2 className="text-lg mt-auto text-white leading-snug font-medium text-balance max-w-[28ch] group-hover:text-sky-200">
                                        {instance.instanceName}
                                    </h2>
                                    <div className="flex flex-col justify-center gap-0.5 mt-1 text-sm text-gray-300">
                                        <p className="text-xs text-gray-400">
                                            Minecraft {instance.minecraftVersion}
                                        </p>
                                        {
                                            instance.forgeVersion && (
                                                <p className="text-xs text-gray-400">
                                                    Forge {instance.forgeVersion}
                                                </p>
                                            )
                                        }
                                    </div>

                                </div>
                                <span className="text-white rounded-lg bg-gray-800/20 border border-gray-400/40 py-2 px-4 flex items-center gap-1.5 group-hover:scale-105 transition text-sm group-hover:bg-gray-800/80">
                                    <LucidePlay className="h-4 w-auto" />
                                    Jugar
                                </span>
                            </div>
                        </Link>
                    </article>
                </ContextMenuTrigger>
                <ContextMenuContent className="w-64 text-gray-100">

                    <ContextMenuItem
                        onClick={() => handleContextAction("settings")}
                        className="hover:bg-neutral-800 focus:bg-neutral-800 cursor-pointer"
                    >
                        <LucideSettings className="mr-2 h-4 w-4" />
                        <span>Configurar</span>
                    </ContextMenuItem>

                    <ContextMenuItem
                        onClick={() => handleContextAction("backup")}
                        className="hover:bg-neutral-800 focus:bg-neutral-800 cursor-pointer"
                    >
                        <LucideDownload className="mr-2 h-4 w-4" />
                        <span>Crear copia de seguridad</span>
                    </ContextMenuItem>

                    <ContextMenuSeparator className="bg-neutral-800" />

                    <ContextMenuItem
                        onClick={() => {
                            if (running) {
                                playSound("ERROR_NOTIFICATION")
                                toast.warning("No puedes eliminar una instancia que está en ejecución")
                                return
                            }
                            setShowDeleteAlert(true)
                        }}
                        className="hover:bg-red-700 focus:bg-red-700 text-red-400 hover:text-white focus:text-white cursor-pointer"
                    >
                        <LucideTrash2 className="mr-2 h-4 w-4 text-inherit" />
                        <span>Eliminar instancia</span>
                    </ContextMenuItem>
                </ContextMenuContent>
            </ContextMenu>

            <AlertDialog open={showDeleteAlert} onOpenChange={setShowDeleteAlert}>
                <AlertDialogContent className="bg-neutral-900 border-neutral-700 text-white">
                    <AlertDialogHeader>
                        <AlertDialogTitle className="text-white">¿Eliminar instancia?</AlertDialogTitle>
                        <AlertDialogDescription className="text-gray-300">
                            ¿Estás seguro de que deseas eliminar la instancia <span className="font-semibold text-white">{instance.instanceName}</span>? Esta acción no se puede deshacer.
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel className="bg-neutral-800 text-white border-neutral-700 hover:bg-neutral-700">
                            Cancelar
                        </AlertDialogCancel>
                        <AlertDialogAction
                            className="bg-red-700 text-white hover:bg-red-600"
                            onClick={(e) => {
                                e.preventDefault();
                                handleDeleteInstance();
                            }}
                        >
                            Eliminar
                        </AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </>
    )
}