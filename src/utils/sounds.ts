export const SOUNDS = {
    "ERROR_NOTIFICATION": "/sounds/error-notification.mp3",
} as const;

type SoundKey = keyof typeof SOUNDS;

export const playSound = (sound: SoundKey, volume: number = 1) => {
    const audio = new Audio(SOUNDS[sound]);
    audio.volume = volume;

    // Limpia la referencia cuando el audio termina
    audio.addEventListener('ended', () => {
        // Esto ayuda al recolector de basura a liberar el objeto más rápido
        audio.src = '';
        audio.remove(); // Esto solo es necesario si en algún punto se añade al DOM
    }, { once: true });

    audio.play();
}

export const preloadSounds = () => {
    Object.values(SOUNDS).forEach((sound) => {
        const audio = new Audio(sound);
        audio.load();
        audio.addEventListener("canplaythrough", () => {
            console.log(`Sound preloaded: ${sound}`);
        }, { once: true });
    });
}