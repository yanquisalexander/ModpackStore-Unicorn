export const getModpacks = async () => {

    /* 
        Para DEMO, usamos los de heberon multimc
    */

    const response = await fetch("https://api.modrinth.com/v2/search?query=&facets=[[\"project_type:modpack\"]]", {
        method: "GET",
        headers: {
            "Content-Type": "application/json",
            "Accept": "application/json"
        }
    })

    const data = await response.json()

    return data.hits
}