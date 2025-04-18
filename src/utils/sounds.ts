export const SOUNDS = {
    "PRELAUNCH_NOTIFICATION": "sounds/prelaunch_notification.mp3",
} as const;

type SoundKey = keyof typeof SOUNDS;

export const playSound = (sound: SoundKey, volume: number = 1) => {
    const audio = new Audio(SOUNDS[sound]);
    audio.volume = volume;
    audio.play();
}