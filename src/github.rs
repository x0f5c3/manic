use hyper::client::connect::Connect;
use serde::{Deserialize, Serialize};

use crate::Result;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// GitHub API release
pub struct Release {
    url: String,
    #[serde(rename = "assets_url")]
    assets_url: String,
    #[serde(rename = "upload_url")]
    upload_url: String,
    #[serde(rename = "html_url")]
    html_url: String,
    id: i64,
    author: Author,
    #[serde(rename = "node_id")]
    node_id: String,
    #[serde(rename = "tag_name")]
    tag_name: String,
    #[serde(rename = "target_commitish")]
    target_commitish: String,
    /// Release name
    pub name: String,
    draft: bool,
    prerelease: bool,
    #[serde(rename = "created_at")]
    created_at: String,
    #[serde(rename = "published_at")]
    published_at: String,
    /// Release assets
    pub assets: Vec<Asset>,
    #[serde(rename = "tarball_url")]
    /// Release tarball url
    pub tarball_url: String,
    #[serde(rename = "zipball_url")]
    /// Release zipball url
    pub zipball_url: String,
    body: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Author {
    login: String,
    id: i64,
    #[serde(rename = "node_id")]
    node_id: String,
    #[serde(rename = "avatar_url")]
    avatar_url: String,
    #[serde(rename = "gravatar_id")]
    gravatar_id: String,
    url: String,
    #[serde(rename = "html_url")]
    html_url: String,
    #[serde(rename = "followers_url")]
    followers_url: String,
    #[serde(rename = "following_url")]
    following_url: String,
    #[serde(rename = "gists_url")]
    gists_url: String,
    #[serde(rename = "starred_url")]
    starred_url: String,
    #[serde(rename = "subscriptions_url")]
    subscriptions_url: String,
    #[serde(rename = "organizations_url")]
    organizations_url: String,
    #[serde(rename = "repos_url")]
    repos_url: String,
    #[serde(rename = "events_url")]
    events_url: String,
    #[serde(rename = "received_events_url")]
    received_events_url: String,
    #[serde(rename = "type")]
    type_field: String,
    #[serde(rename = "site_admin")]
    site_admin: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Github API release asset
pub struct Asset {
    /// Asset API url
    pub url: String,
    id: i64,
    #[serde(rename = "node_id")]
    node_id: String,
    /// Asset name
    pub name: String,
    label: serde_json::Value,
    uploader: Uploader,
    #[serde(rename = "content_type")]
    /// Asset content type
    pub content_type: String,
    state: String,
    /// Asset size
    pub size: i64,
    #[serde(rename = "download_count")]
    download_count: i64,
    #[serde(rename = "created_at")]
    created_at: String,
    #[serde(rename = "updated_at")]
    updated_at: String,
    #[serde(rename = "browser_download_url")]
    /// Release asset download URL
    pub browser_download_url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Uploader {
    login: String,
    id: i64,
    #[serde(rename = "node_id")]
    node_id: String,
    #[serde(rename = "avatar_url")]
    avatar_url: String,
    #[serde(rename = "gravatar_id")]
    gravatar_id: String,
    url: String,
    #[serde(rename = "html_url")]
    html_url: String,
    #[serde(rename = "followers_url")]
    followers_url: String,
    #[serde(rename = "following_url")]
    following_url: String,
    #[serde(rename = "gists_url")]
    gists_url: String,
    #[serde(rename = "starred_url")]
    starred_url: String,
    #[serde(rename = "subscriptions_url")]
    subscriptions_url: String,
    #[serde(rename = "organizations_url")]
    organizations_url: String,
    #[serde(rename = "repos_url")]
    repos_url: String,
    #[serde(rename = "events_url")]
    events_url: String,
    #[serde(rename = "received_events_url")]
    received_events_url: String,
    #[serde(rename = "type")]
    type_field: String,
    #[serde(rename = "site_admin")]
    site_admin: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Github API tag
pub struct Tag {
    /// Tag name
    pub name: String,
    #[serde(rename = "zipball_url")]
    zipball_url: String,
    #[serde(rename = "tarball_url")]
    tarball_url: String,
    commit: Commit,
    #[serde(rename = "node_id")]
    node_id: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Commit {
    sha: String,
    url: String,
}

impl Release {
    /// Get latest release
    ///
    /// Using repo name in format "owner/repo"
    pub async fn latest_assets<C>(repo: &str, client: crate::Client<C>) -> Result<Vec<Asset>>
    where
        C: Connect + Clone + Sync + Send + Unpin + 'static
    {
        let url = format!("https://api.github.com/repos/{}/releases/latest", repo);
        let rel: Self = client.get(&url).await?.json().await?;
        Ok(rel.assets)

    }
}
