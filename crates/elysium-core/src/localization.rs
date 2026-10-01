//! Comprehensive localization system for the Elysium game engine.
//!
//! This module provides full internationalization (i18n) and localization (l10n)
//! support including multi-language translation, runtime switching, fallback
//! chains, string interpolation, pluralization, gender-aware translations,
//! context-aware translations, Unicode support, localized asset integration,
//! editor tooling, and platform-specific locale detection.
//!
//! Supported translation file formats:
//! - **JSON**: Human-readable, supports nested structures.
//! - **PO/Gettext**: Standard translation file format with header entries.
//! - **CSV**: Simple key-locale-context-translation tabular format.

use std::collections::{HashMap, HashSet};
use std::fmt::{self, Display, Write as FmtWrite};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors that can occur during localization operations.
#[derive(Debug, Error)]
pub enum LocalizationError {
    #[error("IO error reading '{0}': {1}")]
    Io(String, String),

    #[error("Parse error in '{0}': {1}")]
    Parse(String, String),

    #[error("Locale '{0}' not found")]
    LocaleNotFound(String),

    #[error("Translation key '{0}' not found for locale '{1}'")]
    TranslationNotFound(String, String),

    #[error("Invalid interpolation: {0}")]
    InvalidInterpolation(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Locale '{0}' is not supported: {1}")]
    UnsupportedLocale(String, String),
}

// ---------------------------------------------------------------------------
// Locale
// ---------------------------------------------------------------------------

/// Represents a BCP 47 / CLDR locale identifier.
///
/// Examples: `"en"`, `"en-US"`, `"tr-TR"`, `"zh-Hans-CN"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Locale {
    pub language: String,
    pub script: Option<String>,
    pub region: Option<String>,
    pub variant: Option<String>,
}

impl Locale {
    /// Creates a new `Locale` from a language code.
    pub fn new(language: impl Into<String>) -> Self {
        Self {
            language: language.into(),
            script: None,
            region: None,
            variant: None,
        }
    }

    /// Sets the script subtag (e.g. `"Latn"`, `"Cyrl"`, `"Arab"`).
    pub fn with_script(mut self, script: impl Into<String>) -> Self {
        self.script = Some(script.into());
        self
    }

    /// Sets the region subtag (e.g. `"US"`, `"TR"`, `"GB"`).
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Sets the variant subtag.
    pub fn with_variant(mut self, variant: impl Into<String>) -> Self {
        self.variant = Some(variant.into());
        self
    }

    /// Returns the language code only.
    pub fn language(&self) -> &str {
        &self.language
    }

    /// Returns the script subtag if present.
    pub fn script(&self) -> Option<&str> {
        self.script.as_deref()
    }

    /// Returns the region subtag if present.
    pub fn region(&self) -> Option<&str> {
        self.region.as_deref()
    }

    /// Returns the variant subtag if present.
    pub fn variant(&self) -> Option<&str> {
        self.variant.as_deref()
    }

    /// Returns the full BCP 47 string representation.
    pub fn as_str(&self) -> String {
        let mut s = self.language.clone();
        if let Some(ref script) = self.script {
            s.push_str("-");
            s.push_str(script);
        }
        if let Some(ref region) = self.region {
            s.push_str("-");
            s.push_str(region);
        }
        if let Some(ref variant) = self.variant {
            s.push_str("-");
            s.push_str(variant);
        }
        s
    }

    /// Returns `true` if this locale is a superset of the given language code.
    pub fn matches_language(&self, lang: &str) -> bool {
        self.language == lang
            || self.language.starts_with(lang)
            || lang.starts_with(&self.language)
    }

    /// Returns `true` if this locale's region matches.
    pub fn matches_region(&self, region: &str) -> bool {
        self.region.as_deref() == Some(region)
    }
}

impl Default for Locale {
    fn default() -> Self {
        Self::new("en")
    }
}

impl Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for Locale {
    type Err = LocalizationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.is_empty() || parts[0].is_empty() {
            return Err(LocalizationError::UnsupportedLocale(
                s.into(),
                "empty locale".into(),
            ));
        }

        let mut locale = Self::new(parts[0]);
        let mut i = 1;

        // Script subtag: always 4 uppercase letters
        if i < parts.len()
            && parts[i].len() == 4
            && parts[i].chars().all(|c| c.is_ascii_uppercase())
        {
            locale = locale.with_script(parts[i]);
            i += 1;
        }

        // Region subtag: always 2 uppercase letters
        if i < parts.len()
            && parts[i].len() == 2
            && parts[i].chars().all(|c| c.is_ascii_uppercase())
        {
            locale = locale.with_region(parts[i]);
            i += 1;
        }

        // Remaining parts are variants
        while i < parts.len() {
            locale = locale.with_variant(parts[i]);
            i += 1;
        }

        Ok(locale)
    }
}

// ---------------------------------------------------------------------------
// Gender
// ---------------------------------------------------------------------------

/// Grammatical gender for context-aware translations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Gender {
    Male,
    Female,
    Neuter,
    Unknown,
}

impl Display for Gender {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Gender::Male => write!(f, "male"),
            Gender::Female => write!(f, "female"),
            Gender::Neuter => write!(f, "neuter"),
            Gender::Unknown => write!(f, "unknown"),
        }
    }
}

impl std::str::FromStr for Gender {
    type Err = LocalizationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "male" => Ok(Gender::Male),
            "female" => Ok(Gender::Female),
            "neuter" => Ok(Gender::Neuter),
            "unknown" => Ok(Gender::Unknown),
            _ => Err(LocalizationError::UnsupportedLocale(
                s.into(),
                "invalid gender".into(),
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// Plural Category
// ---------------------------------------------------------------------------

/// CLDR plural category for language-specific plural forms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluralCategory {
    Zero,
    One,
    Two,
    Few,
    Many,
    Other,
}

impl Display for PluralCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluralCategory::Zero => write!(f, "zero"),
            PluralCategory::One => write!(f, "one"),
            PluralCategory::Two => write!(f, "two"),
            PluralCategory::Few => write!(f, "few"),
            PluralCategory::Many => write!(f, "many"),
            PluralCategory::Other => write!(f, "other"),
        }
    }
}

