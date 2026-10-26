use super::*;

#[test]
fn no_file_means_the_defaults() {
    let (c, err) = Config::load(Path::new("/nonexistent/pullpen/config.toml"));
    assert!(err.is_none());
    let s = c.sections();
    assert_eq!(s.iter().map(|s| s.short.as_str()).collect::<Vec<_>>(), ["Mine", "Review", "Agent", "Armed", "All"]);
    assert!(!s[4].counts, "the catch-all section does not feed the status counts");
    assert_eq!(s[3].view, View::Armed);
    assert_eq!(c.forges().len(), 2);
}

#[test]
fn the_documented_example_parses() {
    let plan = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/PLAN.md")).unwrap();
    let start = plan.find("## 9. Config reference").unwrap();
    let block = &plan[start..];
    let a = block.find("```toml\n").unwrap() + 8;
    let b = a + block[a..].find("```").unwrap();
    let c: Config =
        toml::from_str(&block[a..b]).unwrap_or_else(|e| panic!("PLAN.md §9 does not load as a Config: {e}"));
    assert_eq!(c.forge.len(), 2);
    assert_eq!(c.section.len(), 5);
    assert_eq!(c.sync.every(), Duration::from_secs(60));
}

#[test]
fn an_unreadable_file_is_reported_and_the_defaults_used() {
    let dir = std::env::temp_dir().join(format!("pullpen-cfg-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("config.toml");
    std::fs::write(&p, "[ui]\nglyphs = 3\n").unwrap();
    let (c, err) = Config::load(&p);
    assert!(err.unwrap().contains("using defaults"));
    assert_eq!(c.ui.glyphs, "unicode");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn intervals() {
    let every = |s: &str| Sync { interval: s.into(), on_focus: true }.every().as_secs();
    assert_eq!((every("60s"), every("5m"), every("90"), every("1s"), every("junk")), (60, 300, 90, 10, 60));
}

#[test]
fn every_failed_check_says_how_to_fix_it() {
    let mut c = Config {
        forge: vec![ForgeCfg {
            kind: Kind::Gitlab,
            host: Some("nowhere.invalid".into()),
            token_cmd: None,
            scope: None,
            groups: vec![],
        }],
        section: vec![SectionCfg {
            name: "x".into(),
            short: None,
            query: "is:open athor:@me".into(),
            sort: None,
            view: None,
            counts: None,
        }],
        ..Config::default()
    };
    c.keys.insert("nope".into(), "z".into());
    c.ui.palette = Some("/nonexistent/palette.toml".into());
    let found = check::check(Path::new("/nonexistent/config.toml"), &c, None, &|_| None);
    let failed: Vec<&check::Finding> = found.iter().filter(|f| !f.ok).collect();
    assert_eq!(failed.len(), 4, "token, section term, key, palette");
    assert!(failed.iter().all(|f| f.fix.is_some()), "a failure without a fix line");
}

#[test]
fn the_example_config_file_matches_the_plan() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let plan = std::fs::read_to_string(format!("{root}/docs/PLAN.md")).unwrap();
    let file = std::fs::read_to_string(format!("{root}/docs/config.example.toml")).unwrap();
    let block = &plan[plan.find("## 9. Config reference").unwrap()..];
    let a = block.find("```toml\n").unwrap() + 8;
    let b = a + block[a..].find("```").unwrap();
    assert!(file.ends_with(&block[a..b]), "docs/config.example.toml has drifted from PLAN.md §9");
    toml::from_str::<Config>(&file).unwrap();
}
