#![no_std]
use aidoku::{
    Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeComponent, HomeComponentValue,
    HomeLayout, Listing, ListingProvider, Manga, MangaPageResult, MangaStatus, Page, PageContent,
    Result, Source, Viewer,
    alloc::{String, Vec, string::ToString, format, vec},
    imports::net::{Request, TimeUnit, set_rate_limit},
    prelude::*,
};

mod models;
use models::*;

const BASE_URL: &str = "https://atsu.moe";
const API_BASE: &str = "https://atsu.moe/api";
const SEARCH_URL: &str = "https://atsu.moe/collections/manga/documents/search";

struct Atsumaru;

fn resolve_image_url(path: String) -> String {
    if path.starts_with("http") {
        path
    } else if path.starts_with("/static/") || path.starts_with("static/") {
        format!("{}/{}", BASE_URL, path.trim_start_matches('/'))
    } else if path.starts_with("/") {
        format!("{}{}", BASE_URL, path)
    } else {
        format!("{}/static/{}", BASE_URL, path)
    }
}

fn build_manga_from_doc(doc: &SearchDocument) -> Manga {
    Manga {
        key: doc.id.clone().unwrap_or_default(),
        title: doc.title.clone().unwrap_or_default(),
        cover: doc.poster.clone().map(resolve_image_url),
        url: Some(format!("{}/manga/{}", BASE_URL, doc.id.clone().unwrap_or_default())),
        status: match doc.status.as_deref() {
            Some("Ongoing") => MangaStatus::Ongoing,
            Some("Completed") => MangaStatus::Completed,
            Some("Hiatus") => MangaStatus::Hiatus,
            Some("Dropped") | Some("Cancelled") => MangaStatus::Cancelled,
            _ => MangaStatus::Unknown,
        },
        description: doc.synopsis.clone(),
        tags: doc.tags.clone(),
        authors: doc.authors.clone(),
        viewer: Viewer::Webtoon,
        ..Default::default()
    }
}

impl Atsumaru {
    fn fetch_search(q: &str, page: i32, per_page: i32, sort_by: &str) -> Result<MangaPageResult> {
        let q_escaped = q.replace(" ", "%20");
        let mut url = format!(
            "{}?q={}&page={}&per_page={}&query_by=title,englishTitle,otherNames,authors&include_fields=id,title,englishTitle,poster,posterSmall,posterMedium,type,isAdult,status,year,synopsis,tags,authors",
            SEARCH_URL,
            q_escaped,
            page,
            per_page
        );

        if !sort_by.is_empty() {
            url.push_str(&format!("&sort_by={}", sort_by));
        }

        let json = Request::get(&url)?.json_owned::<SearchResponse>()?;

        let entries: Vec<Manga> = json.hits.into_iter().map(|hit| build_manga_from_doc(&hit.document)).collect();
        let has_next_page = json.found > (json.page * per_page);

        Ok(MangaPageResult {
            entries,
            has_next_page,
        })
    }
}

impl Source for Atsumaru {
    fn new() -> Self {
        set_rate_limit(5, 1, TimeUnit::Seconds);
        Self
    }

    fn get_search_manga_list(
        &self,
        query: Option<String>,
        page: i32,
        filters: Vec<FilterValue>,
    ) -> Result<MangaPageResult> {
        let q = query.unwrap_or_else(|| "*".to_string());
        
        let mut sort_by = String::new();
        for filter in filters {
            if let FilterValue::Sort { id, index, ascending } = filter {
                if id == "sort" {
                    let order = if ascending { "asc" } else { "desc" };
                    let sort_field = match index {
                        0 => "views",
                        1 => "released",
                        2 => "title",
                        _ => "views",
                    };
                    sort_by = format!("{}:{}", sort_field, order);
                }
            }
        }

        Self::fetch_search(&q, page, 24, if sort_by.is_empty() { "" } else { &sort_by })
    }

