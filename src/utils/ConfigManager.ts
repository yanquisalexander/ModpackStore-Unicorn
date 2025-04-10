// src/utils/ConfigManager.ts
import { join } from '@tauri-apps/api/path';
import { appDataDir, homeDir } from '@tauri-apps/api/path';
import { exists, mkdir, readTextFile, writeTextFile } from '@tauri-apps/plugin-fs';

// Enum for config keys
export enum ConfigKey {
    INSTANCES_DIR = "instancesDir",
    JAVA_DIR = "javaDir",
    MEMORY = "memory",
    LANGUAGE = "language",
    CLOSE_ON_LAUNCH = "closeOnLaunch",
    CHECK_UPDATES_ON_STARTUP = "checkUpdatesOnStartup"
}

// Default configuration values
const DEFAULT_CONFIG = {
    [ConfigKey.INSTANCES_DIR]: "",  // Will be initialized in constructor
    [ConfigKey.JAVA_DIR]: "",       // Will be initialized in constructor
    [ConfigKey.MEMORY]: 2048,
    [ConfigKey.LANGUAGE]: "en",
    [ConfigKey.CLOSE_ON_LAUNCH]: true,
    [ConfigKey.CHECK_UPDATES_ON_STARTUP]: true
};

export class ConfigManager {
    private static instance: ConfigManager;
    private configFileName = "config.json";
    private configFileFullPath = "";
    private configData: Record<string, any> = {};
    private initialized = false;

    // Private constructor for singleton pattern
    private constructor() { }

    // Get singleton instance
    public static getInstance(): ConfigManager {
        if (!ConfigManager.instance) {
            ConfigManager.instance = new ConfigManager();
        }
        return ConfigManager.instance;
    }

    // Initialize configuration (async)
    public async init(): Promise<void> {
        if (this.initialized) return;

        try {
            // Get app data directory
            const appDataPath = await appDataDir();

            // Create ModpackStore directory if it doesn't exist
            const configDir = await join(appDataPath, "c");
            const configDirExists = await exists(configDir);
            console.log({ configDir, configDirExists });

            if (!configDirExists) {
                await mkdir(configDir, { recursive: true });
            }

            // Set full path for config file
            this.configFileFullPath = await join(configDir, this.configFileName);

            // Set default values that depend on the environment
            DEFAULT_CONFIG[ConfigKey.INSTANCES_DIR] = await join(await this.getUserHome(), "ModpackStore", "Instances");
            DEFAULT_CONFIG[ConfigKey.JAVA_DIR] = await this.getJavaHome();

            await this.loadConfig();
            this.initialized = true;
        } catch (error) {
            console.error("Error initializing ConfigManager:", error);
            throw error;
        }
    }

    // Load configuration from file
    private async loadConfig(): Promise<void> {
        try {
            const fileExists = await exists(this.configFileFullPath);

            if (fileExists) {
                // Read and parse the config file
                const content = await readTextFile(this.configFileFullPath);
                this.configData = JSON.parse(content);
            } else {
                // Initialize with default values
                this.configData = { ...DEFAULT_CONFIG };
                await this.saveConfig();
            }
        } catch (error) {
            console.error("Error loading config:", error);
            // Initialize with default values on error
            this.configData = { ...DEFAULT_CONFIG };
            await this.saveConfig();
        }
    }

    // Save configuration to file
    public async saveConfig(): Promise<void> {
        try {
            await writeTextFile(this.configFileFullPath, JSON.stringify(this.configData, null, 2));
            console.log("Configuration has been saved");
        } catch (error) {
            console.error("Error saving config:", error);
            throw error;
        }
    }

    // Get configuration content
    public getConfigContent(): Record<string, any> {
        return { ...this.configData };
    }

    // Set configuration content
    public async setConfigContent(content: Record<string, any>): Promise<void> {
        this.configData = { ...content };
        await this.saveConfig();
    }

    // Check if updates should be checked on startup
    public checkUpdateOnStartup(): boolean {
        return this.configData[ConfigKey.CHECK_UPDATES_ON_STARTUP] ?? DEFAULT_CONFIG[ConfigKey.CHECK_UPDATES_ON_STARTUP];
    }

    // Get instances directory
    public getInstancesDir(): string {
        return this.configData[ConfigKey.INSTANCES_DIR] ?? DEFAULT_CONFIG[ConfigKey.INSTANCES_DIR];
    }

    // Get Java directory
    public getJavaDir(): string {
        return this.configData[ConfigKey.JAVA_DIR] ?? DEFAULT_CONFIG[ConfigKey.JAVA_DIR];
    }

    // Check if application should close on Minecraft launch
    public closeOnLaunchMinecraft(): boolean {
        return this.configData[ConfigKey.CLOSE_ON_LAUNCH] ?? DEFAULT_CONFIG[ConfigKey.CLOSE_ON_LAUNCH];
    }

    // Set a specific configuration value
    public async setConfig<T>(key: ConfigKey, value: T): Promise<void> {
        this.configData[key] = value;
        await this.saveConfig();
    }

    // Helper method to get user home directory
    private async getUserHome(): Promise<string> {
        // Using Tauri's way to access environment variables
        return await homeDir() || "";
    }

    // Helper method to get Java home directory
    private async getJavaHome(): Promise<string> {
        try {
            const env = await import('@tauri-apps/plugin-os');
            // Try to get JAVA_HOME environment variable, otherwise fall back to a default
            return ""// await env.getEnv("JAVA_HOME") || "";
        } catch (error) {
            console.error("Error getting Java home:", error);
            return "";
        }
    }
}

export default ConfigManager.getInstance();