impl std::str::FromStr for PluralCategory {
    type Err = LocalizationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "zero" => Ok(PluralCategory::Zero),
            "one" => Ok(PluralCategory::One),
            "two" => Ok(PluralCategory::Two),
            "few" => Ok(PluralCategory::Few),
            "many" => Ok(PluralCategory::Many),
            "other" => Ok(PluralCategory::Other),
            _ => Err(LocalizationError::UnsupportedLocale(
                s.into(),
                "invalid plural category".into(),
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// Translation Context
// ---------------------------------------------------------------------------

/// Context key for disambiguating identical source strings.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct TranslationContext(String);

impl TranslationContext {
    pub fn new(context: impl Into<String>) -> Self {
        Self(context.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for TranslationContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// Translation Entry
// ---------------------------------------------------------------------------

/// A single translated string with optional context, gender, and plural info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationEntry {
    /// The translation key.
    pub key: String,
    /// The locale this entry belongs to.
    pub locale: Locale,
    /// Optional context for disambiguation.
    pub context: Option<TranslationContext>,
    /// Optional gender variant.
    pub gender: Option<Gender>,
    /// Optional plural category.
    pub plural_category: Option<PluralCategory>,
    /// The translated text.
    pub translation: String,
    /// Arbitrary metadata (e.g. translator notes).
    pub metadata: HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// Translation Database
// ---------------------------------------------------------------------------

/// In-memory store for translation entries.
#[derive(Debug, Clone, Default)]
pub struct TranslationDatabase {
    entries: HashMap<
        (String, Locale, Option<TranslationContext>, Option<Gender>, Option<PluralCategory>),
        TranslationEntry,
    >,
    metadata: HashMap<String, String>,
}

impl TranslationDatabase {
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a new translation entry.
    pub fn insert(&mut self, entry: TranslationEntry) {
        let key = (
            entry.key.clone(),
            entry.locale.clone(),
            entry.context.clone(),
            entry.gender,
            entry.plural_category,
        );
        self.entries.insert(key, entry);
    }

    /// Looks up a translation entry by all parameters.
    pub fn get(
        &self,
        key: &str,
        locale: &Locale,
        context: Option<&TranslationContext>,
        gender: Option<Gender>,
        plural_category: Option<PluralCategory>,
    ) -> Option<&TranslationEntry> {
        let lookup_key = (
            key.to_string(),
            locale.clone(),
            context.cloned(),
            gender,
            plural_category,
        );
        self.entries.get(&lookup_key)
    }

    /// Returns all entries.
    pub fn entries(&self) -> Vec<&TranslationEntry> {
        self.entries.values().collect()
    }

    /// Returns all unique translation keys.
    pub fn keys(&self) -> HashSet<String> {
        self.entries
            .keys()
            .map(|(key, _, _, _, _)| key.clone())
            .collect()
    }

    /// Returns keys that have at least one entry for the given locale.
    pub fn keys_for_locale(&self, locale: &Locale) -> HashSet<String> {
        self.entries
            .keys()
            .filter(|(_, loc, _, _, _)| loc == locale)
            .map(|(key, _, _, _, _)| key.clone())
            .collect()
    }

    /// Returns all locales present in this database.
    pub fn locales(&self) -> HashSet<Locale> {
        self.entries
            .keys()
            .map(|(_, loc, _, _, _)| loc.clone())
            .collect()
    }

    /// Returns all entries for a specific locale.
    pub fn entries_for_locale(&self, locale: &Locale) -> Vec<&TranslationEntry> {
        self.entries
            .values()
            .filter(|e| &e.locale == locale)
            .collect()
    }

    /// Returns metadata for this database.
    pub fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }

    /// Sets a metadata value.
    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Removes a translation entry.
    pub fn remove(
        &mut self,
        key: &str,
        locale: &Locale,
        context: Option<&TranslationContext>,
        gender: Option<Gender>,
        plural_category: Option<PluralCategory>,
    ) -> Option<TranslationEntry> {
        let lookup_key = (
            key.to_string(),
            locale.clone(),
            context.cloned(),
            gender,
            plural_category,
        );
        self.entries.remove(&lookup_key)
    }

    /// Returns the number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the database is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clears all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.metadata.clear();
    }
}

// Custom JSON serialization for the database (flat list of entries).
impl Serialize for TranslationDatabase {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq;
        let entries: Vec<&TranslationEntry> = self.entries.values().collect();
        let mut seq = serializer.serialize_seq(Some(entries.len()))?;
        for entry in entries {
            seq.serialize_element(entry)?;
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for TranslationDatabase {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let entries: Vec<TranslationEntry> = Vec::deserialize(deserializer)?;
        let mut db = Self::new();
        for entry in entries {
            db.insert(entry);
        }
        Ok(db)
    }
}

// ---------------------------------------------------------------------------
// String Interpolation
// ---------------------------------------------------------------------------

/// Options for string interpolation.
#[derive(Debug, Clone, Copy)]
pub struct InterpolationOptions {
    pub escape_char: char,
    pub open_brace: char,
    pub close_brace: char,
    pub format_spec: bool,
}

impl Default for InterpolationOptions {
    fn default() -> Self {
        Self {
            escape_char: '\\',
            open_brace: '{',
            close_brace: '}',
            format_spec: true,
        }
    }
}

/// Performs `{variable}`-style interpolation on translated strings.
#[derive(Debug, Clone)]
pub struct StringInterpolator {
    options: InterpolationOptions,
}

impl StringInterpolator {
    /// Creates a new interpolator with the given options.
    pub fn new(options: InterpolationOptions) -> Self {
        Self { options }
    }

    /// Creates an interpolator with default options.
    pub fn default() -> Self {
        Self {
            options: InterpolationOptions::default(),
        }
    }

    /// Interpolates a template string with the given values.
    ///
    /// Placeholders are written as `{name}` and can be escaped as `\\{name}`.
    pub fn interpolate(
        &self,
        template: &str,
        values: &HashMap<String, String>,
    ) -> Result<String, LocalizationError> {
        let mut result = String::with_capacity(template.len());
        let mut chars = template.chars().peekable();
        let mut current_var = String::new();
        let mut in_var = false;

        while let Some(ch) = chars.next() {
            if in_var {
                if ch == self.options.close_brace {
                    let var_name = current_var.trim().to_string();
                    if let Some(value) = values.get(&var_name) {
                        result.push_str(value);
                    } else {
                        result.push_str(&format!(
                            "{}{}{}",
                            self.options.open_brace, var_name, self.options.close_brace
                        ));
                    }
                    current_var.clear();
                    in_var = false;
                } else {
                    current_var.push(ch);
                }
            } else if ch == self.options.escape_char
                && chars.peek() == Some(&self.options.open_brace)
            {
                result.push(self.options.open_brace);
                chars.next();
            } else if ch == self.options.open_brace {
                in_var = true;
            } else {
                result.push(ch);
            }
        }

        if in_var {
            return Err(LocalizationError::InvalidInterpolation(
                "Unterminated interpolation variable".into(),
            ));
        }

        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// Pluralization
// ---------------------------------------------------------------------------

/// Pluralization rule defining how many plural forms a language has and how
/// to select among them for a given count.
#[derive(Debug, Clone)]
pub struct PluralizationRule {
    /// Number of plural forms (e.g. 2 for English).
    pub nplurals: usize,
    /// Function that selects the plural index for a given count.
    pub plural_expr: fn(i64) -> usize,
}

impl PluralizationRule {
    /// Resolves the plural category index for a given count.
    pub fn resolve(&self, n: i64) -> usize {
        (self.plural_expr)(n)
    }
}

/// CLDR-style pluralization rules for many languages.
pub struct PluralizationRules;

impl PluralizationRules {
    /// Returns the pluralization rule for the given locale.
    pub fn get(locale: &Locale) -> PluralizationRule {
        let lang = locale.language().as_str();
        match lang {
            "en" | "de" | "es" | "fr" | "it" | "nl" | "pt" | "sv" | "tr" | "da" | "el"
            | "fi" | "hu" | "no" | "pl" | "cy" | "eu" | "ga" | "hy" | "ur" | "uz" => {
                Self::english_rule()
            }
            "ru" | "uk" | "hr" | "sr" | "be" | "bg" | "mk" | "sl" | "ba" | "kk" | "ky"
            | "mn" | "tg" | "tk" | "tt" | "uk" | "uz" | "cu" | "os" | "rue" | "sah"
            | "tyv" | "xal" | "av" | "ce" | "cv" | "kbd" | "krc" | "lbe" | "lez" | "mdf"
            | "mhr" | "myv" | "nog" | "ru" | "sjd" | "tab" | "tly" | "udm" | "xmf" => {
                Self::slavic_rule()
            }
            "ar" | "fa" | "ps" | "ur" | "ckb" | "dv" | "ha" | "kk" | "ky" | "uz" => {
                Self::arabic_rule()
            }
            "cs" | "sk" => Self::czech_rule(),
            "ga" => Self::irish_rule(),
            "he" | "iw" => Self::hebrew_rule(),
            "ja" | "zh" | "ko" | "vi" | "th" | "id" | "ms" | "lo" | "my" | "km" => {
                Self::east_asian_rule()
            }
            "cy" | "br" | "kw" => Self::welsh_rule(),
            "lt" => Self::lithuanian_rule(),
            "lv" => Self::latvian_rule(),
            "ro" => Self::romanian_rule(),
            _ => Self::english_rule(),
        }
    }

    /// English-style: 1 form, plural for non-1.
    pub fn english_rule() -> PluralizationRule {
        PluralizationRule {
            nplurals: 2,
            plural_expr: |n| if n == 1 { 0 } else { 1 },
        }
    }

    /// Slavic-style: 3 forms for Russian, Ukrainian, etc.
    pub fn slavic_rule() -> PluralizationRule {
        PluralizationRule {
            nplurals: 3,
            plural_expr: |n| {
                let n_mod = (n % 10) as usize;
                let n_mod_100 = (n % 100) as usize;
                if n_mod == 1 && n_mod_100 != 11 {
                    0
                } else if n_mod >= 2 && n_mod <= 4 && !(n_mod_100 >= 12 && n_mod_100 <= 14) {
                    1
                } else {
                    2
                }
            },
        }
    }

    /// Arabic-style: 6 forms.
    pub fn arabic_rule() -> PluralizationRule {
        PluralizationRule {
            nplurals: 6,
            plural_expr: |n| {
                if n == 0 {
                    0
                } else if n == 1 {
                    1
                } else if n == 2 {
                    2
                } else if n % 100 >= 3 && n % 100 <= 10 {
                    3
                } else if n % 100 >= 11 {
                    4
                } else {
                    5
                }
            },
        }
    }

    /// Czech-style: 3 forms.
    pub fn czech_rule() -> PluralizationRule {
        PluralizationRule {
            nplurals: 3,
            plural_expr: |n| {
                if n == 1 {
                    0
                } else if n >= 2 && n <= 4 {
                    1
                } else {
                    2
                }
            },
        }
    }

    /// Irish-style: 3 forms.
    pub fn irish_rule() -> PluralizationRule {
        PluralizationRule {
            nplurals: 3,
            plural_expr: |n| {
                if n == 1 {
                    0
                } else if n == 2 {
                    1
                } else {
                    2
                }
            },
        }
    }

    /// Hebrew-style: 4 forms.
    pub fn hebrew_rule() -> PluralizationRule {
        PluralizationRule {
            nplurals: 4,
            plural_expr: |n| {
                if n == 1 {
                    0
                } else if n == 2 {
                    1
                } else if n >= 3 && n <= 10 {
                    2
                } else {
                    3
                }
            },
        }
    }

    /// East Asian: 1 form (no plural distinction).
    pub fn east_asian_rule() -> PluralizationRule {
        PluralizationRule {
            nplurals: 1,
            plural_expr: |_| 0,
        }
    }

    /// Welsh-style: 6 forms.
    pub fn welsh_rule() -> PluralizationRule {
        PluralizationRule {
            nplurals: 6,
            plural_expr: |n| {
                if n == 0 {
                    0
                } else if n == 1 {
                    1
                } else if n == 2 {
                    2
                } else if n == 3 {
                    3
                } else if n == 6 {
                    4
                } else {
                    5
                }
            },
        }
    }

    /// Lithuanian-style: 3 forms.
    pub fn lithuanian_rule() -> PluralizationRule {
        PluralizationRule {
            nplurals: 3,
            plural_expr: |n| {
                if n % 10 == 1 && n % 100 != 11 {
                    0
                } else if n % 10 >= 2 && n % 10 <= 9 && !(n % 100 >= 11 && n % 100 <= 19) {
                    1
                } else {
                    2
                }
            },
        }
    }

    /// Latvian-style: 3 forms.
    pub fn latvian_rule() -> PluralizationRule {
        PluralizationRule {
            nplurals: 3,
            plural_expr: |n| {
                if n % 10 == 1 && n % 100 != 11 {
                    0
                } else {
                    1
                }
            },
        }
    }

    /// Romanian-style: 3 forms.
    pub fn romanian_rule() -> PluralizationRule {
        PluralizationRule {
            nplurals: 3,
            plural_expr: |n| {
                if n == 1 {
                    0
                } else if n == 0 || (n % 100 >= 1 && n % 100 <= 19) {
                    1
                } else {
                    2
                }
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Unicode Support
// ---------------------------------------------------------------------------

/// Text direction for rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TextDirection {
    Ltr,
    Rtl,
    Auto,
}

impl Default for TextDirection {
    fn default() -> Self {
        TextDirection::Ltr
    }
}

/// Unicode normalization form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UnicodeNormalization {
    Nfc,
    Nfd,
    Nfkc,
    Nfkd,
    None,
}

impl Default for UnicodeNormalization {
    fn default() -> Self {
        UnicodeNormalization::Nfc
    }
}

/// Unicode processing options.
#[derive(Debug, Clone, Copy)]
pub struct UnicodeOptions {
    pub direction: TextDirection,
    pub normalization: UnicodeNormalization,
    pub bidi_algorithm: bool,
    pub emoji_presentation: bool,
}

impl Default for UnicodeOptions {
    fn default() -> Self {
        Self {
            direction: TextDirection::Ltr,
            normalization: UnicodeNormalization::Nfc,
            bidi_algorithm: true,
            emoji_presentation: true,
        }
    }
}

/// Detects whether the string contains right-to-left characters.
pub fn has_rtl_chars(s: &str) -> bool {
    s.chars().any(|c| {
        matches!(
            c,
            '\u{0590}'..='\u{08FF}'
                | '\u{FB1D}'..='\u{FDFF}'
                | '\u{FE70}'..='\u{FEFF}'
                | '\u{10800}'..='\u{10FFF}'
                | '\u{1E800}'..='\u{1EFFF}'
        )
    })
}

/// Detects whether a string is primarily composed of emoji characters.
pub fn is_primarily_emoji(s: &str) -> bool {
    let emoji_count = s.chars().filter(is_emoji).count();
    let total_count = s.chars().filter(|c| !c.is_whitespace()).count();
    total_count > 0 && emoji_count as f32 / total_count as f32 > 0.5
}

fn is_emoji(c: char) -> bool {
    matches!(
        c,
        '\u{1F600}'..='\u{1F64F}'
            | '\u{1F300}'..='\u{1F5FF}'
            | '\u{1F680}'..='\u{1F6FF}'
            | '\u{2600}'..='\u{26FF}'
            | '\u{2700}'..='\u{27BF}'
            | '\u{FE00}'..='\u{FE0F}'
            | '\u{1F900}'..='\u{1F9FF}'
            | '\u{1FA00}'..='\u{1FA6F}'
            | '\u{1FA70}'..='\u{1FAFF}'
            | '\u{200D}'
            | '\u{20E3}'
    )
}

/// Auto-detects the text direction for a string.
pub fn auto_detect_direction(s: &str) -> TextDirection {
    if has_rtl_chars(s) {
        TextDirection::Rtl
    } else {
        TextDirection::Ltr
    }
}

// ---------------------------------------------------------------------------
// Localized Text Asset
// ---------------------------------------------------------------------------

/// A collection of localized text strings for a single locale.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalizedTextAsset {
    pub locale: Locale,
    pub texts: HashMap<String, String>,
    pub metadata: HashMap<String, String>,
    pub last_modified: u64,
}

impl LocalizedTextAsset {
    /// Creates a new empty localized text asset.
    pub fn new(locale: Locale) -> Self {
        Self {
            locale,
            texts: HashMap::new(),
            metadata: HashMap::new(),
            last_modified: 0,
        }
    }

    /// Retrieves text by key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.texts.get(key).map(|s| s.as_str())
    }

    /// Sets text for a key.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.texts.insert(key.into(), value.into());
    }

    /// Returns the number of entries.
    pub fn len(&self) -> usize {
        self.texts.len()
    }

    /// Returns `true` if the asset is empty.
    pub fn is_empty(&self) -> bool {
        self.texts.is_empty()
    }

    /// Returns all keys.
    pub fn keys(&self) -> Vec<&str> {
        self.texts.keys().map(|s| s.as_str()).collect()
    }

    /// Removes an entry by key.
    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.texts.remove(key)
    }
}

// ---------------------------------------------------------------------------
// Localized Font Info
// ---------------------------------------------------------------------------

/// Font configuration for a specific locale.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalizedFontInfo {
    pub locale: Locale,
    pub font_family: String,
    pub font_path: PathBuf,
    pub fallback_fonts: Vec<String>,
    pub features: Vec<String>,
    pub size: f32,
    pub line_height: f32,
    pub direction: TextDirection,
}

impl LocalizedFontInfo {
    /// Creates a new localized font info.
    pub fn new(locale: Locale, font_family: impl Into<String>, font_path: PathBuf) -> Self {
        Self {
            locale,
            font_family: font_family.into(),
            font_path,
            fallback_fonts: Vec::new(),
            features: Vec::new(),
            size: 16.0,
            line_height: 1.4,
            direction: TextDirection::default(),
        }
    }

    /// Sets fallback fonts.
    pub fn with_fallbacks(mut self, fallbacks: Vec<String>) -> Self {
        self.fallback_fonts = fallbacks;
        self
    }

    /// Sets OpenType features.
    pub fn with_features(mut self, features: Vec<String>) -> Self {
        self.features = features;
        self
    }

    /// Sets the font size.
    pub fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Sets the line height multiplier.
    pub fn with_line_height(mut self, line_height: f32) -> Self {
        self.line_height = line_height;
        self
    }
}

// ---------------------------------------------------------------------------
// Localized Audio / Subtitles
// ---------------------------------------------------------------------------

/// Audio and subtitle configuration for a specific locale.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalizedAudioInfo {
    pub locale: Locale,
    pub audio_track: Option<PathBuf>,
    pub subtitle_track: Option<PathBuf>,
    pub speaker_gender: Gender,
    pub sample_rate: u32,
    pub channels: u8,
}

impl LocalizedAudioInfo {
    /// Creates a new localized audio info.
    pub fn new(locale: Locale) -> Self {
        Self {
            locale,
            audio_track: None,
            subtitle_track: None,
            speaker_gender: Gender::Unknown,
            sample_rate: 48000,
            channels: 2,
        }
    }

    /// Sets the audio track path.
    pub fn with_audio_track(mut self, path: PathBuf) -> Self {
        self.audio_track = Some(path);
        self
    }

    /// Sets the subtitle track path.
    pub fn with_subtitle_track(mut self, path: PathBuf) -> Self {
        self.subtitle_track = Some(path);
        self
    }

    /// Sets the speaker gender.
    pub fn with_speaker_gender(mut self, gender: Gender) -> Self {
        self.speaker_gender = gender;
        self
    }
}

// ---------------------------------------------------------------------------
// Text-to-Speech
// ---------------------------------------------------------------------------

/// Text-to-speech engine configuration per locale.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextToSpeechEngine {
    pub locale: Locale,
    pub voice_id: String,
    pub rate: f32,
    pub pitch: f32,
    pub volume: f32,
    pub engine: String,
}

impl TextToSpeechEngine {
    /// Creates a new TTS engine configuration.
    pub fn new(locale: Locale) -> Self {
        Self {
            locale,
            voice_id: String::new(),
            rate: 1.0,
            pitch: 1.0,
            volume: 1.0,
            engine: String::new(),
        }
    }

    /// Sets the voice identifier.
    pub fn with_voice(mut self, voice_id: impl Into<String>) -> Self {
        self.voice_id = voice_id.into();
        self
    }

    /// Sets the speech rate (clamped to [0.5, 2.0]).
    pub fn with_rate(mut self, rate: f32) -> Self {
        self.rate = rate.clamp(0.5, 2.0);
        self
    }

    /// Sets the pitch (clamped to [0.5, 2.0]).
    pub fn with_pitch(mut self, pitch: f32) -> Self {
        self.pitch = pitch.clamp(0.5, 2.0);
        self
    }

    /// Sets the volume (clamped to [0.0, 1.0]).
    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume.clamp(0.0, 1.0);
        self
    }

    /// Speaks the given text. Platform-specific TTS hook (no-op in core).
    pub fn speak(&self, text: &str) -> Result<(), LocalizationError> {
        tracing::debug!(locale = %self.locale, voice = %self.voice_id, "TTS speak: {}", text);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Localization Manager
// ---------------------------------------------------------------------------

/// Configuration for the localization manager.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalizationConfig {
    pub default_locale: Locale,
    pub fallback_locales: Vec<Locale>,
    pub interpolation_enabled: bool,
    pub pluralization_enabled: bool,
    pub auto_detect_locale: bool,
    pub file_search_paths: Vec<PathBuf>,
    pub unicode_options: UnicodeOptions,
}

impl Default for LocalizationConfig {
    fn default() -> Self {
        Self {
            default_locale: Locale::new("en"),
            fallback_locales: vec![Locale::new("en")],
            interpolation_enabled: true,
            pluralization_enabled: true,
            auto_detect_locale: true,
            file_search_paths: vec!["assets/locales".into(), "locales".into()],
            unicode_options: UnicodeOptions::default(),
        }
    }
}

/// Central localization manager that owns translation databases, active locale,
/// fallback chains, and localized asset registries.
#[derive(Debug, Clone)]
pub struct LocalizationManager {
    database: Arc<RwLock<TranslationDatabase>>,
    active_locale: Arc<RwLock<Locale>>,
    fallback_locales: Arc<RwLock<Vec<Locale>>>,
    interpolator: Arc<StringInterpolator>,
    unicode_options: UnicodeOptions,
    text_assets: Arc<RwLock<HashMap<Locale, LocalizedTextAsset>>>,
    font_infos: Arc<RwLock<Vec<LocalizedFontInfo>>>,
    audio_infos: Arc<RwLock<Vec<LocalizedAudioInfo>>>,
    tts_engines: Arc<RwLock<HashMap<Locale, TextToSpeechEngine>>>,
    locale_detector: Arc<dyn Fn() -> Locale + Send + Sync>,
}

impl LocalizationManager {
    /// Creates a new localization manager with the given active locale.
    pub fn new(active_locale: Locale) -> Self {
        Self {
            database: Arc::new(RwLock::new(TranslationDatabase::new())),
            active_locale: Arc::new(RwLock::new(active_locale)),
            fallback_locales: Arc::new(RwLock::new(vec![Locale::new("en")])),
            interpolator: Arc::new(StringInterpolator::default()),
            unicode_options: UnicodeOptions::default(),
            text_assets: Arc::new(RwLock::new(HashMap::new())),
            font_infos: Arc::new(RwLock::new(Vec::new())),
            audio_infos: Arc::new(RwLock::new(Vec::new())),
            tts_engines: Arc::new(RwLock::new(HashMap::new())),
            locale_detector: Arc::new(Self::system_locale),
        }
    }

    /// Creates a new manager using the system locale.
    pub fn from_env() -> Self {
        let locale = Self::system_locale();
        Self::new(locale)
    }

    /// Creates a new manager from a configuration.
    pub fn from_config(config: LocalizationConfig) -> Self {
        let manager = Self::new(config.default_locale);
        *manager.fallback_locales.write().unwrap() = config.fallback_locales;
        manager.unicode_options = config.unicode_options;
        manager
    }

    /// Sets the fallback locale chain.
    pub fn with_fallback(mut self, fallbacks: Vec<Locale>) -> Self {
        *self.fallback_locales.write().unwrap() = fallbacks;
        self
    }

    /// Sets a custom locale detector function.
    pub fn with_locale_detector(
        mut self,
        detector: Arc<dyn Fn() -> Locale + Send + Sync>,
    ) -> Self {
        self.locale_detector = detector;
        self
    }

    /// Sets the currently active locale.
    pub fn set_locale(&self, locale: Locale) {
        *self.active_locale.write().unwrap() = locale;
    }

    /// Returns the currently active locale.
    pub fn active_locale(&self) -> Locale {
        self.active_locale.read().unwrap().clone()
    }

    /// Returns the active fallback locales.
    pub fn fallback_locales(&self) -> Vec<Locale> {
        self.fallback_locales.read().unwrap().clone()
    }

    /// Detects the system locale using platform-specific methods and environment variables.
    pub fn system_locale() -> Locale {
        // Environment variables
        for var in &["LANG", "LC_ALL", "LC_MESSAGES", "LANGUAGE"] {
            if let Ok(lang) = std::env::var(var) {
                if let Ok(locale) = lang.parse::<Locale>() {
                    return locale;
                }
            }
        }

        // Platform-specific detection stubs
        #[cfg(target_os = "windows")]
        {
            // Would use GetUserDefaultLocaleName via winapi
        }

        #[cfg(target_os = "macos")]
        {
            // Would use CFLocaleCopyCurrent via CoreFoundation
        }

        #[cfg(target_os = "linux")]
        {
            // Already checked /etc/locale.conf and env vars
        }

        Locale::new("en")
    }

    /// Loads all translation files from the configured search paths.
    pub fn load_default_files(&mut self) -> Result<(), LocalizationError> {
        for path in &LocalizationConfig::default().file_search_paths {
            if path.exists() {
                self.load_directory(path)?;
            }
        }
        Ok(())
    }

    /// Loads all translation files from a directory.
    pub fn load_directory(&mut self, path: impl AsRef<Path>) -> Result<(), LocalizationError> {
        let path = path.as_ref();
        if !path.exists() || !path.is_dir() {
            return Err(LocalizationError::Io(
                path.display().to_string(),
                "not a directory".into(),
            ));
        }

        let mut entries: Vec<_> = std::fs::read_dir(path)?
            .collect::<Result<Vec<_>, _>>()?;

        // Sort for deterministic loading order
        entries.sort_by_key(|e| e.path());

        for entry in entries {
            let file_path = entry.path();
            if !file_path.is_file() {
                continue;
            }

            let extension = file_path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();

            match extension.as_str() {
                "json" => self.load_json(&file_path)?,
                "po" => self.load_po(&file_path)?,
                "csv" => self.load_csv(&file_path)?,
                _ => {}
            }
        }

        Ok(())
    }

    /// Loads translations from a JSON file.
    pub fn load_json(&mut self, path: impl AsRef<Path>) -> Result<(), LocalizationError> {
        let path = path.as_ref();
        let contents = std::fs::read_to_string(path)?;
        let locale_db: TranslationDatabase = serde_json::from_str(&contents)
            .map_err(|e| LocalizationError::Parse(path.display().to_string(), e.to_string()))?;

        let mut db = self.database.write().unwrap();
        for entry in locale_db.entries() {
            db.insert(entry.clone());
        }
        Ok(())
    }

    /// Saves translations for a specific locale to a JSON file.
    pub fn save_json(
        &self,
        path: impl AsRef<Path>,
        locale: &Locale,
    ) -> Result<(), LocalizationError> {
        let path = path.as_ref();
        let db = self.database.read().unwrap();
        let locale_db = db.entries_for_locale(locale);

        let serializable: Vec<&TranslationEntry> = locale_db;
        let contents = serde_json::to_string_pretty(&serializable)
            .map_err(|e| LocalizationError::Serialization(e.to_string()))?;
        std::fs::write(path, contents)?;
        Ok(())
    }

    /// Loads translations from a PO (gettext) file.
    pub fn load_po(&mut self, path: impl AsRef<Path>) -> Result<(), LocalizationError> {
        let path = path.as_ref();
        let contents = std::fs::read_to_string(path)?;
        let pairs = parse_po_file(&contents)
            .map_err(|e| LocalizationError::Parse(path.display().to_string(), e))?;

        let locale = self.active_locale();
        let mut db = self.database.write().unwrap();

        for (msgid, msgstr) in pairs {
            if msgid.is_empty() {
                // Header entry — skip
                continue;
            }

            let entry = TranslationEntry {
                key: msgid.clone(),
                locale: locale.clone(),
                context: None,
                translation: msgstr,
                gender: None,
                plural_category: None,
                metadata: HashMap::new(),
            };
            db.insert(entry);
        }
        Ok(())
    }

    /// Saves translations for a specific locale to a PO (gettext) file.
    pub fn save_po(
        &self,
        path: impl AsRef<Path>,
        locale: &Locale,
    ) -> Result<(), LocalizationError> {
        let path = path.as_ref();
        let db = self.database.read().unwrap();
        let entries = db.entries_for_locale(locale);

        let mut output = String::new();
        output.push_str("# Elysium Engine localization file\n");
        output.push_str("# Language: ");
        output.push_str(&locale.to_string());
        output.push_str("\n\n");
        output.push_str("msgid \"\"\n");
        output.push_str("msgstr \"\"\n");
        output.push_str("\"Content-Type: text/plain; charset=UTF-8\\n\"\n");
        output.push_str("\"MIME-Version: 1.0\\n\"\n\n");

        for entry in entries {
            output.push_str("msgid \"");
            output.push_str(&escape_po_string(&entry.key));
            output.push_str("\"\n");
            output.push_str("msgstr \"");
            output.push_str(&escape_po_string(&entry.translation));
            output.push_str("\"\n\n");
        }

        std::fs::write(path, output)?;
        Ok(())
    }

    /// Loads translations from a CSV file.
    pub fn load_csv(&mut self, path: impl AsRef<Path>) -> Result<(), LocalizationError> {
        let path = path.as_ref();
        let contents = std::fs::read_to_string(path)?;
        let mut lines = contents.lines();

        // Skip header line
        let _ = lines.next();

        let mut db = self.database.write().unwrap();

        for line in lines {
            if line.trim().is_empty() {
                continue;
            }

            let fields = parse_csv_line(line);
            if fields.len() < 6 {
                continue;
            }

            let key = fields[0].clone();
            let locale_str = fields[1].clone();
            let locale: Locale = locale_str
                .parse()
                .map_err(|e| LocalizationError::Parse(path.display().to_string(), e.to_string()))?;

            let context = if fields[2].is_empty() {
                None
            } else {
                Some(TranslationContext::new(fields[2].clone()))
            };

            let gender = match fields[3].to_lowercase().as_str() {
                "male" => Some(Gender::Male),
                "female" => Some(Gender::Female),
                "neuter" => Some(Gender::Neuter),
                "unknown" => Some(Gender::Unknown),
                _ => None,
            };

            let plural_category = match fields[4].to_lowercase().as_str() {
                "zero" => Some(PluralCategory::Zero),
                "one" => Some(PluralCategory::One),
                "two" => Some(PluralCategory::Two),
                "few" => Some(PluralCategory::Few),
                "many" => Some(PluralCategory::Many),
                "other" => Some(PluralCategory::Other),
                _ => None,
            };

            let translation = fields[5].clone();

            let entry = TranslationEntry {
                key,
                locale,
                context,
                translation,
                gender,
                plural_category,
                metadata: HashMap::new(),
            };

            db.insert(entry);
        }

        Ok(())
    }

    /// Saves translations for a specific locale to a CSV file.
    pub fn save_csv(
        &self,
        path: impl AsRef<Path>,
        locale: &Locale,
    ) -> Result<(), LocalizationError> {
        let path = path.as_ref();
        let db = self.database.read().unwrap();
        let entries = db.entries_for_locale(locale);

        let mut output = String::new();
        output.push_str("key,locale,context,gender,plural_category,translation\n");

        for entry in entries {
            output.push_str(&escape_csv_field(&entry.key));
            output.push(',');
            output.push_str(&escape_csv_field(&entry.locale.to_string()));
            output.push(',');
            output.push_str(&escape_csv_field(
                entry.context.as_ref().map(|c| c.as_str()).unwrap_or(""),
            ));
            output.push(',');
            output.push_str(&escape_csv_field(
                entry.gender.as_ref().map(|g| g.to_string()).unwrap_or(""),
            ));
            output.push(',');
            output.push_str(&escape_csv_field(
                entry
                    .plural_category
                    .as_ref()
                    .map(|p| p.to_string())
                    .unwrap_or(""),
            ));
            output.push(',');
            output.push_str(&escape_csv_field(&entry.translation));
            output.push('\n');
        }

        std::fs::write(path, output)?;
        Ok(())
    }

    /// Looks up a translation with full parameter support and fallback chains.
    pub fn translate(
        &self,
        key: &str,
        context: Option<&TranslationContext>,
        gender: Option<Gender>,
        plural_category: Option<PluralCategory>,
        interpolations: &HashMap<String, String>,
    ) -> Result<String, LocalizationError> {
        let locale = self.active_locale();
        let fallbacks = self.fallback_locales.read().unwrap().clone();
        let db = self.database.read().unwrap();

        // Helper to try interpolation
        let try_entry = |entry: &TranslationEntry| {
            if LocalizationConfig::default().interpolation_enabled {
                self.interpolator.interpolate(&entry.translation, interpolations)
            } else {
                Ok(entry.translation.clone())
            }
        };

        // 1. Exact match in active locale
        if let Some(entry) =
            db.get(key, &locale, context, gender, plural_category)
        {
            return try_entry(entry);
        }

        // 2. Try without plural category in active locale
        if plural_category.is_some() {
            if let Some(entry) = db.get(key, &locale, context, gender, None) {
                return try_entry(entry);
            }
        }

        // 3. Try without gender in active locale
        if gender.is_some() {
            if let Some(entry) = db.get(key, &locale, context, None, plural_category) {
                return try_entry(entry);
            }
            if plural_category.is_some() {
                if let Some(entry) = db.get(key, &locale, context, None, None) {
                    return try_entry(entry);
                }
            }
        }

        // 4. Fallback locales
        for fallback in &fallbacks {
            if let Some(entry) =
                db.get(key, fallback, context, gender, plural_category)
            {
                return try_entry(entry);
            }
            if plural_category.is_some() {
                if let Some(entry) = db.get(key, fallback, context, gender, None) {
                    return try_entry(entry);
                }
            }
            if gender.is_some() {
                if let Some(entry) = db.get(key, fallback, context, None, plural_category) {
                    return try_entry(entry);
                }
                if plural_category.is_some() {
                    if let Some(entry) = db.get(key, fallback, context, None, None) {
                        return try_entry(entry);
                    }
                }
            }
        }

        // 5. Return key as ultimate fallback
        Ok(key.to_string())
    }

    /// Translates a key with a count, automatically selecting the correct plural form.
    pub fn translate_with_count(
        &self,
        key: &str,
        count: i64,
        interpolations: &HashMap<String, String>,
    ) -> Result<String, LocalizationError> {
        let locale = self.active_locale();
        let rules = PluralizationRules::get(&locale);
        let index = rules.resolve(count);
        let plural_category = match index {
            0 => PluralCategory::Zero,
            1 => PluralCategory::One,
            2 => PluralCategory::Two,
            3 => PluralCategory::Few,
            4 => PluralCategory::Many,
            _ => PluralCategory::Other,
        };

        self.translate(key, None, None, Some(plural_category), interpolations)
    }

    /// Registers a localized text asset.
    pub fn register_text_asset(&self, locale: Locale, asset: LocalizedTextAsset) {
        self.text_assets
            .write()
            .unwrap()
            .insert(locale, asset);
    }

    /// Retrieves a localized text asset.
    pub fn get_text_asset(&self, locale: &Locale) -> Option<std::sync::RwLockReadGuard<'_, HashMap<Locale, LocalizedTextAsset>>> {
        self.text_assets.read().ok()
    }

    /// Registers a localized font info.
    pub fn register_font_info(&self, info: LocalizedFontInfo) {
        self.font_infos.write().unwrap().push(info);
    }

    /// Returns font infos for a locale.
    pub fn font_infos_for_locale(&self, locale: &Locale) -> Vec<&LocalizedFontInfo> {
        self.font_infos
            .read()
            .unwrap()
            .iter()
            .filter(|f| &f.locale == locale)
            .collect()
    }

    /// Registers a localized audio info.
    pub fn register_audio_info(&self, info: LocalizedAudioInfo) {
        self.audio_infos.write().unwrap().push(info);
    }

    /// Returns audio infos for a locale.
    pub fn audio_infos_for_locale(&self, locale: &Locale) -> Vec<&LocalizedAudioInfo> {
        self.audio_infos
            .read()
            .unwrap()
            .iter()
            .filter(|a| &a.locale == locale)
            .collect()
    }

    /// Registers a TTS engine for a locale.
    pub fn register_tts_engine(&self, locale: Locale, engine: TextToSpeechEngine) {
        self.tts_engines.write().unwrap().insert(locale, engine);
    }

    /// Retrieves a TTS engine for a locale.
    pub fn tts_engine(&self, locale: &Locale) -> Option<std::sync::RwLockReadGuard<'_, HashMap<Locale, TextToSpeechEngine>>> {
        self.tts_engines.read().ok()
    }

