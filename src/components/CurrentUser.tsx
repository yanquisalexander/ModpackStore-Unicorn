import { useAuthentication } from "@/stores/AuthContext";

export const CurrentUser = ({ titleBarOpaque }: { titleBarOpaque?: boolean }) => {
    // Fake user data for demonstration purposes

    const { session, logout, isAuthenticated } = useAuthentication()
    console.log({ session });


    if (!isAuthenticated) {
        return null
    }

    const baseClasses = "flex h-7 items-center self-center space-x-2 transition-all px-2 rounded-md backdrop-blur-xl";
    const lightMode = "hover:bg-white/40 text-neutral-800";
    const darkMode = "hover:bg-neutral-700 text-white";

    return (
        <div onClick={logout} className={`${baseClasses} ${titleBarOpaque ? darkMode : lightMode}`}>
            <img src={session.avatarUrl} alt="Avatar" className="size-4 rounded-sm" />
            <span className="text-sm font-medium">{session.username}</span>
        </div>
    );
};
