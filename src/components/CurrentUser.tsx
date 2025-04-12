export const CurrentUser = () => {
    // Fake user data for demonstration purposes
    const user = {
        id: "1234567890",
        name: "John Doe",

        email: "john.doe@example.com",
        avatarUrl: "api.dicebear.com/v2/avataaars/john-doe.svg",
        roles: ["user", "admin"],

    };

    if (!user) {
        return (
            <button className="btn btn-primary" onClick={() => alert("Login")}>
                Login
            </button>
        )
    }
    return <div>{user.name}</div>;
}