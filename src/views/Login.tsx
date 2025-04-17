import { useAuthentication } from "@/stores/AuthContext";
import { useGlobalContext } from "@/stores/GlobalContext";
import { LucideLockOpen } from "lucide-react";
import { useEffect } from "react";
import { toast } from "sonner";

export const Login = () => {
    const { startDiscordAuth, error, authStep, isAuthenticated } = useAuthentication()
    const { titleBarState, setTitleBarState } = useGlobalContext()

    console.log({ authStep, error })

    useEffect(() => {
        const TOAST_ID = "login-toast"

        // Dismiss the toast if the user is authenticated
        if (isAuthenticated) {
            toast.dismiss(TOAST_ID)
            return
        }


        const toastMessages = {
            "starting-auth": "Iniciando sesión con Discord...",
            "waiting-callback": "Esperando respuesta de Discord...",
            "processing-callback": "Procesando respuesta de Discord...",
        }

        if (error) {
            toast.error("Error al iniciar sesión", { id: TOAST_ID })

        } else if (authStep !== 'requesting-session') {
            if (authStep && toastMessages[authStep]) {
                toast.loading(toastMessages[authStep], { id: TOAST_ID })
            }
        } else {
            toast.dismiss(TOAST_ID)
        }

    }, [error, authStep, isAuthenticated])


    useEffect(() => {
        setTitleBarState({
            ...titleBarState,
            title: "Login",
            icon: LucideLockOpen,
            customIconClassName: "bg-gray-900",
            canGoBack: false,
            opaque: false
        })
    }, [])

    return (
        <>
            <div className="absolute inset-0 flex items-center justify-center" >
                <img src="/images/login_bg.webp" alt="Login Background"
                    className="absolute inset-0 object-cover w-full h-full -z-10 animate-fade-in animate-delay-200" />
                <div className="z-10 flex flex-col items-center justify-center h-screen bg-gradient-to-br from-blue-950/50 via-green-900/50 to-blue-900/50 flex-1">
                    <article className="flex flex-col items-center justify-center w-full max-w-2xl p-4 mx-auto text-center bg-neutral-900/80 rounded-lg shadow-lg backdrop-blur-md">

                        {
                            error?.error_code === "not_in_guild" && (
                                <p className="mb-4 text-lg max-w-[50ch] bg-neutral-900/80 text-gray-300 rounded-md p-4">
                                    No estás en el servidor de Discord de Modpack Store. Por favor, únete al servidor para continuar.
                                    <br /><a href="https://discord.gg/zXHhjExy92" target="_blank" rel="noopener noreferrer" className="text-blue-500 hover:underline"> Unirme al servidor</a>
                                </p>
                            )
                        }
                        <h1 className="mb-4 font-semibold text-4xl text-[2.5rem] from-[#47fe8d] to-[#058dcc] bg-clip-text text-transparent bg-gradient-to-b">
                            Modpack Store
                        </h1>
                        <p className="mb-4 text-lg text-gray-300">Inicia sesión con tu cuenta de Discord para acceder a todas las funciones.</p>
                        <button
                            disabled={!!authStep}
                            onClick={startDiscordAuth} className="disabled:opacity-50 disabled:cursor-not-allowed not-disabled:cursor-pointer hover:scale-105 duration-200 ease-in-out flex items-center justify-center px-4 py-2 text-lg font-semibold text-white bg-blue-600 rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-opacity-50">
                            Iniciar sesión con Discord
                        </button>
                    </article>

                </div>
            </div>


        </>
    );
}