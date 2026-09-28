#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum FeatureKey {
    PaneLayout,
    TabNaming,
    TabWrapping,
    ThemeSelection,
    TerminalScrollbar,
    PaneDivider,
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
}

impl FeatureKey {
    pub const ALL: [Self; 19] = [
        Self::PaneLayout,
        Self::TabNaming,
        Self::TabWrapping,
        Self::ThemeSelection,
        Self::TerminalScrollbar,
        Self::PaneDivider,
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
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PaneLayout => "paneLayout",
            Self::TabNaming => "tabNaming",
            Self::TabWrapping => "tabWrapping",
            Self::ThemeSelection => "themeSelection",
            Self::TerminalScrollbar => "terminalScrollbar",
            Self::PaneDivider => "paneDivider",
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
            K::ThemeSelection | K::CalmThemeApply => (T::Free, true, R::Active, "themes"),
            K::SettingsAppearance | K::OriginalSettingsImport => {
                (T::Free, true, R::Active, "settings")
            }
            K::GuidedOnboarding => (T::Free, false, R::Planned, "activation"),
            K::EffectsMasterSwitch => (T::Free, true, R::Active, "effects"),
            K::WindowTransparency => (T::Free, false, R::Planned, "appearance"),
            K::ShaderLab => (T::Free, false, R::Planned, "effects"),
            K::AiHelp => (T::Pro, false, R::Planned, "assistant"),
            K::AutomationRemoteControl => (T::Pro, false, R::Planned, "automation"),
            K::VibeCodeThemes => (T::Pro, false, R::Planned, "themes"),
            K::VibeCodeShaders => (T::Pro, false, R::Planned, "effects"),
            K::QuickSecrets => (T::Internal, false, R::Disabled, "security"),
            K::ProfileManagement => (T::Internal, false, R::Disabled, "settings"),
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
