import { relaunch } from "@tauri-apps/plugin-process";
import { createContext, useContext, useState } from "react";
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

interface ReloadContextType {
    showReloadDialog: () => void;
}

const ReloadContext = createContext<ReloadContextType | undefined>(undefined);

export const useReloadApp = () => {
    const context = useContext(ReloadContext);
    if (!context) {
        throw new Error("useReloadApp must be used within a ReloadProvider");
    }
    return context;
};

export const ReloadProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
    const [isDialogOpen, setIsDialogOpen] = useState(false);

    const showReloadDialog = () => {
        setIsDialogOpen(true);
    };

    const confirmReload = async () => {
        await relaunch();
    };

    return (
        <ReloadContext.Provider value={{ showReloadDialog }}>
            {children}

            <AlertDialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
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
        </ReloadContext.Provider>
    );
};