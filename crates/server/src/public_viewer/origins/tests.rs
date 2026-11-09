use super::*;
use axum::http::HeaderValue;

const FIRST: &str = "https://viewer.example.invalid";
const SECOND: &str = "https://alternate.example.invalid";

fn allowed() -> AllowedOrigins {
    AllowedOrigins::new(FIRST)
        .unwrap()
        .additional(SECOND)
        .unwrap()
}

#[test]
fn validates_exact_https_origins_and_bounds_the_list() {
    for origin in [
        "",
        " ",
        "http://alternate.example.invalid",
        "https://*.site",
        "https://*",
        "https://user@alternate.example.invalid",
        "https://user:pass@alternate.example.invalid",
        "https://alternate.example.invalid/path",
        "https://alternate.example.invalid?x=1",
        "https://alternate.example.invalid#x",
        "https://alternate.example.invalid:65536",
        "https://alternate.example.invalid:",
        "https://alternate.example.invalid:0",
        "https://alternate.example.invalid:bad",
        "https://.site",
        "https://bad_host.site",
        "https://-bad.site",
    ] {
        assert!(AllowedOrigins::new(origin).is_err(), "{origin}");
    }
    for csv in [
        "",
        ",",
        "https://alternate.example.invalid,",
        ",https://alternate.example.invalid",
    ] {
        assert!(AllowedOrigins::new(FIRST).unwrap().additional(csv).is_err());
    }
    let dedup = allowed()
        .additional(&format!(" {FIRST}/, {SECOND} "))
        .unwrap();
    assert_eq!(dedup.0.len(), 2);
    let nine = (0..9)
        .map(|i| format!("https://host{i}.site"))
        .collect::<Vec<_>>()
        .join(",");
    assert!(AllowedOrigins::new(FIRST)
        .unwrap()
        .additional(&nine)
        .is_err());
    assert!(allowed().additional(&" ".repeat(4097)).is_err());
    assert!(AllowedOrigins::new("https://localhost:443").is_ok());
    assert!(AllowedOrigins::new("https://[::1]:443").is_ok());
}

#[test]
fn host_selects_origin_without_accepting_cross_pairs_or_ambiguous_headers() {
    let allowed = allowed();
    let mut headers = HeaderMap::new();
    for (host, origin) in [
        ("viewer.example.invalid", FIRST),
        ("alternate.example.invalid", SECOND),
    ] {
        headers.insert(header::HOST, HeaderValue::from_str(host).unwrap());
        assert_eq!(allowed.request(&headers, false).unwrap().0, origin);
        assert_eq!(
            allowed.request(&headers, true).err(),
            Some("ORIGIN_REJECTED")
        );
        headers.insert(header::ORIGIN, HeaderValue::from_str(origin).unwrap());
        assert_eq!(allowed.request(&headers, true).unwrap().0, origin);
        headers.remove(header::ORIGIN);
    }
    for origin in [
        FIRST,
        "https://alternate.example.invalid:443",
        "null",
        "https://alternate.example.invalid/",
    ] {
        headers.insert(header::ORIGIN, HeaderValue::from_str(origin).unwrap());
        assert_eq!(
            allowed.request(&headers, false).err(),
            Some("ORIGIN_REJECTED")
        );
    }
    headers.insert(header::ORIGIN, HeaderValue::from_static(SECOND));
    headers.append(header::ORIGIN, HeaderValue::from_static(SECOND));
    assert_eq!(
        allowed.request(&headers, false).err(),
        Some("ORIGIN_REJECTED")
    );
    headers.remove(header::ORIGIN);
    headers.append(
        header::HOST,
        HeaderValue::from_static("alternate.example.invalid"),
    );
    assert_eq!(
        allowed.request(&headers, false).err(),
        Some("HOST_REJECTED")
    );
    for host in [
        "evil.site",
        "alternate.example.invalid:443",
        "alternate.example.invalid,evil.site",
        "",
    ] {
        headers.insert(header::HOST, HeaderValue::from_str(host).unwrap());
        assert_eq!(
            allowed.request(&headers, false).err(),
            Some("HOST_REJECTED")
        );
    }
}
