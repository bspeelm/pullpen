use super::*;

fn dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("pullpen-cred-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    d
}

#[test]
fn saved_tokens_come_back_and_the_file_is_owner_only() {
    let d = dir("roundtrip");
    let p = d.join("credentials.toml");
    let mut c = Credentials::default();
    c.set("github.com", "ghp_one".into());
    c.set("github.com", "ghp_two".into());
    save(&p, &c).unwrap();
    assert_eq!(load(&p).unwrap().get("github.com"), Some("ghp_two"), "setting a host again replaces it");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(fs::metadata(&p).unwrap().permissions().mode() & 0o777, 0o600);
    }
    fs::remove_dir_all(d).unwrap();
}

#[cfg(unix)]
#[test]
fn a_file_others_can_read_is_refused_with_the_fix() {
    use std::os::unix::fs::PermissionsExt;
    let d = dir("open");
    let p = d.join("credentials.toml");
    save(&p, &Credentials::default()).unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(load(&p).unwrap_err().contains("chmod 600"));
    fs::remove_dir_all(d).unwrap();
}

#[test]
fn no_file_is_no_tokens() {
    assert_eq!(load(Path::new("/nonexistent/credentials.toml")).unwrap(), Credentials::default());
}
