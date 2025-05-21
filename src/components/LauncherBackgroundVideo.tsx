import { useEffect, useRef, useState } from "react";

export function BackgroundVideo({ videoUrls }: { videoUrls: string | string[] }) {
    const videoRef = useRef<HTMLVideoElement>(null);
    const [currentIndex, setCurrentIndex] = useState(0);

    // Aseguramos que siempre trabajamos con un array
    const videos = Array.isArray(videoUrls) ? videoUrls : [videoUrls];
    const isSingleVideo = videos.length === 1;

    useEffect(() => {
        const video = videoRef.current;
        if (!video || isSingleVideo) return;

        const handleEnded = () => {
            setCurrentIndex((prevIndex) => (prevIndex + 1) % videos.length);
        };

        video.addEventListener("ended", handleEnded);
        return () => video.removeEventListener("ended", handleEnded);
    }, [videos, isSingleVideo]);

    useEffect(() => {
        const video = videoRef.current;
        if (video) {
            video.src = videos[currentIndex];
            video.play().catch(error => {
                console.warn("Video autoplay failed:", error);
            });
        }
    }, [currentIndex, videos]);

    return (
        <video
            ref={videoRef}
            className="absolute inset-0 z-0 h-full w-full object-cover animate-fade-in ease-in-out duration-1000"
            autoPlay
            muted
            playsInline
            loop={isSingleVideo}
        />
    );
}
