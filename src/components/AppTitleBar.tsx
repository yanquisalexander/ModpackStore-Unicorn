import { LucideArrowLeft, LucideDownload, LucideIcon, LucideMinus, LucidePictureInPicture2, LucideRefreshCcw, LucideSquare, LucideX } from "lucide-react";
import { useEffect, useState } from "react";
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useGlobalContext } from "../stores/GlobalContext";
import { Link } from "wouter";
import { exit } from '@tauri-apps/plugin-process';
import PatreonIcon from "@/icons/PatreonIcon";
import { open } from "@tauri-apps/plugin-shell";
import { useTasksContext } from "@/stores/TasksContext";

export const AppTitleBar = () => {
    const [window, setWindow] = useState(getCurrentWindow());
    const [isMaximized, setIsMaximized] = useState(false);
    const { titleBarState, updateState, applyUpdate } = useGlobalContext()
    const { hasRunningTasks, taskCount } = useTasksContext()


    useEffect(() => {
        const handleResize = () => {
            window.isMaximized().then(setIsMaximized);
        };

        window.onResized(handleResize).then(() => {
            handleResize();
        })

        return () => {


        };
    }, [window]);

    const handlePatreonClick = async () => {
        try {
            await open("https://www.patreon.com/AlexitooDEV")
        } catch (error) {
            console.error("Error opening Patreon link:", error);
        }
    }


    const handleMaximize = () => {
        if (isMaximized) {
            window.unmaximize().then(() => setIsMaximized(false));
        } else {
            window.maximize().then(() => setIsMaximized(true));
        }
    };

    const handleClose = () => {
        // Confirmation dialog before closing the window
        if (confirm("Are you sure you want to close the window?")) {
            window.close().then(() => {
                exit(0) // Close the application after closing the window
            })

        }
    };



    const handleMinimize = () => {
        window.minimize()
    };


    return (
        <div
            data-tauri-drag-region className={`flex z-999 top-0 h-9 transition  ease-in-out w-full items-center justify-between sticky text-white select-none ${titleBarState.opaque ? 'bg-ms-primary' : 'bg-transparent'}`}>
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
                                <img src={titleBarState.icon} className={`size-6  ${titleBarState.customIconClassName}`} alt="icon" />
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


            <div className="flex ml-auto  border-r px-1 mr-1 border-white/10">
                {
                    updateState === 'done' && (
                        <button
                            onClick={applyUpdate}
                            title="Listo para reiniciar"
                            className="cursor-pointer flex animate-fade-in-down duration-500 size-9 aspect-square items-center justify-center hover:bg-neutral-800" aria-label="Settings">
                            <LucideDownload className="size-4 text-green-400" />
                        </button>
                    )
                }

                {
                    hasRunningTasks && (
                        <button
                            title="Tareas en progreso"
                            className="cursor-pointer relative flex animate-fade-in-down duration-500 size-9 aspect-square items-center justify-center group hover:bg-neutral-800" aria-label="Settings">
                            {
                                taskCount >= 1 ? <span className="absolute top-1 -right-1 bg-sky-600 size-4 text-xs text-white rounded-full px-1">{taskCount}</span> : null
                            }
                            <LucideRefreshCcw className="size-4 animate-delay-700 animate-iteration-count-infinite animate-duration-[1500ms] animate-rotate-360 text-white" />
                        </button>
                    )
                }

                <button
                    onClick={handlePatreonClick}
                    title="Colaborar con el desarrollo"
                    className="cursor-pointer flex group size-9 aspect-square items-center justify-center" aria-label="Settings">
                    <PatreonIcon className="size-4 text-white/80 group-hover:text-pink-500 transition duration-300" />
                </button>

            </div>


            {/* Right side - window controls */}
            <div className="flex items-center justify-end">
                <button
                    className="cursor-pointer flex size-9 aspect-square items-center justify-center hover:bg-neutral-800"
                    aria-label="Minimize"
                    onClick={handleMinimize}>
                    <LucideMinus className="h-4 w-4" />
                </button>
                <button
                    className="cursor-pointer flex size-9 aspect-square items-center justify-center hover:bg-neutral-800"
                    aria-label="Maximize"
                    onClick={handleMaximize}
                >
                    {
                        isMaximized ? <LucidePictureInPicture2 className="h-4 w-4" /> : <LucideSquare className="h-3.5 w-3.5" />
                    }
                </button>
                <button
                    onClick={handleClose}
                    className="cursor-pointer flex size-9 aspect-square items-center justify-center hover:bg-red-600" aria-label="Close">
                    <LucideX className="h-4 w-4" />
                </button>
            </div>
        </div>
    );
}