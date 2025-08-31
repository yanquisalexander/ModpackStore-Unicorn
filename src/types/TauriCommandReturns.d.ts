interface MCAccount {
    username: string;
    uuid: string;
    access_token: string;
    user_type: string;
}

interface ModpackInfo {
    name?: string;
    version?: string;
    author?: string;
    modpackVersionId?: string; // Can be specific version ID or "latest"
}

interface MinecraftInstance {
    instanceId: string;
    usesDefaultIcon: boolean;
    iconName?: string;
    iconUrl?: string;
    instanceName: string;
    accountUuid?: string;
    minecraftPath: string;
    modpackId?: string;
    modpackInfo?: ModpackInfo;
    minecraftVersion: string;
    instanceDirectory?: string;
    forgeVersion?: string;
}

export type TauriCommandReturns = {
    "any": any;
    "get_instance_by_id": MinecraftInstance;
    "get_all_accounts": MCAccount[];
    "get_instances_by_modpack_id": MinecraftInstance[];
    "ensure_account_exists": Boolean;
    "add_offline_account": MCAccount;
    "search_instances": MinecraftInstance[];
    "create_instance": MinecraftInstance;
}