    fn get_manga_update(
        &self,
        manga: Manga,
        needs_details: bool,
        needs_chapters: bool,
    ) -> Result<Manga> {
        let mut updated_manga = manga.clone();
        let mut scanlator_map = aidoku::alloc::collections::BTreeMap::new();

        if needs_details || needs_chapters {
            let url = format!("{}/manga/page?id={}", API_BASE, manga.key);
            if let Ok(json) = Request::get(&url)?.json_owned::<MangaPageWrapper>() {
                println!("Details API parsed successfully!");
                let detail = json.manga_page;
                
                if needs_details {
                    updated_manga.title = detail.title.unwrap_or_default();
                    updated_manga.description = detail.synopsis;
                    updated_manga.cover = detail.poster.and_then(|p| p.image).map(resolve_image_url);
                    println!("Cover URL is correctly formatted: {}", updated_manga.cover.clone().unwrap_or_default());
                    updated_manga.url = Some(format!("{}/manga/{}", BASE_URL, detail.id.clone().unwrap_or_default()));
                    updated_manga.status = match detail.status.as_deref() {
                        Some("Ongoing") => MangaStatus::Ongoing,
                        Some("Completed") => MangaStatus::Completed,
                        Some("Hiatus") => MangaStatus::Hiatus,
                        Some("Dropped") | Some("Cancelled") => MangaStatus::Cancelled,
                        _ => MangaStatus::Unknown,
                    };
                    let authors_vec = detail.authors.unwrap_or_default().into_iter().filter_map(|e| e.name).collect::<Vec<_>>();
                    updated_manga.authors = if authors_vec.is_empty() { None } else { Some(authors_vec) };
                    
                    let tags_vec = detail.genres.unwrap_or_default().into_iter().filter_map(|e| e.name).collect::<Vec<_>>();
                    updated_manga.tags = if tags_vec.is_empty() { None } else { Some(tags_vec) };
                }

                if let Some(scanlators) = detail.scanlators {
                    println!("Found {} scanlators from details API", scanlators.len());
                    for s in scanlators {
                        println!("Mapping scanlator {} -> {}", s.id, s.name);
                        scanlator_map.insert(s.id, s.name);
                    }
                } else {
                    println!("No scanlators found in detail.scanlators");
                }
            } else {
                println!("Failed to parse MangaPageWrapper structurally in Rust!");
            }
        }

        if needs_chapters {
            let url = format!("{}/manga/allChapters?mangaId={}", API_BASE, manga.key);
            if let Ok(json) = Request::get(&url)?.json_owned::<ChapterListResponse>() {
                let mut chapters = Vec::new();
                for chap in json.chapters {
                    let chapter_url = format!("{}/read/{}?chapterId={}", BASE_URL, manga.key, chap.id);
                    let scanlator_name = chap.scanlation_manga_id
                        .as_ref()
                        .and_then(|id| scanlator_map.get(id))
                        .map(|s| s.clone());
                    
                    chapters.push(Chapter {
                        key: chap.id.clone(),
                        title: chap.title.clone(),
                        chapter_number: Some(chap.number),
                        date_uploaded: Some(chap.created_at / 1000), // ms to seconds
                        url: Some(chapter_url),
                        scanlators: scanlator_name.map(|s| vec![s]),
                        ..Default::default()
                    });
                }
                
                // Sort chapters descending by chapter number to fix hierarchy for multiple scanlators
                chapters.sort_by(|a, b| {
                    let a_num = a.chapter_number.unwrap_or(0.0);
                    let b_num = b.chapter_number.unwrap_or(0.0);
                    b_num.partial_cmp(&a_num).unwrap_or(core::cmp::Ordering::Equal)
                });
                
                updated_manga.chapters = Some(chapters);
            }
        }

        Ok(updated_manga)
    }

    fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
        let url = format!("{}/read/chapter?mangaId={}&chapterId={}", API_BASE, manga.key, chapter.key);
        let json = Request::get(&url)?.json_owned::<ChapterPageResponse>()?;

        let pages = json.read_chapter.pages.into_iter().map(|p| {
            let img_url = resolve_image_url(p.image);
            Page {
                content: PageContent::url(img_url),
                ..Default::default()
            }
        }).collect();

        Ok(pages)
    }
}

