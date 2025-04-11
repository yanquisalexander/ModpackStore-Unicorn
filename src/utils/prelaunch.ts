import { PreLaunchAppearance } from "@/types/PreLaunchAppeareance";

export const getDefaultAppeareance = ({ title, description, logoUrl }: { title: string; description: string; logoUrl: string }): PreLaunchAppearance => {

    return {
        title,
        description,

        logo: {
            url: logoUrl,
            height: "132px",
            position: {
                top: "6rem",
                left: "50%",
                transform: "translateX(-50%)"
            },
            fadeInDuration: "500ms",
            fadeInDelay: "1000ms"
        },

        playButton: {
            text: "Jugar ahora",
            fontFamily: "monocraft",
            backgroundColor: "#00a63e",
            hoverColor: "#262626",
            textColor: "#ffffff",
            borderColor: "#ffffff",

            fadeInDuration: "500ms",
            fadeInDelay: "1500ms"
        },

        background: {
            imageUrl: "https://images.steamusercontent.com/ugc/2310974141604980016/B4EF3A7A2D1772DE26B1A6F51CE33A04FD8BB917/",
            videoUrl: null,
        },

        audio: {
            url: "http://cdn.saltouruguayserver.com/sounds/launcher_bg_loop.mp3",
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