/// Returns true for characters allowed in an `https://` image URI
/// (RFC 3986 unreserved / gen-delims / sub-delims plus `%`, EXCEPT quotes:
/// both `"` and `'` are rejected so URIs cannot smuggle HTML/attribute
/// injection into NFT metadata that wallets and frontends render).
/// Whitespace and angle brackets (`<` `>`) are likewise excluded.
fn is_url_char(b: u8) -> bool {
    matches!(
        b,
        b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'~'
            | b':'
            | b'/'
            | b'?'
            | b'#'
            | b'['
            | b']'
            | b'@'
            | b'!'
            | b'$'
            | b'&'
            | b'('
            | b')'
            | b'*'
            | b'+'
            | b','
            | b';'
            | b'='
            | b'%'
    )
}

fn is_base58(b: u8) -> bool {
    matches!(
        b,
        b'1'..=b'9' | b'A'..=b'H' | b'J'..=b'N' | b'P'..=b'Z' | b'a'..=b'k' | b'm'..=b'z'
    )
}

fn is_base32_lower(b: u8) -> bool {
    matches!(b, b'a'..=b'z' | b'2'..=b'7')
}

/// CIDv0: 46-char multihash, `Qm` prefix, base58 alphabet.
fn is_cidv0(cid: &str) -> bool {
    cid.len() == 46 && cid.starts_with("Qm") && cid.bytes().all(is_base58)
}

/// CIDv1: multibase-prefixed (`b` = base32-lower, `z` = base58-btc),
/// 46+ chars, homogeneous alphabet for the detected base.
fn is_cidv1(cid: &str) -> bool {
    let bytes = cid.as_bytes();
    if cid.len() < 46 || cid.len() > 128 {
        return false;
    }
    match bytes[0] {
        b'b' => bytes[1..].iter().all(|b| is_base32_lower(*b)),
        b'z' => bytes[1..].iter().all(|b| is_base58(*b)),
        _ => false,
    }
}

fn is_cid(cid: &str) -> bool {
    is_cidv0(cid) || is_cidv1(cid)
}

pub fn image_uri_is_valid(uri_bytes: &[u8]) -> bool {
    let Ok(text) = core::str::from_utf8(uri_bytes) else {
        return false;
    };
    if let Some(authority) = text.strip_prefix("https://") {
        // Non-empty, URL-safe characters only: no whitespace, quotes or `<>`.
        return !authority.is_empty() && authority.bytes().all(is_url_char);
    }
    if let Some(rest) = text.strip_prefix("ipfs://") {
        // Bare CID, or CID followed by a URL-safe path (`cid/path/to.png`).
        let (cid, path) = match rest.find('/') {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, ""),
        };
        return is_cid(cid) && path.bytes().all(is_url_char);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::image_uri_is_valid;

    fn valid(uri: &str) -> bool {
        image_uri_is_valid(uri.as_bytes())
    }

    #[test]
    fn accepts_https_with_safe_chars() {
        assert!(valid("https://example.com/nft.png"));
        assert!(valid("https://example.com/a/b?c=d&e=f#g"));
        assert!(valid("https://example.com/nft%20img.png"));
    }

    #[test]
    fn rejects_https_injection_and_whitespace() {
        assert!(!valid("https://x\" onerror=\"alert(1)"));
        assert!(!valid("https://a b"));
        assert!(!valid("https://a<b>c"));
        assert!(!valid("https://example.com/'quoted'"));
        assert!(!valid("https://"));
        assert!(!valid("https://   "));
        assert!(!valid(""));
    }

    #[test]
    fn rejects_non_http_schemes() {
        assert!(!valid("http://example.com/nft.png"));
        assert!(!valid("javascript:alert(1)"));
        assert!(!valid("data:text/html,<h1>x</h1>"));
    }

    #[test]
    fn accepts_valid_cids() {
        assert!(valid("ipfs://QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz6v9T8UoTb3"));
        assert!(valid(
            "ipfs://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi"
        ));
    }

    #[test]
    fn rejects_malformed_cids() {
        assert!(!valid("ipfs://reward"));
        assert!(!valid("ipfs://QmExample123"));
        assert!(!valid("ipfs://1"));
        assert!(!valid("ipfs://"));
        // CIDv0 with an ambiguous base58 char (`0` is not in the alphabet).
        assert!(!valid("ipfs://QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz6v9T8UoTb0"));
        // Uppercase breaks base32-lower CIDv1 homogeneity.
        assert!(!valid(
            "ipfs://bafYbeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi"
        ));
    }
}
