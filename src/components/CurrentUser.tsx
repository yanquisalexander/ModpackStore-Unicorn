import { useAuthentication } from "@/stores/AuthContext";
import { relaunch } from "@tauri-apps/plugin-process";
import { LucideAppWindowMac, LucideLogOut, LucidePackageOpen, LucideSettings2, LucideSquareUserRound } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { Link } from "wouter";
import { useConfigDialog } from "@/stores/ConfigDialogContext";
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle
} from "@/components/ui/alert-dialog";

export const CurrentUser = ({ titleBarOpaque }: { titleBarOpaque?: boolean }) => {
    const { session, logout, isAuthenticated } = useAuthentication();
    const { openConfigDialog } = useConfigDialog();
    const [openMenu, setOpenMenu] = useState(false);
    const [showMoreOptions, setShowMoreOptions] = useState(false);
    const [showReloadDialog, setShowReloadDialog] = useState(false);
    const containerRef = useRef<HTMLDivElement>(null);

    const toggleMenu = (event: React.MouseEvent) => {
        const isOpening = !openMenu;
        setOpenMenu(isOpening);

        // Set showMoreOptions to true if opening menu with shift key pressed
        if (isOpening && event.shiftKey) {
            setShowMoreOptions(true);
        } else if (!isOpening) {
            // Reset showMoreOptions when closing the menu
            setShowMoreOptions(false);
        }
    };

    const closeMenu = () => {
        setOpenMenu(false);
        setShowMoreOptions(false);
    };

    const handleReloadApp = () => {
        closeMenu();
        setShowReloadDialog(true);
    };

    const confirmReload = async () => {
        await relaunch();
    };

    const handleLogout = () => {
        closeMenu();
        logout();
    };

    const handleOpenConfig = () => {
        closeMenu();
        openConfigDialog();
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

    const baseClasses = "flex h-7 items-center self-center space-x-2 transition-all px-2 rounded-md backdrop-blur-xl cursor-pointer";
    const lightMode = "hover:bg-white/40 text-neutral-800";
    const darkMode = "hover:bg-neutral-700 text-white";

    if (!isAuthenticated) return null;

    const isPublisher = session?.publisher?.id !== undefined;

    return (
        <div className="relative self-center" ref={containerRef}>
            <div
                onClick={toggleMenu}
                className={`${baseClasses} ${titleBarOpaque ? darkMode : lightMode}`}
                title="Usuario actual"
            >
                <img src={session?.avatarUrl} alt="Avatar" className="size-5 rounded-md object-cover" />
                <span className="text-sm font-medium">{session?.username}</span>
            </div>

            <div
                style={{
                    opacity: openMenu ? 1 : 0,
                    visibility: openMenu ? "visible" : "hidden",
                    transform: openMenu ? "translateY(0)" : "translateY(-5px)",
                    transition: "opacity 0.2s ease, visibility 0.2s ease, transform 0.2s ease",
                }}
                className="absolute right-0 mt-2 w-48 bg-neutral-900 border border-neutral-700 rounded shadow-lg z-50 p-2">
                <ul className="text-sm text-white flex flex-col">
                    <Link
                        href="/profile"
                        onClick={closeMenu}
                        className="w-full flex gap-x-2 items-center py-1 px-2 hover:bg-neutral-800 rounded"
                    >
                        <LucideSquareUserRound size={16} />
                        Ver perfil
                    </Link>

                    <button
                        onClick={handleOpenConfig}
                        className="w-full flex gap-x-2 items-center py-1 px-2 hover:bg-neutral-800 rounded text-left cursor-pointer"
                    >
                        <LucideSettings2 size={16} />
                        Configuración
                    </button>

                    {isPublisher && (
                        <Link
                            href="/creators"
                            onClick={closeMenu}
                            className="w-full flex gap-x-2 items-center py-1 px-2 hover:bg-neutral-800 rounded"
                        >
                            <LucidePackageOpen size={16} />
                            Centro de creadores
                        </Link>
                    )}

                    <button
                        onClick={handleLogout}
                        className="w-full flex gap-x-2 items-center py-1 px-2 hover:bg-red-600/20 rounded text-left cursor-pointer"
                    >
                        <LucideLogOut size={16} />
                        Cerrar sesión
                    </button>

                    {/* Conditional rendering based on showMoreOptions */}
                    {showMoreOptions && (
                        <>
                            <div className="border-t border-neutral-700 my-1"></div>
                            {/* Additional options here when shift is pressed */}
                            <button
                                onClick={handleReloadApp}
                                className="cursor-pointer w-full flex gap-x-2 items-center py-1 px-2 hover:bg-neutral-800 rounded">
                                <LucideAppWindowMac size={16} />
                                Recargar aplicación
                            </button>
                        </>
                    )}
                </ul>
            </div>

            {/* Shadcn Alert Dialog */}
            <AlertDialog open={showReloadDialog} onOpenChange={setShowReloadDialog}>
                <AlertDialogContent className="dark">
                    <AlertDialogHeader className="text-white">
                        <AlertDialogTitle>Recargar aplicación</AlertDialogTitle>
                        <AlertDialogDescription>
                            ¿Estás seguro de que quieres recargar la aplicación? Se detendrán todas las tareas en curso, incluyendo descargas y actualizaciones.
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel className="cursor-pointer text-neutral-500">Cancelar</AlertDialogCancel>
                        <AlertDialogAction
                            className="cursor-pointer"
                            onClick={confirmReload}>Recargar</AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </div>
    );
};