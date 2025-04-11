import { useGlobalContext } from "@/stores/GlobalContext";
import {
    LucideDownload,
    LucideCheckCircle2,
    LucideAlertCircle,
} from "lucide-react";
import { cn } from "@/lib/utils";

export const UpdateStatus = () => {
    const {
        isUpdating,
        updateProgress,
        updateVersion,
        updateState,
    } = useGlobalContext();

    const icon = {
        downloading: <LucideDownload className="size-5 animate-spin text-blue-400" />,
        done: <LucideCheckCircle2 className="size-5 text-green-400" />,
        error: <LucideAlertCircle className="size-5 text-red-400" />,
    }[updateState as string];

    return (
        <div className="flex items-start gap-3 fixed bottom-6 right-6 z-50 w-80 p-4 rounded-2xl shadow-xl border border-white/10 bg-gradient-to-br from-zinc-900/90 to-zinc-800/90 backdrop-blur-md text-white animate-fade-in-up animate-duration-400">
            {/* Icon */}
            <div className="flex-shrink-0 mt-1">
                {icon}
            </div>

            {/* Content */}
            <div className="flex flex-col gap-1 w-full">
                <div className="text-sm font-medium capitalize">
                    {updateState === "downloading" && "Downloading update..."}
                    {updateState === "done" && "Update installed!"}
                    {updateState === "error" && "Ocurrió un error"}
                    {updateState !== "downloading" &&
                        updateState !== "done" &&
                        updateState !== "error" && updateState}
                </div>

                {/* Version */}
                {updateVersion && (
                    <span className="text-xs text-gray-400">
                        Version: {updateVersion}
                    </span>
                )}

                {/* Progress bar */}
                {isUpdating && updateProgress > 0 && updateProgress < 100 && (
                    <div className="h-1.5 bg-zinc-700 rounded-full overflow-hidden mt-1">
                        <div
                            className="h-full bg-blue-500 transition-all duration-200"
                            style={{ width: `${updateProgress}%` }}
                        ></div>
                    </div>
                )}

                {/* Success or Error message */}
                {updateState === "done" && (
                    <span className="text-xs text-green-400 mt-1">Ready to launch!</span>
                )}
                {updateState === "error" && (
                    <span className="text-xs text-red-400 mt-1">
                        { }
                    </span>
                )}
            </div>
        </div>
    );
};
