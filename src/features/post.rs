//! Messages on the wall, stored in the database.

use std::time::{SystemTime, UNIX_EPOCH};

use topcoat::{Result, context::Cx};

use crate::db::db;

/// A Rust program someone posted, after rustc accepted it.
#[derive(Debug, toasty::Model)]
pub struct Post {
    #[key]
    #[auto]
    pub id: u64,

    pub author: String,

    pub code: String,

    /// How long rustc took to check it.
    pub millis: u64,

    /// Seconds since 1970.
    pub posted_at: u64,
}

impl Post {
    /// The newest posts first.
    pub async fn recent(cx: &Cx) -> Result<Vec<Post>> {
        let posts = Post::all()
            .order_by(Post::fields().id().desc())
            .limit(50)
            .exec(&mut db(cx))
            .await?;
        Ok(posts)
    }

    pub async fn add(cx: &Cx, author: &str, code: &str, millis: u64) -> Result<()> {
        let posted_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        toasty::create!(Post { author, code, millis, posted_at })
            .exec(&mut db(cx))
            .await?;
        Ok(())
    }
}
