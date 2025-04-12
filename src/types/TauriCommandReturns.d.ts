
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
    "get_instance_by_id": MinecraftInstance;
}