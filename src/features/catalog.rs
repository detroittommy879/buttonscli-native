#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum FeatureKey {
    PaneLayout,
    TabNaming,
    TabWrapping,
    ThemeSelection,
    TerminalScrollbar,
    PaneDivider,
    KeyboardShortcuts,
    TerminalSearch,
    WorkspaceControls,
    LocalizationSettings,
    ShellProfiles,
    PersonalThemeEditor,
    CustomFonts,
    CoolStuffInstallers,
    SettingsAppearance,
    OriginalSettingsImport,
    GuidedOnboarding,
    EffectsMasterSwitch,
    CalmThemeApply,
    WindowTransparency,
    ShaderLab,
    AiHelp,
    AutomationRemoteControl,
    VibeCodeThemes,
    VibeCodeShaders,
    QuickSecrets,
    ProfileManagement,
    AccountSignIn,
    ReadOnlyGuides,
    UserFeedback,
}

impl FeatureKey {
    pub const ALL: [Self; 30] = [
        Self::PaneLayout,
        Self::TabNaming,
        Self::TabWrapping,
        Self::ThemeSelection,
        Self::TerminalScrollbar,
        Self::PaneDivider,
        Self::KeyboardShortcuts,
        Self::TerminalSearch,
        Self::WorkspaceControls,
        Self::LocalizationSettings,
        Self::ShellProfiles,
        Self::PersonalThemeEditor,
        Self::CustomFonts,
        Self::CoolStuffInstallers,
        Self::SettingsAppearance,
        Self::OriginalSettingsImport,
        Self::GuidedOnboarding,
        Self::EffectsMasterSwitch,
        Self::CalmThemeApply,
        Self::WindowTransparency,
        Self::ShaderLab,
        Self::AiHelp,
        Self::AutomationRemoteControl,
        Self::VibeCodeThemes,
        Self::VibeCodeShaders,
        Self::QuickSecrets,
        Self::ProfileManagement,
        Self::AccountSignIn,
        Self::ReadOnlyGuides,
        Self::UserFeedback,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PaneLayout => "paneLayout",
            Self::TabNaming => "tabNaming",
            Self::TabWrapping => "tabWrapping",
            Self::ThemeSelection => "themeSelection",
            Self::TerminalScrollbar => "terminalScrollbar",
            Self::PaneDivider => "paneDivider",
            Self::KeyboardShortcuts => "keyboardShortcuts",
            Self::TerminalSearch => "terminalSearch",
            Self::WorkspaceControls => "workspaceControls",
            Self::LocalizationSettings => "localizationSettings",
            Self::ShellProfiles => "shellProfiles",
            Self::PersonalThemeEditor => "personalThemeEditor",
            Self::CustomFonts => "customFonts",
            Self::CoolStuffInstallers => "coolStuffInstallers",
            Self::SettingsAppearance => "settingsAppearance",
            Self::OriginalSettingsImport => "originalSettingsImport",
            Self::GuidedOnboarding => "guidedOnboarding",
            Self::EffectsMasterSwitch => "effectsMasterSwitch",
            Self::CalmThemeApply => "calmThemeApply",
            Self::WindowTransparency => "windowTransparency",
            Self::ShaderLab => "shaderLab",
            Self::AiHelp => "aiHelp",
            Self::AutomationRemoteControl => "automationRemoteControl",
            Self::VibeCodeThemes => "vibeCodeThemes",
            Self::VibeCodeShaders => "vibeCodeShaders",
            Self::QuickSecrets => "quickSecrets",
            Self::ProfileManagement => "profileManagement",
            Self::AccountSignIn => "accountSignIn",
            Self::ReadOnlyGuides => "readOnlyGuides",
            Self::UserFeedback => "userFeedback",
        }
    }

    pub const fn definition(self) -> FeatureDefinition {
        use FeatureKey as K;
        use FeatureTier as T;
        use Rollout as R;
        let (tier, enabled, rollout, owner) = match self {
            K::PaneLayout
            | K::TabNaming
            | K::TabWrapping
            | K::TerminalScrollbar
            | K::PaneDivider => (T::Free, true, R::Active, "layout"),
            K::KeyboardShortcuts => (T::Free, true, R::Active, "shortcuts"),
            K::TerminalSearch => (T::Free, true, R::Active, "terminal"),
            K::WorkspaceControls => (T::Free, true, R::Active, "workspace"),
            K::LocalizationSettings => (T::Free, true, R::Active, "localization"),
            K::ShellProfiles => (T::Free, true, R::Active, "terminal"),
            K::PersonalThemeEditor => (T::Free, true, R::Active, "themes"),
            K::CustomFonts => (T::Free, true, R::Active, "fonts"),
            K::CoolStuffInstallers => (T::Free, true, R::Active, "onboarding"),
            K::ThemeSelection | K::CalmThemeApply => (T::Free, true, R::Active, "themes"),
            K::SettingsAppearance | K::OriginalSettingsImport => {
                (T::Free, true, R::Active, "settings")
            }
            K::GuidedOnboarding => (T::Free, false, R::Planned, "activation"),
            K::EffectsMasterSwitch => (T::Free, true, R::Active, "effects"),
            K::WindowTransparency => (T::Free, true, R::Active, "appearance"),
            K::ShaderLab => (T::Free, false, R::Planned, "effects"),
            K::AiHelp => (T::Pro, false, R::Planned, "assistant"),
            K::AutomationRemoteControl => (T::Pro, false, R::Planned, "automation"),
            K::VibeCodeThemes => (T::Pro, false, R::Planned, "themes"),
            K::VibeCodeShaders => (T::Pro, false, R::Planned, "effects"),
            K::QuickSecrets => (T::Internal, false, R::Disabled, "security"),
            K::ProfileManagement => (T::Internal, false, R::Disabled, "settings"),
            K::AccountSignIn => (T::Free, true, R::Active, "account"),
            K::ReadOnlyGuides => (T::Free, true, R::Active, "help"),
            K::UserFeedback => (T::Free, true, R::Active, "privacy"),
        };
        FeatureDefinition {
            key: self,
            tier,
            enabled,
            rollout,
            owner,
            runtime_flag: matches!(tier, T::Pro),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FeatureTier {
    Free,
    Pro,
    Enterprise,
    Internal,
    Experimental,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rollout {
    Active,
    Beta,
    Planned,
    Disabled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FeatureDefinition {
    pub key: FeatureKey,
    pub tier: FeatureTier,
    pub enabled: bool,
    pub rollout: Rollout,
    pub owner: &'static str,
    pub runtime_flag: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_only_guides_are_registered_as_active_free_access() {
        assert!(FeatureKey::ALL.contains(&FeatureKey::ReadOnlyGuides));
        let definition = FeatureKey::ReadOnlyGuides.definition();
        assert_eq!(definition.key.as_str(), "readOnlyGuides");
        assert_eq!(definition.tier, FeatureTier::Free);
        assert_eq!(definition.rollout, Rollout::Active);
        assert!(!definition.runtime_flag);
    }

    #[test]
    fn explicit_user_feedback_is_registered_as_active_free_access() {
        assert!(FeatureKey::ALL.contains(&FeatureKey::UserFeedback));
        let definition = FeatureKey::UserFeedback.definition();
        assert_eq!(definition.key.as_str(), "userFeedback");
        assert_eq!(definition.tier, FeatureTier::Free);
        assert_eq!(definition.rollout, Rollout::Active);
        assert!(!definition.runtime_flag);
    }
}
