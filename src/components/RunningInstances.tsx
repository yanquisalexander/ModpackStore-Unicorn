import { useInstances } from "@/stores/InstancesContext";
import { LucidePackageSearch } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { Link } from "wouter";

/* 
    Titlebar button that shows current active instances of Minecraft
*/

export const RunningInstances = () => {
    const { instances } = useInstances();
    const [openMenu, setOpenMenu] = useState(false);
    const containerRef = useRef<HTMLDivElement>(null);

    const runningInstances = instances.filter(instance => instance.status === "running");

    const toggleMenu = () => {
        setOpenMenu(prev => !prev);
    };

    useEffect(() => {
        const handleClickOutside = (event: MouseEvent) => {
            if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
                setOpenMenu(false);
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

    if (runningInstances.length === 0) return null;

    return (
        <div className="relative" ref={containerRef}>
            <button
                onClick={toggleMenu}
                title="Toggle Instances Menu"
                className={`cursor-pointer flex animate-fade-in-down duration-500 size-9 aspect-square items-center justify-center hover:bg-neutral-800 ${openMenu ? "bg-neutral-800" : ""}`}
                aria-label="Toggle Menu"
            >
                <LucidePackageSearch className="size-4" />
                <span className="absolute top-1 -right-1 bg-sky-600 size-4 text-xs text-white rounded-full px-1">
                    {runningInstances.length}
                </span>
            </button>

            {openMenu && (
                <div className="absolute right-0 mt-2 w-48 bg-neutral-900 border border-neutral-700 rounded shadow-lg z-50 p-2 animate-fade-in animate-duration-100">
                    <h3 className="text-sm font-semibold text-white mb-2">
                        Instancias en ejecución
                    </h3>
                    <div className="border-b border-neutral-700 mb-2"></div>
                    <ul className="text-sm text-white flex flex-col">
                        {runningInstances.map(instance => (
                            <Link
                                href={`/prelaunch/${instance.id}`}
                                key={instance.id} className="w-full py-1 px-2 hover:bg-neutral-800 rounded">
                                {instance.name}
                            </Link>
                        ))}
                    </ul>
                </div>
            )}
        </div>
    );
};
