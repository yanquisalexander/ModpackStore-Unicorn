import {
    LucideArrowBigDownDash,
    LucideArrowBigLeftDash,
    LucideArrowBigRightDash,
    LucideArrowBigUpDash,
    LucideCode,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";

const konamiCode = [
    "ArrowUp",
    "ArrowUp",
    "ArrowDown",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "ArrowLeft",
    "ArrowRight",
    "b",
    "a",
];

const iconMap: Record<string, JSX.Element> = {
    ArrowUp: <LucideArrowBigUpDash />,
    ArrowDown: <LucideArrowBigDownDash />,
    ArrowLeft: <LucideArrowBigLeftDash />,
    ArrowRight: <LucideArrowBigRightDash />,
};

export const KonamiCode = () => {
    const konamiRef = useRef<HTMLDivElement>(null);
    const videoRef = useRef<HTMLVideoElement>(null);

    const [currentKeyIcon, setCurrentKeyIcon] = useState<JSX.Element | string | null>(null);
    const [comboCount, setComboCount] = useState<number>(-1);
    const [showCurrentKey, setShowCurrentKey] = useState(false);
    const [active, setActive] = useState(false);

    useEffect(() => {
        let position = 0;

        const handleKeyDown = (event: KeyboardEvent) => {
            if (active) return; // Si está activo, ignorar

            const key = event.key;

            if (key === konamiCode[position]) {
                position++;

                setCurrentKeyIcon(iconMap[key] ?? key.toUpperCase());
                setShowCurrentKey(true);
                setComboCount(position - 1);

                if (position === konamiCode.length) {
                    setActive(true); // Bloquear nuevas entradas
                    setShowCurrentKey(false);

                    konamiRef.current?.classList.remove("opacity-0", "pointer-events-none");
                    konamiRef.current?.removeAttribute("aria-hidden");
                    videoRef.current?.play();

                    position = 0;
                }
            } else {
                position = 0;
                setShowCurrentKey(false);
                setComboCount(-1);
            }
        };

        const handleVideoEnd = () => {
            konamiRef.current?.classList.add("opacity-0", "pointer-events-none");
            konamiRef.current?.setAttribute("aria-hidden", "true");
            setActive(false); // Permitir otra vez
        };

        const video = videoRef.current;
        video?.addEventListener("ended", handleVideoEnd);
        document.addEventListener("keydown", handleKeyDown);

        return () => {
            document.removeEventListener("keydown", handleKeyDown);
            video?.removeEventListener("ended", handleVideoEnd);
        };
    }, [active]);

    return (
        <>
            <div
                id="konami"
                ref={konamiRef}
                aria-hidden="true"
                className="pointer-events-none z-[1001] opacity-0 fixed transition-opacity inset-0 flex items-center justify-center bg-black/50 text-white font-bold text-lg"
            >
                <div className="flex flex-col items-center justify-center">
                    <span className="fixed bottom-16 flex items-center gap-2">
                        <LucideCode size={24} />
                        <span>¡Código Konami activado!</span>
                    </span>

                    <video
                        ref={videoRef}
                        src="/assets/videos/lava-chicken.mp4"
                        className="size-72 object-cover animate-rotate-360 animate-iteration-count-infinite animate-duration-[3s]"
                        loop={false}
                        playsInline
                    />
                </div>
            </div>

            <div
                id="konami-current-key"
                className={`pointer-events-none transition-opacity fixed bottom-4 z-80 right-4 ${showCurrentKey ? "opacity-100" : "opacity-0"
                    }`}
            >
                <span className="size-8 justify-center items-center flex text-white bg-black border-2 aspect-square overflow-hidden border-white rounded-md">
                    {currentKeyIcon}
                </span>
                {comboCount >= 0 && (
                    <span className="text-white font-bold text-lg">x{comboCount}</span>
                )}
            </div>
        </>
    );
};
