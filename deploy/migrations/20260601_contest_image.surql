OPTION IMPORT;

-- Optional thumbnail per contest (WebP on backend volume; see CONTEST_IMAGE_DIR).
DEFINE FIELD IF NOT EXISTS has_image ON contest TYPE option<bool>;
