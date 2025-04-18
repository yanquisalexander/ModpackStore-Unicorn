import { TauriCommandReturns } from "@/types/TauriCommandReturns";
import { LucideTrash2 } from "lucide-react";

export const AccountCard = ({ account, onRemove }: { account: TauriCommandReturns['get_all_accounts'][0], onRemove: (uuid: string) => void }) => {
    // Construct the URL for the 3D head render using the UUID
    const headUrl = `https://crafatar.com/renders/head/${account.uuid}?overlay=true&scale=8`;

    return (
        <article className="z-10 group relative overflow-hidden rounded-xl border border-white/20 h-64
        transition duration-300
        before:left-1/2 before:bottom-0 before:-translate-x-1/2 before:w-full before:h-1/2 
        before:rounded-full before:bg-black before:absolute before:translate-y-full 
        hover:before:translate-y-1/2 before:blur-3xl before:-z-10 before:transition before:duration-200 
        after:left-0 after:bottom-0 after:-translate-x-full after:translate-y-full 
        hover:after:-translate-x-1/2 hover:after:translate-y-1/2 after:w-2/2 after:aspect-square 
        after:rounded-2xl after:bg-black after:absolute after:blur-3xl hover:after:opacity-40 
        after:-z-10 after:opacity-0 after:transition after:duration-200 cursor-crosshair">

            {/* Background image - using a standard minecraft themed background */}
            <img
                src="/images/account-bg.webp"
                className="absolute inset-0 -z-20 transform-gpu animate-fade-in object-cover w-full h-full rounded-xl transition duration-500 group-hover:scale-105 group-hover:opacity-80"
                alt="Minecraft background"
                style={{
                    maskImage: "linear-gradient(to bottom, rgba(0, 0, 0, 0) 0%, rgba(0, 0, 0, 1) 100%)",
                }}
            />



            <div className="flex flex-col h-full p-4">
                {/* 3D Head and content */}
                <div className="flex flex-col items-center justify-center h-full gap-4">
                    {/* 3D Minecraft Head */}
                    <div className="w-24 h-24 relative transition duration-300 transform group-hover:scale-110 group-hover:rotate-6">
                        <img
                            src={headUrl}
                            alt={account.username}
                            className="w-full h-full object-contain drop-shadow-2xl"
                            onError={(e) => {
                                // Fallback to generic head image if the 3D head fails to load
                                e.currentTarget.src = `https://crafatar.com/renders/head/00000000-0000-0000-0000-000000000000?overlay=true&scale=8`;
                                e.currentTarget.onerror = null; // Prevent infinite loop if the fallback also fails
                            }}
                        />
                    </div>

                    {/* Username with gradient effect */}
                    <div className="text-center">
                        <h2 className="text-lg font-medium text-white group-hover:text-sky-200 transition">
                            {account.username}
                        </h2>
                        <p className="text-xs text-gray-400 mt-1">
                            Premium
                        </p>
                    </div>
                </div>

                {/* Action buttons (appear on hover) */}
                <div className="absolute bottom-0 left-0 right-0 flex justify-center p-4 opacity-0 translate-y-4 group-hover:opacity-100 group-hover:translate-y-0 transition duration-300">
                    <div className="flex gap-2">

                        <button
                            onClick={() => onRemove(account.uuid)}
                            title="Eliminar cuenta"
                            aria-label="Eliminar cuenta"
                            type="button"
                            disabled={false}
                            className="p-2 cursor-pointer rounded-lg bg-gray-800/60 border border-gray-700/40 text-white hover:bg-red-900/60 transition">
                            <LucideTrash2 className="h-4 w-4" />
                        </button>
                    </div>
                </div>
            </div>
        </article>
    );
};