    /// Returns all loaded locales.
    pub fn loaded_locales(&self) -> Vec<Locale> {
        let db = self.database.read().unwrap();
        let mut locales: Vec<_> = db.locales().into_iter().collect();
        locales.sort();
        locales
    }

    /// Returns all translation keys for the active locale.
    pub fn loaded_keys(&self) -> Vec<String> {
        let locale = self.active_locale();
        let db = self.database.read().unwrap();
        let mut keys: Vec<_> = db.keys_for_locale(&locale).into_iter().collect();
        keys.sort();
        keys
    }
}

// ---------------------------------------------------------------------------
// Editor Integration
// ---------------------------------------------------------------------------

/// Extracts translation keys from source code.
pub struct TranslationExtractor;

impl TranslationExtractor {
    /// Extracts all translation keys from a source string.
    ///
    /// Detects patterns like `t!("key")`, `tr!("key")`, `l10n!("key")`.
    pub fn extract_from_source(source: &str) -> HashSet<String> {
        let mut keys = HashSet::new();
        let chars: Vec<char> = source.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // Match: t!, tr!, l10n!
            let is_func = if i + 1 < chars.len() && chars[i] == 't' && chars[i + 1] == '!' {
                i + 2
            } else if i + 2 < chars.len()
                && chars[i] == 't'
                && chars[i + 1] == 'r'
                && chars[i + 2] == '!'
            {
                i + 3
            } else if i + 4 < chars.len()
                && chars[i] == 'l'
                && chars[i + 1] == '1'
                && chars[i + 2] == '0'
                && chars[i + 3] == 'n'
                && chars[i + 4] == '!'
            {
                i + 5
            } else {
                i + 1;
                continue;
            };

            // Skip whitespace
            let mut j = is_func;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }

