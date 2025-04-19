export const SOUNDS = {
    "ERROR_NOTIFICATION": "/sounds/error-notification.mp3",
} as const;

type SoundKey = keyof typeof SOUNDS;

export const playSound = (sound: SoundKey, volume: number = 1) => {
    const audio = new Audio(SOUNDS[sound]);
    audio.volume = volume;
    audio.play();
}