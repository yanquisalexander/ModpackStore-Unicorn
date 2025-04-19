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

export const searchModpacks = async (query: string) => {
    const url = new URL(`${API_ENDPOINT}/explore/search`)
    url.searchParams.append("q", query)

    const response = await fetch(url.toString(), {
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

export const getModpackById = async (modpackId: string) => {
    const response = await fetch(`${API_ENDPOINT}/explore/modpack/${modpackId}`, {
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