impl ListingProvider for Atsumaru {
    fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
        let sort_by = match listing.id.as_str() {
            "views" => "views:desc",
            "latest" => "released:desc",
            _ => "views:desc",
        };
        Atsumaru::fetch_search("*", page, 24, sort_by)
    }
}

impl Home for Atsumaru {
    fn get_home(&self) -> Result<HomeLayout> {
        let mut components = Vec::new();

        if let Ok(popular) = Atsumaru::fetch_search("*", 1, 12, "views:desc") {
            if !popular.entries.is_empty() {
                components.push(HomeComponent {
                    title: Some("Most Popular".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::BigScroller {
                        entries: popular.entries,
                        auto_scroll_interval: Some(6.0),
                    },
                });
            }
        }

        if let Ok(latest) = Atsumaru::fetch_search("*", 1, 24, "released:desc") {
            if !latest.entries.is_empty() {
                components.push(HomeComponent {
                    title: Some("Latest Updates".to_string()),
                    subtitle: None,
                    value: HomeComponentValue::MangaList {
                        ranking: false,
                        page_size: Some(24),
                        entries: latest.entries.into_iter().map(Into::into).collect(),
                        listing: Some(Listing {
                            id: "latest".to_string(),
                            name: "Latest Updates".to_string(),
                            ..Default::default()
                        }),
                    },
                });
            }
        }

        Ok(HomeLayout { components })
    }
}

impl DeepLinkHandler for Atsumaru {
    fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
        if url.contains("/manga/") {
            let parts: Vec<&str> = url.split("/manga/").collect();
            if parts.len() > 1 {
                let id = parts[1].split('/').next().unwrap_or("");
                if !id.is_empty() {
                    return Ok(Some(DeepLinkResult::Manga { key: id.to_string() }));
                }
            }
        }
        if url.contains("/read/") {
            let parts: Vec<&str> = url.split("/read/").collect();
            if parts.len() > 1 {
                let segments: Vec<&str> = parts[1].split('?').collect();
                let manga_key = segments[0].to_string();
                if let Some(query) = segments.get(1) {
                    if query.starts_with("chapterId=") {
                        let chapter_key = query[10..].to_string();
                        return Ok(Some(DeepLinkResult::Chapter { manga_key, key: chapter_key }));
                    }
                }
            }
        }
        Ok(None)
    }
}

register_source!(Atsumaru, ListingProvider, Home, DeepLinkHandler);

#[cfg(test)]
mod tests {
    use super::*;
    use aidoku_test::aidoku_test;

    #[aidoku_test]
    fn test_get_manga_update() {
        let source = Atsumaru;
        let manga = Manga {
            key: "WgrNM".to_string(),
            ..Default::default()
        };
        let updated = source.get_manga_update(manga, true, true).expect("Failed to get manga update");
        
        let chapters = updated.chapters.expect("No chapters found");
        assert!(!chapters.is_empty(), "Chapters should not be empty");
        
        let cover_url = updated.cover.expect("No cover found for the manga details");
        assert!(!cover_url.is_empty(), "Cover URL should not be empty");
        println!("Cover URL is correctly formatted: {}", cover_url);
        
        let mut found_scanlators = false;
        for (i, chapter) in chapters.iter().enumerate().take(5) {
            println!("Chapter {} scanlators: {:?}", i, chapter.scanlators);
            if chapter.scanlators.is_some() {
                found_scanlators = true;
            }
        }
        assert!(found_scanlators, "At least one chapter should have scanlators mapped from the details API");
    }

    #[aidoku_test]
    fn test_chapter_sorting() {
        let source = Atsumaru;
        let manga = Manga {
            key: "TKRmo".to_string(), // Absolute Regression
            ..Default::default()
        };
        let updated = source.get_manga_update(manga, false, true).expect("Failed to get manga chapters");
        
        let chapters = updated.chapters.expect("No chapters found");
        println!("Found {} chapters for TKRmo", chapters.len());
        
        for (i, chapter) in chapters.iter().enumerate().take(20) {
            println!("Index {}: Ch. {:?} - {:?} by {:?}", i, chapter.chapter_number, chapter.title, chapter.scanlators);
        }
    }
}
