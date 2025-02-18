------------------------------------------------------------
-- Create the posts table
------------------------------------------------------------
CREATE TABLE posts (
  post_id UUID PRIMARY KEY,
  user_id UUID NOT NULL,
  post_title VARCHAR NOT NULL,
  post_slug VARCHAR NOT NULL,
  post_content TEXT NOT NULL,
  post_summary TEXT,
  post_created_at TIMESTAMPTZ NOT NULL,
  post_updated_at TIMESTAMPTZ NOT NULL,
  post_published_at TIMESTAMPTZ,
  post_is_published BOOLEAN NOT NULL DEFAULT false,
  CONSTRAINT fk_posts_user
    FOREIGN KEY (user_id)
    REFERENCES users(user_id)
    ON DELETE CASCADE
);

-- Enforce unique slugs for posts.
CREATE UNIQUE INDEX idx_posts_slug ON posts(post_slug);

-- Index on the foreign key to speed up author lookups.
CREATE INDEX idx_posts_user_id ON posts(user_id);

-- Indexes on time columns.
CREATE INDEX idx_posts_created_at ON posts(post_created_at);
CREATE INDEX idx_posts_updated_at ON posts(post_updated_at);
CREATE INDEX idx_posts_published_at ON posts(post_published_at);

------------------------------------------------------------
-- Create the comments table
------------------------------------------------------------
CREATE TABLE comments (
  comment_id UUID PRIMARY KEY,
  post_id UUID NOT NULL,
  user_id UUID NOT NULL,
  comment_content TEXT NOT NULL,
  comment_created_at TIMESTAMPTZ NOT NULL,
  comment_updated_at TIMESTAMPTZ,
  parent_comment_id UUID,
  CONSTRAINT fk_comments_post
    FOREIGN KEY (post_id)
    REFERENCES posts(post_id)
    ON DELETE CASCADE,
  CONSTRAINT fk_comments_user
    FOREIGN KEY (user_id)
    REFERENCES users(user_id)
    ON DELETE CASCADE,
  CONSTRAINT fk_comments_parent
    FOREIGN KEY (parent_comment_id)
    REFERENCES comments(comment_id)
    ON DELETE CASCADE
);

-- Indexes for foreign keys.
CREATE INDEX idx_comments_post_id ON comments(post_id);
CREATE INDEX idx_comments_user_id ON comments(user_id);
CREATE INDEX idx_comments_parent_id ON comments(parent_comment_id);

-- Indexes on time columns.
CREATE INDEX idx_comments_created_at ON comments(comment_created_at);
CREATE INDEX idx_comments_updated_at ON comments(comment_updated_at);
