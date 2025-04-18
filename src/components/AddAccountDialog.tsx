import { useState } from "react"
import { invoke } from "@tauri-apps/api/core"
import { LucideUser } from "lucide-react"
import { TauriCommandReturns } from "@/types/TauriCommandReturns"


import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
    DialogTrigger,
} from "@/components/ui/dialog"
import {
    Tabs,
    TabsContent,
    TabsList,
    TabsTrigger,
} from "@/components/ui/tabs"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { toast } from "sonner"
import { MicrosoftIcon } from "@/icons/MicrosoftIcon"

export const AddAccountDialog = ({
    onAccountAdded
}: {
    onAccountAdded: () => void
}) => {
    const [open, setOpen] = useState(false)
    const [username, setUsername] = useState("")
    const [isLoading, setIsLoading] = useState(false)

    const handleAddOfflineAccount = async () => {
        if (!username.trim()) {
            toast.error("Error", {
                description: "El nombre de usuario no puede estar vacío"
            })
            return
        }

        setIsLoading(true)

        try {
            await invoke<TauriCommandReturns['add_offline_account']>('add_offline_account', { username: username.trim() })

            toast("Cuenta añadida", {
                description: `Se ha añadido la cuenta ${username} correctamente`,
            })

            setUsername("")
            setOpen(false)
            onAccountAdded()
        } catch (error) {
            console.error("Error al añadir cuenta offline:", error)
            toast.error("No se pudo añadir la cuenta. Inténtalo de nuevo.")
        } finally {
            setIsLoading(false)
        }
    }

    const handleMicrosoftLogin = () => {
        toast("Función no disponible", {
            description: "El inicio de sesión con Microsoft aún no está implementado."
        })
    }

    return (
        <Dialog open={open} onOpenChange={setOpen}>
            <DialogTrigger asChild>
                <button
                    className="cursor-pointer z-10 group relative overflow-hidden rounded-xl border border-dashed border-white/20 h-64 flex flex-col items-center justify-center
                    transition duration-300 hover:border-sky-400/50 hover:bg-gray-800/30"
                >
                    <div className="flex flex-col items-center gap-3">
                        <div className="p-3 rounded-full bg-gray-800/80 group-hover:bg-sky-900/40 transition">
                            <LucideUser className="h-8 w-8 text-gray-400 group-hover:text-sky-300" />
                        </div>
                        <span className="text-gray-400 group-hover:text-sky-300 font-medium">Añadir cuenta</span>
                    </div>
                </button>
            </DialogTrigger>

            <DialogContent className="sm:max-w-md dark">
                <DialogHeader>
                    <DialogTitle className="from-[#bcfe47] to-[#05cc2a] bg-clip-text text-transparent bg-gradient-to-b">Añadir una nueva cuenta</DialogTitle>
                    <DialogDescription>
                        Elige el tipo de cuenta que deseas añadir para jugar en los modpacks.
                    </DialogDescription>
                </DialogHeader>

                <Tabs defaultValue="offline" className="mt-4">
                    <TabsList className="grid w-full grid-cols-2">
                        <TabsTrigger value="offline">Cuenta Offline</TabsTrigger>
                        <TabsTrigger value="microsoft">Cuenta Microsoft</TabsTrigger>
                    </TabsList>

                    <TabsContent value="offline" className="mt-4 space-y-4">
                        <div className="space-y-2">
                            <Label className="text-white" htmlFor="username">Nombre de usuario</Label>
                            <Input
                                id="username"
                                value={username}

                                onChange={(e) => {
                                    // Prevent spacing and special characters
                                    const value = e.target.value.replace(/[^a-zA-Z0-9_]/g, "")
                                    setUsername(value)
                                }}
                                placeholder="Ingresa tu nombre de usuario"
                            />
                            <p className="text-sm text-gray-400">
                                Las cuentas offline te permiten jugar sin verificación pero con funcionalidades limitadas.
                            </p>
                        </div>

                        <DialogFooter className="mt-6">
                            <Button
                                type="submit"
                                onClick={handleAddOfflineAccount}
                                disabled={isLoading}

                                className="w-full cursor-pointer disabled:bg-gray-700 disabled:text-gray-400 disabled:cursor-not-allowed"
                            >
                                {isLoading ? "Añadiendo..." : "Añadir cuenta offline"}
                            </Button>
                        </DialogFooter>
                    </TabsContent>

                    <TabsContent value="microsoft" className="mt-4">
                        <div className="flex flex-col items-center justify-center py-8 space-y-4">
                            <div className="p-3 rounded-full bg-blue-900/40">
                                <MicrosoftIcon className="h-8 w-8" />
                            </div>
                            <div className="text-center">
                                <h3 className="text-lg font-medium text-neutral-50">Iniciar sesión con Microsoft</h3>
                                <p className="text-sm text-gray-400 mt-2 mb-6">
                                    Conecta tu cuenta de Microsoft para acceder a todas las funcionalidades.
                                </p>
                            </div>

                            <Button
                                onClick={handleMicrosoftLogin}
                                className="bg-blue-600 hover:bg-blue-700"
                            >
                                Iniciar con Microsoft
                            </Button>

                            <p className="text-xs text-gray-500 italic">
                                (Funcionalidad no implementada aún)
                            </p>
                        </div>
                    </TabsContent>
                </Tabs>
            </DialogContent>
        </Dialog>
    )
}
