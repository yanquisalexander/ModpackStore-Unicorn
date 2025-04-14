import { useAuthentication } from "@/stores/AuthContext";

export const CurrentUser = ({ titleBarOpaque }: { titleBarOpaque?: boolean }) => {
    // Fake user data for demonstration purposes

    const { startDiscordAuth } = useAuthentication()
    const user = {
        id: "1234567890",
        name: "Alexitoo_UY",
        email: "john.doe@example.com",
        avatarUrl: "https://www.alexitoo.dev/favicon.svg",
        roles: ["user", "admin"],
    };

    if (!user) {
        return (
            <button className="btn btn-primary" onClick={() => alert("Login")}>
                Login
            </button>
        );
    }

    const baseClasses = "flex h-7 items-center self-center space-x-2 transition-all px-2 rounded-md backdrop-blur-xl";
    const lightMode = "hover:bg-white/40 text-neutral-800";
    const darkMode = "hover:bg-neutral-700 text-white";

    return (
        <div onClick={startDiscordAuth} className={`${baseClasses} ${titleBarOpaque ? darkMode : lightMode}`}>
            <img src={user.avatarUrl} alt="Avatar" className="size-4 rounded-sm" />
            <span className="text-sm font-medium">{user.name}</span>
        </div>
    );
};
