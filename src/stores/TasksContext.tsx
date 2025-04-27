import { createContext, useContext, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";

export type TaskStatus = "Pending" | "Running" | "Completed" | "Failed" | "Cancelled";

export type TaskInfo = {
    id: string;
    label: string;
    status: TaskStatus;
    progress: number;
    message: string;
};

type TaskContextType = {
    tasks: TaskInfo[];
    setTasks: React.Dispatch<React.SetStateAction<TaskInfo[]>>;
    hasRunningTasks: boolean;
    taskCount: number;
};

const TasksContext = createContext<TaskContextType | undefined>(undefined);

export const TasksProvider = ({ children }: { children: React.ReactNode }) => {
    const [tasks, setTasks] = useState<TaskInfo[]>([]);
    const hasRunningTasks = tasks.some((task) => task.status === "Running");
    const taskCount = tasks.length;

    useEffect(() => {
        const unlisten1 = listen<string>("task-created", (event) => {
            console.log("Nueva tarea creada:", event.payload);
        });

        const unlisten2 = listen<TaskInfo>("task-updated", (event) => {
            setTasks((prev) => {
                const updated = [...prev];
                const idx = updated.findIndex((t) => t.id === event.payload.id);
                if (idx !== -1) {
                    updated[idx] = event.payload;
                } else {
                    updated.push(event.payload);
                }
                return updated;
            });
        });

        const unlisten3 = listen<string>("task-removed", (event) => {
            setTasks((prev) => prev.filter((task) => task.id !== event.payload));
        });

        return () => {
            unlisten1.then((fn) => fn());
            unlisten2.then((fn) => fn());
            unlisten3.then((fn) => fn());
        };
    }, []);

    return (
        <TasksContext.Provider value={{ tasks, setTasks, hasRunningTasks, taskCount }}>
            {children}
        </TasksContext.Provider>
    );
};

export const useTasksContext = () => {
    const ctx = useContext(TasksContext);
    if (!ctx) throw new Error("useTaskContext must be used within a TaskProvider");
    return ctx;
};
