use super::*;

const SAMPLE: &str = "diff --git a/src/a.rs b/src/a.rs
index 1111..2222 100644
--- a/src/a.rs
+++ b/src/a.rs
@@ -10,3 +10,4 @@ fn main()
 keep
-old
+new
+more
\\ No newline at end of file
diff --git a/README.md b/README.md
@@ -1 +1 @@
-x
+y
";

#[test]
fn parses_files_hunks_and_line_numbers() {
    let f = parse(SAMPLE);
    assert_eq!(f.len(), 2);
    assert_eq!((f[0].path.as_str(), f[0].additions, f[0].deletions), ("src/a.rs", 2, 1));
    let l = &f[0].hunks[0].lines;
    assert_eq!(l.len(), 4, "the no-newline marker is not a line");
    assert_eq!((l[0].old, l[0].new, l[0].kind), (Some(10), Some(10), Kind::Context));
    assert_eq!((l[1].old, l[1].new, l[1].kind), (Some(11), None, Kind::Removed));
    assert_eq!((l[2].old, l[2].new, l[2].text.as_str()), (None, Some(11), "new"));
    assert_eq!(l[3].new, Some(12));
    assert_eq!(f[1].hunks[0].lines[0].old, Some(1));
}
