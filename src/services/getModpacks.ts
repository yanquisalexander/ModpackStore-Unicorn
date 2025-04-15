import { API_ENDPOINT } from "@/consts"

export const getModpacks = async () => {
    const response = await fetch(`${API_ENDPOINT}/explore`, {
        method: "GET",
        headers: {
            "Content-Type": "application/json",
            "Accept": "application/json"
        }
    })

    if (!response.ok) {
        throw new Error('Network response was not ok');
    }

    const { data } = await response.json()

    return data
}