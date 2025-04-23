import { LucideArrowLeft, LucideDownload, LucideMinus, LucidePictureInPicture2, LucideSquare, LucideX } from "lucide-react";
import { useEffect, useState } from "react";
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useGlobalContext } from "../stores/GlobalContext";
import { Link } from "wouter";
import { exit } from '@tauri-apps/plugin-process';
import PatreonIcon from "@/icons/PatreonIcon";
import { open } from "@tauri-apps/plugin-shell";
import { CurrentUser } from "./CurrentUser";
import { RunningInstances } from "./RunningInstances";
import { RunningTasks } from "./RunningTasks";
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle
} from '@/components/ui/alert-dialog';

export const AppTitleBar = () => {
    const [window, setWindow] = useState(getCurrentWindow());
    const [isMaximized, setIsMaximized] = useState(false);
    const [isExitDialogOpen, setIsExitDialogOpen] = useState(false);
    const { titleBarState, updateState, applyUpdate } = useGlobalContext();

    useEffect(() => {
        const handleResize = async () => {
            const maximized = await window.isMaximized();
            setIsMaximized(maximized);
        };

        const cleanup = async () => {
            const unlisten = await window.onResized(handleResize);
            handleResize();
            return unlisten;
        };

        const unlistenPromise = cleanup();

        return () => {
            unlistenPromise.then(unlisten => unlisten());
        };
    }, [window]);

    const handlePatreonClick = async () => {
        try {
            await open("https://www.patreon.com/AlexitooDEV");
        } catch (error) {
            console.error("Error opening Patreon link:", error);
        }
    };

    const handleMaximize = async () => {
        if (isMaximized) {
            await window.unmaximize();
            setIsMaximized(false);
        } else {
            await window.maximize();
            setIsMaximized(true);
        }
    };

    const handleClose = () => {
        setIsExitDialogOpen(true);
    };

    const confirmClose = async () => {
        await window.close();
        exit(0); // Close the application after closing the window
    };

    const handleMinimize = () => {
        window.minimize();
    };

    return (
        <>
            <div
                data-tauri-drag-region
                className={`flex z-999 top-0 h-9 transition ease-in-out w-full items-center justify-between sticky text-white select-none ${titleBarState.opaque ? 'bg-ms-primary' : 'bg-transparent'}`}
            >
                <div className="flex items-center justify-center">
                    <div className="flex items-center gap-2">
                        <Link
                            href="/"
                            className={`cursor-pointer transition-transform duration-500 flex size-9 aspect-square items-center justify-center hover:bg-neutral-800 ${!titleBarState.canGoBack && '-translate-x-9'}`}
                            aria-label="Back"
                        >
                            <LucideArrowLeft className="h-4 w-4 text-white" />
                        </Link>

                        <div className={`flex gap-x-2 !pointer-events-none select-none duration-500 items-center justify-center text-white/80 transition ${!titleBarState.canGoBack ? '-translate-x-7' : ''}`}>
                            {
                                titleBarState.icon && typeof titleBarState.icon === "string" ? (
                                    <img
                                        onError={(e) => {
                                            e.currentTarget.onerror = null; // Prevents looping
                                            e.currentTarget.src = "/images/modpack-fallback.webp"; // Fallback icon
                                        }}
                                        src={titleBarState.icon}
                                        className={`size-6 ${titleBarState.customIconClassName}`}
                                        alt="icon"
                                    />
                                ) : (
                                    titleBarState.icon ? (
                                        <titleBarState.icon className={`size-6 p-0.5 rounded-md border border-solid border-white/10 ${titleBarState.customIconClassName ?? 'bg-pink-500/20'}`} />
                                    ) : null
                                )
                            }

                            <span className="text-sm font-normal">
                                {titleBarState.title}
                            </span>
                        </div>
                    </div>
                </div>

                <div className="flex ml-auto border-r px-1 mr-1 border-white/10">
                    {updateState === 'ready-to-install' && (
                        <button
                            onClick={applyUpdate}
                            title="Listo para reiniciar"
                            className="cursor-pointer flex animate-fade-in-down duration-500 size-9 aspect-square items-center justify-center hover:bg-neutral-800"
                            aria-label="Update"
                        >
                            <LucideDownload className="size-4 text-green-400" />
                        </button>
                    )}

                    <RunningTasks />
                    <RunningInstances />

                    <button
                        onClick={handlePatreonClick}
                        title="Colaborar con el desarrollo"
                        className="cursor-pointer flex group size-9 aspect-square items-center justify-center hover:bg-neutral-800"
                        aria-label="Patreon"
                    >
                        <PatreonIcon className="size-4 text-white/80 group-hover:text-pink-500" />
                    </button>

                    <CurrentUser titleBarOpaque={titleBarState.opaque} />
                </div>

                {/* Right side - window controls */}
                <div className="flex items-center justify-end">
                    <button
                        className="cursor-pointer flex size-9 aspect-square items-center justify-center hover:bg-neutral-800"
                        aria-label="Minimize"
                        onClick={handleMinimize}
                    >
                        <LucideMinus className="h-4 w-4" />
                    </button>
                    <button
                        className="cursor-pointer flex size-9 aspect-square items-center justify-center hover:bg-neutral-800"
                        aria-label="Maximize"
                        onClick={handleMaximize}
                    >
                        {isMaximized
                            ? <LucidePictureInPicture2 className="h-4 w-4" />
                            : <LucideSquare className="h-3.5 w-3.5" />
                        }
                    </button>
                    <button
                        onClick={handleClose}
                        className="cursor-pointer flex size-9 aspect-square items-center justify-center hover:bg-red-600"
                        aria-label="Close"
                    >
                        <LucideX className="h-4 w-4" />
                    </button>
                </div>
            </div>

            <AlertDialog open={isExitDialogOpen} onOpenChange={setIsExitDialogOpen}>
                <AlertDialogContent className="bg-neutral-900 border border-neutral-800 text-white">
                    <AlertDialogHeader>
                        <AlertDialogTitle>¿Estás seguro?</AlertDialogTitle>
                        <AlertDialogDescription className="text-neutral-400">
                            ¿Realmente quieres cerrar la aplicación?
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel className="bg-neutral-800 hover:bg-neutral-700 text-white border-none">
                            Cancelar
                        </AlertDialogCancel>
                        <AlertDialogAction
                            onClick={confirmClose}
                            className="bg-red-600 hover:bg-red-700 text-white border-none"
                        >
                            Salir
                        </AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </>
    );
};