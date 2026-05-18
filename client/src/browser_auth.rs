use reqwest::cookie::Jar;
use rookie::common::enums::Cookie;
use url::Url;

pub fn extract_cookies() -> crate::error::Result<Vec<Cookie>> {
    let cookies = rookie::firefox(Some(vec![
        "investors.com".to_string(),
        ".investors.com".to_string(),
    ]))
    .unwrap_or_default();

    if cookies.is_empty() {
        return Err(crate::error::ClientError::Status {
            status: 401,
            body: "no cookies found for investors.com".to_string(),
        });
    }

    Ok(cookies)
}

pub fn build_cookie_jar(cookies: &[Cookie]) -> crate::error::Result<Jar> {
    let jar = Jar::default();
    let url =
        Url::parse("https://www.investors.com").map_err(|_| crate::error::ClientError::Status {
            status: 401,
            body: "no cookies found for investors.com".to_string(),
        })?;

    for cookie in cookies {
        let cookie_str = format!("{}={}", cookie.name, cookie.value);
        jar.add_cookie_str(&cookie_str, &url);
    }

    Ok(jar)
}

#[cfg(test)]
mod tests {
    use super::{build_cookie_jar, extract_cookies};
    use rookie::common::enums::Cookie;

    #[test]
    fn extract_cookies_signature_compiles() {
        let _fn_ptr: fn() -> crate::error::Result<Vec<Cookie>> = extract_cookies;
    }

    #[test]
    fn build_cookie_jar_signature_compiles() {
        let _fn_ptr: fn(&[Cookie]) -> crate::error::Result<reqwest::cookie::Jar> = build_cookie_jar;
    }

    #[test]
    #[ignore]
    fn extract_cookies_works_with_live_firefox_session() {
        let cookies = extract_cookies().expect("expected Firefox cookies for investors.com");
        let jar = build_cookie_jar(&cookies).expect("expected cookie jar to build");
        let _ = jar;
    }
}
