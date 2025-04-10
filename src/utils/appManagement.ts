import { appDataDir } from '@tauri-apps/api/path';
import { open } from "@tauri-apps/plugin-fs";
import { invoke } from '@tauri-apps/api/core';
import { exit } from '@tauri-apps/plugin-process';
import { mkdir } from '@tauri-apps/plugin-fs';

export const APP_NAME = "ModpackStore";
export const APP_VERSION = 1;
export const APP_VERSION_STRING = "1.0.0";
export const LOCK_FILE_NAME = "modpackstore.lock";

export class AppManagement {
    static async onAppStart() {
        console.log("La aplicación ha iniciado.");
        invoke('start_discord_presence'); // asumimos que hay un comando en el backend
    }

    static async onAppClose() {
        console.log("La aplicación se está cerrando.");
        await invoke('stop_discord_presence');
        await exit(0);
    }

    static async askToClose() {
        const confirm = await confirm("¿Estás seguro de que quieres salir?", {
            title: "Confirmar salida",
            type: "warning",
        });

        if (confirm) {
            await this.onAppClose();
        }
    }

    static async getAppDataPath(): Promise<string> {
        const appDataDirpath = await appDataDir();
        await mkdir(appDataDirpath, { recursive: true });
        return appDataDirpath;
    }

    static async isAppAlreadyRunning(): Promise<boolean> {
        const result: boolean = await invoke("check_app_lock", {
            lockFileName: LOCK_FILE_NAME,
        });
        return result;
    }

    static async openAppFolder() {
        const path = await this.getAppDataPath();
        try {
            await open(path);
        } catch (error) {
            console.error("Error al abrir la carpeta de configuración:", error);
        }
    }
}
