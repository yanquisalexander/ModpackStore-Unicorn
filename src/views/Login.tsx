import { DiscordIcon } from "@/icons/DiscordIcon";
import { useAuthentication } from "@/stores/AuthContext";
import { useGlobalContext } from "@/stores/GlobalContext";
import { LucideLockOpen } from "lucide-react";
import { useEffect } from "react";
import { toast } from "sonner";


export const Login = () => {
    const { startDiscordAuth, error, authStep, isAuthenticated } = useAuthentication();
    const { titleBarState, setTitleBarState } = useGlobalContext();

    useEffect(() => {
        const TOAST_ID = "login-toast";

        // Early return if authenticated
        if (isAuthenticated) {
            toast.dismiss(TOAST_ID);
            return;
        }

        // Handle error cases
        if (error) {
            if (error.error_code === "not_in_guild") {
                toast.dismiss(TOAST_ID); // Force disable loading toast
                toast.custom(() => (
                    <div className="flex items-center justify-center p-4 bg-gradient-to-r from-gray-700 to-gray-600 text-white rounded-md shadow-md transform">
                        <span className="mr-2">⚠️</span>
                        <span className="text-lg">
                            No estás en el servidor de Discord de Modpack Store. Por favor, únete al servidor para continuar.
                            <br />
                            <a
                                href="https://discord.gg/zXHhjExy92"
                                target="_blank"
                                rel="noopener noreferrer"
                                className="text-blue-500 hover:underline"
                            >
                                Unirme al servidor
                            </a>
                        </span>
                    </div>
                ), {
                    id: "required-guild",
                    duration: 10000
                });

                return
            } else {
                toast.error("Error al iniciar sesión", { id: TOAST_ID });
            }
            return;
        }

        // Handle different authentication steps
        const toastMessages = {
            "starting-auth": "Conectando con Discord...",
            "waiting-callback": "Esperando respuesta de Discord...",
            "processing-callback": "Procesando información...",
        };

        // Show loading toast for valid auth steps or dismiss for requesting-session
        if (authStep === "requesting-session") {
            toast.dismiss(TOAST_ID);
        } else if (authStep && toastMessages[authStep]) {
            toast.loading(toastMessages[authStep], { id: TOAST_ID });
        }
    }, [error, authStep, isAuthenticated]);

    useEffect(() => {
        setTitleBarState({
            ...titleBarState,
            title: "Login",
            icon: LucideLockOpen,
            customIconClassName: "bg-gray-900",
            canGoBack: false,
            opaque: false,
        });
    }, []);

    return (
        <div className="absolute inset-0 flex items-center justify-center">
            <video
                src="/assets/videos/doggy-bg.webm"
                autoPlay
                loop
                muted
                className="absolute !opacity-70 inset-0 object-cover w-full h-full -z-10"
            />
            <div className="-z-9 w-full h-full absolute inset-0 bg-ms-primary animate-fade-out" />
            <div className="z-10 w-full h-full flex">
                <div className="flex items-center justify-start w-full px-10">
                    <article className="w-full flex flex-col justify-center items-center max-w-md p-6 text-center bg-neutral-950/60 rounded-xl shadow-2xl backdrop-blur-sm backdrop-grayscale py-32">
                        <h1 className="mb-4 font-semibold text-4xl text-[2.5rem] from-[#bcfe47] to-[#05cc2a] bg-clip-text text-transparent bg-gradient-to-b">
                            Modpack Store
                        </h1>
                        <p className="mb-6 text-lg text-gray-300">
                            Inicia sesión con Discord para acceder a todas las funciones de la plataforma.
                        </p>
                        <button
                            disabled={!!authStep}
                            onClick={startDiscordAuth}
                            className="disabled:opacity-50 cursor-pointer disabled:cursor-not-allowed hover:scale-105 duration-200 ease-in-out flex items-center justify-center px-4 py-2 text-lg font-semibold text-white bg-blue-600 rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-opacity-50"
                        >
                            <DiscordIcon className="mr-2" />
                            Conectar con Discord
                        </button>
                    </article>
                </div>
            </div>
        </div>
    );
};
