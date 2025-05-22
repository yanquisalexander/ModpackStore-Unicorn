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
    } | null>(null)

    const hasLocalInstances = localInstances.length > 0

    const handleInstallClick = () => {
        if (hasLocalInstances) {
            setIsInstallOptionsOpen(true)
        } else {
            setIsCreateDialogOpen(true)
        }
    }

    const handleUpdateExisting = () => {
        setIsInstallOptionsOpen(false)
        setIsUpdateDialogOpen(true)
    }

    const handleInstallNew = () => {
        setIsInstallOptionsOpen(false)
        setIsCreateDialogOpen(true)
    }

    const handleConfirmPassword = async (password: string) => {
        setPasswordError(undefined);

        // Ejecutar la acción pendiente con la contraseña proporcionada
        if (pendingAction?.type === 'update' && pendingAction.instanceId) {
            setIsPasswordDialogOpen(false);
            await executeUpdate(pendingAction.instanceId, password);
        } else if (pendingAction?.type === 'create' && pendingAction.instanceName) {
            setIsPasswordDialogOpen(false);
            await executeCreate(pendingAction.instanceName, password);
        }
    }

    const handleConfirmUpdate = async (instanceId: string) => {
        // Si el modpack está protegido, solicitar contraseña antes de actualizar
        if (isPasswordProtected) {
            setPendingAction({
                type: 'update',
                instanceId
            });
            setIsUpdateDialogOpen(false);
            setIsPasswordDialogOpen(true);
            return;
        }

        // Si no está protegido, proceder directamente con la actualización
        await executeUpdate(instanceId);
    }

    const executeUpdate = async (instanceId: string, password?: string) => {
        setIsInstalling(true);
        try {
            await invoke("update_instance", {
                instanceId,
                modpackId,
                password
            });
            console.log(`Instancia ${instanceId} actualizada exitosamente`);
        } catch (err) {
            console.error("Error al actualizar la instancia:", err);

            // Verificar si el error es de contraseña inválida
            const errorObj = err as any;
            if (errorObj?.code === "invalid_password") {
                // Si fue un error de contraseña y teníamos una acción pendiente, 
                // mostrar diálogo de contraseña nuevamente con error
                if (pendingAction?.type === 'update') {
                    setPasswordError("La contraseña no es válida");
                    setIsPasswordDialogOpen(true);
                    setIsInstalling(false);
                    return;
                }
            }

            // Otros errores
            // Aquí puedes mostrar un mensaje de error genérico
        } finally {
            if (!isPasswordDialogOpen) {
                setIsInstalling(false);
                setIsUpdateDialogOpen(false);
                setPendingAction(null);
            }
        }
    }

    const handleConfirmCreate = async (instanceName: string) => {
        // Si el modpack está protegido, solicitar contraseña antes de crear
        if (isPasswordProtected) {
            setPendingAction({
                type: 'create',
                instanceName
            });
            setIsCreateDialogOpen(false);
            setIsPasswordDialogOpen(true);
            return;
        }

        // Si no está protegido, proceder directamente con la creación
        await executeCreate(instanceName);
    }

    const executeCreate = async (instanceName: string, password?: string) => {
        setIsInstalling(true);
        try {
            await invoke("create_instance", {
                instanceName,
                modpackId,
                password
            });
            console.log(`Nueva instancia creada: ${instanceName}`);
        } catch (err) {
            console.error("Error al crear la instancia:", err);

            // Verificar si el error es de contraseña inválida
            const errorObj = err as any;
            if (errorObj?.code === "invalid_password") {
                // Si fue un error de contraseña y teníamos una acción pendiente, 
                // mostrar diálogo de contraseña nuevamente con error
                if (pendingAction?.type === 'create') {
                    setPasswordError("La contraseña no es válida");
                    setIsPasswordDialogOpen(true);
                    setIsInstalling(false);
                    return;
                }
            }

            // Otros errores
            // Aquí puedes mostrar un mensaje de error genérico
        } finally {
            if (!isPasswordDialogOpen) {
                setIsInstalling(false);
                setIsCreateDialogOpen(false);
                setPendingAction(null);
            }
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