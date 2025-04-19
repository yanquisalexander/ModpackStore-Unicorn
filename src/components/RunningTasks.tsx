import { useTasksContext } from "@/stores/TasksContext";
import { LucideCheck, LucideInfo, LucideRefreshCcw, LucideTrash2, LucideX } from "lucide-react";
import { useEffect, useRef, useState } from "react";

export const RunningTasks = () => {
    const { tasks, hasRunningTasks, taskCount } = useTasksContext();
    const [openMenu, setOpenMenu] = useState(false);
    const containerRef = useRef<HTMLDivElement>(null);

    const toggleMenu = () => {
        setOpenMenu(!openMenu);
    };

    const closeMenu = () => {
        setOpenMenu(false);
    };

    useEffect(() => {
        const handleClickOutside = (event: MouseEvent) => {
            if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
                closeMenu();
            }
        };

        if (openMenu) {
            document.addEventListener("mousedown", handleClickOutside);
        } else {
            document.removeEventListener("mousedown", handleClickOutside);
        }

        return () => {
            document.removeEventListener("mousedown", handleClickOutside);
        };
    }, [openMenu]);

    // Return null if there are no running tasks
    if (!hasRunningTasks && taskCount === 0) return null;

    const baseClasses = "flex items-center justify-center size-9 aspect-square transition-all rounded-md backdrop-blur-xl cursor-pointer";

    // Helper function to get status icon
    const getStatusIcon = (status: string) => {
        switch (status) {
            case "Completed":
                return <LucideCheck size={16} className="text-green-500" />;
            case "Failed":
                return <LucideX size={16} className="text-red-500" />;
            case "Cancelled":
                return <LucideTrash2 size={16} className="text-yellow-500" />;
            case "Running":
                return <LucideRefreshCcw size={16} className="animate-spin" />;
            default:
                return <LucideInfo size={16} />;
        }
    };

    return (
        <div className="relative self-center" ref={containerRef}>
            <button
                onClick={toggleMenu}
                className={`relative ${baseClasses}`}
                title="Tareas en progreso"
                aria-label="Tareas en progreso"
            >
                {taskCount >= 1 && (
                    <span className="absolute top-1 -right-1 bg-sky-600 size-4 text-xs text-white rounded-full px-1">
                        {taskCount}
                    </span>
                )}
                <LucideRefreshCcw className={`size-4 ${hasRunningTasks ? "animate-duration-[1500ms] animate-spin-clockwise animate-iteration-count-infinite" : ""} text-white`} />
            </button>

            <div
                style={{
                    opacity: openMenu ? 1 : 0,
                    visibility: openMenu ? "visible" : "hidden",
                    transform: openMenu ? "translateY(0)" : "translateY(-5px)",
                    transition: "opacity 0.2s ease, visibility 0.2s ease, transform 0.2s ease",
                }}
                className="absolute right-0 mt-2 w-64 bg-neutral-900 border border-neutral-700 rounded shadow-lg z-50 p-2"
            >
                <div className="text-sm text-white flex flex-col">
                    <div className="py-1 px-2 font-medium border-b border-neutral-700 mb-2">
                        Tareas activas ({taskCount})
                    </div>

                    {tasks.length === 0 ? (
                        <div className="text-neutral-400 text-center py-4">
                            No hay tareas en progreso
                        </div>
                    ) : (
                        <div className="max-h-64 overflow-y-auto">
                            {tasks.map((task) => (
                                <div key={task.id} className="py-2 px-2 hover:bg-neutral-800 rounded flex flex-col">
                                    <div className="flex justify-between items-center">
                                        <div className="flex items-center gap-2">
                                            {getStatusIcon(task.status)}
                                            <span className="font-medium">{task.label}</span>
                                        </div>
                                        <span className="text-xs text-neutral-400">{task.progress}%</span>
                                    </div>
                                    {task.message && (
                                        <div className="ml-6 text-xs text-neutral-400 truncate">
                                            {task.message}
                                        </div>
                                    )}
                                    <div className="ml-6 mt-1 w-full bg-neutral-800 h-1 rounded-full">
                                        <div
                                            className="bg-sky-600 h-1 rounded-full"
                                            style={{ width: `${task.progress}%` }}
                                        ></div>
                                    </div>
                                </div>
                            ))}
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
};