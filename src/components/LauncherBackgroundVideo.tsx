import { useEffect, useRef, useState } from "react";

export function BackgroundVideo({ videoUrls }: { videoUrls: string | string[] }) {
    const videoRef = useRef<HTMLVideoElement>(null);
    const [currentIndex, setCurrentIndex] = useState(0);

    useEffect(() => {
        const video = videoRef.current;
        if (!video) return;

        const handleEnded = () => {
            setCurrentIndex((prevIndex) =>
                (prevIndex + 1) % videoUrls.length // esto hace loop infinito
            );
        };

        video.addEventListener("ended", handleEnded);
        return () => video.removeEventListener("ended", handleEnded);
    }, [videoUrls]);

    useEffect(() => {
        const video = videoRef.current;
        if (video) {
            video.src = videoUrls[currentIndex];
            video.play();
        }
    }, [currentIndex, videoUrls]);

    return (
        <video
            ref={videoRef}
            className="absolute inset-0 z-0 h-full w-full object-cover animate-fade-in ease-in-out duration-1000"
            autoPlay
            muted
        />
    );
}
