import { LucideIcon, LucideShoppingBag } from "lucide-react";
import React, {
    createContext,
    useContext,
    useState,
    useEffect,
} from "react";
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';


interface TitleBarState {
    title: string;
    icon?: string | LucideIcon;
    canGoBack?: boolean;
    customIconClassName?: string;
    opaque?: boolean;
}

// Estado de actualización
type UpdateState =
    | "idle"
    | "checking"
    | "downloading"
    | "installing"
    | "done"
    | "error";

// Tipo del contexto
interface GlobalContextType {
    titleBarState: TitleBarState;
    setTitleBarState: React.Dispatch<React.SetStateAction<TitleBarState>>;

    isUpdating: boolean;
    updateProgress: number;
    updateVersion: string | null;
    updateState: UpdateState;
    applyUpdate: () => Promise<void>;
}

// Crear el contexto
const GlobalContext = createContext<GlobalContextType | undefined>(undefined);

export const GlobalContextProvider: React.FC<{ children: React.ReactNode }> = ({
    children,
}) => {
    const [titleBarState, setTitleBarState] = useState<TitleBarState>({
        title: "Modpack Store",
        icon: LucideShoppingBag,
        canGoBack: false,
        opaque: true
    });

    const [isUpdating, setIsUpdating] = useState(false);
    const [updateProgress, setUpdateProgress] = useState(0);
    const [updateVersion, setUpdateVersion] = useState<string | null>(null);
    const [updateState, setUpdateState] = useState<UpdateState>("idle");

    const applyUpdate = async () => {
        setUpdateState("installing");

        try {
            // Aquí no descargamos la actualización, porque eso ya ocurrió anteriormente
            await relaunch(); // Reinicia la aplicación para aplicar la actualización
        } catch (err) {
            console.error("Error al aplicar la actualización:", err);
            setUpdateState("error");
            setIsUpdating(false);
        }
    };

    useEffect(() => {
        const checkAndDownload = async () => {
            setUpdateState("checking");

            try {
                const update = await check();
                if (update) {
                    setIsUpdating(true);
                    setUpdateVersion(update.version);
                    setUpdateState("downloading");

                    let downloaded = 0;
                    let contentLength = 0;

                    await update.downloadAndInstall((event) => {
                        switch (event.event) {
                            case 'Started':
                                contentLength = event.data.contentLength || 0;
                                break;
                            case 'Progress':
                                downloaded += event.data.chunkLength;
                                const percent = (downloaded / contentLength) * 100;
                                setUpdateProgress(Math.round(percent));
                                break;
                            case 'Finished':
                                setUpdateState("installing"); // "Lista para aplicar"
                                break;
                        }
                    });
                } else {
                    setUpdateState("idle");
                    setIsUpdating(false);
                }
            } catch (err) {
                console.error("Error checking/downloading update:", err);

                setIsUpdating(false);
            }
        };

        checkAndDownload();

    }, []);

    return (
        <GlobalContext.Provider
            value={{
                titleBarState,
                setTitleBarState,
                isUpdating,
                updateProgress,
                updateVersion,
                updateState,
                applyUpdate,
            }}
        >
            {children}
        </GlobalContext.Provider>
    );
};

// Hook para consumir el contexto
export const useGlobalContext = () => {
    const context = useContext(GlobalContext);
    if (!context) {
        throw new Error("useGlobalContext must be used within a GlobalContextProvider");
    }
    return context;
};
