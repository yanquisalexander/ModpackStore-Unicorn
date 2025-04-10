// Global context for the application

import { LucideIcon, LucideShoppingBag } from "lucide-react";
import React, { createContext, useContext, useState } from "react";

import { useEffect } from "react";

// Tendremos un estado para la titlebar, con un title, icon que puede ser un string o un componente de icono, y un canGoBack que es un booleano

interface TitleBarState {
    title: string;
    icon?: string | LucideIcon;
    canGoBack?: boolean;
    customIconClassName?: string;
}

interface GlobalContextType {
    titleBarState: TitleBarState;
    setTitleBarState: React.Dispatch<React.SetStateAction<TitleBarState>>;
}

// Create the context with default values

const GlobalContext = createContext<GlobalContextType | undefined>(undefined);

export const GlobalContextProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
    const [titleBarState, setTitleBarState] = useState<TitleBarState>({
        title: "Modpack Store",
        icon: LucideShoppingBag,
        canGoBack: false,
    });

    return (
        <GlobalContext.Provider value={{ titleBarState, setTitleBarState }}>
            {children}
        </GlobalContext.Provider>
    );
};

export const useGlobalContext = () => {
    const context = useContext(GlobalContext);
    if (!context) {
        throw new Error("useGlobalContext must be used within a GlobalContextProvider");
    }
    return context;
};