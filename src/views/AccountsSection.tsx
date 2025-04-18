import { TauriCommandReturns } from "@/types/TauriCommandReturns"
import { invoke } from "@tauri-apps/api/core"
import { useEffect, useState } from "react"
import { LucideUser, LucideTrash2, LucideLogOut } from "lucide-react"
import { AccountCard } from "@/components/AccountCard"
import { AddAccountDialog } from "@/components/AddAccountDialog"
import { toast } from "sonner"

export const AccountsSection = () => {
    const [accounts, setAccounts] = useState<TauriCommandReturns['get_all_accounts']>([])
    const [loading, setLoading] = useState(true)

    const fetchAccounts = () => {
        setLoading(true)
        invoke<TauriCommandReturns['get_all_accounts']>('get_all_accounts')
            .then((fetchedAccounts) => {
                console.log("Accounts fetched from Tauri:", fetchedAccounts)
                setAccounts(fetchedAccounts)
            })
            .catch((error) => {
                console.error("Error fetching accounts:", error)
            })
            .finally(() => {
                setLoading(false)
            })
    }

    useEffect(() => {
        fetchAccounts()
    }, [])

    const handleRemoveAccount = async (uuid: string) => {
        try {
            await invoke<TauriCommandReturns['remove_account']>('remove_account', { uuid })
            setAccounts((prevAccounts) => prevAccounts.filter((account) => account.uuid !== uuid))
            toast.success("Cuenta eliminada", {
                description: "La cuenta ha sido eliminada correctamente",
            })
        }
        catch (error) {
            console.error("Error removing account:", error)
        }
    }


    return (
        <div className="mx-auto max-w-7xl px-8 py-10 overflow-y-auto">
            <header className="flex flex-col mb-16">
                <h1 className="tracking-tight inline font-semibold text-2xl bg-gradient-to-b from-[#FF1CF7] to-[#b249f8] bg-clip-text text-transparent">
                    Mis cuentas
                </h1>
                <p className="text-gray-400 text-base max-w-2xl">
                    Gestiona todas tus cuentas de Minecraft que utilizarás para jugar a los modpacks de la plataforma.
                </p>
            </header>

            <div className="grid grid-cols-1 sm:grid-cols-3 lg:grid-cols-4 gap-4">
                {loading ? (
                    <div className="flex items-center justify-center py-8">
                        <div className="flex flex-col items-center gap-2">
                            <div className="animate-spin h-6 w-6 border-2 border-emerald-500 rounded-full border-t-transparent"></div>
                            <p className="text-sm text-neutral-400">Cargando cuentas...</p>
                        </div>
                    </div>
                ) : (
                    <>
                        {accounts.map((account) => (
                            <AccountCard key={account.uuid} account={account} onRemove={handleRemoveAccount} />
                        ))}

                        {/* Add Account Card */}
                        <AddAccountDialog onAccountAdded={fetchAccounts} />
                    </>
                )}
            </div>
        </div>
    )
}

