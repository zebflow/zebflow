use super::*;

fn route_of(host: &str, path: &str, surface: Surface, root: Option<&str>) -> AddressRoute {
    AddressRoute { host: host.into(), path: path.into(), surface, root: root.map(str::to_string) }
}

/// The §2c example: `example.org` rootless, `a.example.net` the section
/// `/site-a` with a shared `/api/`.
fn example() -> ProjectAddressing {
    ProjectAddressing {
        hosts: vec!["example.org".into(), "a.example.net".into()],
        routes: vec![
            route_of("a.example.net", "/", Surface::Pages, Some("/site-a")),
            route_of("a.example.net", "/api/", Surface::Pages, None),
        ],
        disabled: Vec::new(),
        ..ProjectAddressing::default()
    }
}

fn table() -> SectionTable {
    SectionTable::build("superadmin", "northside", &example(), &[]).expect("sectioned")
}

fn on(table: &SectionTable, host: &str, path: &str) -> Routed {
    let Some(segments) = read_path(path) else {
        return Routed::BadPath;
    };
    route(table, &table.hosts[host], &segments, |_| true)
}

fn pages(path: &[&str], root: Option<&str>, mount: &str) -> Routed {
    Routed::Surface {
        surface: Surface::Pages,
        mount: mount.into(),
        segments: path.iter().map(|s| s.to_string()).collect(),
        root: root.map(str::to_string),
    }
}

#[test]
fn a_rootless_project_has_no_table() {
    let mut rootless = example();
    rootless.routes[0].root = None;
    rootless.routes.pop();
    assert!(SectionTable::build("o", "p", &rootless, &[]).is_none());
    assert!(!rootless.is_sectioned());
    let json = serde_json::to_string(&rootless.routes[0]).unwrap();
    assert!(!json.contains("root"), "an absent root is never written: {json}");
}

#[test]
fn every_row_of_the_section_table() {
    let t = table();
    // a.example.net/ · /map → webhook /site-a · /site-a/map
    assert_eq!(on(&t, "a.example.net", "/"), pages(&["site-a"], Some("site-a"), ""));
    assert_eq!(on(&t, "a.example.net", "/map"), pages(&["site-a", "map"], Some("site-a"), ""));
    // a.example.net/api/data → webhook /api/data (shared)
    assert_eq!(on(&t, "a.example.net", "/api/data"), pages(&["api", "data"], None, "/api"));
    // a.example.net/_mcp/tools → trigger.mcp --route /site-a/tools
    assert_eq!(
        on(&t, "a.example.net", "/_mcp/tools"),
        Routed::Surface { surface: Surface::Mcp, mount: "/_mcp".into(), segments: vec!["site-a".into(), "tools".into()], root: Some("site-a".into()) }
    );
    // example.org/map → webhook /map
    assert_eq!(on(&t, "example.org", "/map"), pages(&["map"], None, ""));
    // example.org/site-a/map · //site-a/map · /%73ite-a/map → claimed
    for path in ["/site-a/map", "//site-a/map", "/%73ite-a/map", "/site-a"] {
        assert_eq!(on(&t, "example.org", path), Routed::NotFound, "{path}");
    }
    // example.org/_mcp/site-a/tools → claimed
    assert_eq!(on(&t, "example.org", "/_mcp/site-a/tools"), Routed::NotFound);
    // encoded `/`, `\`, NUL and dot segments → 400
    for path in ["/%2Fsite-a/map", "/x/%2e%2e/y", "/x/../y", "/a%5Cb", "/a%00", "/./map"] {
        assert_eq!(on(&t, "a.example.net", path), Routed::BadPath, "{path}");
    }
    // site-a.northside.superadmin.localhost/map → webhook /site-a/map (dev)
    let dev = "site-a.northside.superadmin.localhost";
    assert_eq!(t.section_dev_hosts.get("site-a").map(String::as_str), Some(dev));
    assert_eq!(on(&t, dev, "/map"), pages(&["site-a", "map"], Some("site-a"), ""));
    assert_eq!(on(&t, dev, "/api/data"), pages(&["api", "data"], None, "/api"), "the dev host mirrors the shared route");
    // the project's own dev host is rootless and claims too
    assert_eq!(on(&t, "northside.superadmin.localhost", "/site-a/map"), Routed::NotFound);
    assert_eq!(on(&t, "northside.superadmin.localhost", "/map"), pages(&["map"], None, ""));
}

#[test]
fn a_section_host_keeps_its_default_mounts_and_reserves_underscore() {
    let t = table();
    assert!(matches!(on(&t, "a.example.net", "/_static/app.js"), Routed::Surface { surface: Surface::Static, .. }));
    assert!(matches!(on(&t, "a.example.net", "/_ws/rooms/demo"), Routed::Surface { surface: Surface::Ws, .. }));
    // A segment right after the root that starts with `_` matches nothing.
    assert_eq!(on(&t, "a.example.net", "/_nothing"), Routed::NotFound);
    // The OAuth suffix of a rooted mcp route is split off first.
    assert_eq!(
        on(&t, "a.example.net", "/_mcp/_oauth/token"),
        Routed::Surface {
            surface: Surface::Mcp,
            mount: "/_mcp".into(),
            segments: vec!["site-a".into(), "_oauth".into(), "token".into()],
            root: Some("site-a".into())
        }
    );
}

#[test]
fn a_switched_off_surface_is_off_on_a_section_host() {
    let t = table();
    let segments = read_path("/_mcp/tools").unwrap();
    assert_eq!(route(&t, &t.hosts["a.example.net"], &segments, |s| s != Surface::Mcp), Routed::Disabled(Surface::Mcp));
}

