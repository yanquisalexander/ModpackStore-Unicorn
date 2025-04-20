import { PreLaunchAppearance } from "@/types/PreLaunchAppeareance";

export const getDefaultAppeareance = ({ title, description, logoUrl }: { title?: string; description?: string; logoUrl?: string }): PreLaunchAppearance => {

    return {
        title,
        description,

        logo: {
            url: logoUrl,
            height: "56px",
            position: {
                top: "8rem",
                left: "50%",
                transform: "translateX(-50%)"
            },
            fadeInDuration: "500ms",
            fadeInDelay: "1000ms"
        },

        playButton: {
            text: "Jugar ahora",
            backgroundColor: "#00a63e",
            hoverColor: "#262626",
            textColor: "#ffffff",
            borderColor: "#ffffff",

            fadeInDuration: "500ms",
            fadeInDelay: "1500ms"
        },

        background: {
            videoUrl: "/assets/videos/prelaunch-default-1.mp4",
        },


        news: {
            position: {
                top: "3rem",
                right: "2rem"
            },
            style: {
                background: "rgba(0,0,0,0.8)",
                color: "#ffffff",
                borderRadius: "0.5rem",
                padding: "1rem",
                width: "20rem",
                fontSize: "0.875rem"
            },
            entries: []
        }
    }
}