export interface ModpackDataOverview {
    id?: string;
    name?: string;
    shortDescription?: null;
    description?: string;
    slug?: string;
    iconUrl?: string;
    bannerUrl?: string;
    trailerUrl?: string;
    visibility?: string;
    showUserAsPublisher?: boolean;
    creatorUserId?: string;
    createdAt?: Date;
    updatedAt?: Date;
    creatorUser?: CreatorUser;
    publisher?: Publisher;
}

export interface CreatorUser {
    id?: string;
    username?: string;
    email?: string;
    avatarUrl?: string;
    discordId?: string;
    discordAccessToken?: string;
    discordRefreshToken?: string;
    patreonId?: null;
    patreonAccessToken?: null;
    patreonRefreshToken?: null;
    createdAt?: Date;
    updatedAt?: Date;
}

export interface Publisher {
    id?: string;
    publisherName?: string;
    verified?: boolean;
    partnered?: boolean;
    isHostingPartner?: boolean;
}