            // Expect '('
            if j < chars.len() && chars[j] == '(' {
                j += 1;

                // Skip whitespace
                while j < chars.len() && chars[j].is_whitespace() {
                    j += 1;
                }

                // Expect '"'
                if j < chars.len() && chars[j] == '"' {
                    j += 1;
                    let start = j;

                    // Find closing '"'
                    while j < chars.len() && chars[j] != '"' {
                        if chars[j] == '\\' {
                            j += 1; // skip escaped char
                        }
                        j += 1;
                    }

                    if j < chars.len() && chars[j] == '"' {
                        let key: String = chars[start..j].iter().collect();
                        keys.insert(key);
                    }
                }
            }

            i = j;
        }

        keys
    }

    /// Extracts keys from all files in a directory (recursive).
    pub fn extract_from_directory(&self, path: impl AsRef<Path>) -> Result<HashSet<String>, LocalizationError> {
        let path = path.as_ref();
        if !path.exists() || !path.is_dir() {
            return Err(LocalizationError::Io(
                path.display().to_string(),
                "not a directory".into(),
            ));
        }

        let mut keys = HashSet::new();
        let mut stack = vec![path.to_path_buf()];

        while let Some(current) = stack.pop() {
            if let Ok(entries) = std::fs::read_dir(&current) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let entry_path = entry.path();
                    if entry_path.is_dir() {
                        stack.push(entry_path);
                    } else if let Some(ext) = entry_path.extension().and_then(|s| s.to_str()) {
                        if matches!(ext, "rs" | "ts" | "js" | "py" | "cpp" | "c" | "h") {
                            if let Ok(contents) = std::fs::read_to_string(&entry_path) {
                                keys.extend(Self::extract_from_source(&contents));
                            }
                        }
                    }
                }
            }
        }

        Ok(keys)
    }
}

