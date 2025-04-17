import { useAuthentication } from "@/stores/AuthContext";
import { LucideLogOut, LucidePackageOpen, LucideSettings2, LucideSquareUserRound } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { Link } from "wouter";

export const CurrentUser = ({ titleBarOpaque }: { titleBarOpaque?: boolean }) => {
    const { session, logout, isAuthenticated } = useAuthentication();
    const [openMenu, setOpenMenu] = useState(false);
    const containerRef = useRef<HTMLDivElement>(null);



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

    const baseClasses = "flex h-7 items-center self-center space-x-2 transition-all px-2 rounded-md backdrop-blur-xl cursor-pointer";
    const lightMode = "hover:bg-white/40 text-neutral-800";
    const darkMode = "hover:bg-neutral-700 text-white";

    if (!isAuthenticated) return null;

    const isPublisher = session.publisher?.id !== undefined;

    return (
        <div className="relative self-center" ref={containerRef}>
            <div
                onClick={toggleMenu}
                className={`${baseClasses} ${titleBarOpaque ? darkMode : lightMode}`}
                title="Usuario actual"
            >
                <img src={session.avatarUrl} alt="Avatar" className="size-5 rounded-md object-cover" />
                <span className="text-sm font-medium">{session.username}</span>
            </div>

            {openMenu && (
                <div className="absolute right-0 mt-2 w-48 bg-neutral-900 border border-neutral-700 rounded shadow-lg z-50 p-2 animate-fade-in animate-duration-100">
                    <ul className="text-sm text-white flex flex-col">
                        <Link
                            href="/profile"
                            className="w-full flex gap-x-2 items-center py-1 px-2 hover:bg-neutral-800 rounded"
                        >
                            <LucideSquareUserRound size={16} />
                            Ver perfil
                        </Link>
                        <Link
                            href="/settings"
                            className="w-full flex gap-x-2 items-center py-1 px-2 hover:bg-neutral-800 rounded"
                        >
                            <LucideSettings2 size={16} />
                            Configuración
                        </Link>
                        {isPublisher && (
                            <Link
                                href="/publisher"
                                className="w-full flex gap-x-2 items-center py-1 px-2 hover:bg-neutral-800 rounded"
                            >
                                <LucidePackageOpen size={16} />
                                Centro de creadores
                            </Link>
                        )}

                        <button
                            onClick={logout}
                            className="w-full flex gap-x-2 items-center py-1 px-2 hover:bg-red-600/20 rounded text-left cursor-pointer"
                        >
                            <LucideLogOut size={16} />
                            Cerrar sesión
                        </button>
                    </ul>
                </div>
            )}
        </div>
    );
};
