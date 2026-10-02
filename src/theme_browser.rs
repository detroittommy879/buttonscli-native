use serde::{Deserialize, Serialize};

use crate::theme::ThemeDefinition;

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ThemeSort {
    #[default]
    Name,
    NameDescending,
    FavoritesFirst,
    NativeVersion,
}

impl ThemeSort {
    pub(crate) const ALL: [Self; 4] = [
        Self::Name,
        Self::NameDescending,
        Self::FavoritesFirst,
        Self::NativeVersion,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Name => "Name A–Z",
            Self::NameDescending => "Name Z–A",
            Self::FavoritesFirst => "Favorites first",
            Self::NativeVersion => "Native version (newest first)",
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ThemeCollection {
    #[default]
    All,
    Native,
    Legacy,
}

impl ThemeCollection {
    pub(crate) const ALL: [Self; 3] = [Self::All, Self::Native, Self::Legacy];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::All => "All collections",
            Self::Native => "Native themes",
            Self::Legacy => "Legacy themes",
        }
    }
}

pub(crate) fn matches(
    themes: &[ThemeDefinition],
    search: &str,
    collection: ThemeCollection,
    sort: ThemeSort,
    favorites: &[String],
) -> Vec<usize> {
    let needle = search.trim().to_lowercase();
    let mut result: Vec<_> = themes
        .iter()
        .enumerate()
        .filter(|(_, theme)| {
            let native = theme.native_version.is_some();
            (collection == ThemeCollection::All
                || native == (collection == ThemeCollection::Native))
                && (needle.is_empty()
                    || [&theme.name, &theme.id, &theme.description]
                        .iter()
                        .any(|text| text.to_lowercase().contains(&needle)))
        })
        .map(|(index, _)| index)
        .collect();
    result.sort_by(|&a, &b| {
        let a = &themes[a];
        let b = &themes[b];
        let name_order = || {
            a.name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then_with(|| a.id.cmp(&b.id))
        };
        match sort {
            ThemeSort::Name => name_order(),
            ThemeSort::NameDescending => name_order().reverse(),
            ThemeSort::FavoritesFirst => favorites
                .contains(&b.id)
                .cmp(&favorites.contains(&a.id))
                .then_with(name_order),
            ThemeSort::NativeVersion => b
                .native_version
                .cmp(&a.native_version)
                .then_with(name_order),
        }
    });
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeCatalog;

    #[test]
    fn collection_filter_sort_and_search_preserve_theme_identity() {
        let catalog = ThemeCatalog::load();
        let mut native = catalog.get("aurora").clone();
        native.id = "personal:next".into();
        native.name = "Z new native".into();
        native.native_version = Some(2);
        let themes = vec![
            catalog.get("basic2").clone(),
            native,
            catalog.get("aurora").clone(),
        ];
        assert_eq!(
            matches(&themes, "", ThemeCollection::Legacy, ThemeSort::Name, &[]),
            vec![0]
        );
        assert_eq!(
            matches(
                &themes,
                "",
                ThemeCollection::Native,
                ThemeSort::NativeVersion,
                &[]
            ),
            vec![1, 2]
        );
        assert_eq!(
            matches(&themes, "NEXT", ThemeCollection::All, ThemeSort::Name, &[]),
            vec![1]
        );
        let sorted = matches(
            &themes,
            "",
            ThemeCollection::All,
            ThemeSort::FavoritesFirst,
            &[themes[1].id.clone()],
        );
        assert_eq!(sorted[0], 1);
        let ascending = matches(&themes, "", ThemeCollection::All, ThemeSort::Name, &[]);
        let descending = matches(
            &themes,
            "",
            ThemeCollection::All,
            ThemeSort::NameDescending,
            &[],
        );
        assert_eq!(ascending.into_iter().rev().collect::<Vec<_>>(), descending);
    }
}