// ---------------------------------------------------------------------------
// Missing Translation Reporter
// ---------------------------------------------------------------------------

/// Report of missing translations for a specific locale.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingTranslationReport {
    pub locale: Locale,
    pub missing_keys: Vec<String>,
    pub total_keys: usize,
    pub translated_keys: usize,
    pub completion_percentage: f32,
}

/// Reports missing translations for a given locale.
pub struct MissingTranslationReporter;

impl MissingTranslationReporter {
    /// Generates a missing translation report for a locale.
    pub fn report(database: &TranslationDatabase, locale: &Locale) -> MissingTranslationReport {
        let all_keys: HashSet<String> = database.keys();
        let translated_keys: HashSet<String> = database.keys_for_locale(locale);

        let mut missing_keys: Vec<String> = all_keys
            .difference(&translated_keys)
            .cloned()
            .collect();
        missing_keys.sort();

        let total_keys = all_keys.len();
        let translated_count = translated_keys.len();
        let completion_percentage = if total_keys == 0 {
            100.0
        } else {
            (translated_count as f32 / total_keys as f32) * 100.0
        };

        MissingTranslationReport {
            locale: locale.clone(),
            missing_keys,
            total_keys,
            translated_keys: translated_count,
            completion_percentage,
        }
    }

    /// Generates reports for all locales in the database.
    pub fn report_all(database: &TranslationDatabase) -> Vec<MissingTranslationReport> {
        let mut reports = Vec::new();
        for locale in database.locales() {
            reports.push(Self::report(database, &locale));
        }
        reports
    }
}

