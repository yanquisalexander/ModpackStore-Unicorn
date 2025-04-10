/* 
    Hook to check if the user is connected to the internet 
    on application startup.
*/

import { useEffect, useState } from "react";
import { toast } from "sonner";
import { invoke } from "@tauri-apps/api/core";

export const useCheckConnection = () => {
    const [isConnected, setIsConnected] = useState<boolean>(false);
    const [isLoading, setIsLoading] = useState<boolean>(true);

    useEffect(() => {
        const checkConnection = async () => {
            try {
                console.log("[checkConnection] Checking connection...");
                const response = await invoke("check_connection");
                setIsConnected(response as boolean);
                console.log("[checkConnection] Connection status:", response);
            } catch (error) {
                console.error("[checkConnection] Error checking connection:", error);
            } finally {
                setIsLoading(false);
            }
        };

        checkConnection();
    }, []);

    return { isConnected, isLoading };
}