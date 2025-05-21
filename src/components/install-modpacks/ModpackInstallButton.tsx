import { Button } from "@/components/ui/button"
import { useState } from "react"
import { invoke } from "@tauri-apps/api/core"
import { LucideDownload, LucideRefreshCw } from "lucide-react"
import { InstallOptionsDialog } from "./InstallOptionsDialog"
import { UpdateInstanceDialog } from "./UpdateInstanceDialog"
import { CreateInstanceDialog } from "./CreateInstanceDialog"
import { PasswordDialog } from "./ModpackPasswordDialog"
import { TauriCommandReturns } from "@/types/TauriCommandReturns"

interface InstallButtonProps {
    modpackId: string;
    modpackName: string;
    localInstances: TauriCommandReturns["get_instances_by_modpack_id"];
    isPasswordProtected?: boolean;
}

export const InstallButton = ({
    modpackId,
    modpackName,
    localInstances,
    isPasswordProtected = false
}: InstallButtonProps) => {
    const [isInstallOptionsOpen, setIsInstallOptionsOpen] = useState<boolean>(false)
    const [isUpdateDialogOpen, setIsUpdateDialogOpen] = useState<boolean>(false)
    const [isCreateDialogOpen, setIsCreateDialogOpen] = useState<boolean>(false)
    const [isPasswordDialogOpen, setIsPasswordDialogOpen] = useState<boolean>(false)
    const [isInstalling, setIsInstalling] = useState<boolean>(false)
    const [passwordError, setPasswordError] = useState<string | undefined>(undefined)

    // Para almacenar temporalmente la acción pendiente que requiere contraseña
    const [pendingAction, setPendingAction] = useState<{
        type: 'update' | 'create';
        instanceId?: string;
        instanceName?: string;
        password?: string;
    } | null>(null)

    const hasLocalInstances = localInstances.length > 0

    const handleInstallClick = () => {
        if (hasLocalInstances) {
            setIsInstallOptionsOpen(true)
        } else {
            if (isPasswordProtected) {
                setPendingAction({ type: 'create' })
                setIsPasswordDialogOpen(true)
            } else {
                setIsCreateDialogOpen(true)
            }
        }
    }

    const handleUpdateExisting = () => {
        setIsInstallOptionsOpen(false)
        if (isPasswordProtected) {
            setPendingAction({ type: 'update' })
            setIsPasswordDialogOpen(true)
        } else {
            setIsUpdateDialogOpen(true)
        }
    }

    const handleInstallNew = () => {
        setIsInstallOptionsOpen(false)
        if (isPasswordProtected) {
            setPendingAction({ type: 'create' })
            setIsPasswordDialogOpen(true)
        } else {
            setIsCreateDialogOpen(true)
        }
    }

    const verifyPassword = async (password: string): Promise<boolean> => {
        try {
            const isValid = await invoke("verify_modpack_password", {
                modpackId,
                password
            }) as boolean

            return isValid
        } catch (err) {
            console.error("Error al verificar la contraseña:", err)
            return false
        }
    }

    const handleConfirmPassword = async (password: string) => {
        setPasswordError(undefined)
        setIsInstalling(true)

        try {
            const isValid = await verifyPassword(password)

            if (!isValid) {
                setPasswordError("La contraseña no es válida")
                setIsInstalling(false)
                return
            }

            // Contraseña válida, proceder con la acción pendiente
            setPendingAction({
                ...pendingAction!,
                password
            });

            if (pendingAction?.type === 'update') {
                setIsPasswordDialogOpen(false)
                setIsUpdateDialogOpen(true)
            } else if (pendingAction?.type === 'create') {
                setIsPasswordDialogOpen(false)
                setIsCreateDialogOpen(true)
            }
        } catch (err) {
            console.error("Error al procesar la contraseña:", err)
            setPasswordError("Ocurrió un error al verificar la contraseña")
        } finally {
            setIsInstalling(false)
        }
    }

    const handleConfirmUpdate = async (instanceId: string) => {
        setIsInstalling(true)
        try {
            await invoke("update_instance", {
                instanceId,
                modpackId,
                password: isPasswordProtected ? pendingAction?.password : undefined
            })
            console.log(`Instancia ${instanceId} actualizada exitosamente`)
            setPendingAction(null)
        } catch (err) {
            console.error("Error al actualizar la instancia:", err)
            // Mostrar mensaje de error
        } finally {
            setIsInstalling(false)
            setIsUpdateDialogOpen(false)
        }
    }

    const handleConfirmCreate = async (instanceName: string) => {
        setIsInstalling(true)
        try {
            await invoke("create_instance", {
                instanceName,
                modpackId,
                password: isPasswordProtected ? pendingAction?.password : undefined
            })
            console.log(`Nueva instancia creada: ${instanceName}`)
            setPendingAction(null)
        } catch (err) {
            console.error("Error al crear la instancia:", err)
            // Mostrar mensaje de error
        } finally {
            setIsInstalling(false)
            setIsCreateDialogOpen(false)
        }
    }

    return (
        <>
            <Button
                variant="default"
                className="w-full md:w-auto bg-indigo-600 hover:bg-indigo-700 text-white flex items-center gap-2"
                onClick={handleInstallClick}
                disabled={isInstalling}
            >
                {isInstalling ? (
                    <>
                        <LucideRefreshCw className="w-4 h-4 animate-spin" />
                        Instalando...
                    </>
                ) : (
                    <>
                        <LucideDownload className="w-4 h-4" />
                        Instalar
                    </>
                )}
            </Button>

            <InstallOptionsDialog
                isOpen={isInstallOptionsOpen}
                onClose={() => setIsInstallOptionsOpen(false)}
                modpackId={modpackId}
                modpackName={modpackName}
                localInstances={localInstances}
                onUpdateExisting={handleUpdateExisting}
                onInstallNew={handleInstallNew}
            />

            <UpdateInstanceDialog
                isOpen={isUpdateDialogOpen}
                onClose={() => setIsUpdateDialogOpen(false)}
                modpackId={modpackId}
                modpackName={modpackName}
                localInstances={localInstances}
                onConfirmUpdate={handleConfirmUpdate}
            />

            <CreateInstanceDialog
                isOpen={isCreateDialogOpen}
                onClose={() => setIsCreateDialogOpen(false)}
                modpackId={modpackId}
                modpackName={modpackName}
                onConfirmCreate={handleConfirmCreate}
            />

            <PasswordDialog
                isOpen={isPasswordDialogOpen}
                onClose={() => {
                    setIsPasswordDialogOpen(false)
                    setPendingAction(null)
                    setPasswordError(undefined)
                }}
                modpackName={modpackName}
                onConfirm={handleConfirmPassword}
                isLoading={isInstalling}
                error={passwordError}
            />
        </>
    )
}