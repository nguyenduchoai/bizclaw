# BizClaw i18n

Internationalization system for BizClaw platform.

## Features

- **Multi-locale support**: Vietnamese (vi), English (en)
- **RTL-ready**: Architecture supports RTL languages
- **Type-safe**: Full Rust type safety
- **Extensible**: Easy to add new locales
- **Zero-cost abstractions**: Efficient design

## Usage

```rust
use bizclaw_i18n::{t, set_locale, get_locale};

fn main() {
    // Set locale
    set_locale("vi").unwrap();

    // Get translation
    let save_text = t("common.save"); // "Lưu"

    // With arguments
    let greeting = bizclaw_i18n::t_args("Hello {name}", &[("name", "World")]);

    // Get current locale
    let current = get_locale();
}
```

## Adding New Locales

```rust
use bizclaw_i18n::{I18n, LocaleConfig, TextDirection, TranslationMap};

fn main() {
    let mut i18n = I18n::new();

    // Register new locale
    i18n.register_locale(LocaleConfig {
        code: "zh".to_string(),
        name: "Chinese".to_string(),
        native_name: "中文".to_string(),
        direction: TextDirection::Ltr,
        date_format: "yyyy/MM/dd".to_string(),
        time_format: "HH:mm".to_string(),
        decimal_separator: ".".to_string(),
        thousands_separator: ",".to_string(),
    });

    // Add translations
    let mut translations = TranslationMap::default();
    translations.insert("common.save".to_string(), "保存".to_string());
    i18n.register_translation("zh", translations);
}
```

## Locale Codes

| Code | Language | Native Name |
|------|----------|-------------|
| vi | Vietnamese | Tiếng Việt |
| en | English | English |
