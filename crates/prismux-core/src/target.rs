use crate::{AccountStatus, ConfigProfile, PrismuxError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    Account,
    Profile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetResolution {
    pub kind: TargetKind,
    pub target_id: String,
}

#[derive(Debug, Clone)]
pub struct TargetCatalog {
    pub accounts: Vec<AccountStatus>,
    pub profiles: Vec<ConfigProfile>,
    candidates: Vec<TargetCandidate>,
}

#[derive(Debug, Clone)]
struct TargetCandidate {
    display_index: u32,
    kind: TargetKind,
    target_id: String,
    name: Option<String>,
}

impl TargetCatalog {
    pub fn new(accounts: Vec<AccountStatus>, profiles: Vec<ConfigProfile>) -> Self {
        let mut next_index = 1_u32;
        let mut candidates = Vec::with_capacity(accounts.len() + profiles.len());

        for status in &accounts {
            candidates.push(TargetCandidate {
                display_index: next_index,
                kind: TargetKind::Account,
                target_id: status.account.local_id.clone(),
                name: status.account.alias.clone(),
            });
            next_index += 1;
        }

        for profile in &profiles {
            candidates.push(TargetCandidate {
                display_index: next_index,
                kind: TargetKind::Profile,
                target_id: profile.local_id.clone(),
                name: Some(profile.name.clone()),
            });
            next_index += 1;
        }

        Self {
            accounts,
            profiles,
            candidates,
        }
    }

    pub fn resolve(&self, platform_id: &str, selector: &str) -> Result<TargetResolution> {
        let mut matches: Vec<&TargetCandidate> = Vec::new();
        if let Ok(number) = selector.parse::<u32>() {
            matches.extend(
                self.candidates
                    .iter()
                    .filter(|candidate| candidate.display_index == number),
            );
        }
        for candidate in self
            .candidates
            .iter()
            .filter(|candidate| candidate.name.as_deref() == Some(selector))
        {
            if !matches.iter().any(|matched| {
                matched.kind == candidate.kind && matched.target_id == candidate.target_id
            }) {
                matches.push(candidate);
            }
        }

        match matches.as_slice() {
            [candidate] => Ok(TargetResolution {
                kind: candidate.kind,
                target_id: candidate.target_id.clone(),
            }),
            [] => Err(PrismuxError::Message(format!(
                "`{selector}` did not match any account or profile for `{platform_id}`"
            ))),
            candidates => Err(PrismuxError::Message(format!(
                "`{selector}` is ambiguous for `{platform_id}`: matched {} target(s). Use a unique alias/profile name.",
                candidates.len()
            ))),
        }
    }

    pub fn account_display_index(&self, account_index: usize) -> u32 {
        account_index as u32 + 1
    }

    pub fn profile_display_index(&self, profile_index: usize) -> u32 {
        self.accounts.len() as u32 + profile_index as u32 + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AccountRef, Availability, AvailabilityState, platform_info};

    #[test]
    fn resolves_profile_by_display_index_after_accounts() {
        let catalog = TargetCatalog::new(
            vec![
                account_status(1, Some("work")),
                account_status(2, Some("personal")),
            ],
            vec![profile(None, "gateway")],
        );

        assert_eq!(
            catalog.resolve("codex", "3").unwrap(),
            TargetResolution {
                kind: TargetKind::Profile,
                target_id: "profile-gateway".to_string(),
            }
        );
    }

    #[test]
    fn rejects_ambiguous_alias_and_profile_name() {
        let catalog = TargetCatalog::new(
            vec![account_status(1, Some("work"))],
            vec![profile(Some(1), "work")],
        );

        let err = catalog.resolve("claude", "work").unwrap_err();

        assert!(err.to_string().contains("ambiguous"));
    }

    #[test]
    fn rejects_ambiguous_display_index_and_numeric_name() {
        let catalog = TargetCatalog::new(
            vec![account_status(1, Some("work"))],
            vec![profile(None, "1")],
        );

        let err = catalog.resolve("codex", "1").unwrap_err();

        assert!(err.to_string().contains("ambiguous"));
    }

    fn account_status(number: u32, alias: Option<&str>) -> AccountStatus {
        AccountStatus {
            account: AccountRef {
                platform: "test".to_string(),
                local_id: format!("account-{number}"),
                number,
                alias: alias.map(str::to_string),
            },
            active: false,
            account_label: None,
            plan_label: None,
            auth_type: None,
            expires_at_unix: None,
            availability: Availability {
                state: AvailabilityState::Unknown,
                display: "unknown".to_string(),
            },
            usage: None,
        }
    }

    fn profile(number: Option<u32>, name: &str) -> ConfigProfile {
        ConfigProfile {
            platform: platform_info("test", "Test"),
            local_id: format!("profile-{name}"),
            name: name.to_string(),
            active: false,
            config_path: format!("{name}.config.toml"),
            provider_id: None,
            base_url: None,
            model: None,
            number,
            auth_type: None,
        }
    }
}
