use serde::Deserialize;
use aidoku::alloc::{String, Vec};

#[derive(Deserialize, Debug, Clone)]
pub struct SearchResponse {
    pub hits: Vec<SearchHit>,
    pub found: i32,
    pub page: i32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SearchHit {
    pub document: SearchDocument,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SearchDocument {
    pub id: String,
    pub title: String,
    pub poster: Option<String>,
    pub status: Option<String>,
    pub synopsis: Option<String>,
    pub tags: Option<Vec<String>>,
    pub authors: Option<Vec<String>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChapterPageResponse {
    pub read_chapter: ReadChapter,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReadChapter {
    pub id: String,
    pub title: Option<String>,
    pub pages: Vec<ChapterPage>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChapterPage {
    pub id: String,
    pub image: String,
    pub number: i32,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MangaPageWrapper {
    pub manga_page: MangaPageDetail,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MangaPageDetail {
    pub id: String,
    pub title: String,
    pub english_title: Option<String>,
    pub poster: Option<String>,
    pub banner: Option<String>,
    pub status: Option<String>,
    pub synopsis: Option<String>,
    pub genres: Option<Vec<String>>,
    pub authors: Option<Vec<String>>,
    pub chapters: Option<Vec<ChapterItem>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChapterListResponse {
    pub chapters: Vec<ChapterItem>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChapterItem {
    pub id: String,
    pub title: Option<String>,
    pub number: f32,
    pub created_at: i64,
}