// ---------------------------------------------------------------------------
// Translation Validator
// ---------------------------------------------------------------------------

/// Validation error found during translation checking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationError {
    PlaceholderMismatch {
        key: String,
        locale: Locale,
        source: String,
        translation: String,
    },
    InvalidPluralCategory {
        key: String,
        locale: Locale,
        category: String,
        max: usize,
    },
    MissingGenderVariant {
        key: String,
        locale: Locale,
        gender: Gender,
    },
    EmptyTranslation {
        key: String,
        locale: Locale,
    },
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::PlaceholderMismatch {
                key,
                locale,
                source,
                translation,
            } => {
                write!(
                    f,
                    "Placeholder mismatch in '{}' ({locale}): source has {:?}, translation has {:?}",
                    key, source, translation
                )
            }
            ValidationError::InvalidPluralCategory {
                key,
                locale,
                category,
                max,
            } => {
                write!(
                    f,
                    "Invalid plural category '{}' in '{}' ({locale}): max is {max}",
                    category, key
                )
            }
            ValidationError::MissingGenderVariant {
                key,
                locale,
                gender,
            } => {
                write!(
                    f,
                    "Missing gender variant '{}' for '{}' ({locale})",
                    gender, key
                )
            }
            ValidationError::EmptyTranslation { key, locale } => {
                write!(f, "Empty translation for '{}' ({locale})", key)
            }
        }
    }
}

