import { useState } from "react"
import { Button } from "@/components/ui/button"
import {
    Dialog,
    DialogContent,
    DialogHeader,
    DialogTitle,
    DialogDescription,
    DialogFooter,
} from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { LucideShieldAlert } from "lucide-react"

interface PasswordDialogProps {
    isOpen: boolean;
    onClose: () => void;
    modpackName: string;
    onConfirm: (password: string) => void;
    isLoading?: boolean;
    error?: string;
}

export const PasswordDialog = ({
    isOpen,
    onClose,
    modpackName,
    onConfirm,
    isLoading = false,
    error,
}: PasswordDialogProps) => {
    const [password, setPassword] = useState<string>("");

    const handleConfirm = () => {
        if (password.trim()) {
            onConfirm(password);
        }
    }

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === "Enter" && password.trim()) {
            handleConfirm();
        }
    }

    return (
        <Dialog open={isOpen} onOpenChange={(open) => {
            if (!open) onClose();
        }}>
            <DialogContent className="sm:max-w-md">
                <DialogHeader>
                    <div className="flex items-center gap-2 text-amber-500">
                        <LucideShieldAlert className="h-5 w-5" />
                        <DialogTitle>Modpack protegido</DialogTitle>
                    </div>
                    <DialogDescription>
                        El modpack <span className="font-medium">{modpackName}</span> está protegido por contraseña. Por favor ingresa la contraseña para continuar.
                    </DialogDescription>
                </DialogHeader>
                <div className="space-y-4">
                    <div className="space-y-2">
                        <Label htmlFor="password">Contraseña</Label>
                        <Input
                            id="password"
                            type="password"
                            value={password}
                            onChange={(e) => setPassword(e.target.value)}
                            onKeyDown={handleKeyDown}
                            placeholder="Ingresa la contraseña"
                            autoComplete="off"
                            autoFocus
                            className={error ? "border-red-500" : ""}
                        />
                        {error && (
                            <p className="text-sm text-red-500">{error}</p>
                        )}
                    </div>
                </div>
                <DialogFooter>
                    <Button
                        variant="secondary"
                        onClick={onClose}
                        disabled={isLoading}
                    >
                        Cancelar
                    </Button>
                    <Button
                        onClick={handleConfirm}
                        disabled={!password.trim() || isLoading}
                        className="bg-indigo-600 hover:bg-indigo-700 text-white"
                    >
                        {isLoading ? "Verificando..." : "Confirmar"}
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
    )
}