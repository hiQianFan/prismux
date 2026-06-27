pub mod redaction {
    pub fn redact(message: &str) -> String {
        let lower = message.to_ascii_lowercase();
        let sensitive = [
            "access_token",
            "refresh_token",
            "api_key",
            "authorization:",
            "cookie:",
            "bearer ",
            "auth payload",
            "raw response",
            "raw log",
            "sk-",
            "@",
        ];
        if sensitive.iter().any(|marker| lower.contains(marker)) {
            "[redacted sensitive diagnostic]".to_string()
        } else {
            message.to_string()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn redacts_common_secret_markers() {
            for value in [
                "access_token=abc",
                "Cookie: session=abc",
                "Authorization: Bearer abc",
                "api_key=abc",
                "person@example.com",
                "raw auth payload",
            ] {
                assert_eq!(redact(value), "[redacted sensitive diagnostic]");
            }
        }
    }
}