#[test]
fn the_visitor_form_routes_back_or_is_not_emitted() {
    let t = table();
    let host = &t.hosts["a.example.net"];
    let seg = |p: &str| written_segments(p).unwrap();
    assert_eq!(pages_visitor_path(&t, host, &seg("/site-a/login")), Some(seg("/login")));
    assert_eq!(pages_visitor_path(&t, host, &seg("/api/login")), Some(seg("/api/login")), "shared stays as it is");
    assert_eq!(pages_visitor_path(&t, host, &seg("/site-a/api/x")), None, "shadowed by the shared route");
    assert_eq!(pages_visitor_path(&t, host, &seg("/other/login")), None, "outside the section");
    assert_eq!(pages_url(&t, "a.example.net", "/site-a/map?x=1").as_deref(), Some("/map?x=1"));
    assert_eq!(pages_url(&t, "a.example.net", "https://elsewhere.example/x").as_deref(), Some("https://elsewhere.example/x"));
    assert_eq!(pages_url(&t, "a.example.net", "//evil.example/x"), None, "never an authority");
    // On the rootless host, a section's path does not route back.
    assert_eq!(pages_url(&t, "example.org", "/site-a/login"), None);
    assert_eq!(pages_url(&t, "example.org", "/login").as_deref(), Some("/login"));
}

#[test]
fn the_serializer_escapes_and_refuses_dot_segments() {
    assert_eq!(serialize(&["a b".into(), "c%d".into()]).as_deref(), Some("/a%20b/c%25d"));
    assert_eq!(serialize(&[]).as_deref(), Some("/"));
    assert_eq!(serialize(&["..".into()]), None);
    // decoded once, then escaped: a double-encoded segment stays one segment
    let read = read_path("/%2573ite-a").unwrap();
    assert_eq!(read, vec!["%73ite-a".to_string()]);
    assert_eq!(serialize(&read).as_deref(), Some("/%2573ite-a"));
}

#[test]
fn saving_refuses_what_section_rules_forbid() {
    let ok = example();
    assert!(validate("superadmin", &ok.hosts, &ok.routes, &[]).is_ok());
    let refused = |routes: Vec<AddressRoute>, hosts: Vec<&str>| {
        let hosts: Vec<String> = hosts.into_iter().map(str::to_string).collect();
        validate("superadmin", &hosts, &routes, &[]).expect_err("refused").code
    };
    let hosts = vec!["example.org", "a.example.net", "b.example.net"];
    // malformed roots
    for root in ["site-a", "/Site-A", "/-a", "/a-", "/a/b", "/"] {
        assert_eq!(refused(vec![route_of("a.example.net", "/", Surface::Pages, Some(root))], hosts.clone()), "ADDRESSING_ROOT", "{root}");
    }
    // a root on any other route
    assert_eq!(
        refused(vec![route_of("a.example.net", "/tiles/", Surface::Ms, Some("/site-a"))], hosts.clone()),
        "ADDRESSING_ROOT"
    );
    // an mcp root on a host that is not that section
    assert_eq!(
        refused(vec![route_of("example.org", "/_mcp/", Surface::Mcp, Some("/site-a"))], hosts.clone()),
        "ADDRESSING_ROOT"
    );
    // aliases with differing routes
    assert_eq!(
        refused(
            vec![
                route_of("a.example.net", "/", Surface::Pages, Some("/site-a")),
                route_of("b.example.net", "/", Surface::Pages, Some("/site-a")),
                route_of("b.example.net", "/api/", Surface::Pages, None),
            ],
            hosts.clone()
        ),
        "ADDRESSING_ROOT"
    );
    // a shared route under a root
    assert_eq!(
        refused(
            vec![route_of("a.example.net", "/", Surface::Pages, Some("/site-a")), route_of("a.example.net", "/site-a/", Surface::Pages, None)],
            hosts.clone()
        ),
        "ADDRESSING_ROOT"
    );
    // the platform's own address
    assert_eq!(
        refused(vec![route_of("a.example.net", "/", Surface::Pages, Some("/site-a"))], vec!["a.example.net", "127.0.0.1"]),
        "ADDRESSING_ROOT"
    );
    // owner `fs`
    let code = validate("fs", &ok.hosts, &ok.routes, &[]).expect_err("fs").code;
    assert_eq!(code, "ADDRESSING_ROOT");
}

#[test]
fn registered_routes_that_a_section_cannot_reach_are_refused() {
    let t = table();
    assert!(check_routes(&t, &["/site-a/map".into(), "/site-a/{id}".into(), "/map".into(), "/api/data".into()], &["/site-a/tools".into()]).is_ok());
    assert_eq!(check_routes(&t, &["/site-a/_x".into()], &[]).unwrap_err().code, "PIPELINE_ROUTE_RESERVED");
    assert_eq!(check_routes(&t, &["/site-a/api/x".into()], &[]).unwrap_err().code, "PIPELINE_ROUTE_SHADOWED");
    assert_eq!(check_routes(&t, &["/site-a/_static/x".into()], &[]).unwrap_err().code, "PIPELINE_ROUTE_RESERVED");
}

#[test]
fn the_platform_address_is_loopback_on_any_port() {
    assert!(is_platform_host("localhost:20621"));
    assert!(is_platform_host("127.0.0.1"));
    assert!(is_platform_host("[::1]:10610"));
    assert!(!is_platform_host("stray.example"));
    assert!(!is_platform_host("a.example.net"));
}
