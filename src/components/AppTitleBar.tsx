import { LucideArrowLeft, LucideIcon, LucideMinus, LucidePictureInPicture2, LucideSquare, LucideX } from "lucide-react";
import { useEffect, useState } from "react";
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useGlobalContext } from "../stores/GlobalContext";
import { Link } from "wouter";

export const AppTitleBar = () => {
    const [window, setWindow] = useState(getCurrentWindow());
    const [isMaximized, setIsMaximized] = useState(false);
    const { titleBarState } = useGlobalContext()

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


    const handleMaximize = () => {
        if (isMaximized) {
            window.unmaximize().then(() => setIsMaximized(false));
        } else {
            window.maximize().then(() => setIsMaximized(true));
        }
    };

    const handleClose = () => {
        // Logic to close the window
        console.log("Close button clicked");
    };

    const handleMinimize = () => {
        window.minimize()
    };

    return (
        <div data-tauri-drag-region className="flex h-9 w-full items-center justify-between bg-transparent sticky z-10 text-white select-none">
            <div className="flex items-center justify-center">
                <div className="flex items-center gap-2">
                    <Link
                        href="/"
                        className={`cursor-pointer transition-transform flex size-9 aspect-square items-center justify-center hover:bg-neutral-800 ${!titleBarState.canGoBack && '-translate-x-9'}`}
                        aria-label="Back"
                    >
                        <LucideArrowLeft className="h-4 w-4 text-white" />
                    </Link>

                    <div className={`flex gap-x-2 !pointer-events-none select-none items-center justify-center text-white/80 transition ${!titleBarState.canGoBack ? '-translate-x-7' : ''}`}>

                        {
                            titleBarState.icon && typeof titleBarState.icon === "string" ? (
                                <img src={titleBarState.icon} className="h-5 w-5 rounded-md border border-solid border-white/10" />
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

            {/* Right side - window controls */}
            <div className="flex w-24 items-center justify-end px-2">
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
                <button className="cursor-pointer flex size-9 aspect-square items-center justify-center hover:bg-red-600" aria-label="Close">
                    <LucideX className="h-4 w-4" />
                </button>
            </div>
        </div>
    );
}