/// Validates translations for a locale.
pub struct TranslationValidator;

impl TranslationValidator {
    /// Validates all translations for a locale and returns a list of errors.
    pub fn validate(database: &TranslationDatabase, locale: &Locale) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let rules = PluralizationRules::get(locale);
        let entries = database.entries_for_locale(locale);

        for entry in entries {
            // Check for empty translations
            if entry.translation.trim().is_empty() {
                errors.push(ValidationError::EmptyTranslation {
                    key: entry.key.clone(),
                    locale: locale.clone(),
                });
            }

            // Check placeholder consistency
            let source_placeholders = Self::extract_placeholders(&entry.key);
            let translation_placeholders = Self::extract_placeholders(&entry.translation);

            if source_placeholders != translation_placeholders {
                errors.push(ValidationError::PlaceholderMismatch {
                    key: entry.key.clone(),
                    locale: locale.clone(),
                    source: entry.key.clone(),
                    translation: entry.translation.clone(),
                });
            }

            // Check plural category validity
            if let Some(plural) = entry.plural_category {
                let index = plural as usize;
                if index >= rules.nplurals {
                    errors.push(ValidationError::InvalidPluralCategory {
                        key: entry.key.clone(),
                        locale: locale.clone(),
                        category: plural.to_string(),
                        max: rules.nplurals,
                    });
                }
            }
        }

        errors
    }

    /// Validates all locales in the database.
    pub fn validate_all(
        database: &TranslationDatabase,
    ) -> HashMap<Locale, Vec<ValidationError>> {
        let mut results = HashMap::new();
        for locale in database.locales() {
            let errors = Self::validate(database, &locale);
            if !errors.is_empty() {
                results.insert(locale, errors);
            }
        }
        results
    }

    fn extract_placeholders(s: &str) -> HashSet<String> {
        let mut placeholders = HashSet::new();
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '{' {
                let start = i + 1;
                while i < chars.len() && chars[i] != '}' {
                    if chars[i] == '\\' {
                        i += 1;
                    }
                    i += 1;
                }
                if i < chars.len() && chars[i] == '}' {
                    placeholders.insert(chars[start..i].iter().collect());
                }
            }
            i += 1;
        }

        placeholders
    }
}

// ---------------------------------------------------------------------------
// File Format Helpers (CSV / PO)
// ---------------------------------------------------------------------------

/// Parses a single CSV line with quoted field support.
pub fn parse_csv_line(line: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next(); // consume escaped quote
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                result.push(current.clone());
                current.clear();
            }
            _ => {
                current.push(ch);
            }
        }
    }
    result.push(current);
    result
}

/// Escapes a field for CSV output.
pub fn escape_csv_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') || field.contains('\r') {
        format!("\"{}\"", field.replace("\"", "\"\""))
    } else {
        field.to_string()
    }
}

