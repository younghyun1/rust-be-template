ALTER TABLE public."comments"
    ALTER COLUMN comment_created_at SET DEFAULT NOW();
