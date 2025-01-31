use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct SpigotResourceVersion {
    pub name: String,
    pub id: i32,
}

pub fn get_resource_id(res: &str) -> &str {
    if let Some(i) = res.find('.') {
        if i < res.len() - 1 {
            return res.split_at(i + 1).1;
        }
    }

    res
}

pub async fn fetch_spigot_info(client: &reqwest::Client, id: &str) -> Result<(String, String)> {
    let json = client
        .get("https://api.spiget.org/v2/resources/".to_owned() + get_resource_id(id))
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;

    let name = json["name"].as_str().unwrap().to_owned();
    let tag = json["tag"].as_str().unwrap().to_owned();

    Ok((name, tag))
}

pub async fn fetch_spigot_resource_latest_ver(
    id: &str,
    client: &reqwest::Client,
) -> Result<String> {
    let project: SpigotResourceVersion = client
        .get(
            "https://api.spiget.org/v2/resources/".to_owned()
                + get_resource_id(id)
                + "/versions/latest",
        )
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(project.id.to_string())
}

pub fn get_spigot_url(id: &str) -> String {
    let id_parsed = get_resource_id(id);
    format!("https://api.spiget.org/v2/resources/{id_parsed}/download")
}