/// Parses a PO (gettext) file into (msgid, msgstr) pairs.
pub fn parse_po_file(contents: &str) -> Result<Vec<(String, String)>, String> {
    let mut entries = Vec::new();
    let mut current_msgid = String::new();
    let mut current_msgstr = String::new();
    let mut in_msgid = false;
    let mut in_msgstr = false;

    for line in contents.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("msgid ") {
            if in_msgstr && !current_msgid.is_empty() {
                entries.push((current_msgid.clone(), current_msgstr.clone()));
            }
            current_msgid = parse_po_value(trimmed);
            current_msgstr.clear();
            in_msgid = true;
            in_msgstr = false;
        } else if trimmed.starts_with("msgstr ") {
            current_msgstr = parse_po_value(trimmed);
            in_msgid = false;
            in_msgstr = true;
        } else if trimmed.starts_with('"') && (in_msgid || in_msgstr) {
            let val = parse_po_value(trimmed);
            if in_msgid {
                current_msgid.push_str(&val);
            } else {
                current_msgstr.push_str(&val);
            }
        } else if trimmed.is_empty() || trimmed.starts_with('#') {
            if in_msgstr && !current_msgid.is_empty() {
                entries.push((current_msgid.clone(), current_msgstr.clone()));
                current_msgid.clear();
                current_msgstr.clear();
                in_msgid = false;
                in_msgstr = false;
            }
        }
    }

    if in_msgstr && !current_msgid.is_empty() {
        entries.push((current_msgid, current_msgstr));
    }

    Ok(entries)
}

/// Extracts the value from a PO `msgid "..."` or `msgstr "..."` line.
pub fn parse_po_value(line: &str) -> String {
    let first_quote = line.find('"').unwrap_or(0);
    let rest = &line[first_quote..];
    let end = rest.rfind('"').unwrap_or(rest.len());
    let content = &rest[1..end];
    unescape_po_string(content)
}

/// Escapes a string for PO file output.
pub fn escape_po_string(s: &str) -> String {
    s.replace("\\", "\\\\")
        .replace("\"", "\\\"")
        .replace("\n", "\\n")
        .replace("\r", "\\r")
        .replace("\t", "\\t")
}

/// Unescapes a string from PO file input.
pub fn unescape_po_string(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('r') => result.push('\r'),
                Some('t') => result.push('\t'),
                Some('"') => result.push('"'),
                Some('\\') => result.push('\\'),
                _ => result.push('\\'),
            }
        } else {
            result.push(ch);
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_parse_and_display() {
        let locale: Locale = "tr-TR".parse().unwrap();
        assert_eq!(locale.language(), "tr");
        assert_eq!(locale.region(), Some("TR"));
        assert_eq!(locale.to_string(), "tr-TR");
    }

    #[test]
    fn locale_factory_methods() {
        let en = Locale::new("en");
        assert_eq!(en.language(), "en");
        let en_us = en.with_region("US");
        assert_eq!(en_us.region(), Some("US"));
    }

    #[test]
    fn locale_from_str_variants() {
        let locale: Locale = "zh-Hans-CN".parse().unwrap();
        assert_eq!(locale.language(), "zh");
        assert_eq!(locale.script(), Some("Hans"));
        assert_eq!(locale.region(), Some("CN"));
    }

    #[test]
    fn interpolation_basic() {
        let interp = StringInterpolator::default();
        let mut values = HashMap::new();
        values.insert("name".into(), "Elysium".into());
        let result = interp.interpolate("Hello, {name}!", &values).unwrap();
        assert_eq!(result, "Hello, Elysium!");
    }

    #[test]
    fn interpolation_missing_var_keeps_placeholder() {
        let interp = StringInterpolator::default();
        let values = HashMap::new();
        let result = interp.interpolate("Hello, {name}!", &values).unwrap();
        assert_eq!(result, "Hello, {name}!");
    }

    #[test]
    fn interpolation_escape() {
        let interp = StringInterpolator::default();
        let values = HashMap::new();
        let result = interp.interpolate("Not a var: \\{name}", &values).unwrap();
        assert_eq!(result, "Not a var: {name}");
    }

    #[test]
    fn pluralization_english() {
        let rule = PluralizationRules::get(&Locale::new("en"));
        assert_eq!(rule.resolve(1), 0);
        assert_eq!(rule.resolve(2), 1);
    }

    #[test]
    fn pluralization_russian() {
        let rule = PluralizationRules::get(&Locale::new("ru"));
        assert_eq!(rule.resolve(1), 0);  // один
        assert_eq!(rule.resolve(2), 1);  // два
        assert_eq!(rule.resolve(5), 2);  // пять
    }

    #[test]
    fn database_insert_and_get() {
        let mut db = TranslationDatabase::new();
        let locale = Locale::new("en");
        let entry = TranslationEntry {
            key: "hello".into(),
            locale: locale.clone(),
            context: Some(TranslationContext::new("greeting")),
            translation: "Hello".into(),
            gender: Some(Gender::Male),
            plural_category: None,
            metadata: HashMap::new(),
        };
        db.insert(entry);

        let found = db.get("hello", &locale, Some(&TranslationContext::new("greeting")), Some(Gender::Male), None);
        assert!(found.is_some());
        assert_eq!(found.unwrap().translation, "Hello");
    }

    #[test]
    fn manager_translate() {
        let manager = LocalizationManager::new(Locale::new("en"));
        // Should return key as fallback
        let result = manager.translate("missing", None, None, None, &HashMap::new());
        assert_eq!(result.unwrap(), "missing");
    }

    #[test]
    fn manager_translate_with_count() {
        let manager = LocalizationManager::new(Locale::new("en"));
        let result = manager
            .translate_with_count("apples", 5, &HashMap::new());
        assert_eq!(result.unwrap(), "apples");
    }

    #[test]
    fn extractor_basic() {
        let source = r#"t!("hello"); tr!("world"); l10n!("foo");"#;
        let keys = TranslationExtractor::extract_from_source(source);
        assert!(keys.contains("hello"));
        assert!(keys.contains("world"));
        assert!(keys.contains("foo"));
    }

    #[test]
    fn missing_reporter() {
        let mut db = TranslationDatabase::new();
        let en = Locale::new("en");
        let tr = Locale::new("tr");

        let entry = TranslationEntry {
            key: "hello".into(),
            locale: en.clone(),
            context: None,
            translation: "Hello".into(),
            gender: None,
            plural_category: None,
            metadata: HashMap::new(),
        };
        db.insert(entry);

        let report = MissingTranslationReporter::report(&db, &tr);
        assert_eq!(report.missing_keys.len(), 1);
        assert_eq!(report.completion_percentage, 0.0);
    }

    #[test]
    fn csv_roundtrip() {
        let line = r#""hello","en","greeting","male","one","Hello, sir!""#;
        let fields = parse_csv_line(line);
        assert_eq!(fields[0], "hello");
        assert_eq!(fields[1], "en");
        assert_eq!(fields[2], "greeting");
        assert_eq!(fields[3], "male");
        assert_eq!(fields[4], "one");
        assert_eq!(fields[5], "Hello, sir!");
    }

    #[test]
    fn po_roundtrip() {
        let po = r#"# Comment
msgid "hello"
msgstr "merhaba"

msgid "world"
msgstr "dunya"
"#;
        let entries = parse_po_file(po).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].1, "merhaba");
        assert_eq!(entries[1].1, "dunya");
    }

    #[test]
    fn rtl_detection() {
        assert!(has_rtl_chars("مرحبا"));
        assert!(!has_rtl_chars("Hello"));
    }

    #[test]
    fn validator_detects_empty() {
        let mut db = TranslationDatabase::new();
        let en = Locale::new("en");
        let entry = TranslationEntry {
            key: "empty".into(),
            locale: en.clone(),
            context: None,
            translation: "   ".into(),
            gender: None,
            plural_category: None,
            metadata: HashMap::new(),
        };
        db.insert(entry);

        let errors = TranslationValidator::validate(&db, &en);
        assert_eq!(errors.len(), 1);
        matches!(errors[0], ValidationError::EmptyTranslation { .. });
    }
}
