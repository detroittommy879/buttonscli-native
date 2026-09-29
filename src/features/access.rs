use std::collections::HashSet;

use super::catalog::{FeatureDefinition, FeatureKey, FeatureTier, Rollout};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Plan {
    Free,
    Pro,
    Enterprise,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GrantSource {
    Server,
    OfflineCache,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Entitlement {
    pub plan: Plan,
    pub source: GrantSource,
    pub expires_at_unix: u64,
}

pub trait EntitlementSource {
    fn entitlement(&self) -> Option<Entitlement>;
}

impl EntitlementSource for Option<Entitlement> {
    fn entitlement(&self) -> Option<Entitlement> {
        *self
    }
}

#[derive(Default)]
pub struct RuntimeAccess {
    pub pro_enabled: bool,
    pub all_free: bool,
    pub killed: HashSet<FeatureKey>,
    pub development_overrides: HashSet<FeatureKey>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Discoverability {
    Visible,
    Locked,
    Hidden,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessReason {
    Available,
    Unreleased,
    KillSwitch,
    ProDisabled,
    UpgradeRequired,
    ExpiredGrant,
    InternalOnly,
    DevelopmentOverride,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AccessDecision {
    pub available: bool,
    pub discoverability: Discoverability,
    pub reason: AccessReason,
}

fn denied(reason: AccessReason, visibility: Discoverability) -> AccessDecision {
    AccessDecision {
        available: false,
        discoverability: visibility,
        reason,
    }
}

fn allowed(reason: AccessReason) -> AccessDecision {
    AccessDecision {
        available: true,
        discoverability: Discoverability::Visible,
        reason,
    }
}

/// Resolve once at UI discovery and again at execution. Runtime kill switches always win.
pub fn resolve(
    key: FeatureKey,
    runtime: &RuntimeAccess,
    entitlements: &impl EntitlementSource,
    now_unix: u64,
) -> AccessDecision {
    resolve_feature(key.definition(), runtime, entitlements, now_unix)
}

fn resolve_feature(
    feature: FeatureDefinition,
    runtime: &RuntimeAccess,
    entitlements: &impl EntitlementSource,
    now_unix: u64,
) -> AccessDecision {
    if runtime.killed.contains(&feature.key) {
        return denied(AccessReason::KillSwitch, Discoverability::Hidden);
    }
    if runtime.development_overrides.contains(&feature.key) {
        return allowed(AccessReason::DevelopmentOverride);
    }
    if !feature.enabled || matches!(feature.rollout, Rollout::Planned | Rollout::Disabled) {
        return denied(AccessReason::Unreleased, Discoverability::Hidden);
    }
    match feature.tier {
        FeatureTier::Free => allowed(AccessReason::Available),
        FeatureTier::Internal | FeatureTier::Experimental => {
            denied(AccessReason::InternalOnly, Discoverability::Hidden)
        }
        FeatureTier::Pro | FeatureTier::Enterprise => {
            if !runtime.pro_enabled {
                return denied(AccessReason::ProDisabled, Discoverability::Locked);
            }
            if runtime.all_free {
                return allowed(AccessReason::Available);
            }
            match entitlements.entitlement() {
                None => denied(AccessReason::UpgradeRequired, Discoverability::Locked),
                Some(grant) if grant.expires_at_unix <= now_unix => {
                    denied(AccessReason::ExpiredGrant, Discoverability::Locked)
                }
                Some(grant)
                    if grant.plan == Plan::Enterprise
                        || (feature.tier == FeatureTier::Pro && grant.plan == Plan::Pro) =>
                {
                    allowed(AccessReason::Available)
                }
                Some(_) => denied(AccessReason::UpgradeRequired, Discoverability::Locked),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_and_internal_have_distinct_default_access() {
        let runtime = RuntimeAccess::default();
        assert!(resolve(FeatureKey::PaneLayout, &runtime, &None, 100).available);
        assert!(resolve(FeatureKey::KeyboardShortcuts, &runtime, &None, 100).available);
        assert!(resolve(FeatureKey::TerminalSearch, &runtime, &None, 100).available);
        assert!(resolve(FeatureKey::WorkspaceControls, &runtime, &None, 100).available);
        assert!(resolve(FeatureKey::LocalizationSettings, &runtime, &None, 100).available);
        assert!(resolve(FeatureKey::ShellProfiles, &runtime, &None, 100).available);
        assert!(resolve(FeatureKey::PersonalThemeEditor, &runtime, &None, 100).available);
        assert!(resolve(FeatureKey::CustomFonts, &runtime, &None, 100).available);
        assert!(resolve(FeatureKey::CoolStuffInstallers, &runtime, &None, 100).available);
        assert!(resolve(FeatureKey::WindowTransparency, &runtime, &None, 100).available);
        assert_eq!(
            resolve(FeatureKey::QuickSecrets, &runtime, &None, 100).discoverability,
            Discoverability::Hidden
        );
    }

    #[test]
    fn pro_requires_explicit_current_entitlement_and_rollout() {
        let mut runtime = RuntimeAccess {
            pro_enabled: true,
            ..Default::default()
        };
        let grant = Some(Entitlement {
            plan: Plan::Pro,
            source: GrantSource::OfflineCache,
            expires_at_unix: 110,
        });
        assert_eq!(
            resolve(FeatureKey::AiHelp, &runtime, &grant, 100).reason,
            AccessReason::Unreleased
        );
        runtime.development_overrides.insert(FeatureKey::AiHelp);
        assert_eq!(
            resolve(FeatureKey::AiHelp, &runtime, &None, 100).reason,
            AccessReason::DevelopmentOverride
        );
        runtime.development_overrides.clear();
        runtime.killed.insert(FeatureKey::AiHelp);
        runtime.development_overrides.insert(FeatureKey::AiHelp);
        assert_eq!(
            resolve(FeatureKey::AiHelp, &runtime, &grant, 100).reason,
            AccessReason::KillSwitch
        );
    }

    #[test]
    fn cached_grant_expires_and_free_plan_cannot_unlock_pro() {
        let runtime = RuntimeAccess {
            pro_enabled: true,
            ..Default::default()
        };
        let feature = FeatureDefinition {
            enabled: true,
            rollout: Rollout::Active,
            ..FeatureKey::AiHelp.definition()
        };
        let grant = |plan, expires_at_unix| {
            Some(Entitlement {
                plan,
                source: GrantSource::OfflineCache,
                expires_at_unix,
            })
        };
        assert_eq!(
            resolve_feature(feature, &runtime, &None, 100).reason,
            AccessReason::UpgradeRequired
        );
        assert_eq!(
            resolve_feature(feature, &runtime, &grant(Plan::Free, 110), 100).reason,
            AccessReason::UpgradeRequired
        );
        assert_eq!(
            resolve_feature(feature, &runtime, &grant(Plan::Pro, 100), 100).reason,
            AccessReason::ExpiredGrant
        );
        assert!(resolve_feature(feature, &runtime, &grant(Plan::Pro, 110), 100).available);
    }
}
