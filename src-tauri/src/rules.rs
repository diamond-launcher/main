//! Evaluation of Mojang's `rules` blocks (libraries and arguments).
use crate::meta::Rule;
use crate::platform;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct Env {
    pub os: &'static str,
    pub arch: &'static str,
    pub os_version: String,
}

impl Env {
    pub fn current() -> Self {
        Self {
            os: platform::os_name(),
            arch: platform::arch(),
            // Java reports "10.0" on both Windows 10 and 11.
            os_version: if cfg!(windows) { "10.0".into() } else { String::new() },
        }
    }
}

fn rule_matches(rule: &Rule, env: &Env) -> bool {
    if let Some(os) = &rule.os {
        if os.name.as_deref().is_some_and(|n| n != env.os) {
            return false;
        }
        if os.arch.as_deref().is_some_and(|a| a != env.arch) {
            return false;
        }
        if let Some(pattern) = &os.version {
            match Regex::new(pattern) {
                Ok(re) if re.is_match(&env.os_version) => {}
                _ => return false,
            }
        }
    }
    // The launcher advertises no optional features (demo mode, custom resolution, quick play).
    if let Some(features) = &rule.features {
        if features.values().any(|&wanted| wanted) {
            return false;
        }
    }
    true
}

/// No rules means allowed; otherwise the last matching rule wins and the default is deny.
pub fn rules_allow(rules: &[Rule], env: &Env) -> bool {
    if rules.is_empty() {
        return true;
    }
    let mut allowed = false;
    for rule in rules {
        if rule_matches(rule, env) {
            allowed = rule.action == "allow";
        }
    }
    allowed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env() -> Env {
        Env { os: "windows", arch: "x86_64", os_version: "10.0".into() }
    }

    fn rules(json: &str) -> Vec<Rule> {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn empty_rules_allow() {
        assert!(rules_allow(&[], &env()));
    }

    #[test]
    fn os_specific_allow_and_disallow() {
        assert!(rules_allow(&rules(r#"[{"action":"allow","os":{"name":"windows"}}]"#), &env()));
        assert!(!rules_allow(&rules(r#"[{"action":"allow","os":{"name":"osx"}}]"#), &env()));
        let r = rules(r#"[{"action":"allow"},{"action":"disallow","os":{"name":"windows"}}]"#);
        assert!(!rules_allow(&r, &env()));
    }

    #[test]
    fn arch_and_version_rules() {
        assert!(!rules_allow(&rules(r#"[{"action":"allow","os":{"arch":"x86"}}]"#), &env()));
        assert!(rules_allow(&rules(r#"[{"action":"allow","os":{"name":"windows","version":"^10\\."}}]"#), &env()));
    }

    #[test]
    fn features_are_not_enabled() {
        assert!(!rules_allow(&rules(r#"[{"action":"allow","features":{"is_demo_user":true}}]"#), &env()));
        assert!(!rules_allow(&rules(r#"[{"action":"allow","features":{"has_custom_resolution":true}}]"#), &env()));
    }
}
