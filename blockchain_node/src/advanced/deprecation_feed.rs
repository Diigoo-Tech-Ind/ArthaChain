/// Deprecation Feed API
/// Provides RSS/Atom feed and REST API for schema/API deprecation announcements

use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router, Json,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeprecationAnnouncement {
    pub id: String,
    pub component: String, // "schema", "api", "sdk", "contract"
    pub name: String, // e.g., "DIDDoc@v0", "registerIdentity()"
    pub announced_at: u64,
    pub sunset_date: u64, // Unix timestamp
    pub replacement: String, // e.g., "DIDDoc@v1", "createDID()"
    pub migration_guide_url: String,
    pub severity: DeprecationSeverity,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeprecationSeverity {
    Info,    // Minor API changes, no breaking changes
    Warning, // Deprecated, but still works
    Critical, // Will break in next version
}

pub struct DeprecationFeed {
    announcements: Arc<RwLock<Vec<DeprecationAnnouncement>>>,
}

impl DeprecationFeed {
    pub fn new() -> Self {
        let mut announcements = Vec::new();
        
        // Seed with example deprecation notices
        announcements.push(DeprecationAnnouncement {
            id: "DEP-001".to_string(),
            component: "sdk".to_string(),
            name: "registerIdentity()".to_string(),
            announced_at: 1730592000,
            sunset_date: 1762128000, // 24 months later
            replacement: "createDID()".to_string(),
            migration_guide_url: "https://docs.arthachain.online/migration/dep-001".to_string(),
            severity: DeprecationSeverity::Warning,
            description: "Legacy registerIdentity() method is deprecated. Use createDID() with explicit auth and encryption keys.".to_string(),
        });
        
        DeprecationFeed {
            announcements: Arc::new(RwLock::new(announcements)),
        }
    }

    pub fn add_announcement(&self, announcement: DeprecationAnnouncement) {
        let mut announcements = self.announcements.write().unwrap();
        announcements.push(announcement);
        
        // Sort by sunset_date (earliest first)
        announcements.sort_by_key(|a| a.sunset_date);
    }

    pub fn get_all_announcements(&self) -> Vec<DeprecationAnnouncement> {
        let announcements = self.announcements.read().unwrap();
        announcements.clone()
    }

    pub fn get_active_announcements(&self) -> Vec<DeprecationAnnouncement> {
        let announcements = self.announcements.read().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        announcements
            .iter()
            .filter(|a| a.sunset_date > now)
            .cloned()
            .collect()
    }

    pub fn to_rss(&self) -> String {
        let announcements = self.get_active_announcements();
        
        let items: Vec<String> = announcements
            .iter()
            .map(|a| {
                format!(
                    r#"    <item>
      <title>{} - {}</title>
      <description>{}</description>
      <link>{}</link>
      <guid isPermaLink="false">{}</guid>
      <pubDate>{}</pubDate>
      <category>{}</category>
    </item>"#,
                    a.component.to_uppercase(),
                    a.name,
                    a.description,
                    a.migration_guide_url,
                    a.id,
                    Self::timestamp_to_rfc2822(a.announced_at),
                    format!("{:?}", a.severity),
                )
            })
            .collect();

        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>ArthaChain Deprecation Feed</title>
    <link>https://arthachain.online/deprecations</link>
    <description>Official deprecation announcements for ArthaChain APIs and schemas</description>
    <language>en-us</language>
    <lastBuildDate>{}</lastBuildDate>
{}
  </channel>
</rss>"#,
            Self::timestamp_to_rfc2822(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()),
            items.join("\n")
        )
    }

    fn timestamp_to_rfc2822(timestamp: u64) -> String {
        use chrono::{DateTime, Utc};
        let dt = DateTime::from_timestamp(timestamp as i64, 0).unwrap_or(Utc::now());
        dt.to_rfc2822()
    }
}

/// API Handlers

pub async fn get_all_deprecations(
    feed: Arc<DeprecationFeed>,
) -> Json<Vec<DeprecationAnnouncement>> {
    Json(feed.get_all_announcements())
}

pub async fn get_active_deprecations(
    feed: Arc<DeprecationFeed>,
) -> Json<Vec<DeprecationAnnouncement>> {
    Json(feed.get_active_announcements())
}

pub async fn get_rss_feed(feed: Arc<DeprecationFeed>) -> impl IntoResponse {
    let rss = feed.to_rss();
    (
        [(axum::http::header::CONTENT_TYPE, "application/rss+xml")],
        rss,
    )
}

pub async fn get_deprecation_by_id(
    Path(id): Path<String>,
    feed: Arc<DeprecationFeed>,
) -> Result<Json<DeprecationAnnouncement>, StatusCode> {
    let announcements = feed.get_all_announcements();
    announcements
        .into_iter()
        .find(|a| a.id == id)
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub fn deprecation_router(feed: Arc<DeprecationFeed>) -> Router {
    Router::new()
        .route("/api/v1/deprecations", get({
            let feed = Arc::clone(&feed);
            move || get_all_deprecations(Arc::clone(&feed))
        }))
        .route("/api/v1/deprecations/active", get({
            let feed = Arc::clone(&feed);
            move || get_active_deprecations(Arc::clone(&feed))
        }))
        .route("/api/v1/deprecations/rss", get({
            let feed = Arc::clone(&feed);
            move || get_rss_feed(Arc::clone(&feed))
        }))
        .route("/api/v1/deprecations/:id", get({
            let feed = Arc::clone(&feed);
            move |path| get_deprecation_by_id(path, Arc::clone(&feed))
        }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deprecation_feed_creation() {
        let feed = DeprecationFeed::new();
        let all = feed.get_all_announcements();
        assert!(!all.is_empty());
    }

    #[test]
    fn test_add_announcement() {
        let feed = DeprecationFeed::new();
        
        feed.add_announcement(DeprecationAnnouncement {
            id: "TEST-001".to_string(),
            component: "test".to_string(),
            name: "testMethod()".to_string(),
            announced_at: 1000,
            sunset_date: 2000,
            replacement: "newMethod()".to_string(),
            migration_guide_url: "https://example.com".to_string(),
            severity: DeprecationSeverity::Info,
            description: "Test deprecation".to_string(),
        });
        
        let all = feed.get_all_announcements();
        assert!(all.iter().any(|a| a.id == "TEST-001"));
    }

    #[test]
    fn test_rss_generation() {
        let feed = DeprecationFeed::new();
        let rss = feed.to_rss();
        
        assert!(rss.contains("<?xml version=\"1.0\""));
        assert!(rss.contains("<rss version=\"2.0\">"));
        assert!(rss.contains("Deprecation Feed"));
